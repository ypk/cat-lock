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
        Self { ui_weak: None }
    }
}

impl OverlayWindow for SlintOverlay {
    fn show(&mut self, privacy_mode: bool, hotkey_str: &str) -> Result<(), String> {
        let hotkey_str = format!("Press {} to unlock", hotkey_str);
        
        // If it's already running, just update and show
        if let Some(ui_weak) = &self.ui_weak {
            if let Some(_ui) = ui_weak.upgrade() {
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.set_is_privacy(privacy_mode);
                    ui.set_hotkey_text(slint::SharedString::from(hotkey_str));
                    let _ = ui.window().show();
                });
                return Ok(());
            }
        }

        let hotkey_str_clone = hotkey_str.clone();
        
        let (tx, rx) = std::sync::mpsc::channel();
        
        std::thread::spawn(move || {
            let ui = match LockWindow::new() {
                Ok(ui) => ui,
                Err(e) => {
                    let _ = tx.send(Err(format!("Failed to create Slint window: {}", e)));
                    return;
                }
            };
            
            ui.set_is_privacy(privacy_mode);
            ui.set_hotkey_text(slint::SharedString::from(hotkey_str_clone));
            
            if tx.send(Ok(ui.as_weak())).is_ok() {
                let _ = ui.run();
            }
        });
        
        self.ui_weak = Some(rx.recv().map_err(|e| format!("Channel error: {}", e))??);
        
        Ok(())
    }

    fn hide(&mut self) -> Result<(), String> {
        if let Some(ui_weak) = &self.ui_weak {
            let _ = ui_weak.upgrade_in_event_loop(|ui| {
                let _ = ui.window().hide();
            });
            // Clear the weak ref so next show() spawns a new window.
            self.ui_weak = None;
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
