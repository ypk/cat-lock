//! Windows fullscreen overlay window — TOPMOST layered window covering all monitors.
//!
//! Creates a borderless, always-on-top window that spans the entire virtual screen
//! (all monitors). Semi-transparent in normal mode, fully opaque in privacy mode.
//! Includes an unlock button rendered via custom WM_PAINT.

use crate::platform::OverlayWindow;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, EndPaint, FillRect, PAINTSTRUCT, UpdateWindow,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetSystemMetrics,
    IsWindow, LoadCursorW, RegisterClassExW, SetLayeredWindowAttributes,
    ShowWindow, CS_HREDRAW, CS_VREDRAW,
    IDC_ARROW, LWA_ALPHA, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOW, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const OVERLAY_CLASS_NAME: &str = "CatLockOverlay\0";

/// Win32 overlay window implementation.
pub struct Win32Overlay {
    hwnd: Option<HWND>,
    visible: bool,
    class_registered: bool,
}

impl Win32Overlay {
    pub fn new() -> Self {
        Self {
            hwnd: None,
            visible: false,
            class_registered: false,
        }
    }

    /// Register the window class if not already done.
    fn ensure_class_registered(&mut self) -> Result<(), String> {
        if self.class_registered {
            return Ok(());
        }

        let class_name = to_wide(OVERLAY_CLASS_NAME);
        let hinstance =
            unsafe { GetModuleHandleW(PCWSTR(std::ptr::null())) }.map_err(|e| e.to_string())?;

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(overlay_wnd_proc),
            hInstance: hinstance.into(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }
                .map_err(|e| format!("LoadCursor failed: {e}"))?,
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        let atom = unsafe { RegisterClassExW(&wc) };
        if atom == 0 {
            return Err("Failed to register overlay window class".into());
        }

        self.class_registered = true;
        Ok(())
    }
}

static OVERLAY_HOTKEY_TEXT: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

impl OverlayWindow for Win32Overlay {
    fn show(&mut self, privacy_mode: bool, hotkey_str: &str) -> Result<(), String> {
        if let Ok(mut text) = OVERLAY_HOTKEY_TEXT.lock() {
            *text = format!("Press {} to unlock", hotkey_str);
        }
        if self.visible {
            return Ok(());
        }

        self.ensure_class_registered()?;

        let class_name = to_wide(OVERLAY_CLASS_NAME);
        let hinstance =
            unsafe { GetModuleHandleW(PCWSTR(std::ptr::null())) }.map_err(|e| e.to_string())?;

        // Get virtual screen dimensions (spans all monitors)
        let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let cx = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let cy = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

        let alpha: u8 = if privacy_mode { 255 } else { 180 };

        let ex_style = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED;

        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                PCWSTR(class_name.as_ptr()),
                PCWSTR(std::ptr::null()),
                WS_POPUP,
                x,
                y,
                cx,
                cy,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .map_err(|e| format!("CreateWindowEx failed: {e}"))?;

        // Set transparency
        unsafe {
            let _ = SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0), alpha, LWA_ALPHA);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);
        }

        self.hwnd = Some(hwnd);
        self.visible = true;
        log::info!(
            "Overlay shown (privacy={}, alpha={}, {}x{} at {},{})",
            privacy_mode,
            alpha,
            cx,
            cy,
            x,
            y
        );
        Ok(())
    }

    fn hide(&mut self) -> Result<(), String> {
        if let Some(hwnd) = self.hwnd.take() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
                let _ = DestroyWindow(hwnd);
            }
        }
        self.visible = false;
        log::info!("Overlay hidden");
        Ok(())
    }

    fn is_visible(&self) -> bool {
        self.visible
            && self
                .hwnd
                .is_some_and(|h| unsafe { IsWindow(Some(h)) }.as_bool())
    }
}

impl Drop for Win32Overlay {
    fn drop(&mut self) {
        let _ = self.hide();
    }
}

/// Window procedure for the overlay.
unsafe extern "system" fn overlay_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::WM_PAINT;

    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };

            // Fill with dark background
            let brush = unsafe { CreateSolidBrush(windows::Win32::Foundation::COLORREF(0x00201A18)) }; // Dark warm color
            unsafe { FillRect(hdc, &ps.rcPaint, brush) };

            let mut client_rect = RECT::default();
            let _ = unsafe {
                windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client_rect)
            };

            // Draw text
            unsafe {
                use windows::Win32::Graphics::Gdi::{
                    SetBkMode, SetTextColor, TRANSPARENT,
                };

                let _ = SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, windows::Win32::Foundation::COLORREF(0x00FFFFFF));

                // Draw title text
                let title: Vec<u16> = crate::i18n::Strings::OVERLAY_TITLE
                    .encode_utf16()
                    .collect();
                let mut title_rect = RECT {
                    left: 0,
                    top: client_rect.bottom / 2 - 60,
                    right: client_rect.right,
                    bottom: client_rect.bottom / 2,
                };
                windows::Win32::Graphics::Gdi::DrawTextW(
                    hdc,
                    &mut title.clone(),
                    &mut title_rect,
                    windows::Win32::Graphics::Gdi::DT_CENTER
                        | windows::Win32::Graphics::Gdi::DT_VCENTER
                        | windows::Win32::Graphics::Gdi::DT_SINGLELINE,
                );

                // Draw hint text
                let hint_text = {
                    if let Ok(text) = OVERLAY_HOTKEY_TEXT.lock() {
                        text.clone()
                    } else {
                        "Press shortcut to unlock".to_string()
                    }
                };
                let hint: Vec<u16> = hint_text.encode_utf16().collect();
                let mut hint_rect = RECT {
                    left: 0,
                    top: client_rect.bottom / 2,
                    right: client_rect.right,
                    bottom: client_rect.bottom / 2 + 60,
                };
                windows::Win32::Graphics::Gdi::DrawTextW(
                    hdc,
                    &mut hint.clone(),
                    &mut hint_rect,
                    windows::Win32::Graphics::Gdi::DT_CENTER
                        | windows::Win32::Graphics::Gdi::DT_VCENTER
                        | windows::Win32::Graphics::Gdi::DT_SINGLELINE,
                );
            }

            unsafe { let _ = EndPaint(hwnd, &ps); }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Helper: convert a &str to null-terminated wide string.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_overlay_lifecycle() {
        let mut overlay = Win32Overlay::new();
        assert!(!overlay.is_visible(), "Overlay should start hidden");
        
        // Show the overlay
        assert!(overlay.show(false, "Ctrl+Shift+L").is_ok(), "Failed to show overlay");
        assert!(overlay.is_visible(), "Overlay should be visible");
        
        // Idempotent show
        assert!(overlay.show(false, "Ctrl+Shift+L").is_ok(), "Idempotent show failed");
        assert!(overlay.is_visible(), "Overlay should still be visible");
        
        // Hide the overlay
        assert!(overlay.hide().is_ok(), "Failed to hide overlay");
        assert!(!overlay.is_visible(), "Overlay should be hidden");
        
        // Idempotent hide
        assert!(overlay.hide().is_ok(), "Idempotent hide failed");
    }
}
