use std::{
    sync::{Arc, Mutex},
};

use batch_state::{BatchInfo, BatchState};
use bundle_state::BundleState;
use rpc::{ProveBatchRequest, ProveBatchResponse, ProveBundleRequest};

use crate::{
    block_tracer::BlockTracer,
    l1_client::L1Client,
    types::{BundleSize, CommitBatchEvent, VerifyBatchEvent},
};
use anyhow::{bail, Result};
pub use error::*;

mod batch_state;
mod bundle_state;
mod error;

pub struct StateManager {
    batch_state: Mutex<BatchState>,
    bundle_state: Mutex<BundleState>,
    last_finalized_batch_index_on_start: u64,
}

impl StateManager {
    pub fn new() -> Self {
        // todo
        let last_finalized_batch_index = 10;
        let bundle_sizes = vec![];

        Self {
            batch_state: Mutex::new(BatchState::new(last_finalized_batch_index)),
            bundle_state: Mutex::new(BundleState::new(last_finalized_batch_index, bundle_sizes)),
            last_finalized_batch_index_on_start: last_finalized_batch_index,
        }
    }

    pub async fn on_batch_commit_event_received(
        &self,
        event: CommitBatchEvent,
        block_tracer: &BlockTracer,
    ) -> Result<ProveBatchRequest> {
        let blocks = event
            .chunks
            .clone()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        if blocks.len() == 0 {
            bail!("block count is zero")
        }
        let batch_info = BatchInfo {
            batch_index: event.batch_index,
            batch_header: None,
            prove_response: None,
        };
        {
            let mut state = self.batch_state.lock().unwrap();
            state.create_batch(&event, batch_info);
            state.update_batch_header(event.batch_index - 1, event.prev_batch_header.clone());
        }

        let block_traces = block_tracer.get_block_traces(blocks).await?;

        let prev_state_root =
        // we need prove the exact batch as last_finalized_batch_index on start
        // to know the paramters for building next first unfinalized batch/bundle.
        // however, this first proved batch's prev_state_root is not known that
        // a trick being made here.
        if event.batch_index == self.last_finalized_batch_index_on_start {
            block_traces[0]
            .storage_trace
            .root_before
            .to_fixed_bytes()
            .into()
        } else {
            let prev_state_root_op = {
                let state = self.batch_state.lock().unwrap();
                state.get_batch_state_root(event.batch_index - 1)
            };
            // theoretically, get last batch's state root must succeed since the batch event
            // is processed in sequence, last batch's state root already being set before processing
            // a new one.
            prev_state_root_op.expect(&format!("failed to get prev_state_root, batch_index: {}", event.batch_index))
        };

        let request = ProveBatchRequest {
            prev_batch_header: event.prev_batch_header,
            prev_state_root,
            batch_version: event.batch_version,
            blocks: block_traces,
            chunks: event.chunks,
        };

        Ok(request)
    }

    pub fn on_batch_proved(&self, batch_index: u64, response: ProveBatchResponse) {
        let mut state = self.batch_state.lock().unwrap();
        state.update_batch_proof(batch_index, response);
    }

    // fn try_build_prove_bundle_request(
    //     &self,
    //     batch_index: u64,
    // ) -> Result<Option<ProveBundleRequest>, StateManagerError> {
    //     let bundle_option = {
    //         let mut bundle_state = self.bundle_state.lock().unwrap();
    //         bundle_state.get_next_bundle(batch_index)
    //     };
    //     match bundle_option {
    //         Some(bundle) => {
    //             let state = self.batch_state.lock().unwrap();
    //             let mut req = state
    //                 .collect_batch_infos(bundle.begin_batch_index, bundle.end_batch_index)
    //                 .ok_or_general_error()?;

    //             let batch_info = state.get_batch_info(bundle.begin_batch_index - 1).expect("");

    //             req.last_finalized_batch_header = batch_info.batch_header.expect("");
    //             req.prev_state_root = batch_info.prove_response.expect("").post_state_root;
    //             Ok(Some(req))
    //         }
    //         None => {
    //             log::info!("bundle is not prepared");
    //             Ok(None)
    //         }
    //     }
    // }

    pub fn try_build_prove_bundle_request(&self) -> Result<ProveBundleRequest, StateManagerError> {
        let last_verified_batch_index = {
            let state = self.batch_state.lock().unwrap();
            state.last_verified_batch_index
        };

        let bundle = {
            let bundle_state = self.bundle_state.lock().unwrap();
            bundle_state.get_next_bundle(last_verified_batch_index)?
        };

        let state = self.batch_state.lock().unwrap();
        let mut req = state
            .collect_batch_infos(bundle.begin_batch_index, bundle.end_batch_index)
            .ok_or_general_error()?;

        let batch_info = state
            .get_batch_info(bundle.begin_batch_index - 1)
            .expect(&format!(
                "failed to get batch_info, batch_index: {}",
                bundle.begin_batch_index - 1
            ));
        req.last_finalized_batch_header = batch_info.batch_header.expect(&format!(
            "failed to get batch_header, batch_index: {}",
            bundle.begin_batch_index - 1
        ));
        req.prev_state_root = batch_info
            .prove_response
            .expect(&format!(
                "failed to get batch_prove_response, batch_index: {}",
                bundle.begin_batch_index - 1
            ))
            .post_state_root;
        Ok(req)
    }

    pub fn on_batch_finalized_event_received(&self, event: VerifyBatchEvent) {
        let mut bundle_state = self.bundle_state.lock().unwrap();
        bundle_state.update_last_finalized_batch(event);
        // todo: update next_prover
    }

    pub fn on_bundle_size_updated_received(&self, event: BundleSize) {
        let mut bundle_state = self.bundle_state.lock().unwrap();
        bundle_state.update_bundle_size(event);
    }
}
