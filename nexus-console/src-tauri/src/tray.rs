//! System tray icon and menu setup for the nexus-console.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tracing::info;

/// Set up the system tray icon with a context menu.
///
/// Menu items: Connect, Disconnect, (separator), Quit.
/// Left-click on the tray icon opens/focuses the main window.
pub fn setup_tray(app: &tauri::App) -> anyhow::Result<()> {
    let connect = MenuItem::with_id(app, "connect", "Connect", true, None::<&str>)?;
    let disconnect = MenuItem::with_id(app, "disconnect", "Disconnect", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&connect, &disconnect, &separator, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
            tauri::image::Image::new(&[], 0, 0)
        }))
        .menu(&menu)
        .on_tray_icon_event(|tray_icon, event| {
            use tauri::tray::TrayIconEvent;
            if let TrayIconEvent::Click { .. } = event {
                if let Some(window) = tray_icon.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .on_menu_event(|app_handle, event| {
            match event.id().as_ref() {
                "connect" => {
                    info!("tray: Connect clicked");
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "disconnect" => {
                    info!("tray: Disconnect clicked");
                }
                "quit" => {
                    info!("tray: Quit clicked");
                    app_handle.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}
