//! System tray integration.
//!
//! Registers the AquaTick system tray icon, context menu, and handles tray
//! action routing to either show/focus the main window or request application
//! shutdown via LifecycleService.

use tauri::{
    menu::{Menu, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager,
};
use crate::services::LifecycleService;

/// Sets up the system tray icon and context menu.
///
/// # Permissions
/// Requires `tray-icon:default` permission or equivalent on the application.
pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_item = MenuItemBuilder::new("Show AquaTick")
        .id("show")
        .build(app)?;

    let quit_item = MenuItemBuilder::new("Quit AquaTick")
        .id("quit")
        .build(app)?;

    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().0.as_str() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    LifecycleService::shutdown(app_handle).await;
                });
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
