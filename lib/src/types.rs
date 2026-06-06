use crate::{
    U256,
    crypto::{PublicKey, Signature},
    error::BtcError,
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
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blockchain {
    utxos: HashMap<Hash, (bool, TransactionOutput)>,
    target: U256,
    blocks: Vec<Block>,
    mempool: Vec<Transaction>,
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

    pub fn add_to_mempool(&mut self, transaction: Transaction) -> Result<(), BtcError> {
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
                    self.mempool.iter().enumerate().find(|(_, transaction)| {
                        transaction
                            .outputs
                            .iter()
                            .any(|output| output.hash() == input.prev_transaction_output_hash)
                    });

                if let Some((idx, referencing_transaction)) = referencing_transaction {
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

        self.mempool.push(transaction);

        self.mempool.sort_by_key(|tx| {
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

    pub fn add_block(&mut self, block: Block) -> Result<(), BtcError> {
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
            .retain(|tx| !block_transactions.contains(&tx.hash()));

        self.blocks.push(block);
        self.try_adjust_target();

        Ok(())
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Block {
            header,
            transactions,
        }
    }

    pub fn calculate_miner_fee(
        &self,
        utxos: &HashMap<Hash, (bool, TransactionOutput)>,
    ) -> Result<u64, BtcError> {
        let mut inputs: HashMap<Hash, TransactionOutput> = HashMap::new();
        let mut outputs: HashMap<Hash, TransactionOutput> = HashMap::new();

        for transaction in self.transactions.iter().skip(1) {
            for input in &transaction.inputs {
                let prev_output = utxos
                    .get(&input.prev_transaction_output_hash)
                    .map(|(_, ouput)| ouput);
                if prev_output.is_none() {
                    return Err(BtcError::InvalidTransection);
                }

                let prev_output = prev_output.unwrap();

                if inputs.contains_key(&input.prev_transaction_output_hash) {
                    return Err(BtcError::InvalidTransection);
                }

                inputs.insert(input.prev_transaction_output_hash, prev_output.clone());
            }

            for output in &transaction.outputs {
                if outputs.contains_key(&output.hash()) {
                    return Err(BtcError::InvalidTransection);
                }

                outputs.insert(output.hash(), output.clone());
            }
        }

        let input_value: u64 = inputs.values().map(|input| input.value).sum();
        let output_value: u64 = outputs.values().map(|output| output.value).sum();
        Ok(input_value - output_value)
    }

    pub fn verify_coinbase_transection(
        &self,
        predicted_block_height: u64,
        utxos: &HashMap<Hash, (bool, TransactionOutput)>,
    ) -> Result<(), BtcError> {
        let coinbase_transection = &self.transactions[0];
        if coinbase_transection.inputs.len() != 0 {
            return Err(BtcError::InvalidTransection);
        }

        if coinbase_transection.outputs.len() == 0 {
            return Err(BtcError::InvalidTransection);
        }

        let miner_fee = self.calculate_miner_fee(&utxos)?;

        let block_reward = crate::INITIAL_REWARD * 10u64.pow(8)
            / 2u64.pow((predicted_block_height / crate::HALVING_REWARD) as u32);

        let total_coinbase_outputs: u64 = coinbase_transection
            .outputs
            .iter()
            .map(|output| output.value)
            .sum();

        if total_coinbase_outputs != block_reward + miner_fee {
            return Err(BtcError::InvalidTransection);
        }

        Ok(())
    }

    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }

    pub fn verify_transactions(
        &self,
        predicted_block_height: u64,
        utxos: &HashMap<Hash, (bool, TransactionOutput)>,
    ) -> Result<(), BtcError> {
        let mut inputs: HashMap<Hash, TransactionOutput> = HashMap::new();
        if self.transactions.is_empty() {
            return Err(BtcError::InvalidTransection);
        }

        self.verify_coinbase_transection(predicted_block_height, utxos)?;

        for transaction in self.transactions.iter().skip(1) {
            let mut input_value = 0;
            let mut output_value = 0;

            for input in &transaction.inputs {
                let prev_output = utxos
                    .get(&input.prev_transaction_output_hash)
                    .map(|(_, ouput)| ouput);
                if prev_output.is_none() {
                    return Err(BtcError::InvalidTransection);
                }

                let prev_output = prev_output.unwrap();

                if inputs.contains_key(&input.prev_transaction_output_hash) {
                    return Err(BtcError::InvalidTransection);
                }

                if !input
                    .signature
                    .verify_signature(&input.prev_transaction_output_hash, &prev_output.pubkey)
                {
                    return Err(BtcError::InvalidSignature);
                }
                input_value += prev_output.value;
                inputs.insert(input.prev_transaction_output_hash, prev_output.clone());
            }

            for output in &transaction.outputs {
                output_value += output.value;

                if input_value < output_value {
                    return Err(BtcError::InvalidTransection);
                }
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockHeader {
    pub timestamp: DateTime<Utc>,
    pub nonce: u64,
    pub previous_hash: Hash,
    pub merkle_root: MerkleRoot,
    pub target: U256,
}

impl BlockHeader {
    pub fn new(
        timestamp: DateTime<Utc>,
        nonce: u64,
        previous_hash: Hash,
        merkle_root: MerkleRoot,
        target: U256,
    ) -> Self {
        BlockHeader {
            timestamp,
            nonce,
            previous_hash,
            merkle_root,
            target,
        }
    }

    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }

    pub fn mine(&mut self, steps: usize) -> bool {
        if self.hash().matches_target(self.target) {
            return true;
        }

        for _ in 0..steps {
            if let Some(new_nonce) = self.nonce.checked_add(1) {
                self.nonce = new_nonce;
            } else {
                self.nonce = 0;
                self.timestamp = Utc::now();
            }

            if self.hash().matches_target(self.target) {
                return true;
            }
        }

        false
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionInput {
    pub prev_transaction_output_hash: Hash,
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionOutput {
    pub value: u64,
    pub unique_id: Uuid,
    pub pubkey: PublicKey,
}
impl TransactionOutput {
    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
}
impl Transaction {
    pub fn new(inputs: Vec<TransactionInput>, outputs: Vec<TransactionOutput>) -> Self {
        Transaction { inputs, outputs }
    }

    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }
}
