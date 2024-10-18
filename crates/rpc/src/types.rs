use scroll_executor::BlockTrace;

use alloy::primitives::{Bytes, Signature, B256};
use alloy::sol;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBatchRequest {
    prev_batch_header: Bytes,
    prev_state_root: B256,
    batch_version: u8,
    blocks: Vec<BlockTrace>,
    chunks: Vec<Vec<u64>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBatchResponse {
    batch_header: Bytes,
    post_state_root: B256,
    post_withdraw_root: B256,
    signature: Signature,
}

sol! {
    #[derive(Default, Serialize)]
    struct ProveBatchSignatureData {
        uint64 layer2ChainId;
        bytes32 prevStateRoot;
        bytes32 postStateRoot;
        bytes32 batchHash;
        bytes32 postWithdrawRoot;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBundleRequest {
    last_finalized_batch_header: Bytes,
    prev_state_root: B256,
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

sol! {
    #[derive(Default, Serialize)]
    struct ProveBundleSignatureData {
        uint64 layer2ChainId;
        uint32 numBatches;
        bytes32 prevStateRoot;
        bytes32 prevBatchHash;
        bytes32 postStateRoot;
        bytes32 batchHash;
        bytes32 postWithdrawRoot;
    }
}
