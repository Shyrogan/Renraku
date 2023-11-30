use clap::Parser;
use color_eyre::eyre::Result;
use renraku_node::NodeArguments;

fn main() -> Result<()> {
    color_eyre::install()?;
    let (id, neighbours) = renraku_node::configure(NodeArguments::try_parse()?)?;

    println!(
        "👋 Hello I'm node #{} and I can access {} neighbours:",
        id.0,
        neighbours.len()
    );
    for (node_id, node_stream) in neighbours.iter() {
        println!("- Node #{} using stream: {:?}", node_id.0, node_stream);
    }

    Ok(())
}
