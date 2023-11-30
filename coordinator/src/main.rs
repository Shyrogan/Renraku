use std::{
    fs::File,
    net::{SocketAddr, UdpSocket},
};

use clap::Parser;
use color_eyre::eyre::Result;
use command::Arguments;
use graph::Graph;
use renraku_shared::NodeId;

pub mod command;
pub mod graph;

fn main() -> Result<()> {
    color_eyre::install()?;

    let arguments = Arguments::try_parse()?;
    let graph = Graph::try_from(File::open(arguments.graph)?)?;

    let socket = UdpSocket::bind(arguments.address)?;
    let mut addresses = Vec::<SocketAddr>::new();
    let mut listeners = Vec::<SocketAddr>::new();
    while addresses.len() < graph.vertices.len() {
        let mut buf = [0; 1024];
        let (_, addr) = socket.recv_from(&mut buf)?;
        let port = bincode::deserialize::<u16>(&buf)?;

        addresses.push(addr.clone());

        let mut addr = addr.clone();
        addr.set_port(port);
        listeners.push(addr);
        println!(
            "👋 A new client has arrived, he is listening on: {:?}",
            addr
        );
    }

    for (i, addr) in addresses.iter().enumerate() {
        // First sends each of the program their ids
        let id = NodeId(i + 1);
        socket.send_to(&bincode::serialize(&id)?, addr)?;
        // Then we count the number of connections they will receive
        let incoming_connections = graph.edges.iter().filter(|e| e.1 == id).count();
        socket.send_to(&bincode::serialize(&incoming_connections)?, addr)?;
        // Then we send the address of each of the programs they have to connect to
        let outgoing_addresses: Vec<SocketAddr> = graph
            .edges
            .iter()
            .filter(|e| e.0 == id)
            .map(|e| e.1.clone())
            .map(|v| listeners.get(v.0 - 1))
            .filter(|o| o.is_some())
            .map(|o| o.unwrap().to_owned())
            .collect();

        socket.send_to(&bincode::serialize(&outgoing_addresses.len())?, addr)?;
        for tcp_addr in outgoing_addresses.iter() {
            socket.send_to(&bincode::serialize(tcp_addr)?, addr)?;
        }
        println!("🥳 Node #{} is now ready ! He will receive {} connections and connect to {} neighbours", id.0, incoming_connections, outgoing_addresses.len());
    }

    Ok(())
}
