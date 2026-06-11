use std::{
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::Duration,
};

use anyhow::{Result, anyhow};
use btclib::{crypto::PublicKey, types::Block, util::Saveable};
use clap::Parser;
use tokio::{net::TcpStream, sync::Mutex, time::interval};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    address: String,

    #[arg(short, long)]
    public_key_file: String,
}

struct Miner {
    public_key: PublicKey,
    stream: Mutex<TcpStream>,
    current_templete: Arc<std::sync::Mutex<Option<Block>>>,
    mining: Arc<AtomicBool>,
    mined_block_sender: flume::Sender<Block>,
    mined_block_receiver: flume::Receiver<Block>,
}
impl Miner {
    fn spawn_mining_thread(&self) -> thread::JoinHandle<()> {
        thread::spawn(|| {})
    }

    async fn fetch_and_validate_templete(&self) -> Result<()> {}

    async fn submit_block(&self, block: Block) -> Result<()> {}

    async fn run(&self) -> Result<()> {
        self.spawn_mining_thread();
        let mut templete_interval = interval(Duration::from_secs(5));

        loop {
            let receiver_clone = self.mined_block_receiver.clone();
            tokio::select! {
                _ = templete_interval.tick() => {
                    self.fetch_and_validate_templete().await?;
                }
                Ok(mined_block) = receiver_clone.recv_async() => {
                    self.submit_block(mined_block).await?;
                }
            }
        }
    }

    async fn new(address: String, public_key: PublicKey) -> Result<Self> {
        let stream = TcpStream::connect(&address).await?;
        let (mined_block_sender, mined_block_receiver) = flume::unbounded();
        Ok(Self {
            public_key,
            stream: Mutex::new(stream),
            current_templete: Arc::new(std::sync::Mutex::new(None)),
            mining: Arc::new(AtomicBool::new(false)),
            mined_block_sender,
            mined_block_receiver,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let public_key = PublicKey::read_from_file(&cli.public_key_file)
        .map_err(|e| anyhow!("Failed to read public key file {}", e))?;

    let miner = Miner::new(cli.address, public_key).await?;
    miner.run().await
}
