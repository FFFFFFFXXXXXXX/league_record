use tauri::{AppHandle, Manager, Window};

use crate::constants::{window, APP_NAME};
use crate::state::WindowState;

pub trait WindowManager {
    fn create_main_window(&self);

    fn save_window_state(&self, window: &Window);
}

impl WindowManager for AppHandle {
    fn create_main_window(&self) {
        if let Some(main) = self.windows().get(window::MAIN) {
            _ = main.unminimize();
            _ = main.set_focus();
        } else {
            let window_state = self.state::<WindowState>();

            let builder = Window::builder(self, window::MAIN, tauri::WindowUrl::default());

            let size = window_state.get_size();
            let position = window_state.get_position();
            let window = builder
                .title(APP_NAME)
                .inner_size(size.0, size.1)
                .position(position.0, position.1)
                .min_inner_size(800.0, 450.0)
                .visible(false);

            if let Err(e) = window.build() {
                log::error!("error creating window: {e}");
            }
        }
    }

    fn save_window_state(&self, window: &Window) {
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
