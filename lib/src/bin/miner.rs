use btclib::{types::Block, util::Saveable};
use std::{env, process::exit};

fn main() {
    let (path, steps) = if let (Some(arg), Some(arg2)) = (env::args().nth(1), env::args().nth(2)) {
        (arg, arg2)
    } else {
        eprintln!("Usage miner block_file <block_file>");
        exit(1)
    };

    let steps: usize = if let Ok(s @ 1..=usize::MAX) = steps.parse() {
        s
    } else {
        eprint!("Usage: should be possive number");
        exit(1)
    };

    let block_0 = Block::read_from_file(path).expect("Failed to load block");
    let mut block = block_0.clone();

    while !block.header.mine(steps) {
        println!("mining...");
    }

    println!("original: {:#?}", block_0);
    println!("original: {}", block_0.header.hash());
    println!("original: {:#?}", block);
    println!("original hex: {}", block.header.hash());
}
