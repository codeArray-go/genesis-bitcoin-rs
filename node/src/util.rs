use anyhow::{Context, Ok, Result};
use btclib::{network::Message, types::Blockchain, util::Saveable};
use tokio::net::TcpStream;

pub async fn load_blockchain(blockchain_file: &str) -> Result<()> {
    println!("Blockchain file exists loading...");
    let new_blockchain = Blockchain::read_from_file(blockchain_file);
    println!("Blockchain file loaded.");
    let mut blockchain = crate::BLOCKCHAIN.write().await;
    *blockchain = new_blockchain;
    println!("rebuilding utxos...");
    blockchain.rebuild_utxos();
    println!("checking if target needs to be adjusted");
    println!("Current target: {}", blockchain.target);
    blockchain.try_adjust_target();
    println!("New target is: {}", blockchain.target);
    println!("Initialization complete");
    Ok(())
}

pub async fn populate_connections(nodes: &[String]) -> Result<()> {
    println!("Trying to connect to other nodes...");
    for node in nodes {
        println!("Connecting to node: {}", node);
        let mut stream = TcpStream::connect(node).await?;
        let message = Message::DiscoverNodes;
        message.send(&mut stream).await;
        println!("Send discoverable node to {}", node);

        let message = Message::receive(&mut stream).await?;
        match message {
            Message::NodesList(child_nodes) => {
                println!("Received NodeList from {}", node);
                for child_node in child_nodes {
                    println!("Adding node to {}", child_node);
                    let new_stream = TcpStream::connect(&child_node).await?;
                    crate::NODES.insert(child_node, new_stream);
                }
            }

            _ => {
                println!("Unexpected error occure.");
            }
        }
        crate::NODES.insert(node.clone(), stream);
    }
    Ok(())
}

pub async fn find_longest_chain_node() -> Result<(String, u32)> {
    println!("Finding nodes with highest blockchain length...");
    let mut longest_name = String::new();
    let mut longest_count = 0;

    let nodes = crate::NODES
        .iter()
        .map(|x| x.key().clone())
        .collect::<Vec<_>>();

    for node in nodes {
        println!("asking {} for blokchain length", node);
        let mut stream = crate::NODES.get_mut(&node).context("No node.")?;
        let message = Message::AskDifference(0);
        message.send(&mut *stream).await.unwrap();
        println!("sent AskDifference to {}", node);

        let message = Message::receive(&mut *stream).await?;
        match message {
            Message::Difference(count) => {
                println!("Received difference from {}", node);
                if count > longest_count {
                    println!(
                        "new longest blockchain: \
                    {} block from {node}",
                        count
                    );
                    longest_count = count;
                    longest_name = node;
                }
            }

            e => {
                println!("Unexpected message from {}: {:?}", node, e);
            }
        }
    }

    Ok((longest_name, longest_count as u32))
}
