use crate::platform::OverlayWindow;

slint::slint! {
    import { VerticalBox } from "std-widgets.slint";
    
    export component LockWindow inherits Window {
        no-frame: true;
        always-on-top: true;
        background: transparent;
        
        in property <bool> is_privacy: false;
        in property <string> hotkey_text: "";
        
        Rectangle {
            // Full black for privacy mode, 40% black otherwise
            background: root.is_privacy ? #000000 : #00000066; 
            animate background { duration: 250ms; }
            
            VerticalBox {
                alignment: center;
                spacing: 20px;
                
                // Cat icon placeholder
                Text { 
                    text: "🐈"; 
                    font-size: 60px; 
                    horizontal-alignment: center; 
                }
                
                // Box
                HorizontalLayout {
                    alignment: center;
                    Rectangle {
                        background: #F8F8F2;
                        border-radius: 20px;
                        width: 300px;
                        height: 150px;
                        drop-shadow-blur: 20px;
                        drop-shadow-color: #00000040;
                        
                        VerticalBox {
                            alignment: center;
                            spacing: 12px;
                            Text { text: "🔓"; font-size: 40px; horizontal-alignment: center; }
                            Text { text: "Click to unlock"; font-size: 22px; font-weight: 500; horizontal-alignment: center; color: #222222; }
                        }
                    }
                }
                
                // Hints
                VerticalBox {
                    spacing: 6px;
                    Text { text: "System will not sleep. Ensure sufficient battery."; color: #FFFFFFB2; horizontal-alignment: center; font-size: 14px; }
                    Text { text: root.hotkey_text; color: #FFFFFF80; horizontal-alignment: center; font-size: 14px; }
                }
            }
        }
    }
}

pub struct SlintOverlay {
    ui_weak: Option<slint::Weak<LockWindow>>,
}

impl SlintOverlay {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        
        std::thread::spawn(move || {
            let ui = match LockWindow::new() {
                Ok(ui) => ui,
                Err(e) => {
                    log::error!("Failed to create Slint window: {}", e);
                    return;
                }
            };
            
            // Initially hidden
            let _ = ui.window().hide();
            
            if tx.send(ui.as_weak()).is_ok() {
                if let Err(e) = slint::run_event_loop_until_quit() {
                    log::error!("Slint event loop error: {}", e);
                }
            }
        });
        
        let ui_weak = rx.recv().ok();
        Self { ui_weak }
    }
}

impl OverlayWindow for SlintOverlay {
    fn show(&mut self, privacy_mode: bool, hotkey_str: &str) -> Result<(), String> {
        let hotkey_str = format!("Press {} to unlock", hotkey_str);
        
        if let Some(ui_weak) = &self.ui_weak {
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.set_is_privacy(privacy_mode);
                ui.set_hotkey_text(slint::SharedString::from(hotkey_str));
                ui.window().set_fullscreen(true);
                let _ = ui.window().show();
            });
            return Ok(());
        }

        Err("Slint UI was not successfully initialized.".to_string())
    }

    fn hide(&mut self) -> Result<(), String> {
        if let Some(ui_weak) = &self.ui_weak {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                let _ = ui.window().hide();
            });
        }
        log::info!("Slint overlay hidden");
        Ok(())
    }

    fn is_visible(&self) -> bool {
        self.ui_weak.as_ref().map_or(false, |w| w.upgrade().is_some())
    }
}

impl Drop for SlintOverlay {
    fn drop(&mut self) {
        let _ = self.hide();
    }
}
