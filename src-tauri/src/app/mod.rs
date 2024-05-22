mod events;
mod manager;
mod recordings;
mod system_tray;
mod window;

pub use events::{AppEvent, EventManager};
pub use manager::AppManager;
pub use recordings::RecordingManager;
pub use system_tray::SystemTrayManager;
pub use window::WindowManager;

pub fn process_app_event(app_handle: &tauri::AppHandle, event: tauri::RunEvent) {
    use tauri::{Manager, RunEvent, WindowEvent};
    use window::WindowManager;

    match event {
        RunEvent::WindowEvent {
            event: WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            use crate::constants::window;

            // triggered on window close (X Button)
            if let Some(window) = app_handle.get_window(window::MAIN) {
                app_handle.save_window_state(&window);
            }
            api.prevent_close();
        }
        RunEvent::ExitRequested { api, .. } => {
            // triggered when no windows remain
            // prevent complete shutdown of program so that just the tray icon stays
            api.prevent_exit();
        }
        _ => {}
    }
}
