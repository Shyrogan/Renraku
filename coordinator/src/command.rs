use std::path::PathBuf;

#[derive(clap::Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    #[arg(short, long, value_name = "FILE")]
    pub graph: PathBuf,
    #[arg(short, long, default_value_t = String::from("localhost:3000"))]
    pub address: String,
}
