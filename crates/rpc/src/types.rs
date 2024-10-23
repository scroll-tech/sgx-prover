use scroll_executor::BlockTrace;

use alloy::primitives::{Bytes, Signature, B256};
use alloy::sol;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBatchRequest {
    pub prev_batch_header: Bytes,
    pub prev_state_root: B256,
    pub batch_version: u8,
    pub blocks: Vec<BlockTrace>,
    pub chunks: Vec<Vec<u64>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBatchResponse {
    pub batch_hash: B256,
    pub post_state_root: B256,
    pub post_withdraw_root: B256,
    pub signature: Signature,
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
    pub last_finalized_batch_header: Bytes,
    pub prev_state_root: B256,
    pub batch_headers: Vec<Bytes>,
    pub state_roots: Vec<B256>,
    pub withdraw_roots: Vec<B256>,
    pub signatures: Vec<Signature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProveBundleResponse {
    pub public_input: Bytes,
    pub signature: Signature,
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
