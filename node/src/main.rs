use std::path::Path;

use anyhow::{Ok, Result};
use argh::FromArgs;
use btclib::types::Blockchain;
use dashmap::DashMap;
use static_init::dynamic;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

mod handler;
mod util;

#[derive(FromArgs)]
/// Blockchain node
struct Args {
    #[argh(option, default = "9000")]
    /// port number
    port: u16,

    #[argh(option, default = "String::from(\"./blockchain.cbor\")")]
    /// blockchain file location
    blockchain_file: String,

    #[argh(positional)]
    /// address of initial nodes
    node: Vec<String>,
}

#[dynamic]
pub static BLOCKCHAIN: RwLock<Blockchain> = RwLock::new(Blockchain::new());

#[dynamic]
pub static NODES: DashMap<String, TcpStream> = DashMap::new();

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = argh::from_env();
    let port = args.port;
    let blockchain_file = args.blockchain_file;
    let node = args.node;
    let add = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&add).await?;
    println!("Listening on port: {}", add);

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(handler::handle_connection(socket));
        break;
    }

    tokio::spawn(util::cleanup());
    tokio::spawn(util::save(blockchain_file.clone()));

    if Path::new(&blockchain_file).exists() {
        util::load_blockchain(&blockchain_file).await?;
    } else {
        println!("No such blokchain file exits.");
        util::populate_connections(&node).await?;
        println!("Total nodes: {}", NODES.len());
        if node.is_empty() {
            println!("No initial node provided, stating as a seed node.");
        } else {
            let (longest_name, longest_count) = util::find_longest_chain_node().await?;

            // Request from nodes for longest blockchain
            util::download_blockchain(&longest_name, longest_count).await?;
            println!("Blockchain downloaded from : {}", longest_name);

            // Rebuild UTXOS
            {
                let mut blockchain = BLOCKCHAIN.write().await;
                blockchain.rebuild_utxos();
            }

            // Try for dificullty adjustments
            {
                let mut blockchain = BLOCKCHAIN.write().await;
                blockchain.try_adjust_target();
            }
        }
    }
    Ok(())
}
