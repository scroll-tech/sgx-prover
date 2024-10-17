#![allow(dead_code)]
#![allow(unused_variables)]

use std::path::{Path, PathBuf};

use automata_sgx_sdk::types::SgxStatus;
use base::{eth::Eth, thread::parallel, trace::Alive};
use clap::Parser;
use scroll_da_codec::{BatchTask, Finalize};
use scroll_executor::{Address, BlockTrace};
use scroll_verifier::{
    block_trace_to_pob, HardforkConfig, PobContext, ScrollBatchVerifier, ScrollExecutionNode,
};
use tee::{AttestationReport, Keypair, ProverRegistry, SGXQuoteBuilder};

#[derive(Debug, Parser)]
struct Opt {
    #[clap(long, default_value = "")]
    download_from: String,
    #[clap(long, default_value = "http://localhost:8545")]
    l1_endpoint: String,
    #[clap(long)]
    private_key: String,
    #[clap(long)]
    registry_addr: Address,
    txs: Vec<PathBuf>,
}

#[no_mangle]
pub extern "C" fn enclave_entrypoint() -> SgxStatus {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run_verifier());
    SgxStatus::Success
}

fn read_batch_task(path: &PathBuf) -> BatchTask {
    assert!(
        path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("commit"),
        "should use commit tx"
    );
    let commit_tx_calldata = std::fs::read(&path).unwrap();
    let commit_tx_calldata = hex::decode(&commit_tx_calldata[2..]).unwrap();
    let batch = BatchTask::from_calldata(&commit_tx_calldata[4..]).unwrap();
    batch
}

fn read_finalize(path: &PathBuf) -> Finalize {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let file_name = file_name.replace("commit", "finalize");
    let path = path.parent().unwrap().join(file_name);
    let calldata = std::fs::read(&path).unwrap();
    let calldata = hex::decode(&calldata[2..]).unwrap();
    Finalize::from_calldata(&calldata[4..]).unwrap()
}

async fn run_verifier() {
    let opt = Opt::parse();

    if cfg!(feature = "tstd_enclave") {
        let l1 = Eth::dial(&opt.l1_endpoint, Some(&opt.private_key)).unwrap();
        let kp = Keypair::new();
        let quote_builder = SGXQuoteBuilder{};
        let report = AttestationReport::build(&quote_builder, &l1, &kp).await.unwrap();
        let registry = ProverRegistry::new(l1, opt.registry_addr);
        let registration = registry.register(report).await.unwrap();
        dbg!(registration);
    }

    for tx in &opt.txs {
        let file_stem = tx.file_stem().unwrap().to_str().unwrap();
        if !file_stem.contains("-commit-") {
            continue;
        }

        log::info!("executing {}...", tx.display());

        let batch = read_batch_task(tx);
        // let finalize = read_finalize(tx);

        let dir = tx.parent().unwrap().join("downloaded").join(file_stem);

        std::fs::create_dir_all(&dir).unwrap();

        if !opt.download_from.is_empty() {
            log::info!("downloading from {}...", opt.download_from);
            let client = ScrollExecutionNode::dial(&opt.download_from).unwrap();

            let block_numbers = batch
                .chunks
                .clone()
                .into_iter()
                .map(|n| n)
                .flatten()
                .collect::<Vec<_>>();

            let total = block_numbers.len();
            let start = *block_numbers.first().unwrap();
            let alive = Alive::new();

            parallel(
                &alive,
                (start, client, dir.clone(), total),
                block_numbers,
                4,
                |block, (start, client, dir, total)| async move {
                    let idx = block - start;
                    let output = dir.join(format!("{}.blocktrace", block));
                    let is_exist = Path::new(&output).exists();
                    if is_exist {
                        return Ok::<(), ()>(());
                    }

                    println!("[{}/{}] downloading block #{}", idx, total, block);
                    let block_trace = client.trace_block(block).await.unwrap();
                    let data = serde_json::to_vec(&block_trace).unwrap();
                    std::fs::write(output, data).unwrap();
                    Ok(())
                },
            )
            .await
            .unwrap();
        }

        log::info!("reading blocktraces...");
        let chunks = batch
            .chunks
            .iter()
            .map(|chunk| {
                chunk.iter().map(|blk| {
                    let block_trace_fp = dir.join(format!("{}.blocktrace", blk));
                    let data = match std::fs::read(&block_trace_fp) {
                        Ok(data) => data,
                        Err(err) => {
                            panic!("read block trace[{:?}] failed: {:?}, try add --download-from=<scroll_endpoint> to download", block_trace_fp, err)
                        }
                    };
                    let block_trace: BlockTrace = serde_json::from_slice(&data).unwrap();
                    PobContext::new(block_trace_to_pob(block_trace).unwrap())
                })
            })
            .flatten()
            .collect::<Vec<_>>();

        let first_block = chunks.first().unwrap();
        let last_block = chunks.last().unwrap();

        let fork = HardforkConfig::default_from_chain_id(first_block.pob.data.chain_id);
        let batch_version = fork.batch_version(
            last_block.pob.block.number.to(),
            last_block.pob.block.timestamp.to(),
        );

        log::info!("build batch header...");
        let new_batch = batch.build_batch(batch_version, &chunks).unwrap();

        log::info!("executing blocks...");
        let poe = ScrollBatchVerifier::verify(&batch, chunks).await.unwrap();

        dbg!(poe);
        
        log::info!("done");
    }
}
