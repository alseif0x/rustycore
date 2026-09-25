// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Packet handlers for the world session.

pub mod account_data;
pub mod battlegrounds;
pub mod battlenet;
pub mod calendar;
pub mod character;
pub mod chat;
pub mod collections;
pub mod combat;
pub mod dungeon_finding;
pub mod economy;
pub mod entities;
pub mod group;
pub mod guild;
pub mod inspect;
pub mod instances;
pub mod loot;
pub mod movement;
pub mod pets;
pub mod progression;
pub mod quest;
pub mod social;
pub mod spell;
pub mod support;
pub mod talent;
pub mod trainer;
pub mod travel;
pub mod vehicle;
pub mod void_storage;

#[cfg(test)]
mod test_support;
