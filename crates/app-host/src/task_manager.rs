use jsonrpsee::http_client::HttpClient;
use rpc::{ProveBatchRequest, ProveBatchResponse, ProveBundleRequest};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::state_manager::StateManager;
use crate::{
    block_tracer::{self, BlockTracer},
    l1_client::L1Client,
    types::{CommitBatchEvent, FinalizeBatchEvent},
};
use anyhow::Result;
use rpc::ScrollSgxClient;
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct TaskManager {
    state_manager: StateManager,
    l1_client: Arc<L1Client>,
    enclave_client: HttpClient,
    block_tracer: BlockTracer,
}

impl TaskManager {
    pub async fn new(
        l1_client: Arc<L1Client>,
        enclave_client: HttpClient,
        l2_endpoint: String,
        max_block_trace_workers: usize,
    ) -> Result<Self> {
        let block_tracer = BlockTracer::new(l2_endpoint, max_block_trace_workers)?;
        Ok(Self {
            state_manager: StateManager::new(l1_client.clone()),
            l1_client,
            enclave_client,
            block_tracer,
        })
    }

    async fn prove_batch(&self, request: ProveBatchRequest) -> ProveBatchResponse {
        loop {
            match self.enclave_client.prove_batch(request.clone()).await {
                Ok(resp) => {
                    break resp;
                }
                Err(err) => {
                    // todo add log
                    tokio::time::sleep(core::time::Duration::from_secs(5));
                }
            }
        }
    }

    async fn prove_bundle_and_submit(&self, request: ProveBundleRequest) -> () {
        let response = loop {
            match self.enclave_client.prove_bundle(request.clone()).await {
                Ok(resp) => {
                    break resp;
                }
                Err(err) => {
                    // todo add log
                    tokio::time::sleep(core::time::Duration::from_secs(5));
                }
            }
        };
        loop {
            let last_index = request.batch_headers.len() - 1;
            let batch_header = request.batch_headers[last_index].clone();
            let post_state_root = request.state_roots[last_index];
            let withdraw_root = request.withdraw_roots[last_index];
            let tee_proof = response.signature.as_bytes().into();
            match self
                .l1_client
                .finalize_bundle_with_tee_proof(
                    batch_header,
                    post_state_root,
                    withdraw_root,
                    tee_proof,
                )
                .await
            {
                Ok(resp) => {
                    break;
                }
                Err(err) => {
                    // todo add log
                    tokio::time::sleep(core::time::Duration::from_secs(5));
                }
            }
        }
    }

    async fn handle_batch_event(
        &self,
        mut rx: Receiver<CommitBatchEvent>,
        prove_batch_tx: Sender<ProveBatchResponse>,
    ) -> () {
        while let Some(event) = rx.recv().await {
            if let Ok(request) = self
                .state_manager
                .on_batch_commit_event_received(event, &self.block_tracer)
                .await
            {
                let response = self.prove_batch(request).await;
                prove_batch_tx.send(response).await;
            } else {
                // todo, retry or handle error
            }
        }
    }

    async fn handle_bundle_event(
        &self,
        mut rx: Receiver<FinalizeBatchEvent>,
        mut prove_batch_rx: Receiver<ProveBatchResponse>,
    ) -> () {
        loop {
            let requests = tokio::select! {
                finalize_batch_option = rx.recv() => {
                    match finalize_batch_option {
                        Some(finalize_batch_event) => {
                            if let Ok(reqs) = self.state_manager.on_batch_finalize_event_received(finalize_batch_event).await {
                                reqs
                            } else {
                                // todo: add error
                                vec![]
                            }
                        },
                        None => break
                    }
                }
                proved_batch_option = prove_batch_rx.recv() => {
                    match proved_batch_option {
                        Some(prove_batch_response) => {
                            if let Ok(reqs) = self.state_manager.on_batch_proved(prove_batch_response).await {
                                reqs
                            } else {
                                vec![]
                            }
                        },
                        None => break
                    }
                }
            };
            for request in requests {
                self.prove_bundle_and_submit(request).await;
            }
        }
    }

    pub async fn start(
        task_manager: Self,
        commit_batch_event_rx: Receiver<CommitBatchEvent>,
        finalize_batch_event_rx: Receiver<FinalizeBatchEvent>,
    ) {
        let task_manager_1 = Arc::new(task_manager);
        let task_manager_2 = task_manager_1.clone();

        let (prove_batch_resp_tx, prove_batch_resp_rx) = mpsc::channel::<ProveBatchResponse>(32);
        tokio::spawn(async move {
            task_manager_1.handle_batch_event(commit_batch_event_rx, prove_batch_resp_tx);
        });

        tokio::spawn(async move {
            task_manager_2.handle_bundle_event(finalize_batch_event_rx, prove_batch_resp_rx);
        });
    }
}
