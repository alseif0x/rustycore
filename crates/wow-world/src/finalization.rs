//! World-facing facade for the lifecycle-owned finalization ledger.

pub use wow_world_lifecycle::{
    FinalizationDisposition, FinalizationMode, FinalizationOutcome, FinalizationReport,
    FinalizationStep,
};

pub(crate) use wow_world_lifecycle::SessionFinalization;
