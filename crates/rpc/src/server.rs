use std::net::SocketAddr;

use jsonrpsee::core::async_trait;
use jsonrpsee::server::Server;
use jsonrpsee::types::ErrorObjectOwned;

use crate::methods::ScrollSgxServer;
use crate::types::*;

pub struct ScrollSgxServerImpl;

#[async_trait]
impl ScrollSgxServer for ScrollSgxServerImpl {
    async fn hello(&self) -> Result<String, ErrorObjectOwned> {
        Ok("hello".to_owned())
    }

    async fn generate_attestation_report(&self) -> Result<String, ErrorObjectOwned> {
        todo!()
    }

    async fn prove_block(
        &self,
        _req: ProveBlockRequest,
    ) -> Result<ProveBlockResponse, ErrorObjectOwned> {
        todo!()
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
    let server = Server::builder()
        .build("127.0.0.1:1234".parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr()?;
    let handle = server.start(ScrollSgxServerImpl.into_rpc());

    handle.stopped().await;

    Ok(addr)
}
