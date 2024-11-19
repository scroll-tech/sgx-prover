use alloy::primitives::{Bytes};
use base::eth::EthError;
use jsonrpsee::http_client::HttpClient;
use rpc::{ProveBatchRequest, ProveBatchResponse, ProveBundleRequest, ProveBundleResponse};
use tee::ProverRegistry;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::time::interval;

use crate::liveness_manager::AddressInfo;
use crate::state_manager::{StateManager, StateManagerError};
use crate::types::{NextProver, ScrollChainEventLog, StateRoot, WithdrawRoot};
use crate::{
    block_tracer::{BlockTracer},
    l1_client::L1Client,
};
use anyhow::Result;
use rpc::ScrollSgxClient;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use std::u64;

base::stack_error! {
    #[derive(Debug)]
    name: TaskManagerError,
    stack_name: TaskManagerErrorStack,
    error: {
        General(String),
    },
    wrap: {
        Eth(EthError),
    },
    stack: {}
}

pub struct TaskManager {
    state_manager: Arc<StateManager>,
    l1_client: Arc<L1Client>,
    prover_registry: Arc<ProverRegistry>,
    enclave_client: Arc<HttpClient>,
    block_tracer: BlockTracer,

    prover_address: Arc<Mutex<AddressInfo>>,
}

impl TaskManager {
    pub async fn new(
        prover_address: Arc<Mutex<AddressInfo>>,
        l1_client: Arc<L1Client>,
        prover_registry: Arc<ProverRegistry>,
        enclave_client: HttpClient,
        l2_endpoint: String,
        max_block_trace_workers: usize,
    ) -> Result<Self> {
        let block_tracer = BlockTracer::new(l2_endpoint, max_block_trace_workers)?;
        Ok(Self {
            state_manager: Arc::new(StateManager::new()),
            l1_client,
            prover_registry,
            enclave_client: Arc::new(enclave_client),
            block_tracer,
            prover_address,
        })
    }

    async fn prove_batch(&self, request: ProveBatchRequest) -> ProveBatchResponse {
        loop {
            match self.enclave_client.prove_batch(request.clone()).await {
                Ok(resp) => {
                    break resp;
                }
                Err(_err) => {
                    // todo add log
                    tokio::time::sleep(core::time::Duration::from_secs(5));
                }
            }
        }
    }

    async fn prove_bundle(
        enclave_client: Arc<HttpClient>,
        request: ProveBundleRequest,
    ) -> Result<ProveBundleResponse, TaskManagerError> {
        let response = loop {
            match enclave_client.prove_bundle(request.clone()).await {
                Ok(resp) => {
                    break resp;
                }
                Err(_err) => {
                    // todo add log
                    tokio::time::sleep(core::time::Duration::from_secs(5));
                }
            }
        };
        Ok(response)
    }

    async fn submit_bundle_proof(
        l1_client: Arc<L1Client>,
        batch_header: Bytes,
        post_state_root: StateRoot,
        withdraw_root: WithdrawRoot,
        tee_proof: Bytes,
    ) -> Result<(), TaskManagerError> {
        loop {
            match l1_client
                .finalize_bundle_with_tee_proof(
                    batch_header.clone(),
                    post_state_root,
                    withdraw_root,
                    tee_proof.clone(),
                )
                .await
            {
                Ok(_resp) => {
                    break;
                }
                Err(_err) => {
                    // todo add log
                    tokio::time::sleep(core::time::Duration::from_secs(5));
                }
            }
        }
        Ok(())
    }

    async fn shift_to_next_prover(
        &self,
        sender: &Sender<NextProver>,
    ) -> Result<(), TaskManagerError> {
        let next_prover: NextProver = self.prover_registry.get_next_prover().await?;
        sender.send(next_prover).await;
        Ok(())
    }

    pub async fn start(
        &mut self,
        mut event_log_rx: Receiver<ScrollChainEventLog>,
    ) -> Result<(), TaskManagerError> {
        let (next_prover_tx, next_prover_rx) = mpsc::channel(1);

        let state_manager = self.state_manager.clone();
        let enclave_client = self.enclave_client.clone();
        let l1_client = self.l1_client.clone();
        let prover_address = self.prover_address.clone();
        tokio::spawn(async move {
            TaskManager::start_prove_bundle_task(
                prover_address,
                state_manager,
                enclave_client,
                l1_client,
                next_prover_rx,
            )
            .await
        });

        self.shift_to_next_prover(&next_prover_tx).await?;

        while let Some(event) = event_log_rx.recv().await {
            match event {
                ScrollChainEventLog::CommitBatch(commit_batch) => {
                    let batch_index = commit_batch.batch_index;
                    if let Ok(prove_batch_request) = self
                        .state_manager
                        .on_batch_commit_event_received(commit_batch, &self.block_tracer)
                        .await
                    {
                        let prove_batch_response = self.prove_batch(prove_batch_request).await;
                        self.state_manager
                            .on_batch_proved(batch_index, prove_batch_response);
                    } else {
                        // todo, retry or handle error
                    }
                }
                ScrollChainEventLog::FinalizeBundle(verify_batch) => {
                    self.state_manager
                        .on_batch_finalized_event_received(verify_batch);

                    self.shift_to_next_prover(&next_prover_tx).await?;
                }
                ScrollChainEventLog::ChangeBundleSize(size) => {
                    self.state_manager.on_bundle_size_updated_received(size);
                }
            }
        }
        Ok(())
    }

