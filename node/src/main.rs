use argh::FromArgs;
use dashmap::DashMap;
use static_init::dynamic;
use btclib::types::Blockchain;

mod handler;
mod util;

#[derive(FromArgs)]
struct Args{
    port: u16,

    #[argh(option, default = "String::from(\"./blockchain.cbor\")")]
    blockchain_file: String,
    node: Vec<String>
}

fn main() {
    println!("Hey");
}
