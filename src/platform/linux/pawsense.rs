//! Linux automatic cat detection (PawSense) polling algorithm.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use x11rb::connection::Connection;

pub static AUTO_DETECT_CAT: AtomicBool = AtomicBool::new(true);

/// Spawns a background thread that polls the X11 keyboard state to detect cat paws.
pub fn spawn_pawsense_thread() {
    thread::spawn(|| {
        let Ok((conn, _)) = x11rb::connect(None) else {
            log::warn!("PawSense: Failed to connect to X11 server");
            return;
        };

        log::info!("Linux PawSense polling thread started");

        loop {
            if AUTO_DETECT_CAT.load(Ordering::Relaxed)
                && !crate::lock_controller::IS_LOCKED.load(Ordering::Relaxed)
            {
                if let Ok(reply) = conn.query_keymap().and_then(|c| c.reply()) {
                    let mut pressed_count = 0;
                    for byte in reply.keys.iter() {
                        pressed_count += byte.count_ones();
                    }

                    // A normal human might press a couple of modifiers and a key or two.
                    // But 5+ keys physically held down at the exact same instant is very rare
                    // for normal typing, but extremely common for a cat standing on the laptop.
                    if pressed_count >= 5 {
                        log::warn!(
                            "🐾 Linux PawSense triggered! {} keys pressed simultaneously",
                            pressed_count
                        );
                        crate::LINUX_TOGGLE.store(true, Ordering::Relaxed);
                        // Sleep a bit longer after a trigger to debounce
                        thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                }
            }
            // Poll every 50ms
            thread::sleep(Duration::from_millis(50));
        }
    });
}
