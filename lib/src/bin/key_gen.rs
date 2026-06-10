use btclib::{crypto::PrivateKey, util::Saveable};
use std::env;

fn main() {
    let name = env::args().nth(1).expect("Please enter valid name");

    let private_key = PrivateKey::new();
    let public_key = PrivateKey::public_key(&private_key);
    let public_key_file = name.clone() + ".pub.pem";
    let private_key_file = name.clone() + ".priv.cbor";

    private_key.save_to_file(&private_key_file).unwrap();
    public_key.save_to_file(&public_key_file).unwrap();
}
