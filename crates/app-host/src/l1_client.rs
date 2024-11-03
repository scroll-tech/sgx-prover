use alloy::{
    primitives::{Address, Bytes, FixedBytes, TxHash},
    rpc::types::{Block, BlockNumberOrTag, Filter, Log, Transaction, TransactionReceipt},
    sol_types::SolEvent,
};

use base::eth::{Eth, EthError};

use crate::types::{ScrollChain, StateRoot, WithdrawRoot};
use std::sync::Arc;

pub struct L1Client {
    eth: Arc<Eth>,
    // private_key: ,
    scroll_chain_address: Address,
    prover_registry_address: Address,
}

base::stack_error! {
    #[derive(Debug)]
    name: FinalizeError,
    stack_name: FinalizeErrorStack,
    error: {
        Revert(ScrollChain::ScrollChainErrors, EthError),
        Eth(EthError),
        FinalizeEventNotFound,
    },
    wrap: {
    },
    stack: {}
}

impl From<EthError> for FinalizeError {
    fn from(value: EthError) -> Self {
        match value.revert_data::<ScrollChain::ScrollChainErrors>() {
            Ok((err, value)) => Self::Revert(err, value),
            Err(err) => Self::Eth(err),
        }
    }
}

impl L1Client {
    pub fn new() -> Self {
        todo!()
    }

    pub async fn get_block_by_number(&self, block_number: BlockNumberOrTag) -> Result<Option<Block>, EthError> {
        let block = self
        .eth
        .provider()
        .get_block_by_number(block_number, false)
        .await?;

        Ok(block)
    }

    pub async fn get_logs<T: Into<BlockNumberOrTag>>(&self, contract: Address, event_signatures: Vec<FixedBytes<32>>, from: T, to: T) -> Result<Vec<Log>, EthError> {
        let filter = Filter::new()
        .address(contract)
        .event_signature(event_signatures)
        .from_block(from)
        .to_block(to);

        let logs = self.eth.provider().get_logs(&filter).await?;
        Ok(logs)
    }

    pub async fn get_transaction_by_hash(&self, tx_hash: TxHash) -> Result<Option<Transaction>, EthError> {
        let transaction = self
        .eth
        .provider()
        .get_transaction_by_hash(tx_hash)
        .await?;
        Ok(transaction)
    }

    fn get_event<T: SolEvent + Clone>(receipt: &TransactionReceipt) -> Option<T> {
        for log in receipt.inner.logs() {
            if let Ok(event) = log.log_decode::<T>() {
                return Some(event.data().clone());
            }
        }
        return None;
    }

    pub async fn finalize_bundle_with_tee_proof(
        &self,
        batch_header: Bytes,
        post_state_root: StateRoot,
        withdraw_root: WithdrawRoot,
        tee_proof: Bytes,
    ) -> Result<u64, FinalizeError> {
        let call = ScrollChain::finalizeBundleWithTeeProofCall {
            _batchHeader: batch_header,
            _postStateRoot: post_state_root,
            _withdrawRoot: withdraw_root,
            _teeProof: tee_proof,
        };

        let tx = self.eth.transact(self.scroll_chain_address, &call).await?;

        log::info!("[register] waiting receipt for: {:?}", tx.tx_hash());
        let receipt = tx.get_receipt().await.map_err(EthError::from)?;

        let batch_finalized = Self::get_event::<ScrollChain::FinalizeBatchWithTEEProof>(&receipt)
            .ok_or(FinalizeError::FinalizeEventNotFound)?;

        Result::Ok(batch_finalized.batchIndex.to())
    }

    pub async fn get_last_tee_finalized_batch_index(&self) -> Result<u64, EthError> {
        let call = ScrollChain::lastTeeFinalizedBatchIndexCall {};
        self.eth.call(self.scroll_chain_address, &call).await.map(|ret| ret._0.to())
    }
}