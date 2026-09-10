// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Packet handlers for the world session.

pub mod battlenet;
pub mod character;
pub(crate) mod character_rules;
pub mod chat;
pub mod combat;
pub mod group;
pub mod inspect;
pub mod loot;
pub(crate) mod loot_rules;
pub mod misc;
pub mod movement;
pub mod quest;
pub(crate) mod quest_rules;
pub mod social;
pub mod spell;
pub mod talent;
pub mod trainer;
pub mod vehicle;
pub mod void_storage;
