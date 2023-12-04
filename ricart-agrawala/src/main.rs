pub mod algorithm;
pub mod receiver;

use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self, sleep},
    time::Duration,
};

use algorithm::{ask_access, RicAgrawala, RicAgrawalaActor};
use clap::Parser;
use color_eyre::eyre::Result;
use receiver::receive_thread;
use renraku_node::NodeArguments;
use tracing::{info, Level};

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // Node configuration
    let configuration = renraku_node::configure(NodeArguments::try_parse()?)?;

    // Begins
    let variables = Arc::from(Mutex::new(RicAgrawala::default()));
    let permission = Arc::from(Condvar::new());
    let configuration = Arc::new(configuration);

    let t = (variables.clone(), permission.clone(), configuration.clone());
    thread::spawn(move || receive_thread(t.0, t.1, t.2));

    loop {
        sleep(Duration::from_millis(rand::random::<u64>() % 5000));
        let mut lock = variables.lock().unwrap();
        // Ask for permission
        lock.ask(configuration.clone())?;
        drop(lock);
        // Waits for permission
        let mut lock = permission.wait(variables.lock().unwrap()).unwrap();
        info!("👍 Entering critical section");
        // We are in critical section
        sleep(Duration::from_millis(rand::random::<u64>() % 5000));
        info!("👍 Leaving critical section and sending authorization to others");
        lock.free(configuration.clone())?;
    }
}
