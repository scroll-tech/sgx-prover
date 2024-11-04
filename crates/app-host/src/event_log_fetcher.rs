use crate::{
    l1_client::{self, L1Client},
    types::{CommitBatchEvent, FinalizeBatchEvent, ScrollChain},
};
use alloy::{eips::BlockNumberOrTag, primitives::Address, sol_types::SolEvent};
use anyhow::Result;
use base::eth::EthError;
use event_log_parser::EventLogParser;
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::mpsc::Sender, time::interval};

base::stack_error! {
    #[derive(Debug)]
    name: EventLogError,
    stack_name: EventLogErrorStack,
    error: {
        Eth(EthError),
        Fetcher(std::borrow::Cow<'static, str>),
        Parser(std::borrow::Cow<'static, str>),
        Other(std::borrow::Cow<'static, str>),
    },
    wrap: {
    },
    stack: {}
}

impl From<EthError> for EventLogError {
    fn from(value: EthError) -> Self {
        Self::Eth(value)
    }
}

pub struct EventLogFetcher {
    event_log_parser: EventLogParser,
    l1_client: Arc<L1Client>,
    scroll_chain_address: Address,
    max_size_per_fetch: u64,
    fetch_interval_seconds: u64,
    fetched_block_number: u64,
    finalized_block_number: u64,

    commit_batch_tx: Sender<CommitBatchEvent>,
    finalize_batch_tx: Sender<FinalizeBatchEvent>,
}

impl EventLogFetcher {
    pub fn new(
        l1_client: Arc<L1Client>,
        scroll_chain_address: Address,
        max_size_per_fetch: u64,
        fetch_interval_seconds: u64,
        commit_batch_tx: Sender<CommitBatchEvent>,
        finalize_batch_tx: Sender<FinalizeBatchEvent>,
    ) -> Self {
        Self {
            event_log_parser: EventLogParser::new(l1_client.clone()),
            l1_client,
            scroll_chain_address,
            max_size_per_fetch,
            fetch_interval_seconds,
            fetched_block_number: 0,
            finalized_block_number: 0,
            commit_batch_tx,
            finalize_batch_tx,
        }
    }

    async fn get_latest_finalized_block(&self) -> Result<u64, EventLogError> {
        let block = self
            .l1_client
            .get_block_by_number(BlockNumberOrTag::Finalized)
            .await
            .map_err(EthError::from)?;
        if block.is_none() {
            return Err(EventLogError::Fetcher("get empty block".into()));
        }
        match block.unwrap().header.number {
            Some(n) => Ok(n),
            None => Err(EventLogError::Fetcher("no block number in header".into())),
        }
    }

    async fn fetch_logs(&self) -> Result<(), EventLogError> {
        let last_finalized_block = self.get_latest_finalized_block().await?;
        let event_signatures = vec![
            ScrollChain::CommitBatch::SIGNATURE_HASH,
            ScrollChain::FinalizeBatch::SIGNATURE_HASH,
        ];

        let from: u64 = self.fetched_block_number;
        let mut to = from + self.max_size_per_fetch;
        if to > last_finalized_block {
            to = last_finalized_block;
        }

        let logs = self
            .l1_client
            .get_logs(self.scroll_chain_address, event_signatures, from, to)
            .await
            .map_err(EthError::from)?;

        for log in logs {
            match log.topic0() {
                Some(&ScrollChain::CommitBatch::SIGNATURE_HASH) => {
                    match self.event_log_parser.parse_commit_batch_log(log).await {
                        Ok(event) => {
                            self.commit_batch_tx.send(event).await;
                        }
                        Err(err) => {
                            todo!()
                        }
                    }
                }
                Some(&ScrollChain::FinalizeBatch::SIGNATURE_HASH) => {
                    match self.event_log_parser.parse_finalize_batch_log(log).await {
                        Ok(event) => {
                            self.finalize_batch_tx.send(event).await;
                        }
                        Err(err) => {
                            todo!()
                        }
                    }
                }
                _ => {
                    todo!()
                }
            }
        }

        Ok(())
    }

    pub async fn start(&self) -> () {
        let mut interval = interval(Duration::from_secs(self.fetch_interval_seconds));
        loop {
            self.fetch_logs();

            interval.tick().await;
        }
    }
}

mod event_log_parser {
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
        ) -> Result<FinalizeBatchEvent, EventLogError> {
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

            Ok(FinalizeBatchEvent {
                batch_index: log_decoded.data().batchIndex.to(),
                batch_hash: log_decoded.data().batchHash,
                end_batch_header: tx_decoded._batchHeader,
                end_state_root: log_decoded.data().stateRoot,
                end_withdraw_root: log_decoded.data().withdrawRoot,
            })
        }
    }
}
