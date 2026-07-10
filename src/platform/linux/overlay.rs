//! Linux overlay window — X11 override-redirect fullscreen window.

use crate::platform::OverlayWindow;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    ConnectionExt, CreateWindowAux, PropMode, WindowClass,
};
use x11rb::rust_connection::RustConnection;

pub struct LinuxOverlay {
    conn: Option<RustConnection>,
    window_ids: Vec<u32>,
}

impl LinuxOverlay {
    pub fn new() -> Self {
        Self {
            conn: None,
            window_ids: Vec::new(),
        }
    }
}

impl OverlayWindow for LinuxOverlay {
    fn show(&mut self, privacy_mode: bool, _hotkey_str: &str) -> Result<(), String> {
        if self.conn.is_some() {
            return Ok(()); // Already showing
        }

        let (conn, _) = x11rb::connect(None).map_err(|e| format!("X11 connect failed: {}", e))?;

        let mut window_ids = Vec::new();
        let setup = conn.setup();

        for screen in &setup.roots {
            let win_id = conn.generate_id().map_err(|e| format!("Failed to generate window ID: {}", e))?;
            
            let aux = CreateWindowAux::new()
                .override_redirect(1)
                .background_pixel(screen.black_pixel); // Black background
            
            conn.create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                win_id,
                screen.root,
                0,
                0,
                screen.width_in_pixels,
                screen.height_in_pixels,
                0,
                WindowClass::INPUT_OUTPUT,
                x11rb::COPY_FROM_PARENT,
                &aux,
            ).map_err(|e| format!("Failed to create window: {}", e))?;

            if !privacy_mode {
                // Set opacity to 50% (0x80000000)
                let opacity_atom_reply = conn.intern_atom(false, b"_NET_WM_WINDOW_OPACITY")
                    .map_err(|e| format!("Failed to intern opacity atom: {}", e))?
                    .reply()
                    .map_err(|e| format!("Failed to get opacity atom reply: {}", e))?;
                
                let opacity_atom = opacity_atom_reply.atom;
                
                // Atom for CARDINAL
                let cardinal_atom = 6; // XA_CARDINAL is statically 6 in X11
                
                let opacity: u32 = 0x80000000;
                
                conn.change_property32(
                    PropMode::REPLACE,
                    win_id,
                    opacity_atom,
                    cardinal_atom, // type is CARDINAL
                    &[opacity],
                ).map_err(|e| format!("Failed to change opacity property: {}", e))?;
            }

            conn.map_window(win_id).map_err(|e| format!("Failed to map window: {}", e))?;
            window_ids.push(win_id);
        }

        conn.flush().map_err(|e| format!("Failed to flush X11 connection: {}", e))?;

        self.conn = Some(conn);
        self.window_ids = window_ids;
        
        log::info!("Linux X11 overlay shown (privacy={})", privacy_mode);
        Ok(())
    }

    fn hide(&mut self) -> Result<(), String> {
        if let Some(conn) = self.conn.take() {
            for win_id in &self.window_ids {
                let _ = conn.destroy_window(*win_id);
            }
            let _ = conn.flush();
            self.window_ids.clear();
            log::info!("Linux X11 overlay hidden");
        }
        Ok(())
    }

    fn is_visible(&self) -> bool {
        self.conn.is_some()
    }
}

impl Drop for LinuxOverlay {
    fn drop(&mut self) {
        let _ = self.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::OverlayWindow;

    #[test]
    fn test_overlay_lifecycle() {
        let overlay = LinuxOverlay::new();
        assert!(!overlay.is_visible());
    }
}
