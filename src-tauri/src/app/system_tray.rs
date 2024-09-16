use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItemBuilder};
use tauri::tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{async_runtime, AppHandle, Manager, Wry};

use super::{AppManager, AppWindow, WindowManager};
use crate::constants::{self, menu_item, EXIT_SUCCESS};
use crate::recorder::LeagueRecorder;
use crate::state::{SettingsWrapper, Shutdown, TrayState};

pub trait SystemTrayManager {
    fn init_tray_menu(&self);

    fn set_tray_menu_update_available(&self, update_button: bool);

    fn set_tray_menu_recording(&self, recording: bool);
}

fn handle_system_tray_event(tray_icon: &TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } = event {
        let app_handle = tray_icon.app_handle() as &AppHandle;
        app_handle.open_window(AppWindow::Main);
    }
}

fn handle_system_tray_menu_event(app_handle: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        menu_item::SETTINGS => SettingsWrapper::let_user_edit_settings(app_handle),
        menu_item::OPEN => app_handle.open_window(AppWindow::Main),
        menu_item::QUIT => {
            app_handle
                .webview_windows()
                .into_values()
                .for_each(|window| _ = window.close());

            async_runtime::spawn({
                let app_handle = app_handle.clone();
                async move {
                    app_handle.state::<LeagueRecorder>().stop().await;

                    app_handle.state::<Shutdown>().set();
                    app_handle.exit(EXIT_SUCCESS);
                }
            });
        }
        menu_item::UPDATE => app_handle.update(),
        _ => {}
    }
}

impl SystemTrayManager for AppHandle {
    fn init_tray_menu(&self) {
        TrayIconBuilder::with_id(constants::TRAY_ID)
            .icon(self.default_window_icon().unwrap().clone())
            .title(constants::APP_NAME)
            .tooltip(constants::APP_NAME)
            .on_tray_icon_event(handle_system_tray_event)
            .menu(&create_tray_menu(self))
            .on_menu_event(handle_system_tray_menu_event)
            .menu_on_left_click(false)
            .build(self)
            .unwrap();
    }

    fn set_tray_menu_update_available(&self, update_available: bool) {
        self.state::<TrayState>().set_update_available(update_available);

        // .unwrap on everything because creating the tray-icon is always the same and should never fail
        self.tray_by_id(constants::TRAY_ID)
            .unwrap()
            .set_menu(Some(create_tray_menu(self)))
            .unwrap();
    }

    fn set_tray_menu_recording(&self, recording: bool) {
        self.state::<TrayState>().set_recording(recording);

        self.tray_by_id(constants::TRAY_ID)
            .unwrap()
            .set_menu(Some(create_tray_menu(self)))
            .unwrap();
    }
}

fn create_tray_menu(app_handle: &AppHandle) -> Menu<Wry> {
    let tray_state = app_handle.state::<TrayState>();
    let recording = tray_state.recording();
    let update_available = tray_state.update_available();

    let settings = MenuItemBuilder::new("Settings")
        .id(menu_item::SETTINGS)
        .build(app_handle)
        .unwrap();
    let open = MenuItemBuilder::new("Open")
        .id(menu_item::OPEN)
        .build(app_handle)
        .unwrap();
    let quit = MenuItemBuilder::new("Quit")
        .id(menu_item::QUIT)
        .build(app_handle)
        .unwrap();
    let update = MenuItemBuilder::new("Update")
        .id(menu_item::UPDATE)
        .build(app_handle)
        .unwrap();

    let tray_menu = if update_available {
        MenuBuilder::new(app_handle)
            .check(menu_item::RECORDING, "Recording")
            .separator()
            .item(&settings)
            .item(&open)
            .item(&quit)
            .separator()
            .item(&update)
    } else {
        MenuBuilder::new(app_handle)
            .check(menu_item::RECORDING, "Recording")
            .separator()
            .item(&settings)
            .item(&open)
            .item(&quit)
    }
    .build()
    .unwrap();

    let recording_item = tray_menu.get(menu_item::RECORDING).unwrap();
    recording_item
        .as_check_menuitem()
        .unwrap()
        .set_checked(recording)
        .unwrap();
    recording_item.as_check_menuitem().unwrap().set_enabled(false).unwrap();

    tray_menu
}
