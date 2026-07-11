//! Automatic cat detection (PawSense) heuristic algorithm.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use windows::Win32::Foundation::{LPARAM, WPARAM};

pub const PAWSENSE_KEY_COUNT: usize = 4;
pub const PAWSENSE_THRESHOLD_MS: u64 = 200;

static PAW_HISTORY: [AtomicU64; PAWSENSE_KEY_COUNT] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];
static PAW_INDEX: AtomicUsize = AtomicUsize::new(0);

fn get_current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Runs the PawSense heuristics on a given keystroke.
/// Returns true if a cat was detected, false otherwise.
pub fn check_pawsense(vk_code: u32) -> bool {
    // Ignore modifier keys, as they are often held down naturally by humans
    let is_modifier = matches!(vk_code, 0x10..=0x12 | 0xA0..=0xA5 | 0x5B | 0x5C);
    if is_modifier {
        return false;
    }

    let now = get_current_time_ms();
    let idx = PAW_INDEX.fetch_add(1, Ordering::Relaxed) % PAWSENSE_KEY_COUNT;
    PAW_HISTORY[idx].store(now, Ordering::Relaxed);

    let oldest_idx = (idx + 1) % PAWSENSE_KEY_COUNT;
    let oldest_time = PAW_HISTORY[oldest_idx].load(Ordering::Relaxed);

    if oldest_time > 0 && now.saturating_sub(oldest_time) <= PAWSENSE_THRESHOLD_MS {
        log::warn!(
            "🐾 PawSense triggered! {} keys in {}ms",
            PAWSENSE_KEY_COUNT,
            now - oldest_time
        );
        
        // Clear the buffer to avoid rapid re-triggering
        for i in 0..PAWSENSE_KEY_COUNT {
            PAW_HISTORY[i].store(0, Ordering::Relaxed);
        }

        // Trigger lock by posting message to main thread
        let _ = unsafe {
            windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                crate::MAIN_THREAD_ID.load(Ordering::Relaxed),
                crate::WM_CATLOCK_TOGGLE,
                WPARAM(0),
                LPARAM(0),
            )
        };
        
        return true;
    }
    
    false
}
