use crate::diagnostic::{
    is_button_down_state, is_button_up_state, normalize_button_name, Diagnostic, TestStatus,
};
use crate::events::InputEvent;
use crate::input_client::InputWorkerMessage;
use crate::lifecycle_client::{LifecycleEvent, LifecycleMessage, LifecycleStatus};
use crate::rendering::{draw_rect, draw_string};
use crate::runtime::{DisplayMode, EnvValue, RuntimeConfig, EXPECTED_PROTOCOL_VERSION};
use pixels::{Pixels, SurfaceTexture};
use std::sync::mpsc;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::monitor::MonitorHandle;
use winit::window::{Fullscreen, WindowBuilder};

const INTERNAL_WIDTH: u32 = 800;
const INTERNAL_HEIGHT: u32 = 600;
const JOYSTICK_MENU_DEADZONE: f32 = 0.55;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameState {
    Running,
    PausedMenu,
    Controls,
}

#[derive(Clone, Copy)]
enum MenuOption {
    Resume,
    RestartTest,
    Controls,
    ExitGame,
}

impl MenuOption {
    fn label(self) -> &'static str {
        match self {
            Self::Resume => "RESUME",
            Self::RestartTest => "RESTART TEST",
            Self::Controls => "CONTROLS",
            Self::ExitGame => "EXIT GAME",
        }
    }
}

const MENU_OPTIONS: [MenuOption; 4] = [
    MenuOption::Resume,
    MenuOption::RestartTest,
    MenuOption::Controls,
    MenuOption::ExitGame,
];

struct MenuState {
    game_state: GameState,
    selected_index: usize,
    axis_latched: bool,
    start_down: bool,
    menu_render_logged: bool,
}

pub fn run(
    input_rx: mpsc::Receiver<InputWorkerMessage>,
    lifecycle_status_rx: mpsc::Receiver<LifecycleStatus>,
    lifecycle_tx: mpsc::Sender<LifecycleMessage>,
    config: RuntimeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();
    let mut diagnostic = Diagnostic::new(&config);
    let display_target = resolve_display_target(&event_loop, &config);
    let mut builder = WindowBuilder::new().with_title("GAME ONA 001  Input Diagnostic");

    match &display_target {
        DisplayTarget::Matched {
            monitor,
            mode,
            position,
            size,
            ..
        } => {
            diagnostic.mark_display_match(
                monitor_label(monitor),
                available_monitor_labels(&event_loop),
            );
            builder = match mode {
                DisplayMode::ConsoleFullscreen => builder
                    .with_fullscreen(Some(Fullscreen::Borderless(Some(monitor.clone()))))
                    .with_inner_size(*size),
                DisplayMode::Windowed => builder.with_position(*position).with_inner_size(*size),
            };
        }
        DisplayTarget::NoMatch { reason } => {
            diagnostic
                .mark_display_match_failed(reason.clone(), available_monitor_labels(&event_loop));
            if let Some((x, y, width, height)) = config.display.target_rect() {
                builder = builder
                    .with_position(PhysicalPosition::new(x, y))
                    .with_inner_size(PhysicalSize::new(width, height));
            } else {
                builder =
                    builder.with_inner_size(PhysicalSize::new(INTERNAL_WIDTH, INTERNAL_HEIGHT));
            }
        }
        DisplayTarget::DevelopmentWindow => {
            builder = builder.with_inner_size(PhysicalSize::new(INTERNAL_WIDTH, INTERNAL_HEIGHT));
        }
    }

    let window = builder.build(&event_loop)?;
    let _ = lifecycle_tx.send(LifecycleMessage::Send(LifecycleEvent::WINDOW_READY));

    if matches!(display_target, DisplayTarget::Matched { .. }) {
        let _ = lifecycle_tx.send(LifecycleMessage::Send(LifecycleEvent::DISPLAY_READY));
    }

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(INTERNAL_WIDTH, INTERNAL_HEIGHT, surface_texture)?
    };
    let mut game_ready_sent = false;
    let mut exiting_sent = false;
    let mut menu = MenuState {
        game_state: GameState::Running,
        selected_index: 0,
        axis_latched: false,
        start_down: false,
        menu_render_logged: false,
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    send_exiting_once(&lifecycle_tx, &mut exiting_sent);
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        if keycode == VirtualKeyCode::Escape && input.state == ElementState::Pressed
                        {
                            send_exiting_once(&lifecycle_tx, &mut exiting_sent);
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
                WindowEvent::Resized(size) => {
                    let _ = pixels.resize_surface(size.width, size.height);
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                while let Ok(message) = input_rx.try_recv() {
                    match message {
                        InputWorkerMessage::Event(json) => {
                            match serde_json::from_value::<InputEvent>(json) {
                                Ok(input_event) => {
                                    handle_game_input(
                                        &input_event,
                                        &mut menu,
                                        &mut diagnostic,
                                        &lifecycle_tx,
                                        &mut exiting_sent,
                                        control_flow,
                                    );
                                    diagnostic.update(&input_event);
                                }
                                Err(error) => {
                                    diagnostic.last_event =
                                        format!("Invalid event payload: {}", error);
                                }
                            }
                        }
                        other => diagnostic.handle_worker_message(other),
                    }
                }
                while let Ok(status) = lifecycle_status_rx.try_recv() {
                    diagnostic.handle_lifecycle_status(status);
                }

                clear(&mut pixels);
                draw_diagnostic(&mut pixels, &diagnostic, &config);
                draw_game_state_overlay(&mut pixels, menu.game_state, menu.selected_index);
                log_menu_rendered(&mut menu);

                match pixels.render() {
                    Ok(()) => {
                        if !game_ready_sent {
                            let _ =
                                lifecycle_tx.send(LifecycleMessage::Send(LifecycleEvent::READY));
                            game_ready_sent = true;
                        }
                    }
                    Err(error) => {
                        eprintln!("Error render: {}", error);
                        send_exiting_once(&lifecycle_tx, &mut exiting_sent);
                        *control_flow = ControlFlow::Exit;
                    }
                }

                window.request_redraw();
            }
            _ => {}
        }
    });
}

