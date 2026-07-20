//! # capture-diff — the port's acceptance gate (issue `[01]` / #66)
//!
//! "Done" across the whole port plan means the Rust wire output is byte/opcode
//! clean versus a C++ capture of the same action (STATE.md §5), except for
//! narrowly reviewed runtime identifiers handled by [`semantic`]. This crate is
//! that harness: it parses a C++ **PKT 3.1** capture ([`pkt`]) and a RustyCore
//! packet dump ([`rustdump`]), normalizes both to a common model ([`model`]),
//! and diffs them opcode-by-opcode ([`diff`]). Flows ([`flow`]) pin a golden
//! capture per scenario so every milestone PR gets an objective regression gate.
//!
//! ## Capturing
//!
//! - **C++ (golden):** set `PacketLogFile = "World.pkt"` (+ `LogsDir`) in the
//!   legacy worldserver config, run the flow, collect the `.pkt`.
//! - **Rust:** run the world server with `RUSTYCORE_PACKET_DUMP_DIR=<dir>`, run
//!   the same flow, collect the dump directory.
//!
//! See `crates/capture-diff/scripts/` and `README.md` for the one-command flow.

// Product names (RustyCore, TrinityCore) appear throughout the docs as prose.
#![allow(clippy::doc_markdown)]

pub mod diff;
pub mod flow;
pub mod lineage;
pub mod model;
pub mod pkt;
pub mod rustdump;
pub mod semantic;

pub use diff::{
    AlignedOp, BaselineDelta, BodyDiff, ConnectionDiff, DiffCounts, DiffReport, DivergenceKind,
    DivergenceSignature, OpKind, baseline_delta,
};
pub use flow::{
    Flow, FlowRequirement, RequiredImportBoundary, RequiredImportSelection, RequiredPacket,
    RequirementSemanticContract, RequirementStatus, list_flows, list_requirements, load_flow,
    load_requirement,
};
pub use model::{Capture, CapturedPacket, Direction, PacketBoundary, opcode_name};
pub use semantic::{
    ExactObjectGuid, InvSlotValue, LogXpGainBody, LootRemovedBody, SemanticBodyDiff,
    SemanticBodySide, StableObjectGuid, UpdateObjectInvSlotsBody, decode_log_xp_gain_body,
    decode_loot_removed_body, validate_loot_single_item_claim_capture,
};
