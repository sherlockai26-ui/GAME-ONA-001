use crate::diagnostic::{Diagnostic, TestStatus};
use crate::input_client::InputWorkerMessage;
use crate::rendering::{draw_rect, draw_string};
use crate::runtime::{RuntimeConfig, EXPECTED_PROTOCOL_VERSION};
use pixels::{Pixels, SurfaceTexture};
use std::sync::mpsc;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

pub fn run(
    rx: mpsc::Receiver<InputWorkerMessage>,
    config: RuntimeConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("GAME ONA 001  Input Diagnostic")
        .with_inner_size(PhysicalSize::new(800, 600))
        .build(&event_loop)?;

    let mut pixels = {
        let surface_texture = SurfaceTexture::new(800, 600, &window);
        Pixels::new(800, 600, surface_texture)?
    };

    let mut diagnostic = Diagnostic::new(&config);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        if keycode == VirtualKeyCode::Escape && input.state == ElementState::Pressed
                        {
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
                while let Ok(message) = rx.try_recv() {
                    diagnostic.handle_worker_message(message);
                }

                clear(&mut pixels);
                draw_diagnostic(&mut pixels, &diagnostic, &config);

                if let Err(error) = pixels.render() {
                    eprintln!("Error render: {}", error);
                    *control_flow = ControlFlow::Exit;
                }

                window.request_redraw();
            }
            _ => {}
        }
    });
}

fn clear(pixels: &mut Pixels) {
    for pixel in pixels.frame_mut().iter_mut() {
        *pixel = 0;
    }
}

