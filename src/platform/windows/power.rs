//! Windows sleep prevention via SetThreadExecutionState.
//!
//! Uses ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED to
//! prevent the system from sleeping and the display from turning off.

use crate::platform::PowerInhibitor;
use windows::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
    EXECUTION_STATE,
};

pub struct Win32PowerInhibitor {
    inhibiting: bool,
}

impl Win32PowerInhibitor {
    pub fn new() -> Self {
        Self { inhibiting: false }
    }
}

impl PowerInhibitor for Win32PowerInhibitor {
    fn inhibit_sleep(&mut self) -> Result<(), String> {
        if self.inhibiting {
            return Ok(());
        }

        let flags = ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED;
        let prev = unsafe { SetThreadExecutionState(flags) };

        if prev == EXECUTION_STATE(0) {
            return Err("SetThreadExecutionState failed".into());
        }

        self.inhibiting = true;
        log::info!("Sleep inhibited");
        Ok(())
    }

    fn allow_sleep(&mut self) -> Result<(), String> {
        if !self.inhibiting {
            return Ok(());
        }

        let prev = unsafe { SetThreadExecutionState(ES_CONTINUOUS) };
        if prev == EXECUTION_STATE(0) {
            return Err("SetThreadExecutionState (release) failed".into());
        }

        self.inhibiting = false;
        log::info!("Sleep allowed");
        Ok(())
    }
}

impl Drop for Win32PowerInhibitor {
    fn drop(&mut self) {
        let _ = self.allow_sleep();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_inhibitor_lifecycle() {
        let mut inhibitor = Win32PowerInhibitor::new();
        
        // Inhibit sleep
        assert!(inhibitor.inhibit_sleep().is_ok(), "Failed to inhibit sleep");
        
        // Idempotent call
        assert!(inhibitor.inhibit_sleep().is_ok(), "Idempotent inhibit failed");
        
        // Allow sleep
        assert!(inhibitor.allow_sleep().is_ok(), "Failed to allow sleep");
        
        // Idempotent call
        assert!(inhibitor.allow_sleep().is_ok(), "Idempotent allow failed");
    }
}
