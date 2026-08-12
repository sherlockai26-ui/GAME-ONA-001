use crate::events::InputEvent;
use crate::input_client::InputWorkerMessage;
use crate::runtime::{RuntimeConfig, EXPECTED_PROTOCOL_VERSION};
use std::collections::HashMap;

const BUTTONS: [&str; 10] = [
    "A", "B", "X", "Y", "L1", "L2", "R1", "R2", "Select", "Start",
];
const JOYSTICK_DEAD_ZONE: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Waiting,
    Pass,
    Fail,
}

impl TestStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Waiting => "WAITING",
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
        }
    }

    pub fn color(self) -> [u8; 3] {
        match self {
            Self::Waiting => [180, 180, 180],
            Self::Pass => [0, 255, 0],
            Self::Fail => [255, 70, 70],
        }
    }
}

pub struct Diagnostic {
    pub joystick_x: f32,
    pub joystick_y: f32,
    pub buttons: HashMap<String, bool>,
    pub last_event: String,
    pub bridge_status: String,
    pub bridge_reason: Option<String>,
    pub runtime_status: TestStatus,
    pub runtime_reason: Option<String>,
    pub protocol_status: TestStatus,
    pub protocol_reason: Option<String>,
    pub protocol_display: String,
    pub input_bridge_status: TestStatus,
    pub player_id_status: TestStatus,
    pub joystick_status: TestStatus,
    pub pressed_status: TestStatus,
    pub released_status: TestStatus,
    pub button_statuses: HashMap<String, TestStatus>,
    pub player_id: Option<u32>,
    pub input_active: bool,
}

impl Diagnostic {
    pub fn new(config: &RuntimeConfig) -> Self {
        let mut button_statuses = HashMap::new();
        for button in BUTTONS {
            button_statuses.insert(button.to_string(), TestStatus::Waiting);
        }

        let runtime_status = if config.runtime_ok() {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        };
        let runtime_reason = if config.runtime_ok() {
            None
        } else {
            Some("ONA_RUNTIME was not provided or was not 1".to_string())
        };

        let protocol_status = if config.protocol_ok() {
            TestStatus::Pass
        } else {
            TestStatus::Fail
        };
        let protocol_reason = if config.protocol_ok() {
            None
        } else {
            Some(format!(
                "Expected: {}, Received: {}",
                EXPECTED_PROTOCOL_VERSION,
                config.protocol_raw.as_deref().unwrap_or("<missing>")
            ))
        };
        let protocol_display = config
            .protocol_raw
            .clone()
            .unwrap_or_else(|| "<missing>".to_string());

        let bridge_reason = if config.runtime_ok() && config.protocol_ok() {
            match (
                &config.input_host,
                &config.input_port_raw,
                config.input_port,
            ) {
                (None, _, _) => Some("ONA_INPUT_HOST was not provided".to_string()),
                (_, None, _) => Some("ONA_INPUT_PORT was not provided".to_string()),
                (_, Some(raw), None) => Some(format!("ONA_INPUT_PORT is invalid: {}", raw)),
                _ => None,
            }
        } else {
            Some("Waiting for valid runtime and protocol configuration".to_string())
        };

        Self {
            joystick_x: 0.0,
            joystick_y: 0.0,
            buttons: HashMap::new(),
            last_event: "None".to_string(),
            bridge_status: "NOT STARTED".to_string(),
            bridge_reason,
            runtime_status,
            runtime_reason,
            protocol_status,
            protocol_reason,
            protocol_display,
            input_bridge_status: TestStatus::Waiting,
            player_id_status: TestStatus::Waiting,
            joystick_status: TestStatus::Waiting,
            pressed_status: TestStatus::Waiting,
            released_status: TestStatus::Waiting,
            button_statuses,
            player_id: None,
            input_active: false,
        }
    }

    pub fn handle_worker_message(&mut self, message: InputWorkerMessage) {
        match message {
            InputWorkerMessage::Connecting => {
                self.bridge_status = "CONNECTING".to_string();
            }
            InputWorkerMessage::Connected => {
                self.bridge_status = "CONNECTED".to_string();
                self.bridge_reason = None;
                self.input_bridge_status = TestStatus::Pass;
            }
            InputWorkerMessage::Disconnected(reason) => {
                self.bridge_status = "DISCONNECTED".to_string();
                self.bridge_reason = Some(reason);
            }
            InputWorkerMessage::InvalidJson(reason) => {
                self.last_event = reason.clone();
                self.bridge_reason = Some(reason);
            }
            InputWorkerMessage::Event(json) => match serde_json::from_value::<InputEvent>(json) {
                Ok(event) => self.update(&event),
                Err(error) => {
                    self.last_event = format!("Invalid event payload: {}", error);
                }
            },
        }
    }

