# Genesis Bitcoin

## Why I Started This Project

I started this project for one simple reason: to learn. Reading about how things work is good, but building them with your own hands is the best way to truly understand them.

Through this project, I am learning two main things:

1. How to write good, safe code using a programming language called Rust.
2. How a blockchain actually works from the inside.


## What Is This Project?

Think of a blockchain like a shared digital notebook that everyone can look at, but nobody can cheat or erase. This project is my own homemade, simple version of Bitcoin.

Instead of copying big, complicated code, I am building the basic building blocks from scratch. Right now, I am focusing on:

- **Transactions:** How a digital coin moves from one person to another.
- **The Waiting Room (Mempool):** A place where new transactions wait in line before they are written into the notebook.
- **Checking the Math:** Writing the rules to make sure nobody is spending money they do not have.



# To Run This Project: -

## Prerequisites

- **Rust** (stable, 1.75+): https://rustup.rs
- No PostgreSQL needed for local testing (mock DB activates automatically)

```powershell
rustup update stable
```

---

## Installation & Build

```powershell
git clone https://github.com/codeArray-go/genesis-bitcoin-rs.git
cd genesis-bitcoin-rs
cargo build
```

and now you could run whole project by entering to it's specified directory in terminal and type cargo run command.