use crate::sha256::Hash;
use crate::types::Transection;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MerkleRoot(Hash);
impl MerkleRoot {
    pub fn calculate(transections: &[Transection]) -> MerkleRoot {
        let mut layer: Vec<Hash> = vec![];
        for transecton in transections {
            layer.push(Hash::hash(transecton));
        }

        while layer.len() > 1 {
            let mut new_layer: Vec<Hash> = vec![];
            for pair in layer.chunks(2) {
                let left = pair[0];
                let right = pair.get(1).unwrap_or(&pair[0]);
                new_layer.push(Hash::hash(&[left, *right]));
            }

            layer = new_layer;
        }

        MerkleRoot(layer[0])
    }
}
