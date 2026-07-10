//! Global hotkey registration — powered by the `global-hotkeys` crate.
//!
//! Registers a system-wide hotkey (default Ctrl+Shift+L) that works
//! from any application, even when CatLock is in the background.

use crate::settings::Settings;
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;

/// Hotkey manager wrapper.
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    current_hotkey: Option<HotKey>,
}

impl HotkeyManager {
    /// Create a new hotkey manager and register the initial hotkey.
    pub fn new(settings: &Settings) -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| format!("Failed to create hotkey manager: {}", e))?;

        let mut hm = Self {
            manager,
            current_hotkey: None,
        };

        hm.register(settings)?;
        Ok(hm)
    }

    /// Register the hotkey from current settings.
    pub fn register(&mut self, settings: &Settings) -> Result<(), String> {
        // Unregister old hotkey if any
        if let Some(old) = self.current_hotkey.take() {
            let _ = self.manager.unregister(old);
        }

        let modifiers = build_modifiers(&settings.hotkey_modifiers);
        let code = vk_to_code(settings.hotkey_key);

        let hotkey = HotKey::new(Some(modifiers), code);

        self.manager
            .register(hotkey)
            .map_err(|e| format!("Failed to register hotkey: {}", e))?;

        self.current_hotkey = Some(hotkey);
        log::info!("Registered global hotkey: {}", settings.hotkey_display_string());
        Ok(())
    }
}

/// Convert our settings modifiers to global-hotkeys Modifiers.
fn build_modifiers(mods: &crate::settings::HotkeyModifiers) -> Modifiers {
    let mut m = Modifiers::empty();
    if mods.ctrl {
        m |= Modifiers::CONTROL;
    }
    if mods.shift {
        m |= Modifiers::SHIFT;
    }
    if mods.alt {
        m |= Modifiers::ALT;
    }
    #[cfg(target_os = "windows")]
    if mods.win {
        m |= Modifiers::SUPER;
    }
    #[cfg(target_os = "linux")]
    if mods.super_key {
        m |= Modifiers::SUPER;
    }
    m
}

/// Convert a Windows virtual key code to a global-hotkeys Code.
fn vk_to_code(vk: u32) -> Code {
    match vk {
        0x41 => Code::KeyA,
        0x42 => Code::KeyB,
        0x43 => Code::KeyC,
        0x44 => Code::KeyD,
        0x45 => Code::KeyE,
        0x46 => Code::KeyF,
        0x47 => Code::KeyG,
        0x48 => Code::KeyH,
        0x49 => Code::KeyI,
        0x4A => Code::KeyJ,
        0x4B => Code::KeyK,
        0x4C => Code::KeyL,
        0x4D => Code::KeyM,
        0x4E => Code::KeyN,
        0x4F => Code::KeyO,
        0x50 => Code::KeyP,
        0x51 => Code::KeyQ,
        0x52 => Code::KeyR,
        0x53 => Code::KeyS,
        0x54 => Code::KeyT,
        0x55 => Code::KeyU,
        0x56 => Code::KeyV,
        0x57 => Code::KeyW,
        0x58 => Code::KeyX,
        0x59 => Code::KeyY,
        0x5A => Code::KeyZ,
        _ => Code::KeyL, // fallback
    }
}
