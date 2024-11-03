
use std::{collections::{HashMap, VecDeque}, sync::{Arc, Mutex}};

use rpc::{ProveBatchRequest, ProveBatchResponse, ProveBundleRequest};

use crate::{block_tracer::BlockTracer, l1_client::L1Client, types::{BatchHash, CommitBatchEvent, FinalizeBatchEvent, StateRoot}, utils::convert_eth_error};
use anyhow::{bail, Ok, Result};
use alloy::primitives::Bytes;


struct BatchInfo {
    batch_index: u64,
    batch_header: Option<Bytes>,
    prove_response: Option<ProveBatchResponse>,
    start_block_number: u64,
}

struct BatchState {
    hash_info_map: HashMap<BatchHash, BatchInfo>,
    index_hash_map: HashMap<u64, BatchHash>,
}

impl BatchState {
    fn create_batch(&mut self, event: &CommitBatchEvent, batch_info: BatchInfo) {
        self.index_hash_map.insert(event.batch_index, event.batch_hash);
        self.hash_info_map.insert(event.batch_hash, batch_info);
    }

    fn update_batch_header(&mut self, batch_index: u64, batch_header: Bytes) {
        if let Some (batch_hash) = self.index_hash_map.get(&batch_index) {
            self.hash_info_map.entry(*batch_hash).and_modify(|info| info.batch_header = Some(batch_header));
        };
    }

    fn update_batch_proof(&mut self, prove_response: ProveBatchResponse) {
        self.hash_info_map.entry(prove_response.batch_hash).and_modify(|info| info.prove_response = Some(prove_response));
    }

    // this method requires that the batch should be proved sequentially by enclave part
    // or it fails to get the prev_state_root
    fn get_batch_prev_state_root(&self, batch_index: u64) -> Option<StateRoot> {
        let prev_batch_index = batch_index - 1;
        self.index_hash_map.get(&prev_batch_index).and_then(|batch_hash| {
            self.hash_info_map[batch_hash].prove_response.and_then(|response| {
                Some(response.post_state_root)
            })
        })
    }

    fn collect_batch_infos(&self, begin_batch_index: u64, end_batch_index: u64) -> Result<ProveBundleRequest> {
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
                match batch_info.batch_header {
                    Some(header) => {
                        batch_headers.push(header.clone());
                    },
                    _ => bail!("")
                }
                match batch_info.prove_response {
                    Some(response) => {
                        state_roots.push(response.post_state_root.clone());
                        withdraw_roots.push(response.post_withdraw_root.clone());
                        signatures.push(response.signature.clone());
                    },
                    _ => bail!("")
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

struct BundleState {
    bundle_info_queue: VecDeque<BundleInfo>,
    last_finalized_batch_index: Option<u64>,
}

struct BundleInfo {
    begin_batch_index: u64,
    end_batch_index: u64,
    end_batch_header: Bytes,
    end_state_root: StateRoot,
}

impl BundleState {
    // the event should be appended in sequencial order
    fn append_event(&mut self, event: FinalizeBatchEvent, last_finalized_batch_index: u64) -> Result<()> {
        if !self.bundle_info_queue.is_empty() {
            assert!(self.bundle_info_queue.front().unwrap().end_batch_index == last_finalized_batch_index,
            "last_finalized_batch_index {} should equals to first end_batch_index {}", last_finalized_batch_index,
            self.bundle_info_queue.front().unwrap().end_batch_index)
        }

        self.bundle_info_queue.push_back(BundleInfo {
            begin_batch_index: 0,
            end_batch_index: event.batch_index,
            end_state_root: event.end_state_root,
            end_batch_header: event.end_batch_header,
        });
        self.last_finalized_batch_index = Some(last_finalized_batch_index);
        Ok(())
    }

    fn get_pending_bundles(&self) -> Option<Vec<BundleInfo>> {
        self.last_finalized_batch_index.map(|last_index| {
            let mut infos = vec![];

            let mut next_begin_index: u64 = 0;
            self.bundle_info_queue.retain(|info| {
                if info.end_batch_index < last_index {
                    false
                } else if info.end_batch_index == last_index {
                    // notice, this block must be entered or the begin_batch_index may starts at 0
                    // this is guarded by error check in `append_event`
                    next_begin_index = info.end_batch_index + 1;
                    true
                } else {
                    let mut cloend_info = *info.clone();
                    cloend_info.begin_batch_index = next_begin_index;
                    infos.push(cloend_info);

                    next_begin_index = info.end_batch_index + 1;
                    true
                }
            });

            infos
        })
    }
}

pub struct StateManager {
    l1_client: Arc<L1Client>,

    genesis_block_number: u64,
    batch_state: Mutex<BatchState>,
    bundle_state: Mutex<BundleState>,
}

impl StateManager {
    pub fn new() -> Self {
        todo!()
    }

    pub async fn on_batch_commit_event_received(&self, event: CommitBatchEvent, block_tracer: Arc<BlockTracer>) -> Result<ProveBatchRequest> {
        let blocks = event.chunks.clone().into_iter().flatten().collect::<Vec<_>>();

        if blocks.len() == 0 {
            bail!("block count is zero")
        }
        let batch_info = BatchInfo {
            batch_index: event.batch_index,
            batch_header: None,
            prove_response: None,
            start_block_number: blocks[0],
        };
        {
            let mut state = self.batch_state.lock().unwrap();
            state.create_batch(&event, batch_info);
            state.update_batch_header(event.batch_index-1, event.prev_batch_header.clone());
        }

        let block_traces = block_tracer.get_block_traces(blocks).await?;

        let prev_state_root_op = {
            let state = self.batch_state.lock().unwrap();
            state.get_batch_prev_state_root(event.batch_index)
        };
        let prev_state_root = match prev_state_root_op {
            Some(root) => root,
            // this can happen at most once on every start
            // todo: add count track here
            None => block_traces[0].storage_trace.root_before.to_fixed_bytes().into()
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

    pub async fn on_batch_proved(&self, response: ProveBatchResponse) -> Result<Vec<ProveBundleRequest>> {
        {
            let mut state = self.batch_state.lock().unwrap();
            state.update_batch_proof(response);
        }
        
        self.try_build_prove_bundle_request()
    }

    fn try_build_prove_bundle_request(&self) -> Result<Vec<ProveBundleRequest>> {
        let mut requests = vec![];
        
        let bundles = {
            let bundle_state = self.bundle_state.lock().unwrap();
            bundle_state.get_pending_bundles()
        };
        // actually this could not be none, the check should perform beforehand.
        if bundles.is_none() {
            // todo: add error log.
            return Ok(requests);
        }

        for bundle in bundles.unwrap() {
            let mut request = {
                let state = self.batch_state.lock().unwrap();
                match state.collect_batch_infos(bundle.begin_batch_index, bundle.end_batch_index) {
                    Result::Ok(req) => {
                        req
                    },
                    Err(err) => {
                        // todo: add log
                        break;
                    }
                }
            };

            request.last_finalized_batch_header = bundle.end_batch_header;
            request.prev_state_root = bundle.end_state_root;
            requests.push(request);
        }

        Ok(requests)
    }

    pub async fn on_batch_finalize_event_received(&self, event: FinalizeBatchEvent) -> Result<Vec<ProveBundleRequest>> {
        // track latest_finalized_batch_index
        let last_finalized_batch_index = self.l1_client.get_last_tee_finalized_batch_index().await.map_err(|e| convert_eth_error(e))?;

        if event.batch_index < last_finalized_batch_index {
            bail!("")
        }
        let event_batch_index = event.batch_index;
        let end_batch_header = event.end_batch_header.clone();
        {
            let mut bundle_state = self.bundle_state.lock().unwrap();
            bundle_state.append_event(event, last_finalized_batch_index);
        }
        {
            let mut state = self.batch_state.lock().unwrap();
            state.update_batch_header(event_batch_index, end_batch_header);
        }

        if event_batch_index == last_finalized_batch_index {
            Ok(vec![])
        } else {
            self.try_build_prove_bundle_request()
        }
    }
}