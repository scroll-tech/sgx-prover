use alloy::{eips::BlockNumberOrTag, primitives::Address, rpc::types::Log, sol_types::SolEvent};

use crate::{event_log_parser::EventLogParser, l1_client::L1Client, types::{CommitBatchEvent, FinalizeBatchEvent, ScrollChain}, utils::{self, convert_eth_error}};

use std::time::Duration;
use tokio::{sync::mpsc::Sender, time::interval};
use anyhow::{bail, Result};


pub struct EventLogFetcher {
    tx: Sender<Log>,
    event_log_parser: EventLogParser,
    l1_client: L1Client,
    scroll_chain_address: Address,
    max_size_per_fetch: u64,
    fetched_block_number: u64,
    finalized_block_number: u64,

    commit_batch_tx: Sender<CommitBatchEvent>,
    finalize_batch_tx: Sender<FinalizeBatchEvent>,
}

impl EventLogFetcher {
    pub fn new() -> Self {
        todo!()
    }

    async fn get_latest_finalized_block(&self) -> Result<u64> {
        let block = self.l1_client.get_block_by_number(BlockNumberOrTag::Finalized).await.map_err(|e| convert_eth_error(e))?;
        if block.is_none() {
            bail!("get empty block")
        }
        match block.unwrap().header.number {
            Some(n) => Ok(n),
            None => anyhow::bail!("no block number in header")
        }
    }

    async fn fetch_logs(&self) -> Result<()> {
        let last_finalize_block = self.get_latest_finalized_block().await?;
        let event_signatures = vec![
            ScrollChain::CommitBatch::SIGNATURE_HASH,
            ScrollChain::FinalizeBatch::SIGNATURE_HASH,
        ];

        let from: u64 = self.fetched_block_number;
        let mut to = from + self.max_size_per_fetch;
        if to > last_finalize_block {
            to = last_finalize_block;
        }

        let logs = self.l1_client.get_logs(self.scroll_chain_address, event_signatures, from, to).await.map_err(|e| convert_eth_error(e))?;

        for log in logs {
            match log.topic0() {
                Some(&ScrollChain::CommitBatch::SIGNATURE_HASH) => {
                    match self.event_log_parser.parse_commit_batch_log(log).await {
                        Ok(event) => {
                            self.commit_batch_tx.send(event).await;
                        },
                        Err(err) => {
                            todo!()
                        }
                    }
                },
                Some(&ScrollChain::FinalizeBatch::SIGNATURE_HASH) => {
                    match self.event_log_parser.parse_finalize_batch_log(log).await {
                        Ok(event) => {
                            self.finalize_batch_tx.send(event).await;
                        },
                        Err(err) => {
                            todo!()
                        }
                    }
                },
                _ => {
                    todo!()
                }
            }
        }

        Ok(())
    }

    pub async fn start(&self) -> () {
        let mut interval = interval(Duration::from_secs(10));
        loop {
            interval.tick().await;

            self.fetch_logs();
        }
    }
}