use tauri::{AppHandle, CustomMenuItem, Manager, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem};

use super::{AppManager, AppWindow, WindowManager};
use crate::constants::{exit, menu_item};
use crate::recorder::LeagueRecorder;
use crate::state::SettingsWrapper;

pub trait SystemTrayManager {
    fn handle_system_tray_event(&self, event: SystemTrayEvent);

    fn set_system_tray(&self, update_button: bool);

    fn set_tray_menu_recording_status(&self, recording: bool);
}

impl SystemTrayManager for AppHandle {
    fn handle_system_tray_event(&self, event: SystemTrayEvent) {
        match event {
            SystemTrayEvent::DoubleClick { .. } => self.open_window(AppWindow::Main),
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                menu_item::SETTINGS => SettingsWrapper::let_user_edit_settings(self),
                menu_item::OPEN => self.open_window(AppWindow::Main),
                menu_item::QUIT => {
                    self.windows().into_values().for_each(|window| _ = window.close());
                    self.state::<LeagueRecorder>().stop();
                    self.exit(exit::SUCCESS);
                }
                menu_item::UPDATE => self.update(),
                _ => {}
            },
            _ => {}
        }
    }

    fn set_system_tray(&self, update_button: bool) {
        let tray_menu = if update_button {
            SystemTrayMenu::new()
                .add_item(CustomMenuItem::new(menu_item::RECORDING, "Recording").disabled())
                .add_native_item(SystemTrayMenuItem::Separator)
                .add_item(CustomMenuItem::new(menu_item::SETTINGS, "Settings"))
                .add_item(CustomMenuItem::new(menu_item::OPEN, "Open"))
                .add_item(CustomMenuItem::new(menu_item::QUIT, "Quit"))
                .add_native_item(SystemTrayMenuItem::Separator)
                .add_item(CustomMenuItem::new(menu_item::UPDATE, "Update"))
        } else {
            SystemTrayMenu::new()
                .add_item(CustomMenuItem::new(menu_item::RECORDING, "Recording").disabled())
                .add_native_item(SystemTrayMenuItem::Separator)
                .add_item(CustomMenuItem::new(menu_item::SETTINGS, "Settings"))
                .add_item(CustomMenuItem::new(menu_item::OPEN, "Open"))
                .add_item(CustomMenuItem::new(menu_item::QUIT, "Quit"))
        };

        if let Err(e) = self.tray_handle().set_menu(tray_menu.clone()) {
            log::error!("failed to update tray menu: {e}");
            self.exit(exit::ERROR);
        }
    }

    fn set_tray_menu_recording_status(&self, recording: bool) {
        let item = self.tray_handle().get_item(menu_item::RECORDING);
        // set selected only updates the tray menu when open if the menu item is enabled
        _ = item.set_enabled(true);
        _ = item.set_selected(recording);
        _ = item.set_enabled(false);
    }
}
