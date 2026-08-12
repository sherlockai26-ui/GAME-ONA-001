use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;

pub fn start_input_thread(host: String, port: u16, tx: mpsc::Sender<Value>) -> Result<(), String> {
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr).map_err(|e| format!("No se pudo conectar: {}", e))?;
    println!("[Input] Conectado al Bridge en {}", addr);

    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            match line {
                Ok(raw) => {
                    if let Ok(json) = serde_json::from_str::<Value>(&raw) {
                        let _ = tx.send(json);
                    }
                }
                Err(e) => {
                    eprintln!("[Input] Error leyendo del Bridge: {}", e);
                    break;
                }
            }
        }
        println!("[Input] Conexion cerrada.");
    });

    Ok(())
}
