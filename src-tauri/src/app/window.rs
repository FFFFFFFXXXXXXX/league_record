use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow};

use crate::constants::APP_NAME;
use crate::state::WindowState;

#[derive(Copy, Clone, strum_macros::IntoStaticStr)]
pub enum AppWindow {
    Main,
}

pub trait WindowManager {
    fn open_window(&self, window: AppWindow);

    fn save_window_state(&self, window: &WebviewWindow);
}

impl WindowManager for AppHandle {
    fn open_window(&self, window: AppWindow) {
        let window: &'static str = window.into();

        if let Some(main) = self.webview_windows().get(window) {
            _ = main.unminimize();
            _ = main.set_focus();
        } else {
            let window_state = self.state::<WindowState>();

            let size = window_state.get_size();
            let window_builder = WebviewWindow::builder(self, window, WebviewUrl::default())
                .title(APP_NAME)
                .visible(false)
                .min_inner_size(800.0, 450.0)
                .inner_size(size.0, size.1);

            let window_builder = if let Some(position) = window_state.get_position() {
                window_builder.position(position.0, position.1)
            } else {
                window_builder.center()
            };

            if let Err(e) = window_builder.build() {
                log::error!("error creating window: {e}");
            }
        }
    }

    fn save_window_state(&self, window: &WebviewWindow) {
        let scale_factor = match window.scale_factor() {
            Ok(scale_factor) => scale_factor,
            Err(e) => {
                log::error!("Error getting window scale factor: {e}");
                return;
            }
        };

        let window_state = self.state::<WindowState>();
        if let Ok(size) = window.inner_size() {
            let size = ((size.width as f64) / scale_factor, (size.height as f64) / scale_factor);
            window_state.set_size(size);
        }
        if let Ok(position) = window.outer_position() {
            let position = ((position.x as f64) / scale_factor, (position.y as f64) / scale_factor);
            window_state.set_position(position);
        }
    }
}
