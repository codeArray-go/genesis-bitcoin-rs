use super::{Block, Transaction, TransactionOutput};
use crate::{
    U256,
    error::{BtcError, Result},
    sha256::Hash,
    util::MerkleRoot,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blockchain {
    utxos: HashMap<Hash, (bool, TransactionOutput)>,
    target: U256,
    blocks: Vec<Block>,
    #[serde(default, skip_serializing)]
    mempool: Vec<(DateTime<Utc>, Transaction)>,
}
impl Blockchain {
    pub fn new() -> Self {
        Blockchain {
            utxos: HashMap::new(),
            target: crate::MIN_TARGET,
            blocks: vec![],
            mempool: vec![],
        }
    }

    pub fn add_to_mempool(&mut self, transaction: Transaction) -> Result<()> {
        let mut known = HashSet::new();
        for input in &transaction.inputs {
            if !self.utxos.contains_key(&input.prev_transaction_output_hash) {
                return Err(BtcError::InvalidTransection);
            }

            if known.contains(&input.prev_transaction_output_hash) {
                return Err(BtcError::InvalidTransection);
            }

            known.insert(input.prev_transaction_output_hash);
        }

        // check if any of the utxos have the bool mark set to true and if so, find the transaction that references them in mempool, remove it, and set all the utxos it references to false
        for input in &transaction.inputs {
            if let Some((true, _)) = self.utxos.get(&input.prev_transaction_output_hash) {
                let referencing_transaction =
                    self.mempool
                        .iter()
                        .enumerate()
                        .find(|(_, (_, transaction))| {
                            transaction
                                .outputs
                                .iter()
                                .any(|output| output.hash() == input.prev_transaction_output_hash)
                        });

                if let Some((idx, (_, referencing_transaction))) = referencing_transaction {
                    for input in &referencing_transaction.inputs {
                        self.utxos
                            .entry(input.prev_transaction_output_hash)
                            .and_modify(|(marked, _)| {
                                *marked = false;
                            });
                    }
                    self.mempool.remove(idx);
                } else {
                    self.utxos
                        .entry(input.prev_transaction_output_hash)
                        .and_modify(|(marked, _)| {
                            *marked = false;
                        });
                }
            }
        }

        let all_input = transaction
            .inputs
            .iter()
            .map(|input| {
                self.utxos
                    .get(&input.prev_transaction_output_hash)
                    .expect("Bug")
                    .1
                    .value
            })
            .sum::<u64>();

        let all_ouput = transaction.outputs.iter().map(|ouput| ouput.value).sum();

        if all_input < all_ouput {
            return Err(BtcError::InvalidTransection);
        }

        for input in &transaction.inputs {
            self.utxos
                .entry(input.prev_transaction_output_hash)
                .and_modify(|(marked, _)| {
                    *marked = true;
                });
        }

        self.mempool.push((Utc::now(), transaction));

        self.mempool.sort_by_key(|(_, tx)| {
            let tx_in: u64 = tx
                .inputs
                .iter()
                .map(|input| {
                    &self
                        .utxos
                        .get(&input.prev_transaction_output_hash)
                        .expect("Bug")
                        .1
                        .value
                })
                .sum::<u64>();

            let tx_out: u64 = tx.outputs.iter().map(|output| output.value).sum();

            let miner_fee = tx_in - tx_out;

            Reverse(miner_fee)
        });

        Ok(())
    }

    pub fn utxos(&self) -> &HashMap<Hash, (bool, TransactionOutput)> {
        &self.utxos
    }

    pub fn target(&self) -> &U256 {
        &self.target
    }

    pub fn blocks(&self) -> impl Iterator<Item = &Block> {
        self.blocks.iter()
    }

    pub fn mempool(&self) -> &[(DateTime<Utc>, Transaction)] {
        &self.mempool
    }

    pub fn block_height(&self) -> u64 {
        self.blocks.len() as u64
    }

    pub fn try_adjust_target(&mut self) {
        if self.blocks.is_empty() {
            return;
        }

        if self.blocks.len() % crate::DIFFICULTY_UPDATE_INTERVAL as usize != 0 {
            return;
        }

        let start_time = self.blocks
            [self.blocks.len() - crate::DIFFICULTY_UPDATE_INTERVAL as usize]
            .header
            .timestamp;
        let end_time = self.blocks.last().unwrap().header.timestamp;

        let time_diff = end_time - start_time;
        let time_diff_seconds = time_diff.num_seconds();

        let target_seconds = crate::IDEAL_BLOCK_TIME * crate::DIFFICULTY_UPDATE_INTERVAL;
        let new_target = BigDecimal::parse_bytes(&self.target.to_string().as_bytes(), 10)
            .expect("BUG: No Bug")
            * (BigDecimal::from(time_diff_seconds) / BigDecimal::from(target_seconds));

        let new_target_str = new_target
            .to_string()
            .split('.')
            .next()
            .expect("Bug: Expected a decimal point")
            .to_owned();

        let new_target: U256 = U256::from_str_radix(&new_target_str, 10).expect("BUG: No Bug.");

        let new_target = if new_target < self.target / 4 {
            self.target / 4
        } else if new_target > self.target * 4 {
            self.target * 4
        } else {
            new_target
        };

        self.target = new_target.min(crate::MIN_TARGET);
    }

    pub fn add_block(&mut self, block: Block) -> Result<()> {
        if let Some(last_block) = self.blocks.last() {
            // Check previous hash
            if last_block.hash() != block.header.previous_hash {
                println!("Invalid previous hash");
                return Err(BtcError::InvalidBlock);
            }

            // Check timestamp
            if last_block.header.timestamp >= block.header.timestamp {
                return Err(BtcError::InvalidBlock);
            }
        } else {
            // For genesis block
            if block.header.previous_hash != Hash::zero() {
                println!("Zero Hash");
                return Err(BtcError::InvalidBlock);
            }
        }

        // If block hash matches target limit
        if !block.header.hash().matches_target(block.header.target) {
            println!("Hash value is less then seted target.");
            return Err(BtcError::InvalidHash);
        }

        // If merkle_root matches expectation
        if MerkleRoot::calculate(&block.transactions) != block.header.merkle_root {
            println!("MerkleRoot didn't match.");
            return Err(BtcError::InvalidMerkeleRoot);
        }

        // Verify all transaction of block
        block.verify_transactions(self.block_height(), &self.utxos)?;

        // Remove transactions from mempool that are now in the block
        let block_transactions: HashSet<_> =
            block.transactions.iter().map(|tx| tx.hash()).collect();
        self.mempool
            .retain(|tx| !block_transactions.contains(&tx.1.hash()));

        self.blocks.push(block);
        self.try_adjust_target();

        Ok(())
    }

    pub fn cleanup_mempool(&mut self) {
        let newtime = Utc::now();
        let mut utxos_hash_to_remove: Vec<Hash> = vec![];

        self.mempool.retain(|(timestamp, transaction)| {
            if newtime - *timestamp
                > chrono::Duration::seconds(crate::MAXIMUM_MEMPOOL_TRANSACTION_AGE as i64)
            {
                utxos_hash_to_remove.extend(
                    transaction
                        .inputs
                        .iter()
                        .map(|input| input.prev_transaction_output_hash),
                );
                false
            } else {
                true
            }
        });

        for hash in utxos_hash_to_remove {
            self.utxos
                .entry(hash)
                .and_modify(|(marked, _)| *marked = false);
        }
    }

    pub fn rebuild_utoxs(&mut self) {
        for block in &self.blocks {
            for transaction in &block.transactions {
                for input in &transaction.inputs {
                    self.utxos.remove(&input.prev_transaction_output_hash);
                }
                for output in transaction.outputs.iter() {
                    self.utxos
                        .insert(transaction.hash(), (false, output.clone()));
                }
            }
        }
    }
}