fn log_menu_rendered(menu: &mut MenuState) {
    if matches!(menu.game_state, GameState::PausedMenu) {
        if !menu.menu_render_logged {
            println!("[GAME MENU] rendered");
            menu.menu_render_logged = true;
        }
    } else {
        menu.menu_render_logged = false;
    }
}

fn handle_game_input(
    event: &InputEvent,
    menu: &mut MenuState,
    diagnostic: &mut Diagnostic,
    lifecycle_tx: &mpsc::Sender<LifecycleMessage>,
    exiting_sent: &mut bool,
    control_flow: &mut ControlFlow,
) {
    match event {
        InputEvent::Joystick { y, .. } => {
            if matches!(menu.game_state, GameState::PausedMenu) {
                if *y < -JOYSTICK_MENU_DEADZONE && !menu.axis_latched {
                    if menu.selected_index == 0 {
                        menu.selected_index = MENU_OPTIONS.len() - 1;
                    } else {
                        menu.selected_index -= 1;
                    }
                    menu.axis_latched = true;
                } else if *y > JOYSTICK_MENU_DEADZONE && !menu.axis_latched {
                    menu.selected_index = (menu.selected_index + 1) % MENU_OPTIONS.len();
                    menu.axis_latched = true;
                } else if y.abs() < 0.25 {
                    menu.axis_latched = false;
                }
            }
        }
        InputEvent::Button { button, state, .. } => {
            let button_name = normalize_button_name(button);
            let is_down = is_button_down_state(state);
            let is_up = is_button_up_state(state);

            log_special_button_input(&button_name, is_down, is_up);

            if button_name == "Start" {
                handle_start_input(menu, is_down, is_up);
                return;
            }

            if !is_down {
                return;
            }

            match button_name.as_str() {
                "B" => match menu.game_state {
                    GameState::PausedMenu => {
                        menu.game_state = GameState::Running;
                    }
                    GameState::Controls => menu.game_state = GameState::PausedMenu,
                    GameState::Running => {}
                },
                "A" if matches!(menu.game_state, GameState::PausedMenu) => {
                    match MENU_OPTIONS[menu.selected_index] {
                        MenuOption::Resume => menu.game_state = GameState::Running,
                        MenuOption::RestartTest => {
                            diagnostic.restart_input_test();
                            menu.game_state = GameState::Running;
                        }
                        MenuOption::Controls => menu.game_state = GameState::Controls,
                        MenuOption::ExitGame => {
                            send_exiting_once(lifecycle_tx, exiting_sent);
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn log_special_button_input(button_name: &str, is_down: bool, is_up: bool) {
    match button_name {
        "Start" if is_down => println!("[GAME INPUT] START DOWN received"),
        "Start" if is_up => println!("[GAME INPUT] START UP received"),
        "Select" if is_down => println!("[GAME INPUT] SELECT DOWN received"),
        "Select" if is_up => println!("[GAME INPUT] SELECT UP received"),
        _ => {}
    }
}

fn handle_start_input(menu: &mut MenuState, is_down: bool, is_up: bool) {
    if is_up {
        menu.start_down = false;
        return;
    }

    if !is_down || menu.start_down {
        return;
    }

    menu.start_down = true;
    println!("[GAME INPUT] START pressed edge");

    match menu.game_state {
        GameState::Running => {
            println!("[GAME STATE] Running -> PausedMenu");
            menu.game_state = GameState::PausedMenu;
            menu.selected_index = 0;
            menu.menu_render_logged = false;
        }
        GameState::PausedMenu | GameState::Controls => {
            menu.game_state = GameState::Running;
        }
    }
}

fn send_exiting_once(lifecycle_tx: &mpsc::Sender<LifecycleMessage>, exiting_sent: &mut bool) {
    if !*exiting_sent {
        let _ = lifecycle_tx.send(LifecycleMessage::Send(LifecycleEvent::EXITING));
        *exiting_sent = true;
    }
}

enum DisplayTarget {
    Matched {
        monitor: MonitorHandle,
        mode: DisplayMode,
        position: PhysicalPosition<i32>,
        size: PhysicalSize<u32>,
    },
    NoMatch {
        reason: String,
    },
    DevelopmentWindow,
}

fn resolve_display_target(event_loop: &EventLoop<()>, config: &RuntimeConfig) -> DisplayTarget {
    let Some((x, y, width, height)) = config.display.target_rect() else {
        return if config.runtime_ok() {
            DisplayTarget::NoMatch {
                reason: config
                    .display
                    .data_problem()
                    .unwrap_or_else(|| "Gaming Display target data missing".to_string()),
            }
        } else {
            DisplayTarget::DevelopmentWindow
        };
    };

    let mode = match config.display.mode {
        EnvValue::Valid(mode) => mode,
        _ if config.runtime_ok() => {
            return DisplayTarget::NoMatch {
                reason: config
                    .display
                    .data_problem()
                    .unwrap_or_else(|| "ONA_DISPLAY_MODE missing or invalid".to_string()),
            }
        }
        _ => DisplayMode::Windowed,
    };

    for monitor in event_loop.available_monitors() {
        let position = monitor.position();
        let size = monitor.size();
        if position.x == x && position.y == y && size.width == width && size.height == height {
            return DisplayTarget::Matched {
                monitor,
                mode,
                position: PhysicalPosition::new(x, y),
                size: PhysicalSize::new(width, height),
            };
        }
    }

    DisplayTarget::NoMatch {
        reason: format!("Expected monitor at {},{} size {}x{}", x, y, width, height),
    }
}

fn available_monitor_labels(event_loop: &EventLoop<()>) -> Vec<String> {
    event_loop
        .available_monitors()
        .map(|monitor| monitor_label(&monitor))
        .collect()
}

fn monitor_label(monitor: &MonitorHandle) -> String {
    let position = monitor.position();
    let size = monitor.size();
    let name = monitor.name().unwrap_or_else(|| "<unnamed>".to_string());
    format!(
        "{} @ {},{} {}x{}",
        name, position.x, position.y, size.width, size.height
    )
}

fn clear(pixels: &mut Pixels) {
    for pixel in pixels.frame_mut().iter_mut() {
        *pixel = 0;
    }
}

fn draw_game_state_overlay(pixels: &mut Pixels, game_state: GameState, selected_menu_index: usize) {
    match game_state {
        GameState::Running => {}
        GameState::PausedMenu => {
            draw_rect(pixels, 245, 135, 310, 190, [18, 18, 18]);
            draw_rect(pixels, 245, 135, 310, 2, [0, 180, 255]);
            draw_string(pixels, 344, 158, "GAME MENU", [255, 255, 255]);

            let mut y = 190;
            for (index, option) in MENU_OPTIONS.iter().enumerate() {
                let selected = index == selected_menu_index;
                let prefix = if selected { "> " } else { "  " };
                let color = if selected {
                    [0, 255, 0]
                } else {
                    [200, 200, 200]
                };
                draw_string(
                    pixels,
                    315,
                    y,
                    &format!("{}{}", prefix, option.label()),
                    color,
                );
                y += 26;
            }
        }
        GameState::Controls => {
            draw_rect(pixels, 210, 115, 380, 300, [18, 18, 18]);
            draw_rect(pixels, 210, 115, 380, 2, [0, 180, 255]);
            draw_string(pixels, 340, 138, "CONTROLS", [255, 255, 255]);
            draw_string(
                pixels,
                250,
                174,
                "Joystick: navigate / diagnostic axes",
                [200, 200, 200],
            );
            draw_string(pixels, 250, 198, "A: select menu option", [200, 200, 200]);
            draw_string(pixels, 250, 222, "B: resume / back", [200, 200, 200]);
            draw_string(pixels, 250, 246, "X/Y: diagnostic buttons", [200, 200, 200]);
            draw_string(
                pixels,
                250,
                270,
                "L1/L2/R1/R2: diagnostic buttons",
                [200, 200, 200],
            );
            draw_string(
                pixels,
                250,
                294,
                "SELECT: reserved contextual input",
                [200, 200, 200],
            );
            draw_string(
                pixels,
                250,
                318,
                "START: open/resume GAME MENU",
                [200, 200, 200],
            );
            draw_string(pixels, 250, 358, "Press B or START to return", [0, 255, 0]);
        }
    }
}

fn draw_diagnostic(pixels: &mut Pixels, diagnostic: &Diagnostic, config: &RuntimeConfig) {
    draw_string(
        pixels,
        10,
        8,
        "GAME ONA 001 - DISPLAY/LIFECYCLE DIAGNOSTIC",
        [255, 255, 255],
    );

    draw_status_line(pixels, 10, 28, "ONA Runtime", diagnostic.runtime_status);
    if let Some(reason) = &diagnostic.runtime_reason {
        draw_reason(pixels, 10, 44, reason);
    }
    draw_string(
        pixels,
        10,
        52,
        &format!(
            "Protocol: {} / expected {}",
            diagnostic.protocol_display, EXPECTED_PROTOCOL_VERSION
        ),
        [200, 200, 200],
    );
    draw_status_badge(pixels, 250, 52, diagnostic.protocol_status);
    if let Some(reason) = &diagnostic.protocol_reason {
        draw_reason(pixels, 10, 68, reason);
    }

    let bridge_text = format!("Input Bridge: {}", diagnostic.bridge_status);
    draw_string(
        pixels,
        10,
        78,
        &bridge_text,
        bridge_color(&diagnostic.bridge_status),
    );
    draw_status_badge(pixels, 250, 78, diagnostic.input_bridge_status);

    draw_string(pixels, 10, 92, "GAMING DISPLAY", [255, 255, 255]);
    draw_string(
        pixels,
        10,
        110,
        &format!("ID: {}", config.display.id.as_label()),
        [200, 200, 200],
    );
    draw_string(
        pixels,
        10,
        126,
        &format!("Name: {}", config.display.name.as_label()),
        [200, 200, 200],
    );
    draw_string(
        pixels,
        10,
        142,
        &format!(
            "Target: {},{} {}x{}",
            config.display.x.as_label(),
            config.display.y.as_label(),
            config.display.width.as_label(),
            config.display.height.as_label()
        ),
        [200, 200, 200],
    );
    draw_string(
        pixels,
        10,
        158,
        &format!("Mode: {}", config.display.mode.as_label()),
        [200, 200, 200],
    );
    draw_test_line(
        pixels,
        10,
        178,
        "Display Data",
        diagnostic.display_data_status,
    );
    draw_test_line(
        pixels,
        10,
        196,
        "Monitor Match",
        diagnostic.display_match_status,
    );
    draw_string(
        pixels,
        10,
        214,
        &format!("Matched: {}", diagnostic.matched_monitor),
        [130, 130, 130],
    );
    if let Some(reason) = &diagnostic.display_match_reason {
        draw_reason(pixels, 10, 232, reason);
    } else if let Some(reason) = &diagnostic.display_data_reason {
        draw_reason(pixels, 10, 232, reason);
    }

    draw_string(pixels, 10, 258, "LIFECYCLE", [255, 255, 255]);
    draw_string(
        pixels,
        10,
        276,
        &format!("Bridge: {}", diagnostic.lifecycle_bridge_label),
        bridge_color(&diagnostic.lifecycle_bridge_label),
    );
    draw_status_badge(pixels, 250, 276, diagnostic.lifecycle_bridge_status);
    draw_test_line(
        pixels,
        10,
        296,
        "GAME_STARTED",
        diagnostic.game_started_status,
    );
    draw_test_line(
        pixels,
        10,
        314,
        "WINDOW_READY",
        diagnostic.game_window_ready_status,
    );
    draw_test_line(
        pixels,
        10,
        332,
        "DISPLAY_READY",
        diagnostic.game_display_ready_status,
    );
    draw_test_line(pixels, 10, 350, "GAME_READY", diagnostic.game_ready_status);
    draw_test_line(
        pixels,
        10,
        368,
        "GAME_EXITING",
        diagnostic.game_exiting_status,
    );
    if let Some(reason) = &diagnostic.lifecycle_reason {
        draw_reason(pixels, 10, 386, reason);
    }

    let player_text = match diagnostic.player_id {
        Some(player_id) => format!("Player ID: {}", player_id),
        None => "Player ID: WAITING".to_string(),
    };
    draw_string(pixels, 10, 404, &player_text, [200, 200, 200]);
    draw_string(
        pixels,
        10,
        418,
        &format!(
            "Player Env: {}",
            config.player_id_raw.as_deref().unwrap_or("<missing>")
        ),
        [130, 130, 130],
    );
    draw_string(
        pixels,
        10,
        432,
        &format!(
            "Joystick: X={:.2}  Y={:.2}",
            diagnostic.joystick_x, diagnostic.joystick_y
        ),
        [100, 200, 255],
    );

    let mut ypos = 450;
    for button in Diagnostic::buttons() {
        let pressed = diagnostic.is_pressed(button);
        draw_rect(
            pixels,
            10,
            ypos,
            12,
            12,
            if pressed {
                [0, 255, 0]
            } else {
                [100, 100, 100]
            },
        );
        draw_string(
            pixels,
            30,
            ypos,
            &format!(
                "{:<6} {}",
                button.to_uppercase(),
                if pressed { "DOWN" } else { "UP" }
            ),
            [200, 200, 200],
        );
        ypos += 13;
    }

    draw_string(pixels, 360, 28, "INTEGRATION TEST", [255, 255, 255]);
    let mut test_y = 50;
    for (label, status) in [
        ("ONA Runtime", diagnostic.runtime_status),
        ("Protocol", diagnostic.protocol_status),
        ("Input Bridge", diagnostic.input_bridge_status),
        ("Gaming Display Data", diagnostic.display_data_status),
        ("Gaming Display Match", diagnostic.display_match_status),
        ("Lifecycle Bridge", diagnostic.lifecycle_bridge_status),
        ("Game Started", diagnostic.game_started_status),
        ("Game Window Ready", diagnostic.game_window_ready_status),
        ("Game Display Ready", diagnostic.game_display_ready_status),
        ("Game Ready", diagnostic.game_ready_status),
        ("Player ID", diagnostic.player_id_status),
        ("Joystick X/Y", diagnostic.joystick_status),
    ] {
        draw_test_line(pixels, 360, test_y, label, status);
        test_y += 18;
    }

    test_y += 10;
    for button in Diagnostic::buttons() {
        draw_test_line(
            pixels,
            360,
            test_y,
            &button.to_uppercase(),
            diagnostic.button_status(button),
        );
        test_y += 16;
    }

    test_y += 8;
    draw_test_line(pixels, 360, test_y, "Pressed", diagnostic.pressed_status);
    test_y += 18;
    draw_test_line(pixels, 360, test_y, "Released", diagnostic.released_status);

    draw_string(
        pixels,
        10,
        582,
        &format!("Last event: {}", diagnostic.last_event),
        [150, 150, 150],
    );

    if diagnostic.all_tests_passed() {
        draw_rect(pixels, 360, 520, 390, 54, [0, 70, 25]);
        draw_string(pixels, 374, 530, "RESULT: ALL TESTS PASSED", [0, 255, 0]);
        draw_string(
            pixels,
            374,
            548,
            "ONA GAME RUNTIME V1 VERIFIED",
            [255, 255, 255],
        );
    } else {
        draw_string(pixels, 360, 548, "RESULT: TEST IN PROGRESS", [255, 255, 0]);
        draw_string(
            pixels,
            360,
            566,
            "WAITING ITEMS ARE NOT FAILURES",
            [150, 150, 150],
        );
    }
}

fn draw_status_line(pixels: &mut Pixels, x: usize, y: usize, label: &str, status: TestStatus) {
    draw_string(pixels, x, y, &format!("{}:", label), [200, 200, 200]);
    draw_status_badge(pixels, x + 170, y, status);
}

fn draw_test_line(pixels: &mut Pixels, x: usize, y: usize, label: &str, status: TestStatus) {
    draw_string(pixels, x, y, &format!("{:.<24}", label), [200, 200, 200]);
    draw_status_badge(pixels, x + 230, y, status);
}

fn draw_status_badge(pixels: &mut Pixels, x: usize, y: usize, status: TestStatus) {
    draw_string(pixels, x, y, status.label(), status.color());
}

fn draw_reason(pixels: &mut Pixels, x: usize, y: usize, reason: &str) {
    draw_string(pixels, x, y, &format!("Reason: {}", reason), [255, 130, 70]);
}

fn bridge_color(status: &str) -> [u8; 3] {
    match status {
        "CONNECTED" => [0, 255, 0],
        "CONNECTING" => [255, 255, 0],
        "DISCONNECTED" => [255, 90, 90],
        _ => [180, 180, 180],
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
            lifecycle_host: Some("127.0.0.1".to_string()),
            lifecycle_port_raw: Some("48191".to_string()),
            lifecycle_port: Some(48191),
            player_id_raw: Some("1".to_string()),
            player_id: Some(1),
            display: crate::runtime::DisplayConfig {
                id: EnvValue::Valid("mock-display".to_string()),
                target: EnvValue::Valid("mock".to_string()),
                name: EnvValue::Valid("Mock Display".to_string()),
                x: EnvValue::Valid(0),
                y: EnvValue::Valid(0),
                width: EnvValue::Valid(800),
                height: EnvValue::Valid(600),
                scale_factor: EnvValue::Valid(1.0),
                mode: EnvValue::Valid(DisplayMode::Windowed),
            },
        }
    }

    fn menu_state() -> MenuState {
        MenuState {
            game_state: GameState::Running,
            selected_index: 0,
            axis_latched: false,
            start_down: false,
            menu_render_logged: false,
        }
    }

    fn button(button: &str, state: &str) -> InputEvent {
        InputEvent::Button {
            player_id: 1,
            button: button.to_string(),
            state: state.to_string(),
        }
    }

    fn joystick(y: f32) -> InputEvent {
        InputEvent::Joystick {
            player_id: 1,
            x: 0.0,
            y,
        }
    }

    fn apply_input(event: &InputEvent, menu: &mut MenuState, diagnostic: &mut Diagnostic) {
        let (lifecycle_tx, _lifecycle_rx) = mpsc::channel();
        let mut exiting_sent = false;
        let mut control_flow = ControlFlow::Poll;
        handle_game_input(
            event,
            menu,
            diagnostic,
            &lifecycle_tx,
            &mut exiting_sent,
            &mut control_flow,
        );
        diagnostic.update(event);
    }

    #[test]
    fn running_start_down_opens_paused_menu() {
        let mut menu = menu_state();
        let mut diagnostic = Diagnostic::new(&valid_config());

        apply_input(&button("START", "down"), &mut menu, &mut diagnostic);

        assert_eq!(menu.game_state, GameState::PausedMenu);
        assert_eq!(menu.selected_index, 0);
        assert!(menu.start_down);
        assert!(diagnostic.is_pressed("Start"));
    }

    #[test]
    fn holding_start_does_not_toggle_repeatedly() {
        let mut menu = menu_state();
        let mut diagnostic = Diagnostic::new(&valid_config());

        apply_input(&button("START", "down"), &mut menu, &mut diagnostic);
        apply_input(&button("START", "down"), &mut menu, &mut diagnostic);

        assert_eq!(menu.game_state, GameState::PausedMenu);
        assert!(menu.start_down);
    }

    #[test]
    fn start_up_does_not_change_state() {
        let mut menu = menu_state();
        let mut diagnostic = Diagnostic::new(&valid_config());

        apply_input(&button("START", "down"), &mut menu, &mut diagnostic);
        apply_input(&button("START", "up"), &mut menu, &mut diagnostic);

        assert_eq!(menu.game_state, GameState::PausedMenu);
        assert!(!menu.start_down);
        assert!(!diagnostic.is_pressed("Start"));
    }

    #[test]
    fn paused_menu_start_down_resumes_running() {
        let mut menu = menu_state();
        let mut diagnostic = Diagnostic::new(&valid_config());

        apply_input(&button("START", "down"), &mut menu, &mut diagnostic);
        apply_input(&button("START", "up"), &mut menu, &mut diagnostic);
        apply_input(&button("START", "down"), &mut menu, &mut diagnostic);

        assert_eq!(menu.game_state, GameState::Running);
    }

    #[test]
    fn select_down_up_does_not_change_game_state() {
        let mut menu = menu_state();
        let mut diagnostic = Diagnostic::new(&valid_config());

        apply_input(&button("SELECT", "down"), &mut menu, &mut diagnostic);
        apply_input(&button("SELECT", "up"), &mut menu, &mut diagnostic);

        assert_eq!(menu.game_state, GameState::Running);
        assert_eq!(diagnostic.button_status("Select"), TestStatus::Pass);
    }

    #[test]
    fn menu_a_b_and_joystick_still_work() {
        let mut menu = menu_state();
        let mut diagnostic = Diagnostic::new(&valid_config());

        apply_input(&button("START", "down"), &mut menu, &mut diagnostic);
        apply_input(&button("START", "up"), &mut menu, &mut diagnostic);
        apply_input(&joystick(0.8), &mut menu, &mut diagnostic);
        apply_input(&joystick(0.8), &mut menu, &mut diagnostic);
        assert_eq!(menu.selected_index, 1);

        apply_input(&joystick(0.0), &mut menu, &mut diagnostic);
        apply_input(&joystick(0.8), &mut menu, &mut diagnostic);
        assert_eq!(menu.selected_index, 2);

        apply_input(&button("A", "down"), &mut menu, &mut diagnostic);
        assert_eq!(menu.game_state, GameState::Controls);

        apply_input(&button("B", "down"), &mut menu, &mut diagnostic);
        assert_eq!(menu.game_state, GameState::PausedMenu);

        apply_input(&button("B", "down"), &mut menu, &mut diagnostic);
        assert_eq!(menu.game_state, GameState::Running);
    }

    #[test]
    fn lifecycle_labels_remain_unchanged() {
        assert_eq!(LifecycleEvent::STARTED.label(), "GAME_STARTED");
        assert_eq!(LifecycleEvent::WINDOW_READY.label(), "GAME_WINDOW_READY");
        assert_eq!(LifecycleEvent::DISPLAY_READY.label(), "GAME_DISPLAY_READY");
        assert_eq!(LifecycleEvent::READY.label(), "GAME_READY");
        assert_eq!(LifecycleEvent::EXITING.label(), "GAME_EXITING");
    }

    #[test]
    fn normal_input_still_updates_diagnostic() {
        let mut menu = menu_state();
        let mut diagnostic = Diagnostic::new(&valid_config());

        apply_input(&joystick(-0.8), &mut menu, &mut diagnostic);
        apply_input(&button("A", "down"), &mut menu, &mut diagnostic);

        assert_eq!(menu.game_state, GameState::Running);
        assert_eq!(diagnostic.joystick_status, TestStatus::Pass);
        assert!(diagnostic.is_pressed("A"));
        assert_eq!(diagnostic.button_status("A"), TestStatus::Pass);
    }
}
