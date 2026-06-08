use super::{Transaction, TransactionOutput};
use crate::{
    U256,
    error::{BtcError, Result},
    sha256::Hash,
    util::MerkleRoot,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    ) -> Result<u64> {
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
    ) -> Result<()> {
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
    ) -> Result<()> {
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
