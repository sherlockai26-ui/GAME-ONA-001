mod app;
mod diagnostic;
mod events;
mod font8x8;
mod input_client;
mod lifecycle_client;
mod rendering;
mod runtime;

use lifecycle_client::{LifecycleEvent, LifecycleMessage};
use std::sync::mpsc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = runtime::RuntimeConfig::from_env();
    let (input_tx, input_rx) = mpsc::channel();
    let (lifecycle_status_tx, lifecycle_status_rx) = mpsc::channel();
    let (lifecycle_tx, lifecycle_rx) = mpsc::channel();

    if config.can_start_input_worker() {
        input_client::start_input_thread(
            config.input_host.clone().unwrap(),
            config.input_port.unwrap(),
            input_tx,
        );
    }

    if config.can_start_lifecycle_worker() {
        lifecycle_client::start_lifecycle_thread(
            config.lifecycle_host.clone().unwrap(),
            config.lifecycle_port.unwrap(),
            lifecycle_rx,
            lifecycle_status_tx,
        );
        let _ = lifecycle_tx.send(LifecycleMessage::Send(LifecycleEvent::Started));
    }

    app::run(input_rx, lifecycle_status_rx, lifecycle_tx, config)?;

    Ok(())
}
