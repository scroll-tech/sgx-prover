use std::net::SocketAddr;

use alloy::primitives::{address, Address};

use jsonrpsee::core::async_trait;
use jsonrpsee::server::Server;
use jsonrpsee::types::ErrorObjectOwned;

use scroll_executor::ExecutionResult;

use crate::enclave_signer::EnclaveSigner;
use crate::error::*;
use crate::methods::ScrollSgxServer;
use crate::prove::execute_block;
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

    async fn prove_block(
        &self,
        req: ProveBlockRequest,
    ) -> Result<ProveBlockResponse, ErrorObjectOwned> {
        if ethers_hash_to_alloy(req.block_trace.storage_trace.root_before) != req.prev_state_root {
            return Err(invalid_params("prev_state_root mismatch"));
        }

        let block_hash = match req.block_trace.header.hash {
            Some(h) => ethers_hash_to_alloy(h),
            None => return Err(invalid_params("missing block hash")),
        };

        let ExecutionResult {
            new_state_root: post_state_root,
            new_withdrawal_root: post_withdraw_root,
        } = execute_block(&req)
            .await
            .map_err(|e| format!("{e:?}"))
            .ok_or_internal_error()?;

        if ethers_hash_to_alloy(req.block_trace.storage_trace.root_after) != post_state_root {
            return Err(invalid_params("post_state_root mismatch"));
        }

        let sig_data = ProveBlockSignatureData {
            block_hash,
            prev_state_root: req.prev_state_root,
            post_state_root,
            post_withdraw_root,
        };

        let signature = self.signer.sign(sig_data).await.ok_or_internal_error()?;

        Ok(ProveBlockResponse {
            post_state_root,
            post_withdraw_root,
            signature,
        })
    }

    async fn prove_batch(
        &self,
        _req: ProveBatchRequest,
    ) -> Result<ProveBatchResponse, ErrorObjectOwned> {
        todo!()
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
