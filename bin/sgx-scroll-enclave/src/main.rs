use automata_sgx_sdk::types::SgxStatus;

automata_sgx_sdk::enclave! {
    name: AppEnclave,
    ecall: {
        fn enclave_entrypoint() -> SgxStatus;
    }
}

pub fn main() {
    let result = AppEnclave::new().enclave_entrypoint().unwrap();
    assert!(result.is_success(), "{:?}", result);
}
