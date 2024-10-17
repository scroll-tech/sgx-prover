use alloy::primitives::{Address, Bytes, B256, U256};

use crate::{Keypair, RegisterCall, ReportData};
use base::eth::{Eth, EthError};

#[derive(Clone, Debug)]
pub struct AttestationReport {
    pub report: Bytes,
    pub address: Address,
    pub reference_block_hash: B256,
    pub reference_block_number: U256,
    pub tee_type: U256,
}

pub trait ReportBuilder {
    fn generate_quote(&self, rp: ReportData) -> Bytes;
    fn tee_type(&self) -> U256;
}

impl AttestationReport {
    pub async fn build<B>(builder: &B, eth: &Eth, sk: &Keypair) -> Result<Self, EthError>
    where
        B: ReportBuilder,
    {
        let (number, hash) = eth.select_reference_block().await?;

        let mut report = Self {
            address: sk.address(),
            report: Bytes::new(),
            reference_block_hash: hash,
            reference_block_number: number,
            tee_type: builder.tee_type(),
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
                teeType: value.tee_type,
                referenceBlockHash: value.reference_block_hash,
                referenceBlockNumber: value.reference_block_number,
            },
        }
    }
}
