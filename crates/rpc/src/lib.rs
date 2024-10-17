mod client;
mod enclave_signer;
mod error;
mod methods;
mod server;
mod types;

pub use types::*;

pub use client::create_client;
pub use methods::ScrollSgxClient;
pub use server::run_server;
