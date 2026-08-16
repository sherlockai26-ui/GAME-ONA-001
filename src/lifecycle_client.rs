use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const CONNECT_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleEvent {
    label: &'static str,
}

#[derive(Debug)]
pub enum LifecycleMessage {
    Send(LifecycleEvent),
}

#[derive(Debug)]
pub enum LifecycleStatus {
    Connecting,
    Connected,
    Disconnected(String),
    Sent(LifecycleEvent),
}

pub fn start_lifecycle_thread(
    host: String,
    port: u16,
    outbound_rx: mpsc::Receiver<LifecycleMessage>,
    status_tx: mpsc::Sender<LifecycleStatus>,
) {
    thread::spawn(move || {
        let addr = format!("{}:{}", host, port);
        let mut stream = None;

        while let Ok(message) = outbound_rx.recv() {
            match message {
                LifecycleMessage::Send(event) => {
                    if stream.is_none() {
                        let _ = status_tx.send(LifecycleStatus::Connecting);
                        stream = Some(connect(&addr, &status_tx));
                    }

                    if let Some(connection) = stream.as_mut() {
                        let line = format!("{}\n", event.label());
                        match connection.write_all(line.as_bytes()) {
                            Ok(()) => match connection.flush() {
                                Ok(()) => {
                                    println!("{} SENT", event.label());
                                    let _ = status_tx.send(LifecycleStatus::Sent(event));
                                }
                                Err(error) => {
                                    eprintln!("{} SEND FAILED: {}", event.label(), error);
                                    let _ = status_tx.send(LifecycleStatus::Disconnected(format!(
                                        "Lifecycle flush failed: {}",
                                        error
                                    )));
                                    stream = None;
                                }
                            },
                            Err(error) => {
                                eprintln!("{} SEND FAILED: {}", event.label(), error);
                                let _ = status_tx.send(LifecycleStatus::Disconnected(format!(
                                    "Lifecycle send failed: {}",
                                    error
                                )));
                                stream = None;
                            }
                        }
                    }
                }
            }
        }
    });
}

impl LifecycleEvent {
    pub const STARTED: Self = Self {
        label: "GAME_STARTED",
    };
    pub const WINDOW_READY: Self = Self {
        label: "GAME_WINDOW_READY",
    };
    pub const DISPLAY_READY: Self = Self {
        label: "GAME_DISPLAY_READY",
    };
    pub const READY: Self = Self {
        label: "GAME_READY",
    };
    pub const EXITING: Self = Self {
        label: "GAME_EXITING",
    };

    pub fn label(self) -> &'static str {
        self.label
    }
}

fn connect(addr: &str, status_tx: &mpsc::Sender<LifecycleStatus>) -> TcpStream {
    loop {
        match TcpStream::connect(addr) {
            Ok(stream) => {
                let _ = status_tx.send(LifecycleStatus::Connected);
                return stream;
            }
            Err(error) => {
                let _ = status_tx.send(LifecycleStatus::Disconnected(format!(
                    "Lifecycle connection failed: {}",
                    error
                )));
                thread::sleep(CONNECT_DELAY);
            }
        }
    }
}
