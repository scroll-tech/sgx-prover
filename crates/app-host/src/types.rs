use alloy::{primitives::{Bytes, B256}, sol};

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Default)]
    ScrollChain,
    "abi/ScrollChain.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    SGXVerifier,
    "abi/SGXVerifier.json"
);

pub struct CommitBatchEvent {
    pub batch_index: u64,
    pub batch_hash: B256,
    pub batch_version: u8,
    pub chunks: Vec<Vec<u64>>,
    pub prev_batch_header: Bytes,
}

pub struct FinalizeBatchEvent {
    pub batch_index: u64,
    pub batch_hash: B256,
    pub end_batch_header: Bytes,
    pub end_state_root: B256,
    pub end_withdraw_root: B256,
}

