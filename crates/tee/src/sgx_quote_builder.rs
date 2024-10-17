use crate::ReportData;
use alloy::primitives::{keccak256, Bytes, U256};

use alloy::sol_types::SolValue;

use crate::ReportBuilder;

pub struct SGXQuoteBuilder {}

impl SGXQuoteBuilder {
    pub fn new() -> Self {
        Self {}
    }
}

impl ReportBuilder for SGXQuoteBuilder {
    fn generate_quote(&self, rp: ReportData) -> Bytes {
        let mut report_data = [0_u8; 64];
        report_data[32..].copy_from_slice(&keccak256(&rp.abi_encode()).0);

        let quote = automata_sgx_sdk::dcap::dcap_quote(report_data).unwrap();
        quote.into()
    }

    fn tee_type(&self) -> U256 {
        U256::from_limbs_slice(&[1])
    }
}
