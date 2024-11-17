use std::collections::HashMap;


use rpc::{ProveBatchResponse, ProveBundleRequest};

use crate::{
    types::{BatchHash, CommitBatchEvent, StateRoot},
};
use alloy::primitives::Bytes;
use anyhow::{bail, Result};

#[derive(Clone)]
pub struct BatchInfo {
    pub batch_index: u64,
    pub batch_header: Option<Bytes>,
    pub prove_response: Option<ProveBatchResponse>,
}

pub struct BatchState {
    hash_info_map: HashMap<BatchHash, BatchInfo>,
    index_hash_map: HashMap<u64, BatchHash>,
    pub last_verified_batch_index: u64,
    smallest_batch_index: u64,
}

impl BatchState {
    pub fn new(smallest_batch_index: u64) -> Self {
        Self {
            hash_info_map: HashMap::new(),
            index_hash_map: HashMap::new(),
            last_verified_batch_index: 0,
            smallest_batch_index,
        }
    }

    pub fn create_batch(&mut self, event: &CommitBatchEvent, batch_info: BatchInfo) {
        self.index_hash_map
            .insert(event.batch_index, event.batch_hash);
        self.hash_info_map.insert(event.batch_hash, batch_info);
    }

    pub fn update_batch_header(&mut self, batch_index: u64, batch_header: Bytes) {
        if let Some(batch_hash) = self.index_hash_map.get(&batch_index) {
            self.hash_info_map
                .entry(*batch_hash)
                .and_modify(|info| info.batch_header = Some(batch_header));
        };
    }

    pub fn update_batch_proof(&mut self, batch_index: u64, prove_response: ProveBatchResponse) {
        if self.last_verified_batch_index != 0 {
            assert!(
                self.last_verified_batch_index + 1 == batch_index,
                "batch proof updated in wrong order"
            );
        }
        self.last_verified_batch_index = batch_index;
        self.hash_info_map
            .entry(prove_response.batch_hash)
            .and_modify(|info| info.prove_response = Some(prove_response));
    }

    // this method requires that the batch should be proved sequentially by enclave part
    // or it fails to get the prev_state_root
    pub fn get_batch_state_root(&self, batch_index: u64) -> Option<StateRoot> {
        self.index_hash_map
            .get(&batch_index)
            .and_then(|batch_hash| {
                self.hash_info_map[batch_hash]
                    .prove_response
                    .as_ref()
                    .map(|response| response.post_state_root)
            })
    }

    pub fn get_batch_info(&self, batch_index: u64) -> Option<BatchInfo> {
        self.index_hash_map
            .get(&batch_index)
            .map(|batch_hash| self.hash_info_map[batch_hash].clone())
    }

    pub fn collect_batch_infos(
        &self,
        begin_batch_index: u64,
        end_batch_index: u64,
    ) -> Result<ProveBundleRequest> {
        let mut batch_headers = vec![];
        let mut state_roots = vec![];
        let mut withdraw_roots = vec![];
        let mut signatures = vec![];
        for i in begin_batch_index..=end_batch_index {
            let batch_hash = self.index_hash_map.get(&i);
            if batch_hash.is_none() {
                bail!("")
            }
            if let Some(batch_info) = self.hash_info_map.get(batch_hash.unwrap()) {
                match batch_info.batch_header.as_ref() {
                    Some(header) => {
                        batch_headers.push(header.clone());
                    }
                    _ => bail!(""),
                }
                match batch_info.prove_response.as_ref() {
                    Some(response) => {
                        state_roots.push(response.post_state_root.clone());
                        withdraw_roots.push(response.post_withdraw_root.clone());
                        signatures.push(response.signature.clone());
                    }
                    _ => bail!(""),
                }
            } else {
                unreachable!()
            }
        }
        let request = ProveBundleRequest {
            batch_headers,
            state_roots,
            withdraw_roots,
            signatures,
            ..Default::default()
        };

        Ok(request)
    }
}
