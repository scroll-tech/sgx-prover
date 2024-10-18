mod client;
mod enclave_signer;
mod error;
mod methods;
mod prove;
mod server;
mod types;
mod utils;

pub use types::*;

pub use client::create_client;
pub use methods::ScrollSgxClient;
pub use server::run_server;
