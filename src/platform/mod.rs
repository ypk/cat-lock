//! Platform abstraction layer — trait definitions for OS-specific functionality.
//!
//! Each platform (Windows, Linux) provides concrete implementations behind
//! these traits, selected at compile time via `cfg` attributes.

/// Intercepts and blocks all keyboard/mouse input system-wide.
pub trait InputInterceptor {
    /// Install system-wide input hooks. All keyboard and mouse events
    /// will be silently consumed (blocked) except for the unlock hotkey.
    fn block_input(&mut self) -> Result<(), String>;

    /// Remove input hooks, restoring normal keyboard/mouse operation.
    fn unblock_input(&mut self) -> Result<(), String>;

    /// Set whether auto-detect cat (PawSense) is enabled.
    fn set_auto_detect_cat(&mut self, _enabled: bool) {}

    /// Returns true if input is currently being blocked.
    #[allow(dead_code)]
    fn is_blocking(&self) -> bool;
}

/// Manages a fullscreen overlay window that covers all monitors.
pub trait OverlayWindow {
    /// Show the overlay on all screens. If `privacy_mode` is true,
    /// the overlay is fully opaque black; otherwise semi-transparent.
    fn show(&mut self, privacy_mode: bool, hotkey_str: &str) -> Result<(), String>;

    /// Hide and destroy all overlay windows.
    fn hide(&mut self) -> Result<(), String>;

    /// Returns true if the overlay is currently visible.
    #[allow(dead_code)]
    fn is_visible(&self) -> bool;
}

/// Prevents the system from sleeping while input is locked.
pub trait PowerInhibitor {
    /// Assert that the system should not sleep.
    fn inhibit_sleep(&mut self) -> Result<(), String>;

    /// Release the sleep inhibition.
    fn allow_sleep(&mut self) -> Result<(), String>;
}

// Platform-specific module selection
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

pub mod slint_overlay;

/// Create platform-specific input interceptor.
pub fn create_input_interceptor() -> Box<dyn InputInterceptor> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::input_hook::Win32InputHook::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::input_grab::LinuxInputGrab::new())
    }
}

/// Create platform-specific overlay window.
pub fn create_overlay() -> Box<dyn OverlayWindow> {
    Box::new(slint_overlay::SlintOverlay::new())
}

/// Create platform-specific power inhibitor.
pub fn create_power_inhibitor() -> Box<dyn PowerInhibitor> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::power::Win32PowerInhibitor::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::power::LinuxPowerInhibitor::new())
    }
}