    pub fn update(&mut self, event: &InputEvent) {
        self.input_active = true;

        match event {
            InputEvent::Joystick { player_id, x, y } => {
                self.joystick_x = *x;
                self.joystick_y = *y;
                self.mark_player(*player_id);
                if x.abs() > JOYSTICK_DEAD_ZONE || y.abs() > JOYSTICK_DEAD_ZONE {
                    self.joystick_status = TestStatus::Pass;
                }
                self.last_event = format!("Joystick p{}: {:.2}, {:.2}", player_id, x, y);
            }
            InputEvent::Button {
                player_id,
                button,
                state,
            } => {
                self.mark_player(*player_id);
                let is_down = matches!(state.as_str(), "down" | "pressed");
                let is_up = matches!(state.as_str(), "up" | "released");

                if is_down || is_up {
                    self.buttons.insert(button.clone(), is_down);
                    if let Some(status) = self.button_statuses.get_mut(button) {
                        *status = TestStatus::Pass;
                    }
                }
                if is_down {
                    self.pressed_status = TestStatus::Pass;
                }
                if is_up {
                    self.released_status = TestStatus::Pass;
                }

                self.last_event = format!("Button p{}: {} {}", player_id, button, state);
            }
        }
    }

    pub fn is_pressed(&self, btn: &str) -> bool {
        *self.buttons.get(btn).unwrap_or(&false)
    }

    pub fn button_status(&self, btn: &str) -> TestStatus {
        *self
            .button_statuses
            .get(btn)
            .unwrap_or(&TestStatus::Waiting)
    }

    pub fn all_tests_passed(&self) -> bool {
        self.runtime_status == TestStatus::Pass
            && self.protocol_status == TestStatus::Pass
            && self.input_bridge_status == TestStatus::Pass
            && self.player_id_status == TestStatus::Pass
            && self.joystick_status == TestStatus::Pass
            && self.pressed_status == TestStatus::Pass
            && self.released_status == TestStatus::Pass
            && BUTTONS
                .iter()
                .all(|button| self.button_status(button) == TestStatus::Pass)
    }

    pub fn buttons() -> &'static [&'static str] {
        &BUTTONS
    }

    fn mark_player(&mut self, player_id: u32) {
        self.player_id = Some(player_id);
        self.player_id_status = TestStatus::Pass;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> RuntimeConfig {
        RuntimeConfig {
            runtime_raw: Some("1".to_string()),
            protocol_raw: Some("1".to_string()),
            input_host: Some("127.0.0.1".to_string()),
            input_port_raw: Some("47191".to_string()),
            input_port: Some(47191),
        }
    }

    #[test]
    fn joystick_needs_real_movement() {
        let mut diagnostic = Diagnostic::new(&valid_config());
        diagnostic.update(&InputEvent::Joystick {
            player_id: 1,
            x: 0.0,
            y: 0.0,
        });
        assert_eq!(diagnostic.joystick_status, TestStatus::Waiting);

        diagnostic.update(&InputEvent::Joystick {
            player_id: 1,
            x: 0.25,
            y: 0.0,
        });
        assert_eq!(diagnostic.joystick_status, TestStatus::Pass);
    }

    #[test]
    fn pressed_and_released_are_tracked_separately() {
        let mut diagnostic = Diagnostic::new(&valid_config());
        diagnostic.update(&InputEvent::Button {
            player_id: 1,
            button: "A".to_string(),
            state: "down".to_string(),
        });
        assert_eq!(diagnostic.pressed_status, TestStatus::Pass);
        assert_eq!(diagnostic.released_status, TestStatus::Waiting);

        diagnostic.update(&InputEvent::Button {
            player_id: 1,
            button: "A".to_string(),
            state: "up".to_string(),
        });
        assert_eq!(diagnostic.released_status, TestStatus::Pass);
    }

    #[test]
    fn full_minimum_sequence_passes_all_tests() {
        let mut diagnostic = Diagnostic::new(&valid_config());
        diagnostic.handle_worker_message(InputWorkerMessage::Connected);
        diagnostic.update(&InputEvent::Joystick {
            player_id: 1,
            x: 0.25,
            y: -0.25,
        });

        for button in BUTTONS {
            diagnostic.update(&InputEvent::Button {
                player_id: 1,
                button: button.to_string(),
                state: "down".to_string(),
            });
            diagnostic.update(&InputEvent::Button {
                player_id: 1,
                button: button.to_string(),
                state: "up".to_string(),
            });
        }

        assert!(diagnostic.all_tests_passed());
    }
}
