use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum InputEvent {
    Joystick {
        #[serde(rename = "playerId")]
        player_id: u32,
        x: f32,
        y: f32,
    },
    Button {
        #[serde(rename = "playerId")]
        player_id: u32,
        button: String,
        state: String,
    },
}
