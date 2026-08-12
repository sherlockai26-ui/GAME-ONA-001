use std::env;

pub const EXPECTED_PROTOCOL_VERSION: &str = "1";

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub runtime_raw: Option<String>,
    pub protocol_raw: Option<String>,
    pub input_host: Option<String>,
    pub input_port_raw: Option<String>,
    pub input_port: Option<u16>,
}

impl RuntimeConfig {
    pub fn from_env() -> Self {
        let runtime_raw = env::var("ONA_RUNTIME").ok();
        let protocol_raw = env::var("ONA_PROTOCOL_VERSION").ok();
        let input_host = env::var("ONA_INPUT_HOST")
            .ok()
            .filter(|host| !host.is_empty());
        let input_port_raw = env::var("ONA_INPUT_PORT").ok();
        let input_port = input_port_raw
            .as_deref()
            .and_then(|port| port.parse::<u16>().ok());

        Self {
            runtime_raw,
            protocol_raw,
            input_host,
            input_port_raw,
            input_port,
        }
    }

    pub fn runtime_ok(&self) -> bool {
        self.runtime_raw.as_deref() == Some("1")
    }

    pub fn protocol_ok(&self) -> bool {
        self.protocol_raw.as_deref() == Some(EXPECTED_PROTOCOL_VERSION)
    }

    pub fn can_start_input_worker(&self) -> bool {
        self.runtime_ok()
            && self.protocol_ok()
            && self.input_host.is_some()
            && self.input_port.is_some()
    }
}
