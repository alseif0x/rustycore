//! First original melee death -> existing APP loot algorithm -> original TARGET.
//! This integration target is feature-gated; no production driver is activated.
#![cfg(feature = "test-fixtures")]
#[path = "original_melee_loot/mod.rs"]
mod original_melee_loot;
