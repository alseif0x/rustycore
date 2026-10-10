// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The creature respawn queue a map owns, with the data that rebuilds each
//! creature.
//!
//! C++ keeps `Map::_respawnTimes` and `_creatureRespawnTimesBySpawnId` on the
//! map (`Map.h:748-777`) and rebuilds a creature in `Map::DoRespawn` through
//! `Creature::LoadFromDB`. Rust has no represented loader on that rail yet, so
//! each queued entry carries its rebuild data ([`PendingRespawn`]).
//!
//! #1263 F6-8D3a-3: the entry types and the queue bodies moved here from the
//! legacy `wow-world-core` map manager unchanged, so the legacy `MapInstance`
//! and the canonical [`crate::Map`] own the same queue type and the creature
//! lifecycle phase runs one body against either.

use std::collections::HashMap;
use std::time::Instant;

use wow_entities::creature_create::CreatureCreateData;
use wow_entities::{
    CreatureAddonLifecycleRecordLikeCpp, CreatureCombatLogStatsLikeCpp, MovementGeneratorType,
    PhaseShift,
};

use crate::SpawnObjectType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistedRespawnRowLikeCpp {
    pub object_type: SpawnObjectType,
    pub spawn_id: u64,
    pub respawn_time: i64,
    pub map_id: u16,
    pub instance_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyRespawnTimeAddOutcomeLikeCpp {
    Inserted,
    ReplacedExisting,
    RejectedZeroSpawnId,
    RejectedUnsupportedType,
    RejectedExistingSoonerOrEqual,
}

/// A creature waiting to respawn after its corpse despawned.
///
/// Owned by a map's [`CreatureRespawnQueueLikeCpp`]; processed by the
/// creature lifecycle phase.
/// C++ refs: `Creature::RemoveCorpse` / `AllLootRemovedFromCorpse` schedule a
/// map-owned `RespawnInfo`, and `Map::ProcessRespawns` later calls
/// `DoRespawn(SPAWN_TYPE_CREATURE, spawnId, gridId)`.
#[derive(Debug)]
pub struct PendingRespawn {
    /// When to respawn.
    pub respawn_at: Instant,
    /// C++ `RespawnInfo::spawnId` / `Creature::m_spawnId`, separate from the live ObjectGuid low counter.
    pub spawn_id: u64,
    /// Whether `spawn_id` is a real DB spawn identity rather than the
    /// queue-only GUID-low fallback used for dynamic creatures.
    pub persistent_spawn: bool,
    /// Home position (spawn point).
    pub home_pos: wow_core::Position,
    /// Full create data retained until the represented loader converges on
    /// C++ `Creature::LoadFromDB(spawnId, map, true, true)`.
    pub create_data: CreatureCreateData,
    /// AI fields needed to rebuild the canonical creature runtime.
    pub max_hp: u32,
    pub level: u8,
    pub min_dmg: u32,
    pub max_dmg: u32,
    /// Live totals used by C++ `SpellCastLogData::Initialize`.
    pub combat_log_stats: CreatureCombatLogStatsLikeCpp,
    /// DB-backed source proofs captured independently of the live aura markers.
    /// The live markers are revoked during death cleanup; the respawn rail may
    /// restore only proofs that crossed the authoritative loaded-grid bridge.
    pub spell_hit_aura_source_authority_like_cpp: bool,
    pub spell_cast_log_aura_source_authority_like_cpp: bool,
    pub aggro_radius: f32,
    pub wander_distance: f32,
    pub flags_extra: u32,
    pub static_flags: [u32; 8],
    pub ai_name: String,
    pub script_name: String,
    pub string_id: Option<String>,
    pub addon: Option<CreatureAddonLifecycleRecordLikeCpp>,
    pub ground_movement_type: u8,
    pub swim_allowed: bool,
    pub flight_movement_type: u8,
    pub rooted: bool,
    pub chase_movement_type: u8,
    pub random_movement_type: u8,
    pub interaction_pause_timer_ms: u32,
    pub default_movement_type: MovementGeneratorType,
    pub waypoint_path_id: u32,
    pub npc_flags: u32,
    pub unit_flags: u32,
    pub map_id: u16,
    pub loot_id: u32,
    pub skin_loot_id: u32,
    pub gold_min: u32,
    pub gold_max: u32,
    pub respawn_delay_secs: u32,
    pub selected_equipment_id: u8,
    pub original_equipment_id: i8,
    pub boss_id: Option<u32>,
    pub dungeon_encounter_id: u32,
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u32,
    pub terrain_swap_map: i32,
    /// Already-resolved DB phase shift from the creature that despawned.
    ///
    /// The global runtime has no `WorldSession` phase stores, so respawn must
    /// reuse the resolved phase state captured at despawn time instead of
    /// recalculating it through session-local helpers.
    pub phase_shift: PhaseShift,
}

/// The persisted respawn rows and the pending respawns of one map.
#[derive(Debug, Default)]
pub struct CreatureRespawnQueueLikeCpp {
    /// C++ `Map::_creatureRespawnTimesBySpawnId` and
    /// `_gameObjectRespawnTimesBySpawnId`, represented as DB-persistable rows.
    pub persisted_respawn_times: HashMap<(SpawnObjectType, u64), PersistedRespawnRowLikeCpp>,
    /// Creatures waiting to respawn. C++ ref: `Map::_respawnTimes` (Map.h:748).
    pub respawn_queue: Vec<PendingRespawn>,
}

impl CreatureRespawnQueueLikeCpp {
    pub fn add_persisted_respawn_time_like_cpp(
        &mut self,
        row: PersistedRespawnRowLikeCpp,
    ) -> LegacyRespawnTimeAddOutcomeLikeCpp {
        if row.spawn_id == 0 {
            return LegacyRespawnTimeAddOutcomeLikeCpp::RejectedZeroSpawnId;
        }
        if !matches!(
            row.object_type,
            SpawnObjectType::Creature | SpawnObjectType::GameObject
        ) {
            return LegacyRespawnTimeAddOutcomeLikeCpp::RejectedUnsupportedType;
        }

        let key = (row.object_type, row.spawn_id);
        if let Some(existing) = self.persisted_respawn_times.get(&key) {
            if row.respawn_time <= existing.respawn_time {
                self.persisted_respawn_times.insert(key, row);
                LegacyRespawnTimeAddOutcomeLikeCpp::ReplacedExisting
            } else {
                LegacyRespawnTimeAddOutcomeLikeCpp::RejectedExistingSoonerOrEqual
            }
        } else {
            self.persisted_respawn_times.insert(key, row);
            LegacyRespawnTimeAddOutcomeLikeCpp::Inserted
        }
    }

    pub fn persisted_respawn_time_like_cpp(
        &self,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<i64> {
        self.persisted_respawn_times
            .get(&(object_type, spawn_id))
            .map(|row| row.respawn_time)
    }

    pub fn persisted_respawn_rows_like_cpp(&self) -> Vec<PersistedRespawnRowLikeCpp> {
        self.persisted_respawn_times.values().copied().collect()
    }

    /// Enqueue a creature waiting to respawn.
    /// C++ ref: `Map::_respawnTimes` insertion path (Map.cpp:2191).
    pub fn push_respawn(&mut self, respawn: PendingRespawn) {
        if let Some(existing_index) = self.respawn_queue.iter().position(|queued| {
            queued.persistent_spawn == respawn.persistent_spawn
                && queued.spawn_id == respawn.spawn_id
        }) {
            if respawn.respawn_at <= self.respawn_queue[existing_index].respawn_at {
                self.respawn_queue.remove(existing_index);
            } else {
                return;
            }
        }
        self.respawn_queue.push(respawn);
    }

    /// Drain entries whose `respawn_at <= now` in insertion order.
    ///
    /// Entries that are NOT yet ready are retained in the queue.
    /// C++ ref: `Map::ProcessRespawns` (Map.cpp:2191).
    pub fn drain_ready_respawns(&mut self, now: Instant) -> Vec<PendingRespawn> {
        let mut remaining = Vec::new();
        let mut spawn_now = Vec::new();
        for r in self.respawn_queue.drain(..) {
            if now >= r.respawn_at {
                spawn_now.push(r);
            } else {
                remaining.push(r);
            }
        }
        self.respawn_queue = remaining;
        spawn_now
    }

    /// Number of entries currently waiting to respawn.
    pub fn respawn_queue_len(&self) -> usize {
        self.respawn_queue.len()
    }

    pub fn remove_persisted_respawn_time_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<PersistedRespawnRowLikeCpp> {
        self.persisted_respawn_times
            .remove(&(object_type, spawn_id))
    }
}
