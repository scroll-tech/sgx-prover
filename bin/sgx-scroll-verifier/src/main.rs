use automata_sgx_sdk::types::SgxStatus;

automata_sgx_sdk::enclave!{
    name: AppScrollVerifier,
    ecall: {
        fn enclave_entrypoint() -> SgxStatus;
    }
}

pub fn main() {
    let result = AppScrollVerifier::new().enclave_entrypoint().unwrap();
    assert!(result.is_success(), "{:?}", result);
}