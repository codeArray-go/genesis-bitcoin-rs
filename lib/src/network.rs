use crate::{
    crypto::PublicKey,
    types::{Block, Transaction, TransactionOutput},
};
use serde::{Deserialize, Serialize};
use std::io::Error as IoError;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    FetchUTXOs(PublicKey),
    UTXOs(Vec<(TransactionOutput, bool)>),
    SubmitTransaction(Transaction),
    NewTrasaction(Transaction),
    FetchTemplate(PublicKey),
    Template(Block),
    ValidateTemplate(Block),
    TemplateValidity(bool),
    SubmitTemplate(Block),
    DiscoverNodes,
    NodesList(Vec<String>),
    AskDifference(u32),
    Difference(i32),
    FetchBlock(usize),
    NewBlock(Block),
}

impl Message {
    pub fn encode(&self) -> Result<Vec<u8>, ciborium::ser::Error<IoError>> {
        let mut bytes: Vec<u8> = vec![];
        ciborium::into_writer(self, &mut bytes)?;
        Ok(bytes)
    }

    pub fn decode(data: &[u8]) -> Result<Self, ciborium::de::Error<IoError>> {
        ciborium::from_reader(data)
    }

    pub async fn send(
        &self,
        stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), ciborium::ser::Error<IoError>> {
        let bytes = self.encode()?;
        let len = bytes.len();
        stream.write_all(&len.to_be_bytes()).await?;
        stream.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn receive(
        &self,
        stream: &mut (impl AsyncRead + Unpin),
    ) -> Result<Self, ciborium::de::Error<IoError>> {
        let mut bytes = [0u8; 8];
        stream.read_exact(&mut bytes).await?;
        let len = u64::from_be_bytes(bytes) as usize;
        let mut data = vec![0u8; len];
        stream.read_exact(&mut data).await?;
        Self::decode(&data)
    }
}
