use std::net::SocketAddr;

use alloy::primitives::{address, Address};

use jsonrpsee::core::async_trait;
use jsonrpsee::server::Server;
use jsonrpsee::types::ErrorObjectOwned;

use crate::enclave_signer::EnclaveSigner;
use crate::error::*;
use crate::methods::ScrollSgxServer;
use crate::prove::prove_batch;
use crate::types::*;
use crate::utils::*;

pub struct ScrollSgxServerImpl {
    signer: EnclaveSigner,
}

#[async_trait]
impl ScrollSgxServer for ScrollSgxServerImpl {
    async fn hello(&self) -> Result<String, ErrorObjectOwned> {
        Ok("hello".to_owned())
    }

    async fn get_address(&self) -> Result<Address, ErrorObjectOwned> {
        Ok(self.signer.address())
    }

    async fn generate_attestation_report(&self) -> Result<String, ErrorObjectOwned> {
        todo!()
    }

    async fn prove_batch(
        &self,
        req: ProveBatchRequest,
    ) -> Result<ProveBatchResponse, ErrorObjectOwned> {
        if req.blocks.len() == 0 {
            return Err(invalid_params("empty block list"));
        }

        if ethers_hash_to_alloy(req.blocks[0].storage_trace.root_before) != req.prev_state_root {
            return Err(invalid_params("prev_state_root mismatch"));
        }

        let sig_data = prove_batch(req).await.ok_or_internal_error()?;

        let signature = self.signer.sign(&sig_data).await.ok_or_internal_error()?;

        Ok(ProveBatchResponse {
            batch_hash: sig_data.batchHash,
            post_state_root: sig_data.postStateRoot,
            post_withdraw_root: sig_data.postWithdrawRoot,
            signature,
        })
    }

    async fn prove_bundle(
        &self,
        _req: ProveBundleRequest,
    ) -> Result<ProveBundleResponse, ErrorObjectOwned> {
        todo!()
    }
}

pub async fn run_server() -> anyhow::Result<SocketAddr> {
    // TODO: pass these from command line
    let chain_id = 534352;
    let verifying_contract = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");

    let signer = EnclaveSigner::new(chain_id, verifying_contract);
    log::info!("Generated new prover identity {}", signer.address());

    let server = Server::builder()
        .build("127.0.0.1:1234".parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr()?;

    let server_impl = ScrollSgxServerImpl { signer };
    let handle = server.start(server_impl.into_rpc());

    handle.stopped().await;

    Ok(addr)
}
