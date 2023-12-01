use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Condvar, Mutex},
    thread::{self, sleep},
    time::Duration,
};

use clap::Parser;
use color_eyre::eyre::{Error, Result};
use renraku_node::NodeArguments;
use renraku_shared::NodeId;
use selecting::{AsRawFd, Selector};
use serde::{Deserialize, Serialize};
use tracing::{info, level_filters::LevelFilter, Level};

#[derive(Debug, Clone, PartialEq, Eq)]
enum CriticalState {
    Out,
    Asking,
    In,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Message {
    Request { clock: usize, requester: NodeId },
    Permission { authorizer: NodeId },
}

#[derive(Debug, Clone)]
struct Variables {
    pub critical_state: CriticalState,
    pub clock: usize,
    pub last: usize,
    pub prioritized: bool,
    pub differed: Vec<NodeId>,
    pub awaited: HashSet<NodeId>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // Node configuration
    let configuration = renraku_node::configure(NodeArguments::try_parse()?)?;

    // Begins
    let variables = Arc::from(Mutex::new(Variables::default()));
    let permission = Arc::from(Condvar::new());
    let configuration = Arc::new(configuration);

    // Thread for receiving
    let receiver_variables = variables.clone();
    let receiver_permission = permission.clone();
    let receiver_config = configuration.clone();

    // Creates reception thread
    thread::spawn(move || {
        receiver(receiver_config, receiver_variables, receiver_permission).unwrap()
    });

    sleep(Duration::from_secs(rand::random::<u64>() % 5));

    // Waits...
    loop {
        ask_access(configuration.clone(), variables.clone(), permission.clone())?;
        info!("📌 Entering critical state...");
        sleep(Duration::from_secs(4));
        info!("📌 Leaving critical state!");
        free_access(configuration.clone(), variables.clone())?;
    }
}

impl Default for Variables {
    fn default() -> Self {
        Self {
            awaited: HashSet::new(),
            critical_state: CriticalState::Out,
            clock: 0,
            last: 0,
            prioritized: false,
            differed: Vec::new(),
        }
    }
}

fn ask_access(
    config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>,
    mutex: Arc<Mutex<Variables>>,
    permission_cond: Arc<Condvar>,
) -> Result<()> {
    let mut v = mutex.lock().expect("Mutex locking should work");
    v.critical_state = CriticalState::Asking;
    v.awaited = (1..config.0 + 1)
        .map(NodeId)
        .filter(|n| n.0 != config.1 .0)
        .collect();
    v.clock += 1;
    v.last = v.clock;
    let clock = v.clock.clone();

    drop(v);
    for (id, mut stream) in config.2.iter() {
        stream.write_all(&bincode::serialize(&Message::Request {
            clock,
            requester: config.1.clone(),
        })?)?;
        info!("❓ Requesting access to {:?}", id);
    }

    v = permission_cond
        .wait(mutex.lock().expect("Mutex locking should work"))
        .expect("Permission cond waiting should work");

    v.critical_state = CriticalState::In;

    Ok(())
}

fn free_access(
    config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>,
    mutex: Arc<Mutex<Variables>>,
) -> Result<()> {
    let mut v = mutex.lock().expect("Mutex locking should work");
    v.critical_state = CriticalState::Out;
    for message in v.differed.iter() {
        let mut stream = config.2.get(message).expect("Impossible ?");
        stream.write_all(&bincode::serialize(&Message::Permission {
            authorizer: config.1.clone(),
        })?)?;
    }
    info!("🧹 Freeing resources... {:?} are now allowed", v.differed);
    v.differed.clear();
    Ok(())
}

fn receiver(
    config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>,
    mutex: Arc<Mutex<Variables>>,
    permission_cond: Arc<Condvar>,
) -> Result<()> {
    loop {
        let streams: Vec<&TcpStream> = config
            .2
            .iter()
            .map(|(_, stream)| stream.to_owned())
            .collect();
        // Select
        let mut selector = Selector::new();
        streams
            .iter()
            .for_each(|stream| selector.add_read(stream.to_owned()));

        let result = selector.select()?;
        let mut v = mutex.lock().expect("Mutex locking should work.");
        // Looks for message
        for stream in streams.iter() {
            if !result.is_read(stream.to_owned()) {
                continue;
            }
            let mut buf = [0; 1024];
            stream.to_owned().read(&mut buf)?;
            let message = bincode::deserialize::<Message>(&buf)?;
            match message {
                Message::Request { clock, requester } => {
                    if clock == 0 || requester.0 == 0 {
                        break;
                    }
                    info!(
                        "📭 Received a message from {} for permission (clock {}).",
                        requester.0, clock
                    );
                    v.clock = clock.max(v.clock);
                    v.prioritized = v.critical_state != CriticalState::Out && v.last < clock;
                    if v.prioritized {
                        v.differed.push(requester);
                    } else {
                        stream
                            .to_owned()
                            .write(&bincode::serialize(&Message::Permission {
                                authorizer: config.1.clone(),
                            })?)?;
                    }
                }
                Message::Permission { authorizer } => match v.critical_state.clone() {
                    CriticalState::Asking => {
                        if authorizer.0 == 0 {
                            break;
                        }

                        v.awaited.remove(&authorizer);
                        info!(
                            "📭 Received a permission from {}. Waiting for {:?}",
                            authorizer.0, v.awaited
                        );
                        if v.awaited.is_empty() {
                            permission_cond.notify_one();
                        }
                    }
                    _ => {}
                },
            }
        }

        drop(v);
    }
}
