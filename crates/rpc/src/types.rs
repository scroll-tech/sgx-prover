use scroll_executor::BlockTrace;

use alloy::primitives::{Bytes, Signature, B256};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBlockRequest {
    prev_state_root: B256,
    block_trace: BlockTrace,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBlockResponse {
    post_state_root: B256,
    post_withdraw_root: B256,
    signature: Signature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBatchRequest {
    prev_batch_header: Bytes,
    batch_version: u8,
    // blocks
    // chunks
    prev_state_root: B256,
    state_roots: Vec<B256>,
    withdraw_roots: Vec<B256>,
    signatures: Vec<Signature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBatchResponse {
    batch_header: Bytes,
    post_state_root: B256,
    post_withdraw_root: B256,
    signature: Signature,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBundleRequest {
    batch_headers: Vec<Bytes>,
    state_roots: Vec<B256>,
    withdraw_roots: Vec<B256>,
    signatures: Vec<Signature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBundleResponse {
    public_input: Bytes,
    signature: Signature,
}
