# Scroll Prover
[![Automata SGX SDK](https://img.shields.io/badge/Power%20By-Automata%20SGX%20SDK-orange.svg)](https://github.com/automata-network/automata-sgx-sdk)

## Compile Contract

1. Install forge: https://book.getfoundry.sh/getting-started/installation
2. cd contracts && forge install

## Deploy Contract
```
$ anvil --fork-url ${SEPOLIA_RPC_URL}
$ ENV=local_sepolia ./scripts/deploy_contract.sh
{
  "AttestationVerifier": "0x2bB4d51B747558CD9AA07aA6819D6b1a1590a595",
  "ProxyRegistry": "0x02D6f953722A085cC8325D442d931aD6c12a7210",
  "remark": "Deployment"
}
```


## Generate Signing Key
```
$ cargo sgx gen-key bin/sgx-scroll-verifier/sgx/private.pem
```

## Run Demo

```
# cargo install cargo-sgx
$ cargo sgx run --release -- --download-from ${scroll_node} testdata/scroll-mainnet-v3-commit-310004.calldata --private-key 0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a --registry-addr 0x02D6f953722A085cC8325D442d931aD6c12a7210

# run in non-SGX simulation mode
$ SGX_MODE=SW cargo sgx run --release -- --download-from ${scroll_node} testdata/scroll-mainnet-v3-commit-310004.calldata --private-key 0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a --registry-addr 0x02D6f953722A085cC8325D442d931aD6c12a7210

# run in Apple Silicon Chips, the on-chain functionality will be turned off
$ STD_MODE=true cargo sgx run --release -- --download-from ${scroll_node} testdata/scroll-mainnet-v3-commit-310004.calldata --private-key 0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a --registry-addr 0x02D6f953722A085cC8325D442d931aD6c12a7210
```