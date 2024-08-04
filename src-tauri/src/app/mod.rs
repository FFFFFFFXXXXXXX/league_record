mod event;
mod manager;
mod recordings;
mod system_tray;
mod window;

pub use event::{AppEvent, EventManager};
pub use manager::AppManager;
pub use recordings::{action, RecordingManager};
pub use system_tray::SystemTrayManager;
pub use window::{AppWindow, WindowManager};

pub fn process_app_event(app_handle: &tauri::AppHandle, event: tauri::RunEvent) {
    use crate::state::Shutdown;
    use tauri::{Manager, RunEvent, WindowEvent};
    use window::WindowManager;

    match event {
        RunEvent::WindowEvent {
            event: WindowEvent::CloseRequested { .. },
            ..
        } => {
            // triggered on window close (X Button)
            if let Some(window) = app_handle.get_webview_window(AppWindow::Main.into()) {
                app_handle.save_window_state(&window);
            }
        }
        RunEvent::ExitRequested { api, .. } => {
            // triggered when no windows remain
            // prevent complete shutdown of program so that just the tray icon stays
            if !app_handle.state::<Shutdown>().get() {
                api.prevent_exit();
            }
        }
        _ => {}
    }
}
