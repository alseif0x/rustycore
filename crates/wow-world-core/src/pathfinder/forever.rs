// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: selected Forever navigation package consumer.
//! Optional verified package admission and explicit raw corridor operations.

mod owner;
mod types;
pub(in crate::pathfinder) mod worker;

pub(super) use owner::ForeverNavigationOwner;
#[cfg(feature = "test-fixtures")]
pub use types::ForeverNavigationTestFailurePlan;
pub use types::{
    FOREVER_NAVIGATION_QUERY_NODE_BUDGET, ForeverNavigationContext, ForeverNavigationCorridor,
    ForeverNavigationError, ForeverNavigationExpected, ForeverNavigationLimits,
    ForeverNavigationLocale, ForeverNavigationQueryRequest, ForeverNavigationStats,
    ForeverNavigationSummary, ForeverNavigationTile, MAX_FOREVER_NAVIGATION_BATCH_BYTES,
    MAX_FOREVER_NAVIGATION_FILE_BYTES, MAX_FOREVER_NAVIGATION_INSTANCE_QUERIES,
    MAX_FOREVER_NAVIGATION_LOADED_BYTES, MAX_FOREVER_NAVIGATION_PATH_POLYS,
    MAX_FOREVER_NAVIGATION_TILES, MAX_FOREVER_NAVIGATION_TILES_PER_BATCH,
};
pub(super) use types::{ForeverNavigationWorkerAction, ForeverNavigationWorkerResponse};

#[cfg(test)]
mod tests;
