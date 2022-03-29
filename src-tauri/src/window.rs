// stolen from https://github.com/robmikh/screenshot-rs : adjusted for version 0.34.0 of windows-rs

use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_SHELL};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetAncestor, GetClassNameW, GetShellWindow, GetWindowLongW, GetWindowTextW,
    IsWindowVisible, GA_ROOT, GWL_EXSTYLE, GWL_STYLE, WS_DISABLED, WS_EX_TOOLWINDOW,
};

const LOLCLASS: &str = "RiotWindowClass";
const LOLWNAME: &str = "League of Legends (TM) Client";

#[derive(Debug, Clone)]
struct WindowInfo {
    handle: HWND,
    title: String,
    class_name: String,
}

struct Window {
    window: Option<HWND>,
}

pub fn find_league_window() -> Option<HWND> {
    let state = Box::into_raw(Box::new(Window { window: None }));
    let state = unsafe {
        EnumWindows(Some(enum_window), LPARAM(state as isize));
        Box::from_raw(state)
    };
    state.window
}

extern "system" fn enum_window(window: HWND, state: LPARAM) -> BOOL {
    unsafe {
        let state = Box::leak(Box::from_raw(state.0 as *mut Window));

        let wi = WindowInfo::new(window);
        if wi.is_capturable_window() && wi.class_name == LOLCLASS && wi.title == LOLWNAME {
            dbg!(wi.clone());
            state.window = Some(wi.handle);
            return false.into();
        }
    }
    true.into()
}

impl WindowInfo {
    fn new(window_handle: HWND) -> Self {
        unsafe {
            let mut title = [0u16; 512];
            GetWindowTextW(window_handle, title.as_mut());
            let mut title = String::from_utf16_lossy(&title);
            truncate_to_first_null_char(&mut title);

            let mut class_name = [0u16; 512];
            GetClassNameW(window_handle, class_name.as_mut());
            let mut class_name = String::from_utf16_lossy(&class_name);
            truncate_to_first_null_char(&mut class_name);

            Self {
                handle: window_handle,
                title,
                class_name,
            }
        }
    }

    fn is_capturable_window(&self) -> bool {
        unsafe {
            if self.title.is_empty()
                || self.handle == GetShellWindow()
                || IsWindowVisible(self.handle).as_bool() == false
                || GetAncestor(self.handle, GA_ROOT) != self.handle
            {
                return false;
            }

            let style = GetWindowLongW(self.handle, GWL_STYLE);
            if style & (WS_DISABLED.0 as i32) == 1 {
                return false;
            }

            // No tooltips
            let ex_style = GetWindowLongW(self.handle, GWL_EXSTYLE);
            if ex_style & (WS_EX_TOOLWINDOW.0 as i32) == 1 {
                return false;
            }

            // Check to see if the self is cloaked if it's a UWP
            if self.class_name == "Windows.UI.Core.CoreWindow"
                || self.class_name == "ApplicationFrameWindow"
            {
                let mut cloaked: u32 = 0;
                if DwmGetWindowAttribute(
                    self.handle,
                    DWMWA_CLOAKED,
                    &mut cloaked as *mut _ as *mut _,
                    std::mem::size_of::<u32>() as u32,
                )
                .is_ok()
                    && cloaked == DWM_CLOAKED_SHELL
                {
                    return false;
                }
            }

            // Unfortunate work-around. Not sure how to avoid this.
            if self.is_known_blocked_window() {
                return false;
            }
        }
        true
    }

    fn is_known_blocked_window(&self) -> bool {
        // Task View
        self.matches_title_and_class_name("Task View", "Windows.UI.Core.CoreWindow") ||
        // XAML Islands
        self.matches_title_and_class_name("DesktopWindowXamlSource", "Windows.UI.Core.CoreWindow") ||
        // XAML Popups
        self.matches_title_and_class_name("PopupHost", "Xaml_WindowedPopupClass")
    }

    fn matches_title_and_class_name(&self, title: &str, class_name: &str) -> bool {
        self.title == title && self.class_name == class_name
    }
}

fn truncate_to_first_null_char(input: &mut String) {
    if let Some(index) = input.find('\0') {
        input.truncate(index);
    }
}
