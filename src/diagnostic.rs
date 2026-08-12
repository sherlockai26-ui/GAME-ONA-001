use crate::events::InputEvent;
use std::collections::HashMap;

pub struct Diagnostic {
    pub joystick_x: f32,
    pub joystick_y: f32,
    pub buttons: HashMap<String, bool>,
    pub last_event: String,
    pub connected: bool,
    pub runtime_ok: bool,
    pub player_id: u32,
}

impl Diagnostic {
    pub fn new() -> Self {
        Self {
            joystick_x: 0.0,
            joystick_y: 0.0,
            buttons: HashMap::new(),
            last_event: "Ninguno".to_string(),
            connected: false,
            runtime_ok: false,
            player_id: 0,
        }
    }

    pub fn update(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Joystick { player_id, x, y } => {
                self.joystick_x = *x;
                self.joystick_y = *y;
                self.player_id = *player_id;
                self.last_event = format!("Joystick p{}: {:.2}, {:.2}", player_id, x, y);
                self.connected = true;
            }
            InputEvent::Button {
                player_id,
                button,
                state,
            } => {
                let pressed = state == "down" || state == "pressed";
                self.buttons.insert(button.clone(), pressed);
                self.player_id = *player_id;
                self.last_event = format!("Button p{}: {} {}", player_id, button, state);
                self.connected = true;
            }
        }
    }

    pub fn is_pressed(&self, btn: &str) -> bool {
        *self.buttons.get(btn).unwrap_or(&false)
    }
}
