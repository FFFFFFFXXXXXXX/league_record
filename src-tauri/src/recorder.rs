use std::{
    sync::mpsc::{channel, RecvTimeoutError},
    thread,
    time::Duration,
};

use tauri::{
    api::process::{Command, CommandEvent, TerminatedPayload},
    AppHandle, Manager, Runtime,
};
use tokio::sync::mpsc::error::TryRecvError;
#[cfg(target_os = "windows")]
use windows::{
    core::PCSTR,
    Win32::{
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::{FindWindowA, GetClientRect},
    },
};

use crate::state::Settings;

const WINDOW_TITLE: &str = "League of Legends (TM) Client";
const WINDOW_CLASS: &str = "RiotWindowClass";

fn set_recording_tray_item<R: Runtime>(app_handle: &AppHandle<R>, recording: bool) {
    let item = app_handle.tray_handle().get_item("rec");
    // set selected only updates the tray menu when open if the menu item is enabled
    _ = item.set_enabled(true);
    _ = item.set_selected(recording);
    _ = item.set_enabled(false);
}

#[cfg(target_os = "windows")]
fn get_lol_window() -> Option<HWND> {
    let mut window_title = WINDOW_TITLE.to_owned();
    window_title.push('\0'); // null terminate
    let mut window_class = WINDOW_CLASS.to_owned();
    window_class.push('\0'); // null terminate

    let title = PCSTR(window_title.as_ptr());
    let class = PCSTR(window_class.as_ptr());

    let hwnd = unsafe { FindWindowA(class, title) };
    if hwnd.is_invalid() {
        return None;
    }
    Some(hwnd)
}

#[cfg(target_os = "windows")]
fn get_window_size(hwnd: HWND) -> Result<(u32, u32), ()> {
    let mut rect = RECT::default();
    let ok = unsafe { GetClientRect(hwnd, &mut rect as _).as_bool() };
    if ok && rect.right > 0 && rect.bottom > 0 {
        Ok((rect.right as u32, rect.bottom as u32))
    } else {
        Err(())
    }
}

pub fn start<R: Runtime>(app_handle: AppHandle<R>) {
    thread::spawn(move || {
        // send stop to channel on "shutdown" event
        let (tx, rx) = channel::<_>();
        app_handle.once_global("shutdown_recorder", move |_| {
            _ = tx.send(());
        });

        // get owned copy of settings so we can change window_size
        let settings = app_handle.state::<Settings>();
        let debug_log = settings.debug_log();
        let mut ingame = false;

        loop {
            // if window exists && we get data from the API && we are not recording => start recording
            match get_lol_window() {
                Some(hwnd) if !ingame => {
                    ingame = true;

                    if debug_log {
                        println!("LoL Window found");
                    }

                    let (mut rcv, mut child) = Command::new_sidecar("lol_rec")
                        .expect("missing lol_rec")
                        .spawn()
                        .expect("error spawing lol_rec");

                    // write serialized config to child
                    let size = get_window_size(hwnd).unwrap_or_default();
                    _ = child.write(settings.create_lol_rec_cfg(size).as_bytes());

                    if debug_log {
                        println!("lol_rec started");
                    }

                    // receive messages
                    loop {
                        match rcv.try_recv() {
                            Ok(CommandEvent::Stdout(line) | CommandEvent::Stderr(line) | CommandEvent::Error(line)) => {
                                if line == "recording started" {
                                    set_recording_tray_item(&app_handle, true);
                                }
                                if debug_log {
                                    println!("lol_rec: {}", line);
                                }
                            }
                            result @ (Ok(CommandEvent::Terminated(_)) | Err(TryRecvError::Disconnected)) => {
                                set_recording_tray_item(&app_handle, false);
                                _ = app_handle.emit_all("new_recording", ());

                                if debug_log {
                                    match result {
                                        Ok(CommandEvent::Terminated(TerminatedPayload {
                                            code: Some(exitcode),
                                            ..
                                        })) => {
                                            println!("lol_rec: Exitcode: {}", exitcode)
                                        }
                                        Err(TryRecvError::Disconnected) => println!("lol_rec: Exitcode: ?"),
                                        _ => println!("lol_rec: Exitcode: -1"),
                                    }
                                }
                                break;
                            }
                            _ => {}
                        }

                        // break if stop event received or sender disconnected
                        match rx.recv_timeout(Duration::from_secs(1)) {
                            Ok(_) | Err(RecvTimeoutError::Disconnected) => {
                                if child.write(b"stop").is_err() {
                                    _ = child.kill();
                                }
                                break;
                            }
                            Err(RecvTimeoutError::Timeout) => {}
                        }
                    }

                    if debug_log {
                        println!(); //formatting: empty line after lol_rec output
                    }
                }
                None => ingame = false,
                _ => {}
            }

            // break if stop event received or sender disconnected
            match rx.recv_timeout(Duration::from_secs(3)) {
                Ok(_) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
        }

        app_handle.exit(0);
    });
}
