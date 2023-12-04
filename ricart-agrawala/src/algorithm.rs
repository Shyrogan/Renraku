use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    net::TcpStream,
    sync::{Arc, Condvar, Mutex, MutexGuard},
};

use color_eyre::eyre::Result;
use renraku_shared::NodeId;
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Message {
    Request { date: usize, requester: NodeId },
    Permission { authorizer: NodeId },
}

impl Message {
    pub fn send_to(self, mut stream: &TcpStream) -> Result<()> {
        stream.to_owned().write_all(&bincode::serialize(&self)?)?;
        Ok(())
    }

    pub fn receive_from(mut stream: &TcpStream) -> Result<Message> {
        let mut buf = [0; 1024];
        stream.read(&mut buf)?;
        Ok(bincode::deserialize(&buf)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum State {
    Idling,
    Askin,
    CriticalSection,
}

#[derive(Debug, Clone)]
pub struct RicAgrawala {
    pub state: State,
    pub timestamp: usize,
    pub last_request_timestamp: usize,
    pub prioritized: bool,
    pub awaited: HashSet<NodeId>,
    pub differed_permission: Vec<NodeId>,
}

impl RicAgrawala {
    fn differ_permission(&mut self, node: NodeId) {
        debug!("🕣 {:?} permission has been differed", node);
        self.differed_permission.push(node);
    }

    fn alter_on(&mut self, message: &Message) {
        match message {
            Message::Request { date, .. } => {
                self.timestamp = date.clone().max(self.timestamp);
                self.prioritized =
                    self.state != State::Idling && self.last_request_timestamp < date.clone()
            }
            Message::Permission { authorizer } => {
                self.awaited.remove(&authorizer);
            }
        }
    }

    pub fn handle(
        &mut self,
        message: Message,
        config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>,
        permission_signal: Arc<Condvar>,
    ) -> Result<()> {
        let (_, id, neighbours) = config.as_ref();
        self.alter_on(&message);
        match message {
            Message::Request { requester, .. } => {
                if self.prioritized {
                    self.differ_permission(requester);
                } else {
                    Message::Permission {
                        authorizer: id.clone(),
                    }
                    .send_to(neighbours.get(&requester).unwrap())?;
                }
            }
            Message::Permission { .. } => {
                if self.awaited.is_empty() {
                    permission_signal.notify_all();
                }
            }
        }
        Ok(())
    }
}

impl Default for RicAgrawala {
    fn default() -> Self {
        Self {
            state: State::Idling,
            timestamp: 0,
            last_request_timestamp: 0,
            prioritized: false,
            awaited: HashSet::new(),
            differed_permission: Vec::new(),
        }
    }
}

pub fn ask_access(
    mutex: Arc<Mutex<RicAgrawala>>,
    config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>,
) -> Result<()> {
    // Then expect to receive a permission at some point
    Ok(())
}

pub fn free_access(
    mutex: Arc<Mutex<RicAgrawala>>,
    config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>,
) -> Result<()> {
    let (_, id, neighbours) = config.as_ref();

    let mut v = mutex.lock().unwrap();
    v.state = State::Idling;
    for m in v.differed_permission.iter() {
        Message::Permission {
            authorizer: id.clone(),
        }
        .send_to(neighbours.get(m).unwrap())?;
    }
    v.differed_permission.clear();

    Ok(())
}

pub trait RicAgrawalaActor {
    fn ask(&mut self, config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>) -> Result<()>;

    fn free(&mut self, config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>) -> Result<()>;
}

impl<'a> RicAgrawalaActor for MutexGuard<'a, RicAgrawala> {
    fn ask(&mut self, config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>) -> Result<()> {
        let (nodes_count, id, neighbours) = config.as_ref();
        self.state = State::Askin;
        self.timestamp += 1;
        self.last_request_timestamp = self.timestamp;
        let timestamp = self.timestamp.clone();
        let awaited = (1..nodes_count.clone() + 1)
            .map(NodeId)
            .filter(|n| n.0 != id.0)
            .collect::<Vec<_>>();
        for node in awaited.clone() {
            self.awaited.insert(node.clone());
        }
        debug!("⚙️ Asked for access, ready to receive a permission");

        // Sends for each program waited a request for permission
        for node in awaited.clone() {
            Message::Request {
                date: timestamp,
                requester: id.clone(),
            }
            .send_to(neighbours.get(&node).unwrap())?;
        }
        debug!(
            "❓ Asked for permission following neighbours: {:?}, should now wait for permission",
            awaited
        );

        Ok(())
    }

    fn free(&mut self, config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>) -> Result<()> {
        let (_, id, neighbours) = config.as_ref();

        self.state = State::Idling;
        for m in self.differed_permission.iter() {
            Message::Permission {
                authorizer: id.clone(),
            }
            .send_to(neighbours.get(m).unwrap())?;
        }

        Ok(())
    }
}
