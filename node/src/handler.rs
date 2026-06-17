use btclib::network::Message;
use tokio::net::TcpStream;

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

            _ => {}
        }
    }
}
