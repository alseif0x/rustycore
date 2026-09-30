//! Narrow loot fixtures for integration tests.
//!
//! This module is mounted only by the `test-fixtures` feature. It gives
//! external tests typed setup and snapshot operations for session-owned loot
//! state without making those fields part of the production API.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::map_manager::{MapManager, WorldCreature};
use crate::session::directory::{
    PlayerDirectoryIdentityLikeCpp, PlayerDirectoryPlacementLikeCpp,
    PlayerSessionRegistrationLikeCpp,
};
use crate::session::{SessionState, WorldSession};
use wow_loot::{
    LOOT_METHOD_GROUP_LIKE_CPP, LOOT_METHOD_MASTER_LIKE_CPP, OwnedLootAuthority,
};
use wow_ai::CreatureAI;
use wow_constants::{InventoryType, ItemBondingType, ItemClass};
use wow_constants::{TypeId, TypeMask};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_data::{ItemRecord, ItemSparseTemplateEntry, ItemStatsStore, ItemStore};
use wow_entities::{
    CreatureLoot, GameObjectLootSource, GameObjectOwnedLoot, GoState, LootEntry,
    LootEntryFlags, LootState,
};
use wow_packet::packets::loot::LOOT_TYPE_CORPSE_LIKE_CPP;
use wow_packet::WorldPacket;

/// Canonical enchanting skill-line id exposed for external loot fixtures.
pub const ENCHANTING_SKILL_FOR_TEST: u16 = crate::session::SKILL_ENCHANTING_LIKE_CPP;

pub use wow_entities::{AccessorObjectKind, Corpse, CorpseType, Creature, GameObject, WorldObject};


mod builders;
mod visibility;
pub use visibility::*;
mod allocation;
pub use allocation::*;
mod canonical_setup;
mod release_snapshots;
mod session_state;
mod persistence;
mod money;
pub use crate::handlers::loot::{generate_quest_loot_for_test, loot_item_quest_allowed_for_test, incomplete_loot_item_objective_for_test, advance_loot_item_objectives_for_test, remove_loot_item_objectives_for_test};
pub use crate::handlers::loot::{install_loot_interruptible_cast_for_test, install_loot_interrupt_aura_for_test, loot_cast_pending_for_test, loot_aura_slot_present_for_test};
pub use crate::handlers::loot::{attach_loot_inventory_port_for_test, applied_loot_item_quantity_for_test, store_loot_materials_for_test, request_master_loot_store_for_test, request_roll_loot_store_for_test, install_loot_wire_channel_for_test};
pub use crate::handlers::loot::{open_gameobject_loot_cycle_for_test, open_fishing_loot_cycle_for_test, open_fishing_hole_cycle_for_test, open_gathering_loot_cycle_for_test, take_local_loot_item_for_test, give_local_master_loot_for_test, release_local_loot_for_test, remove_master_loot_slot_for_test};
pub use crate::handlers::loot::{LootUseEffect, LootUseEffects, gameobject_loot_effects_for_test};
pub use crate::handlers::loot::{prepare_encounter_loot_players_for_test, share_encounter_loot_catalogs_for_test};

pub use builders::*;
pub use canonical_setup::*;
pub use release_snapshots::*;
pub use session_state::*;
pub use persistence::*;
pub use money::*;
pub use crate::handlers::loot::{mutate_loot_creature_for_test, ensure_creature_kill_loot_for_test, creature_loot_revision_for_test, install_creature_kill_loot_for_test, bind_loot_view_for_test};
pub use crate::handlers::loot::{has_cached_loot_generation_for_test, personal_loot_money_entry_for_test, update_loot_gameobject_for_test};
pub use crate::handlers::loot::{open_loot_item_window_for_test, loot_release_values_for_test, loot_item_fanout_at_commit_for_test};
pub use crate::handlers::loot::{loot_committed_visibility_for_test, remove_loot_visibility_for_test, set_loot_combat_logging_for_test, drain_loot_delivery_commands_for_test, deliver_loot_visible_values_for_test, insert_loot_transport_visibility_for_test, clear_loot_transport_visibility_for_test, loot_visibility_matches_for_test};
pub use crate::handlers::loot::{GameObjectLootWitness, observe_gameobject_loot_for_test, upsert_gameobject_pool_for_test, upsert_observed_gameobject_pool_for_test, release_gameobject_observation_for_test, mutate_loot_gameobject_for_test, release_fishing_hole_for_test, release_loot_owner_for_test, close_retired_loot_views_for_test, refresh_loot_summary_for_test, loot_cache_mut_for_test, share_loot_canonical_map_for_test};
pub use crate::handlers::loot::{loot_client_type_for_test, loot_fixture_response, loot_response_for_test, master_loot_inventory_error_for_test, notify_cached_loot_item_for_test, notify_committed_loot_item_for_test, open_loot_response_for_test, push_loot_item_for_test, send_loot_failure_for_test};

pub use crate::handlers::loot::{LootRandomProperties, generate_loot_item_properties_for_test, new_loot_item_flags_for_test, existing_loot_item_flags_for_test, start_loot_roll_for_test};

pub use crate::handlers::loot::{legacy_loot_map_key_for_test, canonical_loot_player_map_key_for_test, legacy_creature_loot_authority_for_test, canonical_creature_loot_authority_for_test, install_loot_inventory_item_for_test, loot_inventory_item_for_test, install_loot_inventory_port_for_test, store_loot_item_for_test};

pub use crate::handlers::loot::{LootGameObjectState, loot_gameobject_state_for_test, process_loot_commands_for_test, set_loot_linked_trap_for_test, use_loot_goober_for_test, record_loot_display_model_for_test, record_loot_lock_for_test, loot_gameobject_can_store_for_test, chest_reward_looters_for_test, generate_chest_loot_for_test};

pub use crate::handlers::loot::{LootRollObservation, loot_roll_observation_for_test, set_loot_roll_deadline_for_test, tick_loot_rolls_for_test, loot_opened_cache_generation_for_test, LootCriterionExpectation, LootCriterion, loot_criterion_for_test, loot_criteria_empty_for_test};

pub use crate::handlers::loot::{set_loot_gameobject_tappers_for_test, has_personal_loot_money_entry_for_test, is_personal_loot_owner_for_test};

pub use crate::handlers::loot::{LootConditionPlayer, evaluate_remote_loot_condition_for_test};

pub use crate::handlers::loot::{apply_cached_gameobject_loot_release_for_test, set_loot_lock_spells_for_test};
