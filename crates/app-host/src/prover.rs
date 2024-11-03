use jsonrpsee::http_client::HttpClient;
use rpc::ScrollSgxClient;
use rpc::{ProveBatchRequest, ProveBatchResponse, ProveBundleRequest, ProveBundleResponse};
use tee::ProverRegistry;

use crate::l1_client::L1Client;
use std::sync::Arc;
use anyhow::Result;

pub struct Prover {
    enclave_client: HttpClient,
    l1_client: Arc<L1Client>,
}

impl Prover {
    pub fn new() -> Self {
        todo!()
    }

    // async fn submit_attestation_report() {

    // }

    // pub async fn prove_bundle(&self, request: ProveBundleRequest) -> Result<ProveBundleResponse> {
    //     let response = self.enclave_client.prove_bundle(request).await?;
    //     Ok(response)
    // }

    // pub async fn prove_batch(&self, request: ProveBatchRequest) -> Result<ProveBatchResponse> {
    //     let response = self.enclave_client.prove_batch(request).await?;
    //     Ok(response)
    // }

    // pub async fn submit_bundle_proof(&self) {
    //     self.l1_client.finalize_bundle_with_tee_proof();
    // }

    pub async fn start(prover_registry: ProverRegistry) -> () {

    }
}