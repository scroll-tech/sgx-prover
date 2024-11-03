use alloy::{
    primitives::Address,
    rpc::types::TransactionReceipt,
    sol_types::SolEvent,
};
use ProverRegistryStub::ProverRegistryStubErrors;

use base::eth::{Eth, EthError};

#[derive(Clone)]
pub struct ProverRegistry {
    eth: Eth,
    contract: Address,
}

pub use ProverRegistryStub::{registerCall as RegisterCall, ReportData};

base::stack_error! {
    #[derive(Debug)]
    name: RegistryError,
    stack_name: RegistryErrorStack,
    error: {
        Revert(ProverRegistryStubErrors, EthError),
        Eth(EthError),
        MissingInstanceIdOnRegister,
    },
    wrap: {
    },
    stack: {}
}

impl From<EthError> for RegistryError {
    fn from(value: EthError) -> Self {
        match value.revert_data::<ProverRegistryStubErrors>() {
            Ok((err, value)) => Self::Revert(err, value),
            Err(err) => Self::Eth(err),
        }
    }
}

impl ProverRegistry {
    pub fn new(eth: Eth, contract: Address) -> Self {
        Self { eth, contract }
    }

    pub async fn attest_validity_seconds(&self) -> Result<u64, RegistryError> {
        let call = ProverRegistryStub::attestValiditySecondsCall {};
        Ok(self.eth.call(self.contract, &call).await?._0.to())
    }

    pub fn address(&self) -> Address {
        self.contract
    }

    fn get_event<T: SolEvent + Clone>(receipt: &TransactionReceipt) -> Option<T> {
        for log in receipt.inner.logs() {
            if let Ok(event) = log.log_decode::<T>() {
                return Some(event.data().clone());
            }
        }
        return None;
    }

    pub async fn register<T>(&self, report: T) -> Result<Registration, RegistryError>
    where
        T: Into<RegisterCall>,
    {
        use ProverRegistryStub::*;

        let call = report.into();

        let tx = self.eth.transact(self.contract, &call).await?;
        log::info!("[register] waiting receipt for: {:?}", tx.tx_hash());
        let receipt = tx.get_receipt().await.map_err(EthError::from)?;

        let instance_add = Self::get_event::<ProverRegistered>(&receipt)
            .ok_or(RegistryError::MissingInstanceIdOnRegister)?;

        Ok(Registration {
            address: instance_add.prover,
            valid_until: instance_add.validUntil.to(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Registration {
    pub address: Address,
    pub valid_until: u64,
}

alloy::sol! {
    #[derive(Debug, Default)]
    ProverRegistryStub,
    "abi/SGXVerifier.json"
}
