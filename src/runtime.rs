use std::{env, fmt};

pub const EXPECTED_PROTOCOL_VERSION: &str = "1";

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub runtime_raw: Option<String>,
    pub protocol_raw: Option<String>,
    pub input_host: Option<String>,
    pub input_port_raw: Option<String>,
    pub input_port: Option<u16>,
    pub lifecycle_host: Option<String>,
    pub lifecycle_port_raw: Option<String>,
    pub lifecycle_port: Option<u16>,
    pub player_id_raw: Option<String>,
    pub player_id: Option<u32>,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone)]
pub struct DisplayConfig {
    pub id: EnvValue<String>,
    pub target: EnvValue<String>,
    pub name: EnvValue<String>,
    pub x: EnvValue<i32>,
    pub y: EnvValue<i32>,
    pub width: EnvValue<u32>,
    pub height: EnvValue<u32>,
    pub scale_factor: EnvValue<f64>,
    pub mode: EnvValue<DisplayMode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnvValue<T> {
    Missing,
    Invalid(String),
    Valid(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    ConsoleFullscreen,
    Windowed,
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
        let lifecycle_host = env::var("ONA_LIFECYCLE_HOST")
            .ok()
            .filter(|host| !host.is_empty());
        let lifecycle_port_raw = env::var("ONA_LIFECYCLE_PORT").ok();
        let lifecycle_port = lifecycle_port_raw
            .as_deref()
            .and_then(|port| port.parse::<u16>().ok());
        let player_id_raw = env::var("ONA_PLAYER_ID").ok();
        let player_id = player_id_raw
            .as_deref()
            .and_then(|player_id| player_id.parse::<u32>().ok());

        Self {
            runtime_raw,
            protocol_raw,
            input_host,
            input_port_raw,
            input_port,
            lifecycle_host,
            lifecycle_port_raw,
            lifecycle_port,
            player_id_raw,
            player_id,
            display: DisplayConfig::from_env(),
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

    pub fn can_start_lifecycle_worker(&self) -> bool {
        self.runtime_ok()
            && self.protocol_ok()
            && self.lifecycle_host.is_some()
            && self.lifecycle_port.is_some()
    }
}

impl DisplayConfig {
    fn from_env() -> Self {
        Self {
            id: read_string("ONA_DISPLAY_ID"),
            target: read_string("ONA_DISPLAY_TARGET"),
            name: read_string("ONA_DISPLAY_NAME"),
            x: read_parse("ONA_DISPLAY_X"),
            y: read_parse("ONA_DISPLAY_Y"),
            width: read_parse("ONA_DISPLAY_WIDTH"),
            height: read_parse("ONA_DISPLAY_HEIGHT"),
            scale_factor: read_parse("ONA_DISPLAY_SCALE_FACTOR"),
            mode: read_display_mode("ONA_DISPLAY_MODE"),
        }
    }

    pub fn has_valid_target_rect(&self) -> bool {
        matches!(self.x, EnvValue::Valid(_))
            && matches!(self.y, EnvValue::Valid(_))
            && matches!(self.width, EnvValue::Valid(_))
            && matches!(self.height, EnvValue::Valid(_))
    }

    pub fn target_rect(&self) -> Option<(i32, i32, u32, u32)> {
        match (&self.x, &self.y, &self.width, &self.height) {
            (
                EnvValue::Valid(x),
                EnvValue::Valid(y),
                EnvValue::Valid(width),
                EnvValue::Valid(height),
            ) => Some((*x, *y, *width, *height)),
            _ => None,
        }
    }

    pub fn display_data_ok(&self) -> bool {
        self.has_valid_target_rect()
            && matches!(self.mode, EnvValue::Valid(_))
            && !matches!(self.id, EnvValue::Invalid(_))
            && !matches!(self.target, EnvValue::Invalid(_))
            && !matches!(self.name, EnvValue::Invalid(_))
            && !matches!(self.scale_factor, EnvValue::Invalid(_))
    }

    pub fn data_problem(&self) -> Option<String> {
        let mut problems = Vec::new();
        push_problem(&mut problems, "ONA_DISPLAY_X", &self.x);
        push_problem(&mut problems, "ONA_DISPLAY_Y", &self.y);
        push_problem(&mut problems, "ONA_DISPLAY_WIDTH", &self.width);
        push_problem(&mut problems, "ONA_DISPLAY_HEIGHT", &self.height);
        push_problem(&mut problems, "ONA_DISPLAY_MODE", &self.mode);
        push_problem(
            &mut problems,
            "ONA_DISPLAY_SCALE_FACTOR",
            &self.scale_factor,
        );

        if problems.is_empty() {
            None
        } else {
            Some(problems.join("; "))
        }
    }
}

impl DisplayMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::ConsoleFullscreen => "CONSOLE_FULLSCREEN",
            Self::Windowed => "WINDOWED",
        }
    }
}

impl<T> EnvValue<T> {
    pub fn as_label(&self) -> String
    where
        T: ToString,
    {
        match self {
            Self::Missing => "<missing>".to_string(),
            Self::Invalid(value) => format!("<invalid:{}>", value),
            Self::Valid(value) => value.to_string(),
        }
    }
}

impl fmt::Display for DisplayMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

fn read_string(key: &str) -> EnvValue<String> {
    match env::var(key) {
        Ok(value) if value.is_empty() => EnvValue::Invalid(value),
        Ok(value) => EnvValue::Valid(value),
        Err(_) => EnvValue::Missing,
    }
}

fn read_parse<T>(key: &str) -> EnvValue<T>
where
    T: std::str::FromStr,
{
    match env::var(key) {
        Ok(value) => match value.parse::<T>() {
            Ok(parsed) => EnvValue::Valid(parsed),
            Err(_) => EnvValue::Invalid(value),
        },
        Err(_) => EnvValue::Missing,
    }
}

fn read_display_mode(key: &str) -> EnvValue<DisplayMode> {
    match env::var(key) {
        Ok(value) => match value.as_str() {
            "CONSOLE_FULLSCREEN" => EnvValue::Valid(DisplayMode::ConsoleFullscreen),
            "WINDOWED" => EnvValue::Valid(DisplayMode::Windowed),
            _ => EnvValue::Invalid(value),
        },
        Err(_) => EnvValue::Missing,
    }
}

fn push_problem<T>(problems: &mut Vec<String>, key: &str, value: &EnvValue<T>) {
    match value {
        EnvValue::Missing => problems.push(format!("{} missing", key)),
        EnvValue::Invalid(raw) => problems.push(format!("{} invalid: {}", key, raw)),
        EnvValue::Valid(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_target_rect_requires_all_numeric_values() {
        let display = DisplayConfig {
            id: EnvValue::Valid("display-1".to_string()),
            target: EnvValue::Valid("tv".to_string()),
            name: EnvValue::Valid("TV".to_string()),
            x: EnvValue::Valid(1920),
            y: EnvValue::Valid(0),
            width: EnvValue::Valid(3840),
            height: EnvValue::Valid(2160),
            scale_factor: EnvValue::Valid(1.0),
            mode: EnvValue::Valid(DisplayMode::ConsoleFullscreen),
        };

        assert_eq!(display.target_rect(), Some((1920, 0, 3840, 2160)));
        assert!(display.display_data_ok());
    }
}
