// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature spawn, despawn and respawn scheduling.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::PreparedStatement;

use super::*;

impl WorldCreature {
    pub(super) fn restore_respawn_aura_source_authority_like_cpp(
        &mut self,
        spell_hit: bool,
        spell_cast_log: bool,
    ) {
        self.respawn_spell_hit_aura_source_authority_like_cpp = spell_hit;
        self.respawn_spell_cast_log_aura_source_authority_like_cpp = spell_cast_log;
        let auras = &mut self.creature.unit_mut().subsystems_mut().auras;
        auras.set_spell_hit_aura_authority_inert_like_cpp(spell_hit);
        auras.set_spell_cast_log_aura_authority_inert_like_cpp(spell_cast_log);
    }

    pub fn corpse_despawn_at(&self) -> Option<Instant> {
        let now = Instant::now();
        let elapsed_ms = self.runtime_elapsed_ms_like_cpp();
        self.creature
            .ai_ownership()
            .corpse_despawn_at_ms
            .map(|due_at_ms| now + Duration::from_millis(due_at_ms.saturating_sub(elapsed_ms)))
    }

    pub const fn corpse_despawn_deadline_ms_like_cpp(&self) -> Option<u64> {
        self.creature.ai_ownership().corpse_despawn_at_ms
    }

    pub fn corpse_despawn_due_like_cpp(&self) -> bool {
        self.corpse_despawn_deadline_ms_like_cpp()
            .is_some_and(|due_at_ms| self.runtime_elapsed_ms_like_cpp() >= due_at_ms)
    }

    pub fn respawn_at_from_death_like_cpp(&self) -> Instant {
        self.respawn_at_from_death_at_game_time_like_cpp(Instant::now(), game_time_secs_like_cpp())
    }

    pub fn respawn_at_from_death_at_game_time_like_cpp(
        &self,
        now: Instant,
        game_time_secs: i64,
    ) -> Instant {
        let elapsed_ms = self.runtime_elapsed_ms_like_cpp();
        let death_at = self
            .creature
            .ai_ownership()
            .death_time_ms
            .map(|death_ms| {
                if death_ms <= elapsed_ms {
                    now.checked_sub(Duration::from_millis(elapsed_ms - death_ms))
                        .unwrap_or(now)
                } else {
                    now + Duration::from_millis(death_ms - elapsed_ms)
                }
            })
            .unwrap_or(now);
        let compatibility_corpse_delay = self
            .creature
            .respawn_compatibility_mode()
            .then_some(u64::from(self.creature.corpse_delay()))
            .unwrap_or(0);
        let death_based = death_at
            + Duration::from_secs(
                self.creature
                    .ai_ownership()
                    .respawn_time_secs
                    .saturating_add(compatibility_corpse_delay),
            );
        let stored_based =
            instant_from_respawn_time_like_cpp(self.creature.respawn_time(), now, game_time_secs);
        death_based.max(stored_based)
    }

    pub fn set_corpse_despawn_at(&mut self, when: Option<Instant>) {
        let now = Instant::now();
        let now_ms = self.runtime_elapsed_ms_like_cpp();
        let at_ms = when.map(|instant| {
            if instant <= now {
                now_ms
            } else {
                now_ms.saturating_add(
                    instant
                        .duration_since(now)
                        .as_millis()
                        .min(u128::from(u64::MAX)) as u64,
                )
            }
        });
        self.creature.set_ai_corpse_despawn_at(at_ms);
    }

    pub fn should_respawn(&self) -> bool {
        self.creature
            .should_ai_respawn(self.runtime_elapsed_ms_like_cpp())
    }

    pub fn respawn(&mut self) {
        self.creature.respawn_ai(self.runtime_elapsed_ms_like_cpp());
    }
}

impl MapInstance {
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

