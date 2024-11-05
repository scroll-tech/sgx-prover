use anyhow::Result;

use l2_client::L2Client;
use scroll_executor::BlockTrace;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct BlockTracer {
    l2_client: Arc<l2_client::L2Client>,
    rt: Runtime,
}

impl BlockTracer {
    pub fn new(l2_endpoint: String, max_workers: usize) -> Result<Self> {
        let l2_client = L2Client::dial(&l2_endpoint).map_err(|e|{anyhow::anyhow!("{e:?}")})?;
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(max_workers)
            .build()
            .unwrap();
        Ok(Self {
            l2_client: Arc::new(l2_client),
            rt,
        })
    }

    pub async fn get_block_traces(&self, blocks: Vec<u64>) -> Result<Vec<BlockTrace>> {
        let mut handles = Vec::with_capacity(blocks.len());

        for block in blocks {
            let l2_client = self.l2_client.clone();
            // todo: use a singleton tokio runtime with limited worker to spawn task
            let handle = self
                .rt
                .spawn(async move { BlockTracer::get_block_trace(l2_client, block).await });
            handles.push(handle);
        }

        let mut block_traces = Vec::with_capacity(handles.len());
        for handle in handles {
            let block_trace = handle.await?;
            block_traces.push(block_trace)
        }
        Result::Ok(block_traces)
    }

    async fn get_block_trace(l2_client: Arc<L2Client>, block: u64) -> BlockTrace {
        loop {
            match l2_client.trace_block(block).await {
                Result::Ok(trace) => break trace,
                Err(err) => {
                    // todo, add log,
                }
            }
        }
    }
}

mod l2_client {
    use super::BlockTrace;
    use alloy::primitives::U64;
    use base::eth::{Eth, EthError, PrimitivesConvert};

    #[derive(Clone)]
    pub struct L2Client {
        eth: Eth,
    }

    impl L2Client {
        pub fn dial(url: &str) -> Result<Self, EthError> {
            let eth = Eth::dial(url, None)?;
            Ok(Self { eth })
        }

        pub async fn trace_block(&self, blk: u64) -> Result<BlockTrace, EthError> {
            let blk: U64 = blk.to();
            let block_trace = self
                .eth
                .client()
                .request("scroll_getBlockTraceByNumberOrHash", (blk,))
                .await?;
            Ok(block_trace)
        }
    }
}
