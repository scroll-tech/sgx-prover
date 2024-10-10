# scroll-verifier


## Generate Signing Key
```
$ cargo sgx gen-key bin/sgx-scroll-verifier/sgx/private.pem
```

## Run
```
# cargo install cargo-sgx
$ cargo sgx run --release -- testdata/scroll-mainnet-v3-commit-310004.calldata

# run in non-SGX simulation mode
$ SGX_MODE=SW cargo sgx run --release -- testdata/scroll-mainnet-v3-commit-310004.calldata
```