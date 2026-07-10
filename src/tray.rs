//! System tray icon and menu — powered by the `tray-icon` crate.
//!
//! Provides a system tray icon with a context menu containing:
//! - Lock / Unlock toggle
//! - Privacy mode toggle
//! - About info
//! - Quit

use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Menu item IDs for event handling.
pub struct TrayMenuIds {
    pub toggle_privacy: MenuItem,
    pub toggle_console: MenuItem,
    pub toggle_auto_detect: MenuItem,
    pub toggle_notifications: MenuItem,
    pub about: MenuItem,
    pub quit: MenuItem,
    // Keep a reference to info so we can update its text if hotkey changes
    pub info_hotkey: MenuItem,
}

/// Build the system tray icon and menu. Returns the tray icon handle and menu item IDs.
pub fn create_tray(
    is_locked: bool,
    is_privacy: bool,
    show_console: bool,
    auto_detect_cat: bool,
    notifications_enabled: bool,
    hotkey_str: &str,
) -> Result<(TrayIcon, TrayMenuIds), String> {
    let info_text = format!("Lock Shortcut: {}", hotkey_str);
    let info_hotkey = MenuItem::new(&info_text, false, None); // Disabled!

    let privacy_text = if is_privacy {
        "Privacy Mode: ON"
    } else {
        "Privacy Mode: OFF"
    };
    let toggle_privacy = MenuItem::new(privacy_text, true, None);

    let console_text = if show_console {
        "Show Console: ON"
    } else {
        "Show Console: OFF"
    };
    let toggle_console = MenuItem::new(console_text, true, None);

    let auto_detect_text = if auto_detect_cat {
        "Auto-Detect Cat (PawSense): ON"
    } else {
        "Auto-Detect Cat (PawSense): OFF"
    };
    let toggle_auto_detect = MenuItem::new(auto_detect_text, true, None);

    let notif_text = if notifications_enabled {
        "Desktop Notifications: ON"
    } else {
        "Desktop Notifications: OFF"
    };
    let toggle_notifications = MenuItem::new(notif_text, true, None);

    let settings_submenu = Submenu::new("Settings", true);
    settings_submenu.append(&toggle_privacy).map_err(|e| e.to_string())?;
    settings_submenu.append(&toggle_console).map_err(|e| e.to_string())?;
    settings_submenu.append(&toggle_auto_detect).map_err(|e| e.to_string())?;
    settings_submenu.append(&toggle_notifications).map_err(|e| e.to_string())?;

    let about = MenuItem::new("About CatLock", true, None);
    let quit = MenuItem::new("Quit", true, None);

    let menu = Menu::new();
    menu.append(&info_hotkey).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator()).map_err(|e| e.to_string())?;
    menu.append(&settings_submenu).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator()).map_err(|e| e.to_string())?;
    menu.append(&about).map_err(|e| e.to_string())?;
    menu.append(&PredefinedMenuItem::separator()).map_err(|e| e.to_string())?;
    menu.append(&quit).map_err(|e| e.to_string())?;

    // Create a simple icon (16x16 solid color — we'll embed a real icon later)
    let icon = create_default_icon(is_locked)?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(if is_locked {
            "CatLock - Locked"
        } else {
            "CatLock - Unlocked"
        })
        .with_icon(icon)
        .build()
        .map_err(|e| format!("Failed to create tray icon: {}", e))?;

    let ids = TrayMenuIds {
        toggle_privacy,
        toggle_console,
        toggle_auto_detect,
        toggle_notifications,
        about,
        quit,
        info_hotkey,
    };

    log::info!("System tray created");
    Ok((tray, ids))
}

/// Update the tray menu text and tooltip based on current state.
pub fn update_tray(
    tray: &TrayIcon,
    ids: &TrayMenuIds,
    is_locked: bool,
    is_privacy: bool,
    show_console: bool,
    auto_detect_cat: bool,
    notifications_enabled: bool,
    hotkey_str: &str,
) {
    let info_text = format!("Lock Shortcut: {}", hotkey_str);
    let privacy_text = if is_privacy {
        "Privacy Mode: ON"
    } else {
        "Privacy Mode: OFF"
    };
    let console_text = if show_console {
        "Show Console: ON"
    } else {
        "Show Console: OFF"
    };
    let auto_detect_text = if auto_detect_cat {
        "Auto-Detect Cat (PawSense): ON"
    } else {
        "Auto-Detect Cat (PawSense): OFF"
    };
    let notif_text = if notifications_enabled {
        "Desktop Notifications: ON"
    } else {
        "Desktop Notifications: OFF"
    };

    ids.info_hotkey.set_text(&info_text);
    ids.toggle_privacy.set_text(privacy_text);
    ids.toggle_console.set_text(console_text);
    ids.toggle_auto_detect.set_text(auto_detect_text);
    ids.toggle_notifications.set_text(notif_text);

    let _ = tray.set_tooltip(Some(if is_locked {
        "CatLock - Locked"
    } else {
        "CatLock - Unlocked"
    }));

    // Update icon
    if let Ok(icon) = create_default_icon(is_locked) {
        let _ = tray.set_icon(Some(icon));
    }
}

/// Create the icon by parsing the embedded PNGs.
fn create_default_icon(is_locked: bool) -> Result<Icon, String> {
    let bytes = if is_locked {
        include_bytes!("../assets/active.png").as_slice()
    } else {
        include_bytes!("../assets/inactive.png").as_slice()
    };
    
    let image = image::load_from_memory(bytes)
        .map_err(|e| format!("Failed to parse icon: {}", e))?
        .into_rgba8();
        
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    
    Icon::from_rgba(rgba, width, height)
        .map_err(|e| format!("Icon creation failed: {}", e))
}
