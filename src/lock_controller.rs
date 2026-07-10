//! Lock controller — core state machine managing lock/unlock transitions.
//!
//! This is the central coordinator that orchestrates:
//! - Input blocking (keyboard + mouse hooks)
//! - Overlay window display
//! - Sleep prevention
//!
//! It is platform-agnostic; all OS-specific behavior is delegated
//! to the trait implementations injected at construction.

use crate::platform::{InputInterceptor, OverlayWindow, PowerInhibitor};
use crate::settings::Settings;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global atomic flag for ultra-fast lock state checks from hook callbacks.
/// Hook callbacks need sub-microsecond access and cannot take locks.
pub static IS_LOCKED: AtomicBool = AtomicBool::new(false);

/// Core lock/unlock state machine.
pub struct LockController {
    input: Box<dyn InputInterceptor>,
    overlay: Box<dyn OverlayWindow>,
    power: Box<dyn PowerInhibitor>,
    locked: bool,
}

impl LockController {
    /// Create a new lock controller with platform-specific implementations.
    pub fn new(
        input: Box<dyn InputInterceptor>,
        overlay: Box<dyn OverlayWindow>,
        power: Box<dyn PowerInhibitor>,
    ) -> Self {
        Self {
            input,
            overlay,
            power,
            locked: false,
        }
    }

    /// Update PawSense state.
    pub fn set_auto_detect_cat(&mut self, enabled: bool) {
        self.input.set_auto_detect_cat(enabled);
    }

    /// Toggle between locked and unlocked states.
    pub fn toggle(&mut self, settings: &Settings) {
        if self.locked {
            self.unlock();
            if settings.notifications_enabled {
                crate::notify::send("CatLock Unlocked", "Keyboard and mouse are restored.");
            }
        } else {
            self.lock(settings);
            if settings.notifications_enabled {
                crate::notify::send("CatLock Locked", &format!("Press {} to unlock.", settings.hotkey_display_string()));
            }
        }
    }

    /// Enter locked state: block input, show overlay, prevent sleep.
    pub fn lock(&mut self, settings: &Settings) {
        if self.locked {
            log::warn!("Already locked, ignoring lock request");
            return;
        }

        log::info!("Locking input...");

        // Order matters: show overlay first so the user sees feedback,
        // then block input, then inhibit sleep.
        if let Err(e) = self.overlay.show(settings.privacy_mode, &settings.hotkey_display_string()) {
            log::error!("Failed to show overlay: {}", e);
            return;
        }

        if let Err(e) = self.input.block_input() {
            log::error!("Failed to block input: {}", e);
            // Roll back overlay
            let _ = self.overlay.hide();
            return;
        }

        if let Err(e) = self.power.inhibit_sleep() {
            log::warn!("Failed to inhibit sleep (non-fatal): {}", e);
            // Continue — sleep prevention is nice-to-have, not critical
        }

        self.locked = true;
        IS_LOCKED.store(true, Ordering::SeqCst);
        log::info!("Input locked successfully");
    }

    /// Exit locked state: unblock input, hide overlay, allow sleep.
    pub fn unlock(&mut self) {
        if !self.locked {
            log::warn!("Not locked, ignoring unlock request");
            return;
        }

        log::info!("Unlocking input...");

        // Order matters: unblock input first so the user can interact,
        // then hide overlay, then release sleep inhibition.
        IS_LOCKED.store(false, Ordering::SeqCst);

        if let Err(e) = self.input.unblock_input() {
            log::error!("Failed to unblock input: {}", e);
        }

        if let Err(e) = self.overlay.hide() {
            log::error!("Failed to hide overlay: {}", e);
        }

        if let Err(e) = self.power.allow_sleep() {
            log::warn!("Failed to release sleep inhibition (non-fatal): {}", e);
        }

        self.locked = false;
        log::info!("Input unlocked successfully");
    }

    /// Returns true if input is currently locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Clean shutdown — ensure everything is unlocked before exit.
    pub fn shutdown(&mut self) {
        if self.locked {
            self.unlock();
        }
    }
}

impl Drop for LockController {
    fn drop(&mut self) {
        self.shutdown();
    }
}
