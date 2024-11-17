use crate::types::{
    BundleSize, VerifyBatchEvent,
};

use super::StateManagerError;

pub struct BundleState {
    pub last_finalized_batch_index: u64,
    pub bundle_sizes: Vec<BundleSize>,
}

pub struct BundleInfo {
    pub begin_batch_index: u64,
    pub end_batch_index: u64,
}

impl BundleState {
    pub fn new(last_finalized_batch_index: u64, bundle_sizes: Vec<BundleSize>) -> Self {
        Self {
            last_finalized_batch_index,
            bundle_sizes,
        }
    }

    pub fn update_last_finalized_batch(&mut self, event: VerifyBatchEvent) {
        self.last_finalized_batch_index = event.batch_index;
    }

    pub fn update_bundle_size(&mut self, bundle_size: BundleSize) {
        self.bundle_sizes.push(bundle_size);
    }

    fn get_bundle_size(&self, batch_index: u64) -> Option<u64> {
        let mut i = self.bundle_sizes.len();
        while i > 0 {
            i -= 1;
            let size = self.bundle_sizes[i];
            if batch_index > size.start_batch_index {
                return Some(size.bundle_size);
            }
        }
        None
    }

    pub fn get_next_bundle(&self, batch_index: u64) -> Result<BundleInfo, StateManagerError> {
        let last_finalized_batch_index = self.last_finalized_batch_index;

        let next_bundle_size = self
            .get_bundle_size(last_finalized_batch_index + 1)
            .expect("failed to get bundle_size");

        let end_batch_index_for_next_bundle = last_finalized_batch_index + next_bundle_size;
        if end_batch_index_for_next_bundle > batch_index {
            Err(StateManagerError::BatchesNotEnough(
                batch_index,
                end_batch_index_for_next_bundle,
            ))
        } else {
            Ok(BundleInfo {
                begin_batch_index: last_finalized_batch_index + 1,
                end_batch_index: end_batch_index_for_next_bundle,
            })
        }
    }
}
