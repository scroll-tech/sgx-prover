#!/bin/bash -e

. $(dirname $0)/env.sh

AUTOMATA_DCAP_ATTESTATION=$(_get_env AUTOMATA_DCAP_ATTESTATION) _test