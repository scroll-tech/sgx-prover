use std::time::Instant;

use scroll_executor::{Context, ExecutionError, ExecutionResult, ScrollEvmExecutor};
use scroll_verifier::{block_trace_to_pob, PobContext};

use crate::types::*;

pub async fn execute_block(req: &ProveBlockRequest) -> Result<ExecutionResult, ExecutionError> {
    // todo: avoid clone
    let pob = block_trace_to_pob(req.block_trace.clone()).unwrap();

    let ctx = PobContext::new(pob);
    let memdb = ctx.memdb();
    let db = ctx.db(memdb.clone());
    let spec_id = ctx.spec_id();
    let now = Instant::now();

    let result = ScrollEvmExecutor::new(&db, memdb, spec_id).handle_block(&ctx)?;

    log::info!(
        "[scroll] generate poe: {} -> {:?}",
        ctx.number(),
        now.elapsed()
    );

    Ok(result)
}
