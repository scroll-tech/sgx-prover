use crate::{
    l1_client::L1Client,
    types::{CommitBatchEvent, FinalizeBatchEvent, ScrollChain},
};
use alloy::{eips::BlockNumberOrTag, primitives::Address, sol_types::SolEvent};
use alloy::rpc::types::Log;
use anyhow::Result;
use base::eth::EthError;
use event_log_parser::EventLogParser;
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::mpsc::Sender, time::interval};

mod event_log_parser;

base::stack_error! {
    #[derive(Debug)]
    name: EventLogError,
    stack_name: EventLogErrorStack,
    error: {
        Eth(EthError),
        Fetcher(std::borrow::Cow<'static, str>),
        Parser(std::borrow::Cow<'static, str>),
        Other(std::borrow::Cow<'static, str>),
        Fatal(String),
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

trait OkOrEventError<T> {
    fn ok_or_fatal_error(self) -> Result<T, EventLogError>;
}

impl<T, E: core::fmt::Debug> OkOrEventError<T> for Result<T, E> {
    fn ok_or_fatal_error(self) -> Result<T, EventLogError> {
        self.map_err(|e| {
            EventLogError::Fatal(format!("{e:?}"))
        })
    }
}

pub struct EventLogFetcher {
    event_log_parser: EventLogParser,
    l1_client: Arc<L1Client>,
    scroll_chain_address: Address,
    max_size_per_fetch: u64,
    fetch_interval_seconds: u64,
    latest_processed_block_number: u64,

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
            latest_processed_block_number: 0,
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

    async fn fetch_logs(&self, from: u64, to: u64) -> Result<Vec<Log>, EventLogError> {
        assert!(from <= to, "invalid fetch_logs range: from {}, to {}", from, to);
        let event_signatures = vec![
            ScrollChain::CommitBatch::SIGNATURE_HASH,
            ScrollChain::FinalizeBatch::SIGNATURE_HASH,
        ];

        let logs = self
            .l1_client
            .get_logs(self.scroll_chain_address, event_signatures, from, to)
            .await
            .map_err(EthError::from)?;

        Ok(logs)
    }

    async fn parse_and_send_logs(&self, logs: Vec<Log>) -> Result<(), EventLogError> {
        let mut parsed_commit_batch_event = vec![];
        let mut parsed_finalize_batch_event = vec![];
        for log in logs {
            match log.topic0() {
                Some(&ScrollChain::CommitBatch::SIGNATURE_HASH) => {
                    let event = self.event_log_parser.parse_commit_batch_log(log).await?;
                    parsed_commit_batch_event.push(event);
                }
                Some(&ScrollChain::FinalizeBatch::SIGNATURE_HASH) => {
                    let event = self.event_log_parser.parse_finalize_batch_log(log).await?;
                    parsed_finalize_batch_event.push(event);
                }
                _ => {},
            }
        }
        for event in parsed_commit_batch_event {
            self.commit_batch_tx.send(event).await.ok_or_fatal_error()?;
        }
        for event in parsed_finalize_batch_event {
            self.finalize_batch_tx.send(event).await.ok_or_fatal_error()?;
        }
        Ok(())
    }

    pub async fn find_block_by_batch_index(&self, target_batch_index: u64) -> Result<u64, EventLogError> {
        let mut range = 100;
        let mut to = self.get_latest_finalized_block().await? - range;
        loop {
            let from = to - range + 1;
            log::info!("fetch_logs, from: {}, to: {}", from, to);
            let logs = self.fetch_logs(from, to).await?;
            let mut first_batch_index: Option<u64> = None;
            let mut last_batch_index: u64 = 0;
            for log in logs {
                if Some(&ScrollChain::CommitBatch::SIGNATURE_HASH) == log.topic0() {
                    let log_decoded: Log<ScrollChain::CommitBatch> = log.log_decode().map_err(EthError::from)?;
                    if first_batch_index.is_none() {
                        first_batch_index = Some(log_decoded.data().batchIndex.to());
                    }
                    last_batch_index = log_decoded.data().batchIndex.to();
                }
            }
            match first_batch_index {
                Some(batch_index) => {
                    if batch_index <= target_batch_index {
                        log::info!("find valid batch_index: {}, target: {}", batch_index, target_batch_index);
                        break Ok(from);
                    } else {
                        log::info!("find invalid batch_index: {}, target: {}", batch_index, target_batch_index);
                        let avg_block_num_per_batch = range / (last_batch_index - batch_index + 1);
                        let estimate_gap = (target_batch_index - batch_index) * avg_block_num_per_batch;
                        to = to - estimate_gap;
                    }
                },
                None => {
                    to = from - 1;
                    range *= 2;
                    log::info!("no batch_index found, expand range to {}", range);
                }
            }
        }
    }

    pub async fn process_to_latest_finalized_block(&mut self) -> Result<(), EventLogError> {
        let mut from: u64 = self.latest_processed_block_number + 1;

        let last_finalized_block = self.get_latest_finalized_block().await?;
        while from <= last_finalized_block {
            let mut to = from + self.max_size_per_fetch - 1;
            if to > last_finalized_block {
                to = last_finalized_block;
            }

            log::info!("fetch_logs, from: {}, to: {}", from, to);
            let logs = self.fetch_logs(from, to).await?;
            self.parse_and_send_logs(logs).await?;

            from += self.max_size_per_fetch;
            self.latest_processed_block_number = to;
        }
        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), EventLogError> {
        // search mode
        let last_tee_finalized_batch_index = self.l1_client.get_last_tee_finalized_batch_index().await?;
        let from_block_number = self.find_block_by_batch_index(last_tee_finalized_batch_index).await?;
        self.latest_processed_block_number = from_block_number - 1;

        // work mode
        let mut interval = interval(Duration::from_secs(self.fetch_interval_seconds));
        loop {
            let begin = self.latest_processed_block_number+1;
            if let Err(err) = self.process_to_latest_finalized_block().await {
                if let EventLogError::Fatal(_) = err {
                    break Err(err);
                }
                log::error!("error in process_to_latest_finalized_block, {:?}, latest_processed_block_number: {}", err, self.latest_processed_block_number)
            } else {
                log::info!("finish a round of process_to_latest_finalized_block, begin: {}, end: {}", begin, self.latest_processed_block_number)
            }

            interval.tick().await;
        }
    }
}
