use anyhow::{Ok, Result, anyhow};
use btclib::{crypto::PublicKey, network::Message, types::Block, util::Saveable};
use clap::Parser;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
    thread,
    time::Duration,
};
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
    current_template: Arc<std::sync::Mutex<Option<Block>>>,
    mining: Arc<AtomicBool>,
    mined_block_sender: flume::Sender<Block>,
    mined_block_receiver: flume::Receiver<Block>,
}
impl Miner {
    fn spawn_mining_thread(&self) -> thread::JoinHandle<()> {
        let template = self.current_template.clone();
        let mining = self.mining.clone();
        let sender = self.mined_block_sender.clone();

        thread::spawn(move || {
            loop {
                if mining.load(Relaxed) {
                    if let Some(mut block) = template.lock().unwrap().clone() {
                        println!("Mining block with target {}", block.header.hash());
                        if block.header.mine(2_000_000) {
                            println!("Block mined {}", block.hash());
                            sender.send(block).expect("Failed to send mined Block");
                            mining.store(false, Relaxed);
                        }
                    }
                }
                thread::yield_now();
            }
        })
    }

    async fn fetch_template(&self) -> Result<()> {
        println!("Fetching your template");
        let message = Message::FetchTemplate(self.public_key.clone());
        let mut stream = self.stream.lock().await;
        message.send(&mut *stream).await?;
        drop(stream);
        let mut stream = self.stream.lock().await;
        match Message::receive(&mut *stream).await? {
            Message::Template(template) => {
                println!(
                    "New template received with target {}",
                    template.header.target
                );
                drop(stream);

                *self.current_template.lock().unwrap() = Some(template);
                self.mining.store(true, Relaxed);
                Ok(())
            }

            _ => Err(anyhow!("Error while Fetching template")),
        }
    }

    async fn validate_template(&self) -> Result<()> {
        if let Some(template) = self.current_template.lock().unwrap().clone() {
            let message = Message::ValidateTemplate(template);
            let mut stream = self.stream.lock().await;
            message.send(&mut *stream);
            drop(stream);

            let mut stream = self.stream.lock().await;
            match Message::receive(&mut *stream).await? {
                Message::TemplateValidity(valid) => {
                    drop(stream);
                    if !valid {
                        println!("Current templat is no longer valid.");
                    } else {
                        println!("Current template is still valid")
                    }
                    Ok(())
                }

                _ => Err(anyhow!("Error occured while validating templete.")),
            }
        } else {
            Ok(())
        }
    }

    async fn fetch_and_validate_template(&self) -> Result<()> {
        if !self.mining.load(Relaxed) {
            self.fetch_template().await?;
        } else {
            self.validate_template().await?;
        }
        Ok(())
    }

    async fn submit_block(&self, block: Block) -> Result<()> {
        println!("Submission of block start.");
        let message = Message::SubmitTemplate(block);
        let mut stream = self.stream.lock().await;
        message.send(&mut *stream);
        self.mining.store(false, Relaxed);
        Ok(())
    }

    async fn run(&self) -> Result<()> {
        self.spawn_mining_thread();
        let mut template_interval = interval(Duration::from_secs(5));

        loop {
            let receiver_clone = self.mined_block_receiver.clone();
            tokio::select! {
                _ = template_interval.tick() => {
                    self.fetch_and_validate_template().await?;
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
            current_template: Arc::new(std::sync::Mutex::new(None)),
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
