use jsonrpsee::http_client::HttpClient;
use tee::ProverRegistry;

use crate::l1_client::L1Client;
use std::sync::Arc;

pub struct Prover {
    enclave_client: HttpClient,
    prover_registry: ProverRegistry,
    l1_client: Arc<L1Client>,
}

impl Prover {
    pub fn new() -> Self {
        todo!()
    }

    pub async fn start(&self) -> () {

    }
}