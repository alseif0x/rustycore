// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared immutable game-data schemas and pure projections.

pub mod aura_effects;
pub mod creature;
pub mod creature_create;
pub mod currency;
pub mod difficulty;
pub mod dungeon_encounter;
pub mod game_object;
pub mod jump_charge;
pub mod map;
pub mod pet;
pub mod player_stats;
pub mod power;
pub mod quest;
pub mod quest_poi;
pub mod reputation;
pub mod vehicle;

pub use quest::{
    QuestDayCooldownBlock, QuestEligibilityRules, QuestInfoEntry, QuestStatusBlock, QuestTemplate,
};
