use std::io::Write;
use std::net::TcpStream;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const CONNECT_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    Started,
    WindowReady,
    DisplayReady,
    Ready,
    Exiting,
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
                            Ok(()) => {
                                let _ = connection.flush();
                                let _ = status_tx.send(LifecycleStatus::Sent(event));
                            }
                            Err(error) => {
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
    pub fn label(self) -> &'static str {
        match self {
            Self::Started => "GAME_STARTED",
            Self::WindowReady => "GAME_WINDOW_READY",
            Self::DisplayReady => "GAME_DISPLAY_READY",
            Self::Ready => "GAME_READY",
            Self::Exiting => "GAME_EXITING",
        }
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
