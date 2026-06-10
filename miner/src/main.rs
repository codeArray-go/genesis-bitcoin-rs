use anyhow::{Result, anyhow};
use btclib::{crypto::PublicKey, network::Message, util::Saveable};
use std::{env, process::exit};
use tokio::net::TcpStream;

fn usage() -> ! {
    eprintln!(
        "Usage: {} <address> <public_key_file>",
        env::args().next().unwrap()
    );
    exit(1);
}

#[derive(Parser)]
#[command(author, version, aboutm long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub address: String,

    #[arg(short, long)]
    pub public_key_file: String,
}

#[tokio::main]
async fn main() {
    let address = match env::args().nth(1) {
        Some(address) => address,
        None => usage(),
    };

    let public_key_file = match env::args().nth(2) {
        Some(public_key_fle) => public_key_fle,
        None => usage(),
    };

    let Ok(public_key) = PublicKey::read_from_file(&public_key_file) else {
        eprint!("Failed to read public key");
        exit(1);
    };

    let mut stream = match TcpStream::connect(&address).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("Failed to connect to server: {}", e);
            exit(1);
        }
    };

    println!("Connecting to {address} to mine with {public_key:?}");

    println!("Requesting node for mining.");
    let message = Message::FetchTemplete(public_key);
    message.send(&mut stream).await.unwrap();
}
