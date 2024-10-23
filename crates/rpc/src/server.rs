use std::net::SocketAddr;

use alloy::primitives::{address, Address};

use alloy::sol_types::SolValue;
use jsonrpsee::core::async_trait;
use jsonrpsee::server::Server;
use jsonrpsee::types::ErrorObjectOwned;

use scroll_da_codec::DABatch;

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
        req: ProveBundleRequest,
    ) -> Result<ProveBundleResponse, ErrorObjectOwned> {
        if req.batch_headers.len() == 0 {
            return Err(invalid_params("empty batch list"));
        }

        if [
            req.batch_headers.len(),
            req.state_roots.len(),
            req.withdraw_roots.len(),
            req.signatures.len(),
        ]
        .windows(2)
        .any(|w| w[0] != w[1])
        {
            return Err(invalid_params("array length mismatch"));
        }

        let chain_id = self.signer.chain_id();
        let num_batches = req.batch_headers.len() as u32;

        let prev_state_root = req.prev_state_root;

        let batch = DABatch::from_bytes(&req.last_finalized_batch_header).ok_or_invalid_params()?;
        let prev_batch_hash = batch.hash();

        let mut state_root = prev_state_root;
        let mut batch_hash = prev_batch_hash;
        let mut withdraw_root = Default::default();

        for ii in 0..req.batch_headers.len() {
            let next_state_root = req.state_roots[ii];
            let next_withdraw_root = req.withdraw_roots[ii];
            let signature = req.signatures[ii];

            let next_batch = DABatch::from_bytes(&req.batch_headers[ii]).ok_or_invalid_params()?;
            let next_batch_hash = next_batch.hash();

            // check batch chaining
            if next_batch.parent_batch_hash() != batch_hash {
                return Err(invalid_params("invalid batch chain"));
            }

            // check signature
            let sig_data = ProveBatchSignatureData {
                layer2ChainId: chain_id,
                prevStateRoot: state_root,
                postStateRoot: next_state_root,
                batchHash: next_batch_hash,
                postWithdrawRoot: next_withdraw_root,
            };

            self.signer
                .verify(&sig_data, signature)
                .await
                .ok_or_invalid_params()?;

            state_root = next_state_root;
            batch_hash = next_batch_hash;
            withdraw_root = next_withdraw_root;
        }

        let sig_data = ProveBundleSignatureData {
            layer2ChainId: chain_id,
            numBatches: num_batches,
            prevStateRoot: prev_state_root,
            prevBatchHash: prev_batch_hash,
            postStateRoot: state_root,
            batchHash: batch_hash,
            postWithdrawRoot: withdraw_root,
        };

        let signature = self.signer.sign(&sig_data).await.ok_or_internal_error()?;

        Ok(ProveBundleResponse {
            public_input: sig_data.abi_encode().into(),
            signature,
        })
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
