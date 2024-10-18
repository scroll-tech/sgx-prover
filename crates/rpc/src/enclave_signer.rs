use alloy::primitives::{keccak256, Address, ChainId, Signature};
use alloy::signers::{local::PrivateKeySigner, Signer};
use alloy::sol_types::{eip712_domain, Eip712Domain, SolStruct};

#[derive(Clone, Debug)]
pub struct EnclaveSigner {
    signer: PrivateKeySigner,
    domain: Eip712Domain,
}

impl EnclaveSigner {
    pub fn new(chain_id: ChainId, verifying_contract: Address) -> Self {
        let signer = PrivateKeySigner::random();

        let domain = eip712_domain! {
            name: "ScrollChain",
            version: "1",
            chain_id: chain_id,
            verifying_contract: verifying_contract,
            salt: keccak256("test"),
        };

        Self { signer, domain }
    }

    pub fn address(&self) -> Address {
        self.signer.address()
    }

    pub async fn sign(&self, data: impl SolStruct) -> anyhow::Result<Signature> {
        let hash = data.eip712_signing_hash(&self.domain);
        let signature = self.signer.sign_hash(&hash).await?;
        Ok(signature)
    }
}
