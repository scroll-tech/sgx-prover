use jsonrpsee::http_client::HttpClient;
use rpc::{ProveBatchRequest, ProveBatchResponse, ProveBundleRequest};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{block_tracer::BlockTracer, l1_client::L1Client, types::{CommitBatchEvent, FinalizeBatchEvent}};
use std::sync::Arc;
use crate::state_manager::StateManager;
use rpc::ScrollSgxClient;

pub struct TaskManager {
}

impl TaskManager {
    pub fn new() -> Self {
        todo!()
    }

    async fn prove_batch(enclave_client: Arc<HttpClient>, request: ProveBatchRequest) -> ProveBatchResponse {
        loop {
            match enclave_client.prove_batch(request.clone()).await {
                Ok(resp) => {
                    break resp;
                },
                Err(err) => {
                    // todo add log
                    tokio::time::sleep(core::time::Duration::from_secs(5));
                }
            }
        }
    }

    async fn prove_bundle_and_submit(enclave_client: Arc<HttpClient>, l1_client: Arc<L1Client>, request: ProveBundleRequest) -> () {
        let response = loop {
            match enclave_client.prove_bundle(request.clone()).await {
                Ok(resp) => {
                    break resp;
                },
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
            match l1_client.finalize_bundle_with_tee_proof(
                batch_header,
                post_state_root,
                withdraw_root,
                tee_proof
            ).await {
                Ok(resp) => {
                    break;
                },
                Err(err) => {
                    // todo add log
                    tokio::time::sleep(core::time::Duration::from_secs(5));
                }
            }
        }
    }

    async fn handle_batch_event(
        proof_state: Arc<StateManager>,
        enclave_client: Arc<HttpClient>,
        block_tracer: Arc<BlockTracer>,
        mut rx: Receiver<CommitBatchEvent>,
        prove_batch_tx: Sender<ProveBatchResponse>
    ) -> () {
        while let Some(event) = rx.recv().await {
            if let Ok(request) = proof_state.on_batch_commit_event_received(event, block_tracer.clone()).await {
                let response = TaskManager::prove_batch(enclave_client.clone(), request).await;
                prove_batch_tx.send(response).await;
            } else {
                // todo, retry or handle error
            }
        }
    }

    async fn handle_bundle_event(
        proof_state: Arc<StateManager>,
        enclave_client: Arc<HttpClient>,
        l1_client: Arc<L1Client>,
        mut rx: Receiver<FinalizeBatchEvent>,
        mut prove_batch_rx: Receiver<ProveBatchResponse>) -> () {
        
        loop {
            let requests = tokio::select! {
                finalize_batch_option = rx.recv() => {
                    match finalize_batch_option {
                        Some(finalize_batch_event) => {
                            if let Ok(reqs) = proof_state.on_batch_finalize_event_received(finalize_batch_event).await {
                                reqs
                            } else {
                                vec![]
                            }
                        },
                        None => break
                    }
                }
                proved_batch_option = prove_batch_rx.recv() => {
                    match proved_batch_option {
                        Some(prove_batch_response) => {
                            if let Ok(reqs) = proof_state.on_batch_proved(prove_batch_response).await {
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
                TaskManager::prove_bundle_and_submit(enclave_client.clone(),
                    l1_client.clone(),
                    request).await;
            }
        }
    }

    pub async fn start(&self,
        commit_batch_event_rx: Receiver<CommitBatchEvent>,
        finalize_batch_event_rx: Receiver<FinalizeBatchEvent>,
    ) {
        let proof_state_manager = Arc::new(StateManager::new());
        let l1_client = Arc::new(L1Client::new());
        let enclave_client = Arc::new();
        let enclave_client_copy = enclave_client.clone();
        let block_tracer = Arc::new(BlockTracer::new());

        let (prove_batch_resp_tx, prove_batch_resp_rx) = mpsc::channel::<ProveBatchResponse>(32);

        let proof_state_manager_copy = proof_state_manager.clone();
        tokio::spawn(async move {
            TaskManager::handle_batch_event(
                proof_state_manager_copy, 
                enclave_client_copy,
                block_tracer,
                commit_batch_event_rx,
                prove_batch_resp_tx);
        });

        tokio::spawn(async move {
            TaskManager::handle_bundle_event(
                proof_state_manager, 
                enclave_client,
                l1_client,
                finalize_batch_event_rx,
                prove_batch_resp_rx);
        });

        ()
    }
}