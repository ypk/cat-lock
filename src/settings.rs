//! Settings manager — persistent configuration via JSON file.
//!
//! Settings are stored at:
//! - Windows: `%APPDATA%\CatLock\config.json`
//! - Linux: `~/.config/catlock/config.json`
//!
//! The log file (when enabled) is written alongside the config:
//! - Windows: `%APPDATA%\CatLock\catlock.log`
//! - Linux: `~/.config/catlock/catlock.log`

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ── Logging settings ─────────────────────────────────────────────────────────

/// Controls how CatLock writes log output.
///
/// All fields are persisted in `config.json` under the `"logging"` key.
/// Omitting the key entirely falls back to the `Default` values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    /// Show or hide the **console window** (Windows-only; no-op on Linux).
    ///
    /// - Debug builds: a console is attached by default; set `false` to hide it.
    /// - Release builds: no console is attached by default; set `true` to open one.
    pub show_console: bool,

    /// Write log output to **stderr / the terminal**.
    pub log_to_console: bool,

    /// Write log output to **`catlock.log`** in the config directory.
    pub log_to_file: bool,

    /// Minimum log level to record.
    /// Accepted values (case-insensitive): `"error"`, `"warn"`, `"info"`,
    /// `"debug"`, `"trace"`. Unknown values default to `"info"`.
    pub log_level: String,

    /// Prefix every log line with an ISO-8601 timestamp
    /// (e.g. `2026-07-10T14:04:29.123`).
    pub log_timestamps: bool,
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            show_console: false,
            log_to_console: true,
            log_to_file: true,
            log_level: "info".to_string(),
            log_timestamps: true,
        }
    }
}

// ── Main settings ─────────────────────────────────────────────────────────────

/// Persistent application settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Virtual key code for the hotkey (platform-specific).
    /// Default: 'L' key (0x4C on Windows, 0x006C X11 keysym).
    pub hotkey_key: u32,

    /// Modifier flags for the hotkey.
    pub hotkey_modifiers: HotkeyModifiers,

    /// When true, the overlay is fully opaque black instead of semi-transparent.
    /// This is a session-only toggle and is always OFF by default on startup.
    #[serde(skip)]
    pub privacy_mode: bool,

    /// When true, runs background heuristics (PawSense) to automatically detect 
    /// if a cat is walking on the keyboard, and locks instantly. (Windows only)
    pub auto_detect_cat: bool,

    /// When true, displays system desktop notifications on lock/unlock.
    pub notifications_enabled: bool,

    /// Logging configuration. Missing from existing config files → `Default`.
    #[serde(default)]
    pub logging: LoggingSettings,
}

/// Modifier key combination for the global hotkey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    #[cfg(target_os = "windows")]
    pub win: bool,
    #[cfg(target_os = "linux")]
    pub super_key: bool,
}

impl Default for HotkeyModifiers {
    fn default() -> Self {
        Self {
            ctrl: true,
            shift: true,
            alt: false,
            #[cfg(target_os = "windows")]
            win: false,
            #[cfg(target_os = "linux")]
            super_key: false,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // 'L' key — 0x4C is the virtual key code on Windows
            hotkey_key: 0x4C,
            hotkey_modifiers: HotkeyModifiers::default(),
            privacy_mode: false,
            auto_detect_cat: true,
            notifications_enabled: true,
            logging: LoggingSettings::default(),
        }
    }
}

impl Settings {
    /// Get the config directory path.
    fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| {
            #[cfg(target_os = "windows")]
            {
                d.join("CatLock")
            }
            #[cfg(target_os = "linux")]
            {
                d.join("catlock")
            }
        })
    }

    /// Get the full path to the config file.
    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("config.json"))
    }

    /// Get the full path to the log file (`catlock.log`).
    ///
    /// The log file lives in the same directory as `config.json`:
    /// - Windows: `%APPDATA%\CatLock\catlock.log`
    /// - Linux:   `~/.config/catlock/catlock.log`
    pub fn log_file_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("catlock.log"))
    }

    /// Load settings from disk, returning defaults if the file doesn't exist or is invalid.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            log::warn!("Could not determine config directory, using defaults");
            return Self::default();
        };

        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(settings) => {
                    log::info!("Loaded settings from {}", path.display());
                    settings
                }
                Err(e) => {
                    log::warn!("Failed to parse settings ({}), creating new config with defaults", e);
                    let s = Self::default();
                    let _ = s.save();
                    s
                }
            },
            Err(_) => {
                log::info!("No config file found, creating new config with defaults");
                let s = Self::default();
                let _ = s.save();
                s
            }
        }
    }

    /// Save current settings to disk.
    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir().ok_or("Could not determine config directory")?;
        let path = dir.join("config.json");

        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&path, json).map_err(|e| format!("Failed to write config: {}", e))?;

        log::info!("Saved settings to {}", path.display());
        Ok(())
    }

    /// Get a human-readable description of the current hotkey combo.
    pub fn hotkey_display_string(&self) -> String {
        let mut parts = Vec::new();
        let mods = &self.hotkey_modifiers;

        if mods.ctrl {
            parts.push("Ctrl");
        }
        if mods.shift {
            parts.push("Shift");
        }
        if mods.alt {
            parts.push("Alt");
        }
        #[cfg(target_os = "windows")]
        if mods.win {
            parts.push("Win");
        }
        #[cfg(target_os = "linux")]
        if mods.super_key {
            parts.push("Super");
        }

        // Convert key code to character name
        let key_name = match self.hotkey_key {
            0x41..=0x5A => {
                // A-Z
                String::from(char::from(self.hotkey_key as u8))
            }
            _ => format!("0x{:02X}", self.hotkey_key),
        };

        parts.push(&key_name);
        // We need to own the string, so collect differently
        let mut result = String::new();
        let mods_str: Vec<&str> = parts[..parts.len() - 1].to_vec();
        result.push_str(&mods_str.join("+"));
        if !mods_str.is_empty() {
            result.push('+');
        }
        result.push_str(&key_name);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let s = Settings::default();
        assert!(!s.privacy_mode);
        assert!(s.hotkey_modifiers.ctrl);
        assert!(s.hotkey_modifiers.shift);
        assert!(!s.hotkey_modifiers.alt);
        assert_eq!(s.hotkey_key, 0x4C); // L key
    }

    #[test]
    fn test_serialize_roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let s2: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s.hotkey_key, s2.hotkey_key);
    }

    #[test]
    fn test_hotkey_display() {
        let s = Settings::default();
        let display = s.hotkey_display_string();
        assert!(display.contains("Ctrl"));
        assert!(display.contains("Shift"));
        assert!(display.contains("L"));
    }
}
