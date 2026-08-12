mod app;
mod diagnostic;
mod events;
mod font8x8;
mod input_client;
mod rendering;
mod runtime;

use std::sync::mpsc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = runtime::RuntimeConfig::from_env();
    let (tx, rx) = mpsc::channel();

    if config.can_start_input_worker() {
        input_client::start_input_thread(
            config.input_host.clone().unwrap(),
            config.input_port.unwrap(),
            tx,
        );
    }

    app::run(rx, config)?;

    Ok(())
}
