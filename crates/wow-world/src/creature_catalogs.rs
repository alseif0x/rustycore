// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature template and creature-data catalogs a session reads.
//!
//! Separated from `WorldSession` under #670: the session holds exactly one
//! field of this type, so no slot is mirrored. The owner depends on `wow-data`
//! stores only and performs no async work.

use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct CreatureCatalogsLikeCpp {
    pub(crate) display_info_extra_store: Option<Arc<wow_data::CreatureDisplayInfoExtraStore>>,
    pub(crate) display_info_store: Option<Arc<wow_data::CreatureDisplayInfoStore>>,
    pub(crate) model_data_store: Option<Arc<wow_data::CreatureModelDataStore>>,
    pub(crate) model_info_store: Option<Arc<wow_data::CreatureModelInfoStoreLikeCpp>>,
    pub(crate) onkill_reputation_store:
        Option<Arc<wow_data::reputation::CreatureOnKillReputationStoreLikeCpp>>,
    pub(crate) template_lifecycle_store_like_cpp:
        Option<Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>>,
    pub(crate) template_mount_store: Option<Arc<wow_data::CreatureTemplateMountStoreLikeCpp>>,
}
