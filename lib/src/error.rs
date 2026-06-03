#[macro_export]
macro_rules! custom_error {
    ($name:ident, $($variant:ident => $msg:expr),*$(,)?) => {
        #[derive(Debug)]
        pub enum $name {
            $( $variant),*,
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( $name::$variant => write!(f, "{}", $msg)), *
                }
            }
        }

        impl std::error::Error for $name {}
    };
}

custom_error!(BtcError,
    InvalidTransection => "Invalid Transection",
    InvalidBlock => "Invalid Block",
    InvalidBlockHeader => "Invalid Block header",
    InvalidInputTransection => "Invalid input transection",
    InvalidOutputTransection => "Invalid output transection",
    InvalidMerkeleRoot => "Invalid merkele root",
    InvalidHash => "Invalid hash",
    InvalidSignature => "Invalid signature",
    InvalidPublicKey => "Invalid public key",
    InvalidPrivateKey => "Invalid private key"
);

pub type Result<T> = std::result::Result<T, BtcError>;


// Use this only when you wanna to pass something to errors as uper micro is hard coded by me and is Declarative macro which is fine working till we only need to show error on screen but below one is taken from library named as thiserror which is Procedural Macro 

// use thiserror::Error;

// #[derive(Error, Debug)]
// pub enum BtcError {
//     #[error("Invalid Transection")]
//     InvalidTransection,

//     #[error("Invalid Block")]
//     InvalidBlock,

//     #[error("Invalid Block Header")]
//     InvalidBlockHeader,

//     #[error("Invalid Input Transection")]
//     InvalidInputTransection,

//     #[error("Invalid Output Transection")]
//     InvalidOutPutTransection,

//     #[error("Invalid Merkele Root")]
//     InvalidMerkeleRoot,

//     #[error("Invalid Hash")]
//     InvalidHash,

//     #[error("Invalid Signature")]
//     InvalidSignature,

//     #[error("Invalid public key")]
//     InvalidPublicKey,

//     #[error("Invalid private key")]
//     InvalidPrivateKey,
// }