mod app;
mod diagnostic;
mod events;
mod font8x8;
mod input_client;
mod rendering;

use std::env;
use std::sync::mpsc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = env::var("ONA_RUNTIME").unwrap_or_else(|_| "0".to_string());
    let protocol = env::var("ONA_PROTOCOL_VERSION").unwrap_or_else(|_| "1".to_string());
    let host = env::var("ONA_INPUT_HOST").unwrap_or_else(|_| "".to_string());
    let port_str = env::var("ONA_INPUT_PORT").unwrap_or_else(|_| "".to_string());

    let runtime_ok = runtime == "1";

    if runtime_ok {
        if host.is_empty() || port_str.is_empty() {
            eprintln!(
                "[GAME ONA 001] ERROR: ONA_RUNTIME=1 pero falta ONA_INPUT_HOST o ONA_INPUT_PORT"
            );
            eprintln!("[GAME ONA 001] Este juego solo debe ejecutarse bajo ONA Runtime V1.");
            std::process::exit(1);
        }
        if protocol != "1" {
            eprintln!(
                "[GAME ONA 001] ERROR: ONA_PROTOCOL_VERSION debe ser 1 (actual: {})",
                protocol
            );
            std::process::exit(1);
        }
    } else {
        println!("[GAME ONA 001] Modo standalone: sin conexion a Input Bridge.");
        println!("[GAME ONA 001] Para usar con ONA, ejecutar bajo ONA Runtime V1.");
    }

    let port: u16 = if runtime_ok {
        port_str.parse().expect("ONA_INPUT_PORT debe ser un numero")
    } else {
        0
    };

    println!(
        "[GAME ONA 001] Runtime: {}, Protocol: {}",
        runtime, protocol
    );
    if runtime_ok {
        println!("[GAME ONA 001] Input Bridge: {}:{}", host, port);
    }

    let (tx, rx) = mpsc::channel();

    if runtime_ok {
        if let Err(e) = input_client::start_input_thread(host, port, tx) {
            eprintln!("[GAME ONA 001] Error iniciando cliente de input: {}", e);
            std::process::exit(1);
        }
    } else {
        println!("[GAME ONA 001] Ejecutando en modo offline (sin input real).");
    }

    app::run(rx, runtime_ok)?;

    Ok(())
}
