// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Game data file readers (DB2/WDC4).

pub mod wdc4;
pub mod item;
pub mod item_stats;
pub mod hotfix_cache;
pub mod player_stats;
pub mod skill;
pub mod area_trigger;
pub mod spell;
pub mod quest;

pub use item::{ItemRecord, ItemStore};
pub use item_stats::{ItemStatEntry, ItemStatsStore};
pub use hotfix_cache::{HotfixBlobCache, build_hotfix_blob_cache};
pub use player_stats::{PlayerLevelStats, PlayerStatsStore};
pub use skill::{SkillInfoEntry, SkillStore};
pub use area_trigger::{
    AreaTriggerTeleport, AreaTriggerData, AreaTriggerStore, TriggerShape, load_area_triggers,
};
pub use spell::{SpellInfo, SpellStore};
