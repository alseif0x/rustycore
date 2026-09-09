// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map and map-difficulty catalogs a session reads.
//!
//! Separated from `WorldSession` under #670: the session holds exactly one
//! field of this type, so no slot is mirrored. The owner depends on `wow-data`
//! stores only and performs no async work.

use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct MapCatalogsLikeCpp {
    pub(crate) difficulty_store: Option<Arc<wow_data::MapDifficultyStore>>,
    pub(crate) difficulty_x_condition_store: Option<Arc<wow_data::MapDifficultyXConditionStore>>,
    pub(crate) store: Option<Arc<wow_data::MapStore>>,
}
