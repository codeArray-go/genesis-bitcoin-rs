use crate::{
    U256,
    crypto::{PublicKey, Signature},
    error::BtcError,
    sha256::Hash,
    util::MerkleRoot,
};
use chrono::{DateTime, Utc};
use ecdsa::signature::Verifier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Blockchain {
    pub utxos: HashMap<Hash, TransactionOutput>,
    pub blocks: Vec<Block>,
}
impl Blockchain {
    pub fn new() -> Self {
        Blockchain {
            utxos: HashMap::new(),
            blocks: vec![],
        }
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
        // block.verify_transactions(self.block_height(), &self.utxos)?;

        self.blocks.push(block);
        Ok(())
    }

    pub fn rebuild_utoxs(&mut self) {
        for block in &self.blocks {
            for transaction in &block.transactions {
                for input in &transaction.inputs {
                    self.utxos.remove(&input.prev_transaction_output_hash);
                }
                for output in transaction.outputs.iter() {
                    self.utxos.insert(transaction.hash(), output.clone());
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

    pub fn hash(&self) -> Hash {
        Hash::hash(self)
    }

    pub fn verify_transactions(
        &self,
        utxos: &HashMap<Hash, TransactionOutput>,
    ) -> Result<(), BtcError> {
        let mut inputs: HashMap<Hash, TransactionOutput> = HashMap::new();
        if self.transactions.is_empty() {
            return Err(BtcError::InvalidTransection);
        }
        for transaction in &self.transactions {
            let mut input_value = 0;
            let mut output_value = 0;

            for input in &transaction.inputs {
                let prev_output = utxos.get(&input.prev_transaction_output_hash);
                if prev_output.is_none() {
                    return Err(BtcError::InvalidTransection);
                }

                let prev_output = prev_output.unwrap();

                if inputs.contains_key(&input.prev_transaction_output_hash) {
                    return Err(BtcError::InvalidTransection);
                }

                if !input
                    .signature
                    .verify(&input.prev_transaction_output_hash, &prev_output.pubkey)
                {
                    return Err(BtcError::InvalidSignature);
                }
                input_value += prev_output.value;
                input.insert(input.prev_transaction_output_hash, prev_output.clone());
            }

            for outpu in &transaction.outputs {
                output_value += output_value;

                if input_value < output_value {
                    return Err(Error::InvalidTransection);
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
