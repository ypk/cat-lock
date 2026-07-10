//! Linux input grab — X11 XGrabKeyboard/XGrabPointer implementation.

use crate::platform::InputInterceptor;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    ConnectionExt, EventMask, GrabMode, KeyButMask, Time,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

pub struct LinuxInputGrab {
    blocking: bool,
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl LinuxInputGrab {
    pub fn new() -> Self {
        super::pawsense::spawn_pawsense_thread();
        
        Self {
            blocking: false,
            stop_flag: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
        }
    }
}

impl InputInterceptor for LinuxInputGrab {
    fn set_auto_detect_cat(&mut self, enabled: bool) {
        super::pawsense::AUTO_DETECT_CAT.store(enabled, Ordering::Relaxed);
    }

    fn block_input(&mut self) -> Result<(), String> {
        if self.blocking {
            return Ok(());
        }

        let (conn, screen_num) =
            x11rb::connect(None).map_err(|e| format!("X11 connect failed: {}", e))?;
        let root = conn.setup().roots[screen_num].root;

        // Grab keyboard
        conn.grab_keyboard(
            false,
            root,
            Time::CURRENT_TIME,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )
        .map_err(|e| format!("GrabKeyboard failed: {}", e))?
        .reply()
        .map_err(|e| format!("GrabKeyboard reply failed: {}", e))?;

        // Grab pointer
        conn.grab_pointer(
            false,
            root,
            (EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION).into(),
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            x11rb::NONE,
            x11rb::NONE,
            Time::CURRENT_TIME,
        )
        .map_err(|e| format!("GrabPointer failed: {}", e))?
        .reply()
        .map_err(|e| format!("GrabPointer reply failed: {}", e))?;

        self.stop_flag.store(false, Ordering::SeqCst);
        let stop_flag = self.stop_flag.clone();

        // Get keycode for 'l' or 'L' via X11 keyboard mapping
        let min_kc = conn.setup().min_keycode;
        let max_kc = conn.setup().max_keycode;
        let mut l_keycode = 46; // fallback to QWERTY 'L'
        
        if let Ok(reply) = conn.get_keyboard_mapping(min_kc, max_kc - min_kc + 1).and_then(|c| c.reply()) {
            let keysyms_per_keycode = reply.keysyms_per_keycode as usize;
            'outer: for kc_idx in 0..(reply.keysyms.len() / keysyms_per_keycode) {
                for sym_idx in 0..keysyms_per_keycode {
                    let sym = reply.keysyms[kc_idx * keysyms_per_keycode + sym_idx];
                    if sym == 0x6c || sym == 0x4c { // XK_l or XK_L
                        l_keycode = min_kc + kc_idx as u8;
                        break 'outer;
                    }
                }
            }
        }

        let handle = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                if let Ok(Some(event)) = conn.poll_for_event() {
                    if let Event::KeyPress(e) = event {
                        let state = e.state;
                        // Check Control (4) and Shift (1) modifiers
                        let ctrl = (state & u16::from(KeyButMask::CONTROL)) != 0;
                        let shift = (state & u16::from(KeyButMask::SHIFT)) != 0;

                        if ctrl && shift && e.detail == l_keycode {
                            // Trigger unlock by signaling the main loop
                            crate::LINUX_TOGGLE.store(true, Ordering::Relaxed);
                        }
                    }
                } else {
                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            // `conn` is dropped here, which automatically releases the grabs.
        });

        self.thread_handle = Some(handle);
        self.blocking = true;
        log::info!("Linux input grabbed via X11");
        Ok(())
    }

    fn unblock_input(&mut self) -> Result<(), String> {
        if !self.blocking {
            return Ok(());
        }

        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        self.blocking = false;
        log::info!("Linux input grabs released");
        Ok(())
    }

    fn is_blocking(&self) -> bool {
        self.blocking
    }
}

impl Drop for LinuxInputGrab {
    fn drop(&mut self) {
        let _ = self.unblock_input();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_grab_lifecycle() {
        let mut grab = LinuxInputGrab::new();
        assert!(!grab.is_blocking());
    }
}
