use crate::types::*;

use alloy::primitives::Address;

use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::ErrorObjectOwned;

#[rpc(server, client, namespace = "sgx")]
pub trait ScrollSgx {
    #[method(name = "hello")]
    async fn hello(&self) -> Result<String, ErrorObjectOwned>;

    #[method(name = "getAddress")]
    async fn get_address(&self) -> Result<Address, ErrorObjectOwned>;

    #[method(name = "generateAttestationReport")]
    async fn generate_attestation_report(&self) -> Result<String, ErrorObjectOwned>;

    #[method(name = "proveBatch")]
    async fn prove_batch(
        &self,
        req: ProveBatchRequest,
    ) -> Result<ProveBatchResponse, ErrorObjectOwned>;

    #[method(name = "proveBundle")]
    async fn prove_bundle(
        &self,
        req: ProveBundleRequest,
    ) -> Result<ProveBundleResponse, ErrorObjectOwned>;
}
