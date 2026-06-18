use btclib::{
    network::Message,
    sha256::Hash,
    types::{Block, BlockHeader, Transaction, TransactionOutput},
    util::MerkleRoot,
};
use chrono::Utc;
use tokio::net::TcpStream;
use uuid::Uuid;

pub async fn handle_connection(mut socket: TcpStream) {
    loop {
        let message = match Message::receive(&mut socket).await {
            Ok(message) => message,
            Err(e) => {
                println!("Invalid message from peer: {e}, closing that connection.");
                return;
            }
        };

        use btclib::network::Message::*;
        match message {
            UTXOs(_) | Template(_) | Difference(_) | TemplateValidity(_) | NodesList(_) => {
                println!(
                    "I am neither a miner not a \
                         wallet! Goodby."
                );
                return;
            }

            FetchBlock(height) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let Some(block) = blockchain.blocks().nth(height as usize).cloned() else {
                    return;
                };

                let message = Message::NewBlock(block);
                message.send(&mut socket).await.unwrap();
            }

            DiscoverNodes => {
                let nodes = crate::NODES
                    .iter()
                    .map(|x| x.key().clone())
                    .collect::<Vec<_>>();
                let message = Message::NodesList(nodes);

                message.send(&mut socket).await.unwrap();
            }

            AskDifference(height) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let count = blockchain.block_height() as i32 - height as i32;
                let message = Difference(count);

                message.send(&mut socket).await.unwrap();
            }

            FetchUTXOs(key) => {
                println!("Key reveived.");
                let blockchain = crate::BLOCKCHAIN.read().await;
                let utxo = blockchain
                    .utxos()
                    .iter()
                    .filter(|(_, (_, txout))| txout.pubkey == key)
                    .map(|(_, (mark, txout))| (txout.clone(), *mark))
                    .collect::<Vec<_>>();

                let message = Message::UTXOs(utxo);
                message.send(&mut socket).await.unwrap();
            }

            NewBlock(block) => {
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                println!("New block received.");
                if blockchain.add_block(block).is_err() {
                    println!("Block rejected.");
                    return;
                }
            }

            NewTrasaction(tx) => {
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                println!("Recieved transaction from friend.");
                if blockchain.add_to_mempool(tx).is_err() {
                    println!("Transaction rejected.");
                    return;
                }
            }

            ValidateTemplate(templete) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let status = templete.header.previous_hash
                    == blockchain
                        .blocks()
                        .last()
                        .map(|last_block| last_block.hash())
                        .unwrap_or(Hash::zero());

                let message = Message::TemplateValidity(status);
                message.send(&mut socket).await.unwrap();
            }

            SubmitTemplate(block) => {
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                println!("Received transaction.");
                if let Err(e) = blockchain.add_block(block.clone()) {
                    println!("Block rejected {e}, closing connection.");
                    return;
                }

                blockchain.rebuild_utxos();
                println!("Block looks good, broadcasting.");

                let nodes = crate::NODES
                    .iter()
                    .map(|x| x.key().clone())
                    .collect::<Vec<_>>();

                for node in nodes {
                    if let Some(mut stream) = crate::NODES.get_mut(&node) {
                        let message = Message::NewBlock(block.clone());

                        if message.send(&mut *stream).await.is_err() {
                            println!("Failed to send block to {}", node);
                        }
                    }
                }
            }

            SubmitTransaction(tx) => {
                println!("Submit transaction.");
                let mut blockchain = crate::BLOCKCHAIN.write().await;
                if let Err(e) = blockchain.add_to_mempool(tx.clone()) {
                    println!("transaction rejected, closing connection: {e}");
                    return;
                }

                println!("Transaction added to mempool successfully.");
                let nodes = crate::NODES
                    .iter()
                    .map(|x| x.key().clone())
                    .collect::<Vec<_>>();

                for node in nodes {
                    if let Some(mut stream) = crate::NODES.get_mut(&node) {
                        let message = Message::NewTrasaction(tx.clone());
                        if message.send(&mut *stream).await.is_err() {
                            println!("faild to send transaction to {}", node);
                        }
                    }
                }

                println!("Transaction sended successfully.");
            }

            FetchTemplate(pubkey) => {
                let blockchain = crate::BLOCKCHAIN.read().await;
                let mut transactions = vec![];
                transactions.extend(
                    blockchain
                        .mempool()
                        .iter()
                        .take(btclib::BLOCK_TRANSACTION_CAP)
                        .map(|(_, tx)| tx)
                        .cloned()
                        .collect::<Vec<_>>(),
                );

                transactions.insert(
                    0,
                    Transaction {
                        inputs: vec![],
                        outputs: vec![TransactionOutput {
                            value: 0,
                            pubkey,
                            unique_id: Uuid::new_v4(),
                        }],
                    },
                );

                let merkle_root = MerkleRoot::calculate(&transactions);
                let mut block = Block::new(
                    BlockHeader {
                        timestamp: Utc::now(),
                        merkle_root,
                        nonce: 0,
                        previous_hash: blockchain
                            .blocks()
                            .last()
                            .map(|last_block| last_block.hash())
                            .unwrap_or(Hash::zero()),
                        target: *blockchain.target(),
                    },
                    transactions,
                );

                let miner_fee = match block.calculate_miner_fee(blockchain.utxos()) {
                    Ok(fee) => fee,
                    Err(e) => {
                        eprint!("{e}");
                        return;
                    }
                };

                let reward = blockchain.calculate_block_reward();
                block.transactions[0].outputs[0].value = reward + miner_fee;

                block.header.merkle_root = MerkleRoot::calculate(&block.transactions);

                let message = Message::Template(block);
                message.send(&mut socket).await.unwrap();
            }
        }
    }
}
