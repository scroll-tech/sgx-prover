use crate::types::{CommitBatchEvent, VerifyBatchEvent};

use super::*;
use alloy::{primitives::hex, rpc::types::Log, sol_types::SolCall};
use std::sync::Arc;

pub struct EventLogParser {
    l1_client: Arc<L1Client>,
}

// todo, move to another crate
fn decode_block_numbers(mut data: &[u8]) -> Option<Vec<u64>> {
    if data.len() < 1 {
        return None;
    }
    let num_blocks = data[0] as usize;
    data = &data[1..];
    if data.len() < num_blocks * 60 {
        return None;
    }

    let mut numbers = Vec::new();
    let mut tmp = [0_u8; 8];
    for i in 0..num_blocks {
        tmp.copy_from_slice(&data[i * 60..i * 60 + 8]);
        let block_number = u64::from_be_bytes(tmp);
        numbers.push(block_number);
    }
    Some(numbers)
}

impl EventLogParser {
    pub fn new(l1_client: Arc<L1Client>) -> Self {
        Self { l1_client }
    }

    pub async fn parse_commit_batch_log(
        &self,
        log: Log,
    ) -> Result<CommitBatchEvent, EventLogError> {
        let log_decoded: Log<ScrollChain::CommitBatch> =
            log.log_decode().map_err(EthError::from)?;

        if log.transaction_hash.is_none() {
            return Err(EventLogError::Parser("empty transaction hash".into()));
        }

        let tx = self
            .l1_client
            .get_transaction_by_hash(log.transaction_hash.unwrap())
            .await
            .map_err(EthError::from)?;
        if tx.is_none() {
            return Err(EventLogError::Parser("empty transaction".into()));
        }
        let input = hex::decode(tx.unwrap().input)
            .map_err(|err| EventLogError::Parser(format!("{err:?}").into()))?;

        let tx_decoded = ScrollChain::commitBatchWithBlobProofCall::abi_decode(&input, false)
            .map_err(EthError::from)?;

        let mut chunks = vec![];
        for chunk in tx_decoded._chunks {
            if let Some(blks) = decode_block_numbers(&chunk) {
                chunks.push(blks);
            } else {
                todo!()
            }
        }

        Ok(CommitBatchEvent {
            batch_index: log_decoded.data().batchIndex.to(),
            batch_hash: log_decoded.data().batchHash,
            batch_version: tx_decoded._version,
            chunks,
            prev_batch_header: tx_decoded._parentBatchHeader,
        })
    }

    pub async fn parse_finalize_batch_log(
        &self,
        log: Log,
    ) -> Result<VerifyBatchEvent, EventLogError> {
        let log_decoded: Log<ScrollChain::FinalizeBatch> =
            log.log_decode().map_err(EthError::from)?;

        if log.transaction_hash.is_none() {
            return Err(EventLogError::Parser("empty transaction hash".into()));
        }

        let tx = self
            .l1_client
            .get_transaction_by_hash(log.transaction_hash.unwrap())
            .await
            .map_err(EthError::from)?;
        if tx.is_none() {
            return Err(EventLogError::Parser("empty transaction".into()));
        }
        let input = hex::decode(tx.unwrap().input)
            .map_err(|err| EventLogError::Parser(format!("{err:?}").into()))?;

        // Decode the input using the generated `swapExactTokensForTokens` bindings.
        let tx_decoded = ScrollChain::finalizeBundleWithProofCall::abi_decode(&input, false)
            .map_err(EthError::from)?;

        Ok(VerifyBatchEvent {
            batch_index: log_decoded.data().batchIndex.to(),
            batch_hash: log_decoded.data().batchHash,
            end_batch_header: tx_decoded._batchHeader,
            end_state_root: log_decoded.data().stateRoot,
            end_withdraw_root: log_decoded.data().withdrawRoot,
        })
    }
}
