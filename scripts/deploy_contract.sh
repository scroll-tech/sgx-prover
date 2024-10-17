#!/bin/bash -e

. $(dirname $0)/env.sh

function deploy() {
    if [[ "$CHAIN_ID" == "" ]]; then
        # scroll mainnet: 534352
        # scroll sepolia testnet: 534351
        CHAIN_ID=534352
    fi 
    if [[ "$ATTEST_VALIDITY_SECONDS" == "" ]]; then
        ATTEST_VALIDITY_SECONDS=3600
    fi
    if [[ "$MAX_BLOCK_NUMBER_DIFF" == "" ]]; then
        MAX_BLOCK_NUMBER_DIFF=25
    fi 
    if [[ ! -f "contracts/deployment/tee_deploy_$ENV.json" ]]; then
        mkdir -p contracts/deployment
        echo '{"remark": "Deployment"}' > contracts/deployment/tee_deploy_$ENV.json
    fi

    CHAIN_ID=$CHAIN_ID \
    ATTEST_VALIDITY_SECONDS=$ATTEST_VALIDITY_SECONDS \
    MAX_BLOCK_NUMBER_DIFF=$MAX_BLOCK_NUMBER_DIFF \
    AUTOMATA_DCAP_ATTESTATION=$(_get_env AUTOMATA_DCAP_ATTESTATION) \
    DEPLOY_KEY_SUFFIX=DEPLOY_KEY \
    ENV=$ENV \
    _script script/Deploy.s.sol --sig 'deployAll()'

    cat contracts/deployment/tee_deploy_$ENV.json
}

deploy "$@"