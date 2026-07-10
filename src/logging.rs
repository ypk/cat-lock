//! Logging initialisation — configures console and/or file output with
//! optional ISO-8601 timestamps and a runtime-adjustable log level.
//!
//! All behaviour is driven by [`crate::settings::LoggingSettings`], which
//! persists in `config.json` so the user can change it without recompiling.

use crate::settings::LoggingSettings;
use std::path::PathBuf;

/// Initialise the global logger from the provided settings.
///
/// Must be called **exactly once**, before any `log::*` macros are used.
/// If `log_file_path` is `Some` and `settings.log_to_file` is `true`, a log
/// file is opened (appending) at that path.
pub fn init(settings: &LoggingSettings, log_file_path: Option<PathBuf>) {
    let level = parse_level(&settings.log_level);
    let timestamps = settings.log_timestamps;

    // Shared formatter — applied to every output chain.
    let formatter = move |out: fern::FormatCallback,
                          message: &std::fmt::Arguments,
                          record: &log::Record| {
        if timestamps {
            out.finish(format_args!(
                "[{} {:<5} {}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f"),
                record.level(),
                record.target(),
                message
            ))
        } else {
            out.finish(format_args!(
                "[{:<5} {}] {}",
                record.level(),
                record.target(),
                message
            ))
        }
    };

    let mut dispatch = fern::Dispatch::new()
        .format(formatter)
        .level(level);

    // ── Console / stderr ─────────────────────────────────────────────────────
    if settings.log_to_console {
        dispatch = dispatch.chain(std::io::stderr());
    }

    // ── File output ──────────────────────────────────────────────────────────
    if settings.log_to_file {
        if let Some(ref path) = log_file_path {
            // Ensure the parent directory exists before creating the file.
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match fern::log_file(path) {
                Ok(file_out) => dispatch = dispatch.chain(file_out),
                Err(e) => eprintln!(
                    "[WARN ] CatLock: could not open log file {:?}: {}",
                    path, e
                ),
            }
        }
    }

    if let Err(e) = dispatch.apply() {
        eprintln!("[WARN ] CatLock: failed to initialise logger: {}", e);
    }
}

/// Show or hide the console window that is attached to this process.
///
/// | Build   | `show = true`                      | `show = false`           |
/// |---------|------------------------------------|--------------------------|
/// | Debug   | Console window is made visible     | Console window is hidden |
/// | Release | `AllocConsole()` then show window  | No-op (never attached)   |
///
/// This is a **Windows-only** operation. On Linux the function is a no-op —
/// stdout/stderr simply go wherever the calling shell directed them.
#[cfg(target_os = "windows")]
pub fn apply_console_visibility(show: bool) {
    use windows::Win32::System::Console::{AllocConsole, GetConsoleWindow};
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE, SW_SHOW};

    unsafe {
        let hwnd = GetConsoleWindow();
        if show {
            if hwnd.0 == std::ptr::null_mut() {
                // Release build: no console is attached — allocate a new one.
                let _ = AllocConsole();
                let new_hwnd = GetConsoleWindow();
                if new_hwnd.0 != std::ptr::null_mut() {
                    let _ = ShowWindow(new_hwnd, SW_SHOW);
                }
            } else {
                // Debug build: a console exists but may be hidden — show it.
                let _ = ShowWindow(hwnd, SW_SHOW);
            }
        } else if hwnd.0 != std::ptr::null_mut() {
            // Debug build: hide the existing console window.
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        // Release build + show=false: no console was ever created, nothing to do.
    }
}

#[cfg(target_os = "linux")]
pub fn apply_console_visibility(_show: bool) {
    // No-op: Linux has no separate "console window" concept.
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_level(level: &str) -> log::LevelFilter {
    match level.to_lowercase().as_str() {
        "error" => log::LevelFilter::Error,
        "warn"  => log::LevelFilter::Warn,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _       => log::LevelFilter::Info, // "info" and any unknown value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_level_known() {
        assert_eq!(parse_level("error"), log::LevelFilter::Error);
        assert_eq!(parse_level("WARN"),  log::LevelFilter::Warn);
        assert_eq!(parse_level("Info"),  log::LevelFilter::Info);
        assert_eq!(parse_level("debug"), log::LevelFilter::Debug);
        assert_eq!(parse_level("trace"), log::LevelFilter::Trace);
    }

    #[test]
    fn test_parse_level_unknown_defaults_to_info() {
        assert_eq!(parse_level("verbose"), log::LevelFilter::Info);
        assert_eq!(parse_level(""),        log::LevelFilter::Info);
    }
}
