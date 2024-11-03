use crate::{l1_client::L1Client, types::{CommitBatchEvent, FinalizeBatchEvent, ScrollChain}, utils::convert_eth_error};

use alloy::{primitives::hex, rpc::types::Log, sol, sol_types::SolCall};
use anyhow::{bail, Result};

use std::sync::Arc;

sol!(
    #[allow(missing_docs)]
    function commitBatchWithBlobProof(
        uint8 version,
        bytes calldata parentBatchHeader,
        bytes[] memory chunks,
        bytes calldata skippedL1MessageBitmap,
        bytes calldata blobDataProof
    ) external;
);

sol!(
    #[allow(missing_docs)]
    function finalizeBundleWithProof(
        bytes calldata _batchHeader,
        bytes32 _postStateRoot,
        bytes32 _withdrawRoot,
        bytes calldata _aggrProof
    ) external;
);

pub struct EventLogParser {
    l1_client: Arc<L1Client>
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
    pub async fn parse_commit_batch_log(&self, log: Log) -> Result<CommitBatchEvent> {
        let log_decoded: Log<ScrollChain::CommitBatch> = log.log_decode()?;

        if log.transaction_hash.is_none() {
            bail!("empty transaction hash");
        }

        let tx = self.l1_client.get_transaction_by_hash(log.transaction_hash.unwrap()).await.map_err(|e| convert_eth_error(e))?;
        if tx.is_none() {
            bail!("empty transaction")
        }
        let input = hex::decode(tx.unwrap().input)?;

        let tx_decoded = commitBatchWithBlobProofCall::abi_decode(&input, false)?;

        let mut chunks = vec![];
        for chunk in tx_decoded.chunks {
            if let Some(blks) = decode_block_numbers(&chunk) {
                chunks.push(blks);
            } else {
                todo!()
            }
        }

        Ok(CommitBatchEvent{
            batch_index: log_decoded.data().batchIndex.to(),
            batch_hash: log_decoded.data().batchHash,
            batch_version: tx_decoded.version,
            chunks,
            prev_batch_header: tx_decoded.parentBatchHeader,
        })
    }

    pub async fn parse_finalize_batch_log(&self, log: Log) -> Result<FinalizeBatchEvent> {
        let log_decoded: Log<ScrollChain::FinalizeBatch> = log.log_decode()?;

        if log.transaction_hash.is_none() {
            bail!("empty transaction hash");
        }

        let tx = self.l1_client.get_transaction_by_hash(log.transaction_hash.unwrap()).await.map_err(|e| convert_eth_error(e))?;
        if tx.is_none() {
            bail!("empty transaction")
        }
        let input = hex::decode(tx.unwrap().input)?;

        // Decode the input using the generated `swapExactTokensForTokens` bindings.
        let tx_decoded = finalizeBundleWithProofCall::abi_decode(&input, false)?;

        Ok(FinalizeBatchEvent{
            batch_index: log_decoded.data().batchIndex.to(),
            batch_hash: log_decoded.data().batchHash,
            end_batch_header: tx_decoded._batchHeader,
            end_state_root: log_decoded.data().stateRoot,
            end_withdraw_root: log_decoded.data().withdrawRoot,
        })
    }
}