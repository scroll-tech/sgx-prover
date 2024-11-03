use task_manager::TaskManager;
use tokio::sync::mpsc;

use event_log_fetcher::EventLogFetcher;
use rpc::ScrollSgxClient;
use types::{CommitBatchEvent, FinalizeBatchEvent};

mod config;
mod types;
mod utils;
mod l1_client;
mod event_log_fetcher;
mod event_log_parser;
mod task_manager;
mod state_manager;
mod prover;
mod block_tracer;


pub async fn host_entrypoint() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let (commit_batch_tx, commit_batch_rx) = mpsc::channel::<CommitBatchEvent>(32);

    let (finalize_batch_tx, finalize_batch_rx) = mpsc::channel::<FinalizeBatchEvent>(32);

    let event_fetcher = EventLogFetcher::new(commit_batch_tx, finalize_batch_tx);


    let h = tokio::spawn(async move {
        event_fetcher.start();
    });

    let task_manager = TaskManager::new();
    TaskManager::start(task_manager, commit_batch_rx, finalize_batch_rx).await;

    tokio::join!(h);


    let client = rpc::create_client("http://127.0.0.1:1234").unwrap();
    let res = client.get_address().await.unwrap();
    log::info!("res = {:?}", res);
}
