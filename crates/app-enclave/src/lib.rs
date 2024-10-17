use automata_sgx_sdk::types::SgxStatus;

#[no_mangle]
pub extern "C" fn enclave_entrypoint() -> SgxStatus {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(rpc::run_server()).unwrap();
    SgxStatus::Success
}