    async fn prove_bundle_and_submit(
        enclave_client: Arc<HttpClient>,
        l1_client: Arc<L1Client>,
        request: ProveBundleRequest,
    ) -> Result<(), TaskManagerError> {
        let response = TaskManager::prove_bundle(enclave_client, request.clone()).await?;
        let last_index = request.batch_headers.len() - 1;
        let batch_header = request.batch_headers[last_index].clone();
        let post_state_root = request.state_roots[last_index];
        let withdraw_root = request.withdraw_roots[last_index];
        let tee_proof = response.signature.as_bytes().into();

        TaskManager::submit_bundle_proof(
            l1_client,
            batch_header,
            post_state_root,
            withdraw_root,
            tee_proof,
        )
        .await?;

        Ok(())
    }

    async fn handle_next_bundle(
        state_manager: Arc<StateManager>,
        enclave_client: Arc<HttpClient>,
        l1_client: Arc<L1Client>,
    ) -> Result<(), TaskManagerError> {
        let mut interval = interval(Duration::from_secs(60));
        let request = loop {
            match state_manager.try_build_prove_bundle_request() {
                Ok(request) => {
                    break request;
                }
                Err(err) => match err {
                    StateManagerError::BatchesNotEnough(current, next_batch_index_for_bundle) => {
                        log::info!("failed to build prove_bundle_request, batches not enough. current: {}, next_batch_index_for_bundle: {}", current, next_batch_index_for_bundle);
                    }
                    StateManagerError::Fatal(_msg) => {}
                    other => {
                        log::warn!("failed to build prove_bundle_request, other: {:?}", other);
                    }
                },
            }
            interval.tick().await;
        };
        TaskManager::prove_bundle_and_submit(enclave_client, l1_client, request).await
    }

    async fn start_prove_bundle_task(
        prover_address: Arc<Mutex<AddressInfo>>,
        state_manager: Arc<StateManager>,
        enclave_client: Arc<HttpClient>,
        l1_client: Arc<L1Client>,
        mut next_prover_rx: Receiver<NextProver>,
    ) {
        async fn wait_until(expire_time: u64) {
            let now_ts = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if expire_time > now_ts {
                // todo, add a random jitter
                let jitter = 10;
                tokio::time::sleep(Duration::from_secs(expire_time - now_ts + jitter)).await;
            }
        }

        let mut should_wait = false;
        let wait_until_expire = wait_until(u64::MAX);
        tokio::pin!(wait_until_expire);
        let mut prove_bundle_handle = None;
        loop {
            tokio::select! {
                _ = &mut wait_until_expire, if should_wait => {
                    should_wait = false;
                    log::info!("expired for selected prover");

                    let state_manager_clone = state_manager.clone();
                    let enclave_client_clone = enclave_client.clone();
                    let l1_client_clone = l1_client.clone();
                    // in case the selected prover will not create prove_bundle task twice
                    if prove_bundle_handle.is_none() {
                        prove_bundle_handle = Some(tokio::spawn(async move {
                            TaskManager::handle_next_bundle(
                                state_manager_clone,
                                enclave_client_clone,
                                l1_client_clone
                            ).await
                        }));
                    }
                }
                Some(next_prover) = next_prover_rx.recv() => {
                    wait_until_expire.set(wait_until(next_prover.expire_time));
                    should_wait = true;
                    prove_bundle_handle = None; // drop prove_bundle task

                    let address = {
                        prover_address.lock().unwrap().address
                    };
                    if address == next_prover.address {
                        let state_manager_clone = state_manager.clone();
                        let enclave_client_clone = enclave_client.clone();
                        let l1_client_clone = l1_client.clone();
                        prove_bundle_handle = Some(tokio::spawn(async move {
                            TaskManager::handle_next_bundle(
                                state_manager_clone,
                                enclave_client_clone,
                                l1_client_clone
                            ).await
                        }));
                    }
                }
            }
        }
    }
}
