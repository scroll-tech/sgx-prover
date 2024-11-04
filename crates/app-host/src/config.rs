use anyhow::Result;
use serde::Deserialize;
use std::fs::File;
use alloy::primitives::Address;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub l1_endpoint: String,
    pub l1_account_pk: String,
    pub scroll_chain_address: Address,
    pub prover_registry_address: Address,
    pub max_size_per_fetch_l1_event: u64,
    pub max_block_trace_workers: usize,
    pub l2_endpoint: String,
    pub enclave_endpoint: String,
}

impl Config {
    pub fn from_reader<R>(reader: R) -> Result<Self>
    where
        R: std::io::Read,
    {
        serde_json::from_reader(reader).map_err(|e| anyhow::anyhow!(e))
    }

    pub fn from_file(file_name: String) -> Result<Self> {
        let file = File::open(file_name)?;
        Config::from_reader(&file)
    }
}
