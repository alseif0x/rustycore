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
mod character_login_host;
#[cfg(test)]
pub mod chat;
pub mod collections;
pub mod combat;
#[cfg(test)]
pub mod dungeon_finding;
pub mod economy;
pub mod entities;
pub mod group;
pub mod guild;
pub mod instances;
mod leaf_session_hosts;
pub mod loot;
mod loot_ops_session_host;
pub mod movement;
mod movement_host;
mod npc_host;
mod open_item_host;
pub mod pets;
pub mod progression;
pub mod quest;
mod quest_item_storage_host;
pub mod social;
pub mod spell;
pub mod support;
pub mod talent;
pub mod trainer;
pub mod travel;
pub mod vehicle;
pub mod void_storage;

#[cfg(test)]
mod aura_test_shims;
#[cfg(test)]
mod battle_pet_test_shims;
#[cfg(test)]
mod canonical_objects_test_shims;
#[cfg(test)]
mod character_query_test_shims;
#[cfg(test)]
mod collections_session_test_shims;
#[cfg(test)]
mod continue_login_test_shims;
#[cfg(test)]
mod group_session_test_shims;
#[cfg(test)]
mod instance_session_test_shims;
#[cfg(test)]
mod item_mods_session_test_shims;
#[cfg(test)]
mod item_test_shims;
#[cfg(test)]
mod login_data_gossip_test_shims;
#[cfg(test)]
mod login_load_test_shims;
#[cfg(test)]
mod login_test_shims;
#[cfg(test)]
mod loot_handler_test_shims;
#[cfg(test)]
mod loot_test_shims;
#[cfg(test)]
mod open_item_test_shims;
#[cfg(test)]
mod player_cast_test_shims;
#[cfg(test)]
mod player_item_builders_test_shims;
#[cfg(test)]
mod player_items_test_shims;
#[cfg(test)]
mod player_test_shims;
#[cfg(test)]
mod progression_test_shims;
#[cfg(test)]
mod quest_handler_test_shims;
#[cfg(test)]
mod quest_item_test_shims;
#[cfg(test)]
mod quest_session_test_shims;
#[cfg(test)]
mod retired_forward_test_shims;
#[cfg(test)]
mod session_leftovers_test_shims;
#[cfg(test)]
mod session_lifecycle_test_shims;
#[cfg(test)]
mod social_quest_test_shims;
#[cfg(test)]
mod spell_effect_session_test_shims;
#[cfg(test)]
mod spell_effect_test_shims;
#[cfg(test)]
mod spell_execution_test_shims;
#[cfg(test)]
mod spell_state_session_test_shims;
#[cfg(test)]
mod spellbook_test_shims;
#[cfg(test)]
mod split_forward_test_shims;
#[cfg(test)]
mod talent_test_shims;
#[cfg(test)]
mod teleport_test_shims;
#[cfg(test)]
mod test_shims;
#[cfg(test)]
#[path = "../../unit_tests/handlers/test_support/mod.rs"]
mod test_support;
#[cfg(test)]
mod trainer_talent_test_shims;
#[cfg(test)]
mod travel_test_shims;
#[cfg(test)]
mod vendor_test_shims;
#[cfg(test)]
mod visibility_test_shims;
#[cfg(test)]
mod void_storage_test_shims;
#[cfg(test)]
mod world_entities_registry_test_shims;
#[cfg(test)]
mod world_entities_test_shims;
#[cfg(test)]
mod worldport_test_shims;
