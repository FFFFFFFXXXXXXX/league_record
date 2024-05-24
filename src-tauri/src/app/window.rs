use tauri::{AppHandle, Manager, Window};

use crate::constants::APP_NAME;
use crate::state::WindowState;

#[derive(Copy, Clone, strum_macros::IntoStaticStr)]
pub enum AppWindow {
    Main,
}

pub trait WindowManager {
    fn open_window(&self, window: AppWindow);

    fn save_window_state(&self, window: &Window);
}

impl WindowManager for AppHandle {
    fn open_window(&self, window: AppWindow) {
        let window: &'static str = window.into();

        log::info!("getting window: {window}");
        if let Some(main) = self.windows().get(window) {
            log::info!("focusing window: {window}");
            _ = main.unminimize();
            _ = main.set_focus();
        } else {
            log::info!("getting window_state: {window}");
            let window_state = self.state::<WindowState>();
            log::info!("got window_state: {window}");

            let size = window_state.get_size();
            log::info!("got window_state size: {window}");
            let window_builder = Window::builder(self, window, tauri::WindowUrl::default())
                .title(APP_NAME)
                .visible(false)
                .min_inner_size(800.0, 450.0)
                .inner_size(size.0, size.1);

            let window_builder = if let Some(position) = window_state.get_position() {
                window_builder.position(position.0, position.1)
            } else {
                window_builder.center()
            };
            log::info!("got window_state position: {window}");

            if let Err(e) = window_builder.build() {
                log::error!("error creating window: {e}");
            }
        }

        log::info!("created window: {window}");
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
