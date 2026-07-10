//! Windows low-level input hooks — blocks keyboard and mouse via SetWindowsHookEx.
//!
//! Uses WH_KEYBOARD_LL and WH_MOUSE_LL hooks. When locked, the callback
//! returns a non-zero value to swallow the event. The unlock hotkey is
//! always passed through.

use crate::lock_controller::IS_LOCKED;
use crate::platform::InputInterceptor;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};


/// Thread-local hook handles. We use statics because the hook callbacks
/// are C-style function pointers that cannot capture state.
static mut KEYBOARD_HOOK: HHOOK = HHOOK(std::ptr::null_mut());
static mut MOUSE_HOOK: HHOOK = HHOOK(std::ptr::null_mut());

/// Configured unlock hotkey (updated from main)
pub static UNLOCK_KEY: AtomicU32 = AtomicU32::new(0x4C);
pub static UNLOCK_CTRL: AtomicBool = AtomicBool::new(true);
pub static UNLOCK_SHIFT: AtomicBool = AtomicBool::new(true);
pub static UNLOCK_ALT: AtomicBool = AtomicBool::new(false);
pub static UNLOCK_WIN: AtomicBool = AtomicBool::new(false);

/// Track modifier states manually because GetAsyncKeyState might fail when we swallow inputs
static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static ALT_DOWN: AtomicBool = AtomicBool::new(false);
static WIN_DOWN: AtomicBool = AtomicBool::new(false);

/// Virtual key codes for allowed keys even while locked.
/// VK_VOLUME_UP, VK_VOLUME_DOWN, VK_VOLUME_MUTE — let volume controls through.
const ALLOWED_VKEYS: &[u32] = &[0xAF, 0xAE, 0xAD];

pub static AUTO_DETECT_CAT: AtomicBool = AtomicBool::new(true);

/// Low-level keyboard hook callback.
///
/// Runs permanently. If IS_LOCKED is true, swallows all events (except hotkey).
/// If IS_LOCKED is false, runs PawSense heuristics.
unsafe extern "system" fn keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let kb = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
        let w = w_param.0 as u32;

        let is_down = w == WM_KEYDOWN || w == WM_SYSKEYDOWN;
        let is_up = w == windows::Win32::UI::WindowsAndMessaging::WM_KEYUP || w == windows::Win32::UI::WindowsAndMessaging::WM_SYSKEYUP;

        // Track modifiers (always track so unlock works even if modifiers pressed before lock)
        match kb.vkCode {
            0x11 | 0xA2 | 0xA3 => { if is_down || is_up { CTRL_DOWN.store(is_down, Ordering::Relaxed); } }
            0x10 | 0xA0 | 0xA1 => { if is_down || is_up { SHIFT_DOWN.store(is_down, Ordering::Relaxed); } }
            0x12 | 0xA4 | 0xA5 => { if is_down || is_up { ALT_DOWN.store(is_down, Ordering::Relaxed); } }
            0x5B | 0x5C => { if is_down || is_up { WIN_DOWN.store(is_down, Ordering::Relaxed); } }
            _ => {}
        }

        if IS_LOCKED.load(Ordering::Relaxed) {
            if ALLOWED_VKEYS.contains(&kb.vkCode) {
                return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
            }

            // Check for unlock hotkey
            if is_down && kb.vkCode == UNLOCK_KEY.load(Ordering::Relaxed) {
                if CTRL_DOWN.load(Ordering::Relaxed) == UNLOCK_CTRL.load(Ordering::Relaxed) &&
                   SHIFT_DOWN.load(Ordering::Relaxed) == UNLOCK_SHIFT.load(Ordering::Relaxed) &&
                   ALT_DOWN.load(Ordering::Relaxed) == UNLOCK_ALT.load(Ordering::Relaxed) &&
                   WIN_DOWN.load(Ordering::Relaxed) == UNLOCK_WIN.load(Ordering::Relaxed) {
                    let _ = unsafe {
                        windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                            crate::MAIN_THREAD_ID.load(Ordering::Relaxed),
                            crate::WM_CATLOCK_TOGGLE, WPARAM(0), LPARAM(0),
                        )
                    };
                    return LRESULT(1);
                }
            }
            // Swallow all other keyboard input
            return LRESULT(1);
        } else if AUTO_DETECT_CAT.load(Ordering::Relaxed) && is_down {
            // Unlocked state + PawSense
            if super::pawsense::check_pawsense(kb.vkCode) {
                return LRESULT(1);
            }
        }
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

/// Low-level mouse hook callback.
///
/// When IS_LOCKED is true, this swallows all mouse events by returning 1.
/// Mouse movement is still technically blocked but the cursor stays visible.
unsafe extern "system" fn mouse_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 && IS_LOCKED.load(Ordering::Relaxed) {
        // Block all mouse input — clicks, scroll, movement
        return LRESULT(1);
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

/// Win32 low-level input hook implementation.
pub struct Win32InputHook {
    blocking: bool,
}

impl Win32InputHook {
    pub fn new() -> Self {
        unsafe {
            // Install permanent keyboard hook
            let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0)
                .expect("Failed to set permanent keyboard hook");
            KEYBOARD_HOOK = kb_hook;
        }
        Self { blocking: false }
    }
}

impl InputInterceptor for Win32InputHook {
    fn set_auto_detect_cat(&mut self, enabled: bool) {
        AUTO_DETECT_CAT.store(enabled, Ordering::Relaxed);
    }
    fn block_input(&mut self) -> Result<(), String> {
        if self.blocking {
            return Ok(());
        }

        unsafe {
            let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0)
                .map_err(|e| format!("Failed to set mouse hook: {}", e))?;
            MOUSE_HOOK = mouse_hook;
        }

        self.blocking = true;
        log::info!("Win32 input hooks installed");
        Ok(())
    }

    fn unblock_input(&mut self) -> Result<(), String> {
        if !self.blocking {
            return Ok(());
        }

        unsafe {

            if !MOUSE_HOOK.0.is_null() {
                let _ = UnhookWindowsHookEx(MOUSE_HOOK);
                MOUSE_HOOK = HHOOK(std::ptr::null_mut());
            }
        }

        self.blocking = false;
        log::info!("Win32 input hooks removed");
        Ok(())
    }

    fn is_blocking(&self) -> bool {
        self.blocking
    }
}

impl Drop for Win32InputHook {
    fn drop(&mut self) {
        let _ = self.unblock_input();
        unsafe {
            if !KEYBOARD_HOOK.0.is_null() {
                let _ = UnhookWindowsHookEx(KEYBOARD_HOOK);
                KEYBOARD_HOOK = HHOOK(std::ptr::null_mut());
            }
        }
    }
}

