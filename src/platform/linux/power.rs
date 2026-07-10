//! Linux sleep prevention via D-Bus logind Inhibit.

use crate::platform::PowerInhibitor;
use zbus::blocking::Connection;
use zbus::zvariant::OwnedFd;

pub struct LinuxPowerInhibitor {
    fd: Option<OwnedFd>,
}

impl LinuxPowerInhibitor {
    pub fn new() -> Self {
        Self { fd: None }
    }
}

impl PowerInhibitor for LinuxPowerInhibitor {
    fn inhibit_sleep(&mut self) -> Result<(), String> {
        if self.fd.is_some() {
            return Ok(());
        }

        let connection = Connection::system()
            .map_err(|e| format!("Failed to connect to system D-Bus: {}", e))?;

        let reply = connection.call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "Inhibit",
            &("sleep", "CatLock", "Prevent sleep while locked", "block"),
        ).map_err(|e| format!("Failed to call Inhibit on logind: {}", e))?;

        let fd: OwnedFd = reply.body().deserialize()
            .map_err(|e| format!("Failed to deserialize logind Inhibit response: {}", e))?;

        self.fd = Some(fd);
        log::info!("Sleep inhibited via logind");
        Ok(())
    }

    fn allow_sleep(&mut self) -> Result<(), String> {
        if self.fd.is_none() {
            return Ok(());
        }

        // Dropping the OwnedFd automatically closes it, releasing the inhibit lock
        self.fd = None;
        log::info!("Sleep allowed");
        Ok(())
    }
}

impl Drop for LinuxPowerInhibitor {
    fn drop(&mut self) {
        let _ = self.allow_sleep();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_inhibitor_lifecycle() {
        let inhibitor = LinuxPowerInhibitor::new();
        assert!(inhibitor.fd.is_none());
    }
}
