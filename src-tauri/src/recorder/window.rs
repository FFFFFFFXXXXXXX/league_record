use anyhow::{anyhow, Result};
use libobs_recorder::settings::Resolution;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

pub const WINDOW_TITLE: &str = "League of Legends (TM) Client";
pub const WINDOW_CLASS: &str = "RiotWindowClass";
pub const WINDOW_PROCESS: &str = "League of Legends.exe";

pub fn get_lol_window() -> Option<HWND> {
    use windows::{core::PCSTR, Win32::UI::WindowsAndMessaging::FindWindowA};

    let mut window_title = WINDOW_TITLE.to_owned();
    window_title.push('\0'); // null terminate
    let mut window_class = WINDOW_CLASS.to_owned();
    window_class.push('\0'); // null terminate

    let title = PCSTR(window_title.as_ptr());
    let class = PCSTR(window_class.as_ptr());

    let hwnd = unsafe { FindWindowA(class, title) };
    if hwnd.0 == 0 {
        None
    } else {
        Some(hwnd)
    }
}

pub fn get_window_size(hwnd: HWND) -> Result<Resolution> {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect as _) }?;
    // when the LoL ingame window is created windows reports the size as (1, 1) for a short time
    // this is only the case when the DPI-AwarenessContent is set to PER-MONITOR and PER-MONITOR(V2)
    // which are necessary to the the properly scaled screen resolution for hidpi screens
    if rect.right > 1 && rect.bottom > 1 {
        Ok(Resolution::new(rect.right as u32, rect.bottom as u32))
    } else {
        Err(anyhow!("invalid window size"))
    }
}
