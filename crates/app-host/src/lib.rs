use anyhow::Result;
use base::eth::Eth;
use config::Config;
use l1_client::L1Client;
use task_manager::TaskManager;
use tokio::sync::mpsc;

use clap::{ArgAction, Parser};
use event_log_fetcher::EventLogFetcher;
use std::sync::Arc;
use types::{CommitBatchEvent, FinalizeBatchEvent};

mod block_tracer;
mod config;
mod event_log_fetcher;
mod l1_client;
mod prover;
mod state_manager;
mod task_manager;
mod types;

#[derive(Debug, Parser)]
#[command(version, about = "SGX Prover Host")]
struct Opts {
    #[clap(short, default_value = "18232")]
    port: u64,
    #[clap(long = "config", default_value = "config/config.json")]
    config_file: String,
}

pub async fn start() -> Result<()> {
    let opts = Opts::parse();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let config = Config::from_file(opts.config_file)?;

    let eth = Eth::dial(&config.l1_endpoint, Some(&config.l1_account_pk)).map_err(|e|{anyhow::anyhow!("{e:?}")})?;

    let l1_client = Arc::new(L1Client {
        eth,
        scroll_chain_address: config.scroll_chain_address,
        prover_registry_address: config.prover_registry_address,
    });

    let (commit_batch_tx, commit_batch_rx) = mpsc::channel::<CommitBatchEvent>(32);

    let (finalize_batch_tx, finalize_batch_rx) = mpsc::channel::<FinalizeBatchEvent>(32);

    let event_fetcher = EventLogFetcher::new(
        l1_client.clone(),
        config.scroll_chain_address,
        config.l1_event_max_size_per_fetch,
        config.l1_event_fetch_interval_seconds,
        commit_batch_tx,
        finalize_batch_tx,
    );

    let h = tokio::spawn(async move {
        event_fetcher.start();
    });

    let task_manager = TaskManager::new(
        l1_client.clone(),
        config.enclave_endpoint,
        config.l2_endpoint,
        config.max_block_trace_workers,
    )
    .await?;

    TaskManager::start(task_manager, commit_batch_rx, finalize_batch_rx).await;

    tokio::join!(h);
    Ok(())
}

pub async fn host_entrypoint() {
    if let Err(err) = start().await {
        log::error!("exit due to error: {:?}", err);
    }
}
