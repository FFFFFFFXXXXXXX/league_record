use std::{
    sync::mpsc::{channel, RecvTimeoutError},
    thread,
    time::Duration,
};

use tauri::{
    api::process::{Command, CommandEvent},
    AppHandle, Manager, Runtime,
};
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

const SLEEP_SECS: u64 = 5;

fn set_recording_tray_item<R: Runtime>(app_handle: &AppHandle<R>, recording: bool) {
    let item = app_handle.tray_handle().get_item("rec");
    // set selected only updates the tray menu when open if the menu item is enabled
    let _ = item.set_enabled(true);
    let _ = item.set_selected(recording);
    let _ = item.set_enabled(false);
}

#[cfg(target_os = "windows")]
fn get_window() -> Result<HWND, ()> {
    let mut window_title = WINDOW_TITLE.to_owned();
    window_title.push('\0'); // null terminate
    let mut window_class = WINDOW_CLASS.to_owned();
    window_class.push('\0'); // null terminate

    let title = PCSTR(window_title.as_ptr());
    let class = PCSTR(window_class.as_ptr());

    let hwnd = unsafe { FindWindowA(class, title) };
    if hwnd.is_invalid() {
        return Err(());
    }
    Ok(hwnd)
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
            let _ = tx.send(());
        });

        // get owned copy of settings so we can change window_size
        let settings = app_handle.state::<Settings>();
        let debug_log = settings.debug_log();

        let mut recording = false;
        let mut lol_rec = None;
        loop {
            // if window exists && we get data from the API && we are not recording => start recording
            if let Ok(hwnd) = get_window() {
                if !recording {
                    if debug_log {
                        println!("LoL Window found");
                    }

                    let (mut rcv, mut child) = Command::new_sidecar("lol_rec")
                        .expect("missing lol_rec")
                        .spawn()
                        .expect("error spawing lol_rec");

                    if debug_log {
                        println!("lol_rec started");

                        // log received messages
                        thread::spawn({
                            let app_handle = app_handle.clone();
                            move || {
                                while let Some(line) = rcv.blocking_recv() {
                                    println!(
                                        "lol_rec: {}",
                                        match line {
                                            CommandEvent::Stderr(line)
                                            | CommandEvent::Stdout(line)
                                            | CommandEvent::Error(line) => line,
                                            CommandEvent::Terminated(payload) => {
                                                let _ = app_handle.emit_all("new_recording", ());
                                                format!("Exitcode: {}", payload.code.unwrap_or(-1))
                                            }
                                            _ => String::from("unknown event"),
                                        }
                                    );
                                }
                                println!(); //formatting: empty line after lol_rec output
                            }
                        });
                    }

                    // write serialized config to child
                    let size = get_window_size(hwnd).unwrap_or_default();
                    if let Ok(cfg) = settings.create_lol_rec_cfg(size) {
                        let _ = child.write(cfg.as_bytes());
                        lol_rec = Some(child);

                        set_recording_tray_item(&app_handle, true);
                        recording = true;
                    }
                }

                // if we are recording and we the window doesn't exist anymore => stop recording
            } else if recording {
                set_recording_tray_item(&app_handle, false);
                recording = false;

                if debug_log {
                    println!("LoL window closed: waiting for next game");
                }
            }

            // delay SLEEP_MS milliseconds waiting for stop event
            // break if stop event received or sender disconnected
            match rx.recv_timeout(Duration::from_secs(SLEEP_SECS)) {
                Ok(_) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
        }

        if let Some(mut lol_rec) = lol_rec {
            if lol_rec.write(b"stop").is_err() {
                let _ = lol_rec.kill();
            }
        }
        app_handle.exit(0);
    });
}
