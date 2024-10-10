# Scroll Verifier
[![Automata SGX SDK](https://img.shields.io/badge/Power%20By-Automata%20SGX%20SDK-orange.svg)](https://github.com/automata-network/automata-sgx-sdk)

## Generate Signing Key
```
$ cargo sgx gen-key bin/sgx-scroll-verifier/sgx/private.pem
```

## Run
```
# cargo install cargo-sgx
$ cargo sgx run --release -- --download-from ${scroll_node} testdata/scroll-mainnet-v3-commit-310004.calldata

# run in non-SGX simulation mode
$ SGX_MODE=SW cargo sgx run --release -- --download-from ${scroll_node} testdata/scroll-mainnet-v3-commit-310004.calldata
```