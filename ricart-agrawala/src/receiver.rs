use std::{
    collections::HashMap,
    io::Read,
    net::TcpStream,
    sync::{Arc, Condvar, Mutex},
};

use color_eyre::eyre::Result;
use renraku_shared::NodeId;
use selecting::Selector;
use tracing::{debug, info};

use crate::algorithm::{Message, RicAgrawala};

pub fn receive_thread(
    mutex: Arc<Mutex<RicAgrawala>>,
    permission_signal: Arc<Condvar>,
    config: Arc<(usize, NodeId, HashMap<NodeId, TcpStream>)>,
) -> Result<()> {
    let streams: Vec<&TcpStream> = config
        .2
        .iter()
        .map(|(_, stream)| stream.to_owned())
        .collect();

    loop {
        // Select
        let mut selector = Selector::new();
        streams
            .iter()
            .for_each(|stream| selector.add_read(stream.to_owned()));

        let result = selector.select()?;
        let mut v = mutex.lock().unwrap();
        for stream in streams
            .iter()
            .filter(|s| result.is_read(s.to_owned().to_owned()))
        {
            let message = Message::receive_from(stream)?;
            v.handle(message, config.clone(), permission_signal.clone())?;
        }
    }

    Ok(())
}
