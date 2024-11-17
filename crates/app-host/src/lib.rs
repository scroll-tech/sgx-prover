use anyhow::Result;
use base::eth::Eth;
use config::Config;
use l1_client::L1Client;
use liveness_manager::LivenessManager;
use task_manager::TaskManager;
use tokio::sync::mpsc;

use clap::{ArgAction, Parser};
use event_log_fetcher::EventLogFetcher;
use std::sync::Arc;
use types::ScrollChainEventLog;

mod block_tracer;
mod config;
mod event_log_fetcher;
mod l1_client;
mod liveness_manager;
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

    let eth = Eth::dial(&config.l1_endpoint, Some(&config.l1_account_pk))
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;

    let l1_client = Arc::new(L1Client {
        eth: eth.clone(),
        scroll_chain_address: config.scroll_chain_address,
        prover_registry_address: config.prover_registry_address,
    });

    let (event_log_tx, event_log_rx) = mpsc::channel::<ScrollChainEventLog>(128);

    let mut event_fetcher = EventLogFetcher::new(
        l1_client.clone(),
        config.scroll_chain_address,
        config.l1_event_max_size_per_fetch,
        config.l1_event_fetch_interval_seconds,
        event_log_tx,
    );

    let fetcher_handler = tokio::spawn(async move { event_fetcher.start().await });

    let enclave_client = rpc::create_client(config.enclave_endpoint)?;

    let liveness_manager =
        LivenessManager::new(eth, config.prover_registry_address, enclave_client.clone()).await?;

    let prover_address = liveness_manager.get_shared_address_info();

    let liveness_handler = tokio::spawn(async move { liveness_manager.start().await });

    let mut task_manager = TaskManager::new(
        prover_address,
        l1_client.clone(),
        enclave_client,
        config.l2_endpoint,
        config.max_block_trace_workers,
    )
    .await?;

    let task_handler = tokio::spawn(async move { task_manager.start(event_log_rx).await });

    tokio::select! {
        liveness_join = liveness_handler => {
            if let Err(err) = liveness_join.expect("liveness join failed") {
                log::error!("liveness failed: {:?}", err);
            }
        }
        fetcher_join = fetcher_handler => {
            if let Err(err) = fetcher_join.expect("fetcher join failed") {
                log::error!("fetcher failed: {:?}", err);
            }
        }
        task_join = task_handler => {
            if let Err(err) = task_join.expect("task join failed") {
                log::error!("task manager failed: {:?}", err);
            }
        }
    };
    Ok(())
}

pub async fn host_entrypoint() {
    if let Err(err) = start().await {
        log::error!("exit due to error: {:?}", err);
    }
}
