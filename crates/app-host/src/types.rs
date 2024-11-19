use alloy::{
    primitives::{Bytes, B256},
    sol,
};

pub type BatchHash = B256;
pub type StateRoot = B256;
pub type WithdrawRoot = B256;

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Default)]
    ScrollChain,
    "abi/ScrollChain.json"
);

pub enum ScrollChainEventLog {
    CommitBatch(CommitBatchEvent),
    FinalizeBundle(VerifyBatchEvent),
    ChangeBundleSize(BundleSize),
}

pub struct CommitBatchEvent {
    pub batch_index: u64,
    pub batch_hash: BatchHash,
    pub batch_version: u8,
    pub chunks: Vec<Vec<u64>>,
    pub prev_batch_header: Bytes,
}

pub struct VerifyBatchEvent {
    pub batch_index: u64,
    pub batch_hash: BatchHash,
    pub end_batch_header: Bytes,
    pub end_state_root: StateRoot,
    pub end_withdraw_root: WithdrawRoot,
}

#[derive(Clone, Copy)]
pub struct BundleSize {
    pub bundle_size: u64,
    pub start_batch_index: u64,
}

pub use tee::NextProver;
