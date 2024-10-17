use crate::types::*;

use scroll_da_codec::{BatchTask, DABatch};
use scroll_verifier::block_trace_to_pob;
use scroll_verifier::{PobContext, ScrollBatchVerifier};

pub async fn prove_batch(req: ProveBatchRequest) -> anyhow::Result<ProveBatchSignatureData> {
    let batch = BatchTask {
        chunks: req.chunks,
        // todo: handle error
        parent_batch_header: DABatch::from_bytes(&req.prev_batch_header).unwrap(),
    };

    let chunks = req
        .blocks
        .into_iter()
        .map(|block| block_trace_to_pob(block).map(PobContext::new))
        .collect::<Option<Vec<_>>>()
        .unwrap();

    log::info!("executing blocks...");
    let poe = ScrollBatchVerifier::verify(&batch, chunks).await.unwrap();

    Ok(ProveBatchSignatureData {
        prev_state_root: poe.prev_state_root,
        post_state_root: poe.new_state_root,
        batch_hash: poe.batch_hash,
        post_withdraw_root: poe.withdrawal_root,
    })
}
