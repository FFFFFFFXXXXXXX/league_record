use std::{
    sync::mpsc::{channel, RecvTimeoutError},
    thread,
    time::Duration,
};

use libobs_recorder::{
    settings::{RateControl, Resolution, Size, Window},
    Recorder, RecorderSettings,
};
use tauri::{AppHandle, Manager, Runtime};
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
const WINDOW_PROCESS: &str = "League of Legends.exe";

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
fn get_window_size(hwnd: HWND) -> Result<Size, ()> {
    let mut rect = RECT::default();
    let ok = unsafe { GetClientRect(hwnd, &mut rect as _).as_bool() };
    if ok && rect.right > 0 && rect.bottom > 0 {
        Ok(Size::new(rect.right as u32, rect.bottom as u32))
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
        let settings_state = app_handle.state::<Settings>();
        let debug_log = settings_state.debug_log();

        let mut recorder: Option<Recorder> = None;

        loop {
            match get_lol_window() {
                Some(window_handle) if recorder.is_none() => {
                    if debug_log {
                        println!("LoL Window found");
                    }

                    let Ok(mut rec) = Recorder::new(None, None, None) else {
                        continue;
                    };

                    let mut settings = RecorderSettings::new();
                    settings.set_window(Window::new(
                        WINDOW_TITLE,
                        Some(WINDOW_CLASS.into()),
                        Some(WINDOW_PROCESS.into()),
                    ));
                    settings.set_input_size(
                        get_window_size(window_handle).unwrap_or_else(|_| Resolution::_1080p.get_size()),
                    );
                    settings.set_output_resolution(settings_state.get_output_resolution());
                    settings.set_framerate(settings_state.get_framerate());
                    settings.set_rate_control(RateControl::CQP(settings_state.get_encoding_quality()));
                    settings.record_audio(settings_state.get_audio_source());
                    let mut output_path = settings_state.get_recordings_path();
                    output_path.push(format!(
                        "{}",
                        chrono::Local::now().format(&settings_state.get_filename_format())
                    ));
                    settings.set_output_path(output_path.to_str().expect("error converting video_path to &str"));
                    rec.configure(&settings);

                    if rec.start_recording() {
                        recorder = Some(rec);
                        set_recording_tray_item(&app_handle, true);
                    }
                }
                None => {
                    if let Some(mut rec) = recorder.take() {
                        rec.stop_recording();
                        _ = rec.shutdown();
                    };
                }
                _ => { /* do nothing while recording */ }
            }

            // break if stop event received or sender disconnected
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(_) | Err(RecvTimeoutError::Disconnected) => {
                    // stop recorder if running
                    if let Some(mut rec) = recorder {
                        rec.stop_recording();
                        _ = rec.shutdown();
                    }
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }

        app_handle.trigger_global("recorder_shutdown", None);
    });
}
