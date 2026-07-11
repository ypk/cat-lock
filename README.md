# CatLock

**A lightweight system tray app that locks your keyboard and mouse** — because cats have opinions about your work.

Cross-platform rewrite of [CatLock](https://github.com/hou-physics/CatLock), breaking free from macOS to support **Windows** and **Linux**.

## Features

- **Global hotkey** — `Ctrl+Shift+L` from any app to instantly lock all input
- **Fullscreen overlay** — Beautiful semi-transparent Slint GUI overlay blocks all keyboard and mouse input
- **PawSense (Auto-Detect Cat)** — Automatically locks your screen if a cat walks on your keyboard (Detects 4+ keys pressed within 100ms)
- **Privacy mode** — Fully opaque black overlay hides your screen
- **Silent startup** — Runs purely in the system tray without spawning a console window by default
- **System tray** — Lives in your system tray, stays out of your way
- **Sleep prevention** — Keeps your PC awake so long-running tasks aren't interrupted
- **Multi-monitor** — Overlay spans all connected displays
- **Configurable logging** — Console window, file logging, log level and timestamps all controlled from `config.json`
- **Minimal footprint** — ~1-3 MB RAM, zero runtime dependencies

## Quick Start

```bash
# Build
cargo build --release

# Run
cargo run --release
```

The app appears as a tray icon. Right‑click for options, or press `Ctrl+Shift+L` to lock.

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+Shift+L` | Toggle lock/unlock |
| Click unlock button | Unlock |

## Configuration

Settings are stored at:
- **Windows**: `%APPDATA%\CatLock\config.json`
- **Linux**: `~/.config/catlock/config.json`

```json
{
  "hotkey_key": 76,
  "hotkey_modifiers": {
    "ctrl": true,
    "shift": true,
    "alt": false,
    "win": false
  },
  "auto_detect_cat": true,
  "logging": {
    "show_console": false,
    "log_to_console": true,
    "log_to_file": true,
    "log_level": "info",
    "log_timestamps": true
  }
}
```

### Logging options

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `show_console` | bool | `false` (release) / `true` (debug) | Show or hide the console window (Windows-only). Set `true` to reveal the log output in a terminal window. |
| `log_to_console` | bool | `true` | Write log output to stderr / the terminal. |
| `log_to_file` | bool | `true` | Append log output to `catlock.log` in the config directory. |
| `log_level` | string | `"info"` | Minimum level to record: `"error"`, `"warn"`, `"info"`, `"debug"`, `"trace"`. |
| `log_timestamps` | bool | `true` | Prefix every line with an ISO‑8601 timestamp (`2026-07-10T14:17:02.111`). |

The log file is written to:
- **Windows**: `%APPDATA%\CatLock\catlock.log`
- **Linux**: `~/.config/catlock/catlock.log`

## Linux Installation & Runtime

CatLock works on any Linux desktop that provides a native **Xorg (X11)** server. 

> [!WARNING]
> **Wayland is NOT supported.** Because Wayland's strict security architecture fundamentally forbids applications from creating global input grabs or functioning as global keyloggers, CatLock cannot intercept inputs globally under a Wayland session. While the UI may launch under XWayland, the core lock and PawSense mechanisms will not function. Please use your compositor's built-in lock screen for Wayland.

1. **Install Xorg** (if not already present):
   - Ubuntu/Debian: `sudo apt install xorg`
   - Fedora: `sudo dnf install xorg-x11-server-Xorg`
2. Ensure the `x11rb` crate can connect to the X server (the default `$DISPLAY` environment variable is used).
3. Run the binary as usual – the lock will capture input via X11, display the fullscreen overlay, and inhibit sleep using the Linux D‑Bus logind interface.

## Architecture

Written in Rust using `slint` for the UI and a custom platform abstraction layer:

```
src/
├── main.rs                 # Event loop + wiring
├── lock_controller.rs      # Core state machine
├── settings.rs             # JSON config persistence (incl. LoggingSettings)
├── logging.rs              # Multi-output logger (console + file, timestamps)
├── tray.rs                 # System tray icon + menu
├── hotkey.rs               # Global hotkey registration
├── i18n.rs                 # String table
└── platform/
    ├── mod.rs              # Trait definitions
    ├── windows/            # Win32 API backends
    └── linux/              # X11/D-Bus backends
```

## Platform Status

| Platform | Status |
|---|---|
| Windows 10/11 | ✅ Fully implemented |
| Linux (X11) | ✅ Fully implemented |
| Linux (Wayland) | ❌ Unsupported (Blocked by Wayland security model) |
| macOS | Use [original CatLock](https://github.com/hou-physics/CatLock) |

## Dependencies

Only ~9 crates — no Electron, no Tauri, no frameworks:
- `windows` (Microsoft official Win32 bindings)
- `tray-icon` / `global-hotkeys` (system tray + hotkey)
- `serde` / `serde_json` (config persistence)
- `dirs` (platform config paths)
- `log` / `fern` (logging dispatcher — console + file)
- `chrono` (ISO‑8601 timestamps in log output)
- `image` (PNG icon decoding)
- `slint` (cross-platform GUI for the lock screen overlay)
- `x11rb` (X11 communication, Linux only)
- `zbus` (D‑Bus power inhibition, Linux only)
- `winres` (Windows executable icon compilation)

## License

GPL-3.0 — same as the original CatLock.