fn draw_diagnostic(pixels: &mut Pixels, diagnostic: &Diagnostic, config: &RuntimeConfig) {
    draw_string(
        pixels,
        10,
        10,
        "GAME ONA 001 - ONA RUNTIME V1 DIAGNOSTIC",
        [255, 255, 255],
    );

    draw_status_line(pixels, 10, 34, "ONA Runtime", diagnostic.runtime_status);
    if let Some(reason) = &diagnostic.runtime_reason {
        draw_reason(pixels, 10, 48, reason);
    }

    let protocol_text = format!(
        "Protocol Version: {} / expected {}",
        diagnostic.protocol_display, EXPECTED_PROTOCOL_VERSION
    );
    draw_string(pixels, 10, 66, &protocol_text, [200, 200, 200]);
    draw_status_badge(pixels, 250, 66, diagnostic.protocol_status);
    if let Some(reason) = &diagnostic.protocol_reason {
        draw_reason(pixels, 10, 80, reason);
    }

    let bridge_text = format!("Input Bridge: {}", diagnostic.bridge_status);
    draw_string(
        pixels,
        10,
        98,
        &bridge_text,
        bridge_color(&diagnostic.bridge_status),
    );
    draw_status_badge(pixels, 250, 98, diagnostic.input_bridge_status);
    if let Some(reason) = &diagnostic.bridge_reason {
        draw_reason(pixels, 10, 112, reason);
    }

    let player_text = match diagnostic.player_id {
        Some(player_id) => format!("Player ID: {}", player_id),
        None => "Player ID: WAITING".to_string(),
    };
    draw_string(pixels, 10, 134, &player_text, [200, 200, 200]);

    let activity_text = if diagnostic.input_active {
        "Input Activity: ACTIVE"
    } else {
        "Input Activity: WAITING"
    };
    draw_string(pixels, 10, 150, activity_text, [200, 200, 200]);

    let endpoint_text = format!(
        "Bridge Env: {}:{}",
        config.input_host.as_deref().unwrap_or("<missing>"),
        config.input_port_raw.as_deref().unwrap_or("<missing>")
    );
    draw_string(pixels, 10, 166, &endpoint_text, [130, 130, 130]);

    let joy_txt = format!(
        "Joystick: X={:.2}  Y={:.2}",
        diagnostic.joystick_x, diagnostic.joystick_y
    );
    draw_string(pixels, 10, 194, &joy_txt, [100, 200, 255]);

    let mut ypos = 222;
    for button in Diagnostic::buttons() {
        let pressed = diagnostic.is_pressed(button);
        let color = if pressed {
            [0, 255, 0]
        } else {
            [100, 100, 100]
        };
        draw_rect(pixels, 10, ypos, 14, 14, color);
        let label = if pressed { "DOWN" } else { "UP" };
        let text = format!("{:<6} {}", button.to_uppercase(), label);
        draw_string(pixels, 32, ypos, &text, [200, 200, 200]);
        ypos += 20;
    }

    let last_txt = format!("Last event: {}", diagnostic.last_event);
    draw_string(pixels, 10, 444, &last_txt, [150, 150, 150]);

    draw_string(pixels, 430, 34, "INTEGRATION TEST", [255, 255, 255]);
    let mut test_y = 62;
    draw_test_line(
        pixels,
        430,
        test_y,
        "ONA Runtime",
        diagnostic.runtime_status,
    );
    test_y += 18;
    draw_test_line(
        pixels,
        430,
        test_y,
        "Protocol Version",
        diagnostic.protocol_status,
    );
    test_y += 18;
    draw_test_line(
        pixels,
        430,
        test_y,
        "Input Bridge",
        diagnostic.input_bridge_status,
    );
    test_y += 18;
    draw_test_line(
        pixels,
        430,
        test_y,
        "Player ID",
        diagnostic.player_id_status,
    );
    test_y += 18;
    draw_test_line(
        pixels,
        430,
        test_y,
        "Joystick X/Y",
        diagnostic.joystick_status,
    );
    test_y += 28;

    for button in Diagnostic::buttons() {
        draw_test_line(
            pixels,
            430,
            test_y,
            &button.to_uppercase(),
            diagnostic.button_status(button),
        );
        test_y += 18;
    }

    test_y += 10;
    draw_test_line(pixels, 430, test_y, "Pressed", diagnostic.pressed_status);
    test_y += 18;
    draw_test_line(pixels, 430, test_y, "Released", diagnostic.released_status);

    if diagnostic.all_tests_passed() {
        draw_rect(pixels, 430, 520, 330, 58, [0, 70, 25]);
        draw_string(pixels, 444, 530, "RESULT: ALL TESTS PASSED", [0, 255, 0]);
        draw_string(pixels, 444, 548, "ONA GAME RUNTIME V1", [255, 255, 255]);
        draw_string(pixels, 444, 564, "INTEGRATION VERIFIED", [255, 255, 255]);
    } else {
        draw_string(pixels, 430, 548, "RESULT: TEST IN PROGRESS", [255, 255, 0]);
        draw_string(
            pixels,
            430,
            566,
            "WAITING ITEMS ARE NOT FAILURES",
            [150, 150, 150],
        );
    }
}

fn draw_status_line(pixels: &mut Pixels, x: usize, y: usize, label: &str, status: TestStatus) {
    let text = format!("{}:", label);
    draw_string(pixels, x, y, &text, [200, 200, 200]);
    draw_status_badge(pixels, x + 170, y, status);
}

fn draw_test_line(pixels: &mut Pixels, x: usize, y: usize, label: &str, status: TestStatus) {
    let text = format!("{:.<24}", label);
    draw_string(pixels, x, y, &text, [200, 200, 200]);
    draw_status_badge(pixels, x + 230, y, status);
}

fn draw_status_badge(pixels: &mut Pixels, x: usize, y: usize, status: TestStatus) {
    draw_string(pixels, x, y, status.label(), status.color());
}

fn draw_reason(pixels: &mut Pixels, x: usize, y: usize, reason: &str) {
    let text = format!("Reason: {}", reason);
    draw_string(pixels, x, y, &text, [255, 130, 70]);
}

fn bridge_color(status: &str) -> [u8; 3] {
    match status {
        "CONNECTED" => [0, 255, 0],
        "CONNECTING" => [255, 255, 0],
        "DISCONNECTED" => [255, 90, 90],
        _ => [180, 180, 180],
    }
}