    pub fn save_pending_respawn_time_like_cpp(
        &mut self,
        respawn: &PendingRespawn,
        now: Instant,
        now_secs: i64,
    ) -> Option<PreparedStatement> {
        let row = PersistedRespawnRowLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: respawn.spawn_id,
            respawn_time: respawn_time_from_instant_like_cpp(respawn.respawn_at, now, now_secs),
            map_id: self.map_id,
            instance_id: self.instance_id,
        };
        match self.add_persisted_respawn_time_like_cpp(row) {
            LegacyRespawnTimeAddOutcomeLikeCpp::Inserted
            | LegacyRespawnTimeAddOutcomeLikeCpp::ReplacedExisting => {
                Some(respawn_replace_statement_like_cpp(&row))
            }
            LegacyRespawnTimeAddOutcomeLikeCpp::RejectedZeroSpawnId
            | LegacyRespawnTimeAddOutcomeLikeCpp::RejectedUnsupportedType
            | LegacyRespawnTimeAddOutcomeLikeCpp::RejectedExistingSoonerOrEqual => None,
        }
    }

    pub fn load_persisted_respawns_into_queue_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = PersistedRespawnRowLikeCpp>,
        now: Instant,
        now_secs: i64,
        mut resolve_creature: impl FnMut(&PersistedRespawnRowLikeCpp, Instant) -> Option<PendingRespawn>,
    ) -> LegacyRespawnQueueReloadReportLikeCpp {
        let mut report = LegacyRespawnQueueReloadReportLikeCpp::default();
        for row in rows {
            report.rows += 1;
            match self.add_persisted_respawn_time_like_cpp(row) {
                LegacyRespawnTimeAddOutcomeLikeCpp::Inserted
                | LegacyRespawnTimeAddOutcomeLikeCpp::ReplacedExisting => {
                    report.timers_loaded += 1;
                }
                LegacyRespawnTimeAddOutcomeLikeCpp::RejectedZeroSpawnId => {
                    report.rejected_zero_spawn_id += 1;
                    continue;
                }
                LegacyRespawnTimeAddOutcomeLikeCpp::RejectedUnsupportedType => {
                    report.rejected_unsupported_type += 1;
                    continue;
                }
                LegacyRespawnTimeAddOutcomeLikeCpp::RejectedExistingSoonerOrEqual => {
                    report.rejected_existing_later += 1;
                    continue;
                }
            }

            let respawn_at = instant_from_respawn_time_like_cpp(row.respawn_time, now, now_secs);
            match row.object_type {
                SpawnObjectType::Creature => {
                    if let Some(mut pending) = resolve_creature(&row, respawn_at) {
                        pending.respawn_at = respawn_at;
                        pending.spawn_id = row.spawn_id;
                        pending.map_id = row.map_id;
                        self.push_respawn(pending);
                        report.creature_queued += 1;
                    } else {
                        report.missing_creature_runtime += 1;
                    }
                }
                SpawnObjectType::GameObject => {
                    report.gameobject_loaded += 1;
                }
                SpawnObjectType::AreaTrigger => {
                    report.rejected_unsupported_type += 1;
                }
            }
        }
        report
    }
}

impl MapManager {
    pub fn find_creature_guid_by_spawn_id_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        spawn_id: u64,
    ) -> Option<ObjectGuid> {
        (spawn_id != 0).then_some(())?;
        self.creature_guids(map_id, instance_id)
            .into_iter()
            .find(|guid| {
                self.find_creature(map_id, instance_id, *guid)
                    .is_some_and(|creature| {
                        creature.is_alive() && creature.creature.spawn_id() == spawn_id
                    })
            })
    }

    /// Enqueue a pending respawn on the given map instance.
    /// Creates the instance if it does not yet exist.
    pub fn push_respawn(&mut self, map_id: u16, instance_id: u32, respawn: PendingRespawn) {
        self.get_or_create_map(map_id, instance_id)
            .push_respawn(respawn);
    }

    /// Drain ready respawns (`respawn_at <= now`) from the given map instance.
    /// Returns an empty `Vec` if the instance does not exist.
    pub fn drain_ready_respawns(
        &mut self,
        map_id: u16,
        instance_id: u32,
        now: Instant,
    ) -> Vec<PendingRespawn> {
        if let Some(map) = self.get_map_mut(map_id, instance_id) {
            map.drain_ready_respawns(now)
        } else {
            Vec::new()
        }
    }

    /// Number of entries currently in the respawn queue of the given map
    /// instance.  Returns 0 if the instance does not exist.
    pub fn respawn_queue_len(&self, map_id: u16, instance_id: u32) -> usize {
        self.get_map(map_id, instance_id)
            .map(|m| m.respawn_queue_len())
            .unwrap_or(0)
    }

    pub fn save_pending_respawn_time_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        respawn: &PendingRespawn,
        now: Instant,
        now_secs: i64,
    ) -> Option<PreparedStatement> {
        self.get_or_create_map(map_id, instance_id)
            .save_pending_respawn_time_like_cpp(respawn, now, now_secs)
    }

    pub fn persisted_respawn_time_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        object_type: SpawnObjectType,
        spawn_id: u64,
    ) -> Option<i64> {
        self.get_map(map_id, instance_id)?
            .persisted_respawn_time_like_cpp(object_type, spawn_id)
    }

    pub fn persisted_respawn_rows_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
    ) -> Vec<PersistedRespawnRowLikeCpp> {
        self.get_map(map_id, instance_id)
            .map(|map| map.persisted_respawn_rows_like_cpp())
            .unwrap_or_default()
    }

    pub fn load_persisted_respawns_into_queue_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = PersistedRespawnRowLikeCpp>,
        now: Instant,
        now_secs: i64,
        mut resolve_creature: impl FnMut(&PersistedRespawnRowLikeCpp, Instant) -> Option<PendingRespawn>,
    ) -> LegacyRespawnQueueReloadReportLikeCpp {
        let mut report = LegacyRespawnQueueReloadReportLikeCpp::default();
        for row in rows {
            let row_report = self
                .get_or_create_map(row.map_id, row.instance_id)
                .load_persisted_respawns_into_queue_like_cpp([row], now, now_secs, |row, at| {
                    resolve_creature(row, at)
                });
            report.rows += row_report.rows;
            report.timers_loaded += row_report.timers_loaded;
            report.creature_queued += row_report.creature_queued;
            report.gameobject_loaded += row_report.gameobject_loaded;
            report.rejected_zero_spawn_id += row_report.rejected_zero_spawn_id;
            report.rejected_unsupported_type += row_report.rejected_unsupported_type;
            report.rejected_existing_later += row_report.rejected_existing_later;
            report.missing_creature_runtime += row_report.missing_creature_runtime;
        }
        report
    }
}
