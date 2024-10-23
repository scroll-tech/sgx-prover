use alloy::primitives::B256;
use eth_types::H256;

pub fn ethers_hash_to_alloy(h: H256) -> B256 {
    h.to_fixed_bytes().into()
}
