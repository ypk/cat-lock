//! Minimal i18n — embedded string table.
//!
//! For v1, we use English strings directly. This module provides a centralized
//! place for all user-facing strings so future localization is straightforward.

/// Application strings — all user-facing text in one place.
pub struct Strings;

#[allow(dead_code)]
impl Strings {
    pub const APP_NAME: &'static str = "CatLock";
    pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
    pub const APP_DESCRIPTION: &'static str =
        "A lightweight system tray app that locks your keyboard and mouse.";

    pub const TRAY_TOOLTIP_LOCKED: &'static str = "CatLock — Locked";
    pub const TRAY_TOOLTIP_UNLOCKED: &'static str = "CatLock — Unlocked";

    pub const OVERLAY_TITLE: &'static str = "CatLock — Input Locked";
    pub const OVERLAY_SHORTCUT_HINT: &'static str = "or press {} to unlock";

    pub const ABOUT_TEXT: &'static str = concat!(
        "CatLock v",
        env!("CARGO_PKG_VERSION"),
        "\n\n",
        "A lightweight system tray app that locks your keyboard ",
        "and mouse to prevent accidental input from cats, kids, or cleaning.\n\n",
        "Cross-platform rewrite — Windows & Linux.\n",
        "App by ypk (github.com/ypk)\n\n",
        "License: GNU GPL-3.0\n",
        "Original concept by hou-physics."
    );
}
