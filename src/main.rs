//! CatLock — Cross-platform keyboard/mouse locker.
//!
//! A lightweight system tray app that locks your keyboard and mouse
//! to prevent accidental input from cats, kids, or cleaning.
//!
//! This is the main entrypoint. It sets up the event loop, system tray,
//! global hotkey, and wires everything to the lock controller.

// Hide the console window on Windows release builds.
// In debug builds the console is still attached; logging.rs can hide it at
// runtime based on the user's config.
#![windows_subsystem = "windows"]

mod hotkey;
mod i18n;
mod lock_controller;
mod logging;
mod notify;
mod platform;
mod settings;
mod tray;

use lock_controller::LockController;
use settings::Settings;
use std::sync::atomic::{AtomicU32, Ordering};

use global_hotkey::GlobalHotKeyEvent;
use tray_icon::menu::MenuEvent;

/// Custom Windows message ID for lock toggle (from hook callbacks).
pub const WM_CATLOCK_TOGGLE: u32 = 0x0400 + 1; // WM_APP + 1

/// Main thread ID — used by hook callbacks to post messages.
pub static MAIN_THREAD_ID: AtomicU32 = AtomicU32::new(0);

/// Custom Linux signal for lock toggle (from X11 thread).
#[cfg(target_os = "linux")]
pub static LINUX_TOGGLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn main() {
    // ── 1. Load settings (before logging so we know how to configure it) ──────
    // Note: any log::* calls inside Settings::load() are silently dropped
    // until the logger is initialised below — that is intentional.
    let mut settings = Settings::load();

    // ── 2. Initialise logging from settings ───────────────────────────────────
    logging::init(&settings.logging, Settings::log_file_path());

    // ── 3. Apply console window visibility (Windows-only) ─────────────────────
    logging::apply_console_visibility(settings.logging.show_console);

    log::info!("CatLock v{} starting...", env!("CARGO_PKG_VERSION"));
    log::info!("Hotkey            : {}", settings.hotkey_display_string());
    log::info!("Privacy mode      : {}", settings.privacy_mode);
    log::info!("Log level         : {}", settings.logging.log_level);
    log::info!("Log to console    : {}", settings.logging.log_to_console);
    log::info!("Log to file       : {}", settings.logging.log_to_file);
    log::info!("Log timestamps    : {}", settings.logging.log_timestamps);
    log::info!("Show console      : {}", settings.logging.show_console);

    // ── 4. Windows-specific startup tasks ─────────────────────────────────────
    #[cfg(target_os = "windows")]
    {
        // Store main thread ID so hook callbacks can PostThreadMessage.
        let tid = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
        MAIN_THREAD_ID.store(tid, Ordering::SeqCst);

        // Single-instance guard via a named Win32 Mutex.
        use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
        use windows::Win32::System::Threading::CreateMutexW;
        use windows::core::PCWSTR;

        let mutex_name: Vec<u16> = "CatLock_SingleInstanceMutex\0".encode_utf16().collect();
        unsafe {
            let _handle = CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr()));
            if GetLastError() == ERROR_ALREADY_EXISTS {
                log::error!("CatLock is already running. Exiting second instance.");
                std::process::exit(1);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        crate::platform::windows::input_hook::UNLOCK_KEY.store(settings.hotkey_key, Ordering::Relaxed);
        crate::platform::windows::input_hook::UNLOCK_CTRL.store(settings.hotkey_modifiers.ctrl, Ordering::Relaxed);
        crate::platform::windows::input_hook::UNLOCK_SHIFT.store(settings.hotkey_modifiers.shift, Ordering::Relaxed);
        crate::platform::windows::input_hook::UNLOCK_ALT.store(settings.hotkey_modifiers.alt, Ordering::Relaxed);
        crate::platform::windows::input_hook::UNLOCK_WIN.store(settings.hotkey_modifiers.win, Ordering::Relaxed);
    }

    // Create platform-specific backends
    let input = platform::create_input_interceptor();
    let overlay = platform::create_overlay();
    let power = platform::create_power_inhibitor();

    // Create lock controller
    let mut controller = LockController::new(input, overlay, power);
    controller.set_auto_detect_cat(settings.auto_detect_cat);

    // Create system tray
    let (tray, menu_ids) = tray::create_tray(
        controller.is_locked(),
        settings.privacy_mode,
        settings.logging.show_console,
        settings.auto_detect_cat,
        settings.notifications_enabled,
        &settings.hotkey_display_string(),
    )
    .expect("Failed to create system tray");

    // Register global hotkey
    let _hotkey_manager =
        hotkey::HotkeyManager::new(&settings).expect("Failed to register global hotkey");

    // Receive channels
    let menu_rx = MenuEvent::receiver();
    let hotkey_rx = GlobalHotKeyEvent::receiver();

    log::info!("CatLock ready. Waiting in system tray...");

    // ── Toggle debounce ───────────────────────────────────────────────────────
    // The Win32 LL hook (WM_CATLOCK_TOGGLE) and the global-hotkey crate both
    // fire for the same physical Ctrl+Shift+L keypress. The hook posts its
    // message synchronously; global-hotkey delivers its event ~72 ms later via
    // a background thread running RegisterHotKey. A simple channel-drain is not
    // enough because the event arrives AFTER the drain window closes.
    //
    // Solution: track the wall-clock time of the last accepted toggle and reject
    // any further toggle that arrives within DEBOUNCE_MS. 500 ms is imperceptible
    // to a human but comfortably larger than the ~72 ms race window observed in
    // production logs.
    const DEBOUNCE_MS: u128 = 500;
    let mut last_toggle_at = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_millis(DEBOUNCE_MS as u64))
        .unwrap_or_else(std::time::Instant::now);

    // Main event loop — process Win32 messages, tray menu events, and hotkey events.
    // On Windows we must pump the message loop for LL hooks to work.
    loop {
        // Process platform messages (required for Win32 hooks + tray + hotkey)
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
            };

            let mut msg = MSG::default();
            while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
                // Check for our custom toggle message from hook callbacks.
                if msg.message == WM_CATLOCK_TOGGLE {
                    let elapsed = last_toggle_at.elapsed().as_millis();
                    if elapsed >= DEBOUNCE_MS {
                        last_toggle_at = std::time::Instant::now();
                        controller.toggle(&settings);
                        tray::update_tray(
                            &tray,
                            &menu_ids,
                            controller.is_locked(),
                            settings.privacy_mode,
                            settings.logging.show_console,
                            settings.auto_detect_cat,
                            settings.notifications_enabled,
                            &settings.hotkey_display_string(),
                        );
                    } else {
                        log::debug!(
                            "WM_CATLOCK_TOGGLE debounced ({} ms since last toggle)",
                            elapsed
                        );
                    }
                    continue;
                }

                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }

        // Check for Linux X11-based toggle
        #[cfg(target_os = "linux")]
        {
            if crate::LINUX_TOGGLE.swap(false, Ordering::Relaxed) {
                let elapsed = last_toggle_at.elapsed().as_millis();
                if elapsed >= DEBOUNCE_MS {
                    last_toggle_at = std::time::Instant::now();
                    controller.toggle(&settings);
                    tray::update_tray(
                        &tray,
                        &menu_ids,
                        controller.is_locked(),
                        settings.privacy_mode,
                        settings.logging.show_console,
                        settings.auto_detect_cat,
                        settings.notifications_enabled,
                        &settings.hotkey_display_string(),
                    );
                } else {
                    log::debug!("LINUX_TOGGLE debounced ({} ms since last toggle)", elapsed);
                }
            }
        }

        // Check for global hotkey events
        if let Ok(event) = hotkey_rx.try_recv() {
            let elapsed = last_toggle_at.elapsed().as_millis();
            if elapsed >= DEBOUNCE_MS {
                log::debug!("Hotkey event: {:?}", event);
                last_toggle_at = std::time::Instant::now();
                controller.toggle(&settings);
                tray::update_tray(
                    &tray,
                    &menu_ids,
                    controller.is_locked(),
                    settings.privacy_mode,
                    settings.logging.show_console,
                    settings.auto_detect_cat,
                    settings.notifications_enabled,
                    &settings.hotkey_display_string(),
                );
            } else {
                log::debug!(
                    "GlobalHotKeyEvent debounced ({} ms since last toggle) — likely duplicate of WM_CATLOCK_TOGGLE",
                    elapsed
                );
            }
        }

        // Check for tray menu events
        if let Ok(event) = menu_rx.try_recv() {
            if event.id() == menu_ids.toggle_privacy.id() {
                settings.privacy_mode = !settings.privacy_mode;
                if let Err(e) = settings.save() {
                    log::error!("Failed to save settings: {}", e);
                }
                tray::update_tray(
                    &tray,
                    &menu_ids,
                    controller.is_locked(),
                    settings.privacy_mode,
                    settings.logging.show_console,
                    settings.auto_detect_cat,
                    settings.notifications_enabled,
                    &settings.hotkey_display_string(),
                );
                log::info!("Privacy mode: {}", settings.privacy_mode);
            } else if event.id() == menu_ids.toggle_console.id() {
                settings.logging.show_console = !settings.logging.show_console;
                if let Err(e) = settings.save() {
                    log::error!("Failed to save settings: {}", e);
                }
                logging::apply_console_visibility(settings.logging.show_console);
                tray::update_tray(
                    &tray,
                    &menu_ids,
                    controller.is_locked(),
                    settings.privacy_mode,
                    settings.logging.show_console,
                    settings.auto_detect_cat,
                    settings.notifications_enabled,
                    &settings.hotkey_display_string(),
                );
                log::info!("Console visibility: {}", settings.logging.show_console);
            } else if event.id() == menu_ids.toggle_auto_detect.id() {
                settings.auto_detect_cat = !settings.auto_detect_cat;
                if let Err(e) = settings.save() {
                    log::error!("Failed to save settings: {}", e);
                }
                controller.set_auto_detect_cat(settings.auto_detect_cat);
                tray::update_tray(
                    &tray,
                    &menu_ids,
                    controller.is_locked(),
                    settings.privacy_mode,
                    settings.logging.show_console,
                    settings.auto_detect_cat,
                    settings.notifications_enabled,
                    &settings.hotkey_display_string(),
                );
                log::info!("Auto-detect cat mode: {}", settings.auto_detect_cat);
            } else if event.id() == menu_ids.toggle_notifications.id() {
                settings.notifications_enabled = !settings.notifications_enabled;
                if let Err(e) = settings.save() {
                    log::error!("Failed to save settings: {}", e);
                }
                tray::update_tray(
                    &tray,
                    &menu_ids,
                    controller.is_locked(),
                    settings.privacy_mode,
                    settings.logging.show_console,
                    settings.auto_detect_cat,
                    settings.notifications_enabled,
                    &settings.hotkey_display_string(),
                );
                log::info!("Notifications enabled: {}", settings.notifications_enabled);
            } else if event.id() == menu_ids.about.id() {
                log::debug!("About dialog opened");
                // Show a simple message box on Windows
                #[cfg(target_os = "windows")]
                {
                    show_about_dialog();
                }
            } else if event.id() == menu_ids.quit.id() {
                log::info!("Quit requested");
                controller.shutdown();
                let _ = tray.set_visible(false);
                break;
            }
        }

        // Sleep briefly to avoid busy-waiting (1ms — maintains <2ms response time)
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    log::info!("CatLock exiting.");
}

/// Show a Windows MessageBox with About information.
#[cfg(target_os = "windows")]
fn show_about_dialog() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let text: Vec<u16> = OsStr::new(i18n::Strings::ABOUT_TEXT)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let title: Vec<u16> = OsStr::new("About CatLock")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            None,
            windows::core::PCWSTR(text.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            windows::Win32::UI::WindowsAndMessaging::MB_OK
                | windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION,
        );
    }
}
