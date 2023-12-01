use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
};

use color_eyre::eyre::Result;
use renraku_shared::NodeId;

/// Represents the arguments required to configure a node.
///
/// The `NodeArguments` struct encapsulates the necessary arguments to properly
/// configure a node within the distributed system. It utilizes clap for parsing.
///
/// # Examples
///
/// ```
/// # use renraku_node::NodeArguments;
///
/// let args = NodeArguments {
///     controller: "localhost:3000".to_string(),
/// };
/// ```
#[derive(clap::Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct NodeArguments {
    #[arg(short, long, default_value_t = String::from("localhost:3000"))]
    pub controller: String,
}

///
pub fn configure(args: NodeArguments) -> Result<(usize, NodeId, HashMap<NodeId, TcpStream>)> {
    let controller_socket = UdpSocket::bind("localhost:0")?;
    let tcp_listener = TcpListener::bind("localhost:0")?;
    let mut buf = [0; 1024];

    // Sends a message to let the controller identify we are a program
    controller_socket.send_to(
        &bincode::serialize(&tcp_listener.local_addr()?.port())?,
        args.controller,
    )?;
    // Receive a first message that contains the ID.
    controller_socket.recv(&mut buf)?;
    let (node_count, id) = bincode::deserialize::<(usize, NodeId)>(&buf)?;
    // Receive a second message with the number of addresses we have to connect to
    // since at least one program will only receive connections, we know this will
    // not block each of our nodes.
    controller_socket.recv(&mut buf)?;
    let read_streams_count = bincode::deserialize::<usize>(&buf)?;
    controller_socket.recv(&mut buf)?;
    let write_streams_count = bincode::deserialize::<usize>(&buf)?;

    let mut id_to_stream = HashMap::with_capacity(read_streams_count + write_streams_count);

    for _ in 0..read_streams_count {
        let mut stream = tcp_listener.accept()?.0;
        stream.read(&mut buf)?;
        let stream_id = bincode::deserialize::<NodeId>(&buf)?;

        stream.write(&bincode::serialize(&id)?)?;

        id_to_stream.insert(stream_id, stream);
    }

    // Receive the addresses we have to connect to
    for _ in 0..write_streams_count {
        controller_socket.recv(&mut buf)?;
        let addr = bincode::deserialize::<SocketAddr>(&buf)?;

        let mut stream = TcpStream::connect(addr)?;
        stream.write(&bincode::serialize(&id)?)?;

        stream.read(&mut buf)?;
        let stream_id = bincode::deserialize::<NodeId>(&buf)?;

        id_to_stream.insert(stream_id, stream);
    }

    Ok((node_count, id, id_to_stream))
}
