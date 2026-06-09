use btclib::{types::Block, util::Saveable};
use std::{env, fs::File, process::exit};

fn main() {
    let path = if let Some(arg) = env::args().nth(1) {
        arg
    } else {
        println!("Usage: block_print <block_file>");
        exit(1)
    };

    if let Ok(file) = File::open(path) {
        let block = Block::load(file).expect("Failed to load Block");
        println!("{:#?}", block);
    }
}
