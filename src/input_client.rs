use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const RECONNECT_DELAY: Duration = Duration::from_millis(1000);

#[derive(Debug)]
pub enum InputWorkerMessage {
    Connecting,
    Connected,
    Disconnected(String),
    Event(Value),
    InvalidJson(String),
}

pub fn start_input_thread(host: String, port: u16, tx: mpsc::Sender<InputWorkerMessage>) {
    thread::spawn(move || {
        let addr = format!("{}:{}", host, port);

        loop {
            if tx.send(InputWorkerMessage::Connecting).is_err() {
                break;
            }

            match TcpStream::connect(&addr) {
                Ok(stream) => {
                    if tx.send(InputWorkerMessage::Connected).is_err() {
                        break;
                    }

                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        match line {
                            Ok(raw) => match serde_json::from_str::<Value>(&raw) {
                                Ok(json) => {
                                    if tx.send(InputWorkerMessage::Event(json)).is_err() {
                                        return;
                                    }
                                }
                                Err(error) => {
                                    let message = format!("Invalid JSON: {}", error);
                                    if tx.send(InputWorkerMessage::InvalidJson(message)).is_err() {
                                        return;
                                    }
                                }
                            },
                            Err(error) => {
                                let message = format!("Read error: {}", error);
                                if tx.send(InputWorkerMessage::Disconnected(message)).is_err() {
                                    return;
                                }
                                break;
                            }
                        }
                    }

                    if tx
                        .send(InputWorkerMessage::Disconnected(
                            "Connection closed by Input Bridge".to_string(),
                        ))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(error) => {
                    let message = format!("Connection could not be established: {}", error);
                    if tx.send(InputWorkerMessage::Disconnected(message)).is_err() {
                        break;
                    }
                }
            }

            thread::sleep(RECONNECT_DELAY);
        }
    });
}
