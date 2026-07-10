use global_hotkey::GlobalHotKeyEvent;

fn main() {
    let event = GlobalHotKeyEvent {
        id: 0,
        state: global_hotkey::HotKeyState::Pressed,
    };
    if event.state == global_hotkey::HotKeyState::Pressed {
        println!("Pressed");
    }
}
