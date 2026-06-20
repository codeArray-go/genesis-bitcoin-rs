use anyhow::{Ok, Result};
use btclib::{
    crypto::{PrivateKey, PublicKey},
    network::Message,
    types::{Transaction, TransactionOutput},
    util::Saveable,
};
use crossbeam_skiplist::SkipMap;
use cursive::reexports::serde_json::value;
use kanal::AsyncSender;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc};
use tokio::net::TcpStream;

#[derive(Clone)]
struct UtxoStore {
    my_keys: Vec<LoadedKey>,
    utxos: Arc<SkipMap<PublicKey, Vec<(bool, TransactionOutput)>>>,
}
impl UtxoStore {
    fn new() -> Self {
        UtxoStore {
            my_keys: vec![],
            utxos: Arc::new(SkipMap::new()),
        }
    }

    fn add_key(&mut self, key: LoadedKey) {
        self.my_keys.push(key);
    }
}

#[derive(Clone)]
pub struct Core {
    pub config: Config,
    utxos: UtxoStore,
    pub tx_sender: AsyncSender<Transaction>,
}
impl Core {
    fn new(config: Config, utxos: UtxoStore) -> Self {
        let (tx_sender, _) = kanal::bounded(10);
        Core {
            config,
            utxos,
            tx_sender: tx_sender.clone_async(),
        }
    }

    pub fn load(config_path: PathBuf) -> Result<Self> {
        let config: Config = toml::from_str(&fs::read_to_string(&config_path)?)?;
        let mut utxos = UtxoStore::new();

        // load key from utxos
        for key in &config.my_keys {
            let public = PublicKey::read_from_file(&key.public)?;
            let private = PrivateKey::read_from_file(&key.private)?;
            utxos.add_key(LoadedKey { public, private });
        }

        Ok(Core::new(config, utxos))
    }

    pub async fn fetch_utxos(&self) -> Result<()> {
        let mut stream = TcpStream::connect(&self.config.default_node).await?;
        for key in &self.utxos.my_keys {
            let message = Message::FetchUTXOs(key.public.clone());
            message.send(&mut stream).await?;

            if let Message::UTXOs(utxos) = Message::receive(&mut stream).await? {
                self.utxos.utxos.insert(
                    key.public.clone(),
                    utxos
                        .into_iter()
                        .map(|(output, marked)| (marked, output))
                        .collect(),
                );
            } else {
                return Err(anyhow::anyhow!("Unexpected response from node."));
            }
        }

        Ok(())
    }

    pub async fn send_transaction(&self, transaction: Transaction) -> Result<()> {
        let mut stream = TcpStream::connect(&self.config.default_node).await?;
        let message = Message::SubmitTransaction(transaction);
        message.send(&mut stream).await?;
        Ok(())
    }

    pub fn get_balance(&self) -> u64 {
        self.utxos
            .utxos
            .iter()
            .map(|entry| entry.value().iter().map(|utxo| utxo.1.value).sum::<u64>())
            .sum()
    }

    pub async fn create_transaction(
        &self,
        recipient: &PublicKey,
        amount: u64,
    ) -> Result<Transaction> {
    }

    pub fn calculate_fee(&self, amount: u64) -> u64 {
        match self.config.fee_config.fee_type {
            FeeType::Fixed => self.config.fee_config.value as u64,
            FeeType::Percent => (amount as f64 * self.config.fee_config.value / 100.0) as u64,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Key {
    public: PathBuf,
    private: PathBuf,
}

#[derive(Clone)]
struct LoadedKey {
    public: PublicKey,
    private: PrivateKey,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Recipient {
    pub name: String,
    pub key: PathBuf,
}

#[derive(Clone)]
pub struct LoadRecipient {
    pub name: String,
    pub key: PublicKey,
}
impl Recipient {
    pub fn load(&self) -> Result<LoadRecipient> {
        let key = PublicKey::read_from_file(&self.key)?;
        Ok(LoadRecipient {
            name: self.name.clone(),
            key,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum FeeType {
    Fixed,
    Percent,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FeeConfig {
    pub fee_type: FeeType,
    pub value: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub my_keys: Vec<Key>,
    pub contacts: Vec<Recipient>,
    pub default_node: String,
    pub fee_config: FeeConfig,
}
