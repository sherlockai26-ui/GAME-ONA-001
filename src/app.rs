use crate::diagnostic::Diagnostic;
use crate::events::InputEvent;
use crate::rendering::{draw_rect, draw_string};
use pixels::{Pixels, SurfaceTexture};
use serde_json::Value;
use std::sync::mpsc;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

pub fn run(rx: mpsc::Receiver<Value>, runtime_ok: bool) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("GAME ONA 001  Input Diagnostic")
        .with_inner_size(PhysicalSize::new(800, 600))
        .build(&event_loop)?;

    let mut pixels = {
        let surface_texture = SurfaceTexture::new(800, 600, &window);
        Pixels::new(800, 600, surface_texture)?
    };

    let mut diagnostic = Diagnostic::new();
    diagnostic.runtime_ok = runtime_ok;

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
                while let Ok(json) = rx.try_recv() {
                    if let Ok(event) = serde_json::from_value::<InputEvent>(json) {
                        diagnostic.update(&event);
                    }
                }

                let frame = pixels.frame_mut();
                for pixel in frame.iter_mut() {
                    *pixel = 0;
                }

                draw_string(
                    &mut pixels,
                    10,
                    10,
                    "GAME ONA 001  Diagnostic",
                    [255, 255, 255],
                );

                let runtime_txt = if diagnostic.runtime_ok {
                    "ONA Runtime: OK"
                } else {
                    "ONA Runtime: NO DETECTADO"
                };
                let color = if diagnostic.runtime_ok {
                    [0, 255, 0]
                } else {
                    [255, 255, 0]
                };
                draw_string(&mut pixels, 10, 30, runtime_txt, color);

                let conn_txt = if diagnostic.connected {
                    "Input Bridge: CONECTADO"
                } else {
                    "Input Bridge: DESCONECTADO"
                };
                let color2 = if diagnostic.connected {
                    [0, 255, 0]
                } else {
                    [255, 0, 0]
                };
                draw_string(&mut pixels, 10, 50, conn_txt, color2);

                let pid_txt = format!("Player ID: {}", diagnostic.player_id);
                draw_string(&mut pixels, 10, 70, &pid_txt, [200, 200, 200]);

                let joy_txt = format!(
                    "Joystick: X={:.2}, Y={:.2}",
                    diagnostic.joystick_x, diagnostic.joystick_y
                );
                draw_string(&mut pixels, 10, 100, &joy_txt, [100, 200, 255]);

                let buttons = [
                    ("A", "A"),
                    ("B", "B"),
                    ("X", "X"),
                    ("Y", "Y"),
                    ("L1", "L1"),
                    ("L2", "L2"),
                    ("R1", "R1"),
                    ("R2", "R2"),
                    ("Select", "SELECT"),
                    ("Start", "START"),
                ];
                let mut ypos = 140;
                for (key, label) in buttons.iter() {
                    let pressed = diagnostic.is_pressed(key);
                    let color = if pressed {
                        [0, 255, 0]
                    } else {
                        [100, 100, 100]
                    };
                    draw_rect(&mut pixels, 10, ypos, 16, 16, color);
                    let label_txt = format!(
                        "{}: {}",
                        label,
                        if pressed { "PRESSED" } else { "RELEASED" }
                    );
                    draw_string(&mut pixels, 32, ypos, &label_txt, [200, 200, 200]);
                    ypos += 26;
                }

                let last_txt = format!("Last event: {}", diagnostic.last_event);
                draw_string(&mut pixels, 10, ypos + 20, &last_txt, [150, 150, 150]);

                if let Err(e) = pixels.render() {
                    eprintln!("Error render: {}", e);
                    *control_flow = ControlFlow::Exit;
                }

                window.request_redraw();
            }
            _ => {}
        }
    });
}
