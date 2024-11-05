use alloy::primitives::{Address, Bytes, B256, U256};
use serde::{Deserialize, Serialize};

use crate::{RegisterCall, ReportData};
use base::eth::{Eth, EthError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttestationReport {
    pub report: Bytes,
    pub address: Address,
    pub reference_block_hash: B256,
    pub reference_block_number: U256,
}

pub trait ReportBuilder {
    fn generate_quote(&self, rp: ReportData) -> Bytes;
}

impl AttestationReport {
    pub async fn build<B>(builder: &B, eth: &Eth, address: Address) -> Result<Self, EthError>
    where
        B: ReportBuilder,
    {
        let (number, hash) = eth.select_reference_block().await?;

        let mut report = Self {
            address,
            report: Bytes::new(),
            reference_block_hash: hash,
            reference_block_number: number,
        };

        let call: RegisterCall = report.clone().into();
        report.report = builder.generate_quote(call._data);

        Ok(report)
    }
}

impl From<AttestationReport> for RegisterCall {
    fn from(value: AttestationReport) -> Self {
        RegisterCall {
            _report: value.report,
            _data: ReportData {
                addr: value.address,
                referenceBlockHash: value.reference_block_hash,
                referenceBlockNumber: value.reference_block_number,
            },
        }
    }
}
