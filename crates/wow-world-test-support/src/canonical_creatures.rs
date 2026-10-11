//! Canonical creature store fixtures shared by the session and handler tests.
//!
//! #1263 F6-8E deleted the legacy creature store: the canonical map is the only
//! creature store. [`CanonicalCreatureFixtureLikeCpp`] is read/setup sugar over
//! the canonical map for scenario tests: it addresses a creature by
//! `(map_id, instance_id, guid)` and lends the canonical `Creature` itself —
//! there is no copy.
//!
//! Moved from `wow-world` `canonical_creature_fixtures` by #1363 slice 0; the
//! session-bound store installers stay there.

use std::sync::{Arc, Mutex};

use wow_core::ObjectGuid;

use wow_world_core::session::SharedCanonicalMapManager;

/// A fresh canonical map manager for one scenario.
pub fn shared_canonical_map_manager() -> SharedCanonicalMapManager {
    Arc::new(Mutex::new(wow_map::MapManager::default()))
}

/// One canonical creature lent by [`CanonicalCreatureFixtureLikeCpp::find_creature`].
pub struct CanonicalCreatureFixtureRefLikeCpp<'a> {
    pub creature: &'a wow_entities::Creature,
}

impl std::ops::Deref for CanonicalCreatureFixtureRefLikeCpp<'_> {
    type Target = wow_entities::Creature;

    fn deref(&self) -> &Self::Target {
        self.creature
    }
}

/// One canonical creature lent mutably by
/// [`CanonicalCreatureFixtureLikeCpp::find_creature_mut`].
pub struct CanonicalCreatureFixtureMutLikeCpp<'a> {
    pub creature: &'a mut wow_entities::Creature,
}

impl std::ops::Deref for CanonicalCreatureFixtureMutLikeCpp<'_> {
    type Target = wow_entities::Creature;

    fn deref(&self) -> &Self::Target {
        self.creature
    }
}

impl std::ops::DerefMut for CanonicalCreatureFixtureMutLikeCpp<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.creature
    }
}

impl CanonicalCreatureFixtureRefLikeCpp<'_> {
    /// Milliseconds until the creature's template spell `slot` is due.
    pub fn creature_spell_due_in_ms_for_test(&self, slot: usize) -> Option<u64> {
        creature_spell_due_in_ms_like_cpp(self.creature, slot)
    }
}

impl CanonicalCreatureFixtureMutLikeCpp<'_> {
    /// Milliseconds until the creature's template spell `slot` is due.
    pub fn creature_spell_due_in_ms_for_test(&self, slot: usize) -> Option<u64> {
        creature_spell_due_in_ms_like_cpp(self.creature, slot)
    }

    /// Set the creature's `Unit::Update` clock to `elapsed`.
    pub fn backdate_runtime_clock_for_test(&mut self, elapsed: std::time::Duration) {
        self.creature
            .runtime_like_cpp_mut()
            .set_runtime_elapsed_ms_like_cpp(elapsed.as_millis().min(u128::from(u64::MAX)) as u64);
    }

    /// Seed the creature's runtime RNG.
    pub fn seed_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.creature
            .runtime_like_cpp_mut()
            .seed_runtime_rng_like_cpp(seed);
    }
}

fn creature_spell_due_in_ms_like_cpp(
    creature: &wow_entities::Creature,
    slot: usize,
) -> Option<u64> {
    creature
        .runtime_like_cpp()
        .creature_spell_due_at_ms_like_cpp
        .get(slot)
        .copied()
        .flatten()
        .map(|due_at| due_at.saturating_sub(creature.runtime_elapsed_ms_like_cpp()))
}

/// Fixture access to the canonical creature store by `(map_id, instance_id)`.
pub trait CanonicalCreatureFixtureLikeCpp {
    fn find_creature(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<CanonicalCreatureFixtureRefLikeCpp<'_>>;
    fn find_creature_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<CanonicalCreatureFixtureMutLikeCpp<'_>>;
    /// Admit `creature` as a canonical incarnation on the map (created when
    /// absent), bound to that map and in world. `false` when the canonical
    /// admission refused or deferred it.
    fn add_creature(
        &mut self,
        map_id: u16,
        instance_id: u32,
        creature: wow_entities::Creature,
    ) -> bool;
    /// Remove the incarnation from its map (`RemoveFromMap`) and return it.
    fn remove_creature_any(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<wow_entities::Creature>;
    /// The map's creature respawn queue length (0 without the map).
    fn respawn_queue_len(&self, map_id: u16, instance_id: u32) -> usize;
    /// Queue one creature respawn on the map (created when absent).
    fn push_respawn(
        &mut self,
        map_id: u16,
        instance_id: u32,
        respawn: wow_world_core::creature_respawn::PendingRespawn,
    );
    /// Save one pending respawn's time in the map's queue (`SaveRespawnTime`).
    fn save_pending_respawn_time_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        respawn: &wow_world_core::creature_respawn::PendingRespawn,
        now: std::time::Instant,
        now_secs: i64,
    ) -> Option<wow_persistence::RespawnPersistenceMutationLikeCpp>;
    /// The persisted respawn time of one spawn on the map, if any.
    fn persisted_respawn_time_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        object_type: wow_map::SpawnObjectType,
        spawn_id: u64,
    ) -> Option<i64>;
}

impl CanonicalCreatureFixtureLikeCpp for wow_map::MapManager {
    fn find_creature(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<CanonicalCreatureFixtureRefLikeCpp<'_>> {
        self.find_map(u32::from(map_id), instance_id)?
            .map()
            .get_typed_creature(guid)
            .map(|creature| CanonicalCreatureFixtureRefLikeCpp { creature })
    }

    fn find_creature_mut(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<CanonicalCreatureFixtureMutLikeCpp<'_>> {
        self.find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_creature_mut(guid)
            .map(|creature| CanonicalCreatureFixtureMutLikeCpp { creature })
    }

    fn add_creature(
        &mut self,
        map_id: u16,
        instance_id: u32,
        mut creature: wow_entities::Creature,
    ) -> bool {
        let _ = creature
            .unit_mut()
            .world_mut()
            .set_map(u32::from(map_id), instance_id);
        self.create_world_map(u32::from(map_id), instance_id);
        wow_world_entities::insert_canonical_creature_map_object_on_locked_map_like_cpp(
            self,
            u32::from(map_id),
            instance_id,
            creature,
        )
        .is_some()
    }

    fn remove_creature_any(
        &mut self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> Option<wow_entities::Creature> {
        let map = self.find_map_mut(u32::from(map_id), instance_id)?.map_mut();
        let creature = map.get_typed_creature(guid)?.clone();
        let _ = map.remove_from_map_like_cpp(guid, true);
        Some(creature)
    }

    fn respawn_queue_len(&self, map_id: u16, instance_id: u32) -> usize {
        self.find_map(u32::from(map_id), instance_id)
            .map_or(0, |map| {
                map.map()
                    .creature_respawn_queue_like_cpp()
                    .respawn_queue_len()
            })
    }

    fn push_respawn(
        &mut self,
        map_id: u16,
        instance_id: u32,
        respawn: wow_world_core::creature_respawn::PendingRespawn,
    ) {
        self.create_world_map(u32::from(map_id), instance_id)
            .map_mut()
            .creature_respawn_queue_like_cpp_mut()
            .push_respawn(respawn);
    }

    fn save_pending_respawn_time_like_cpp(
        &mut self,
        map_id: u16,
        instance_id: u32,
        respawn: &wow_world_core::creature_respawn::PendingRespawn,
        now: std::time::Instant,
        now_secs: i64,
    ) -> Option<wow_persistence::RespawnPersistenceMutationLikeCpp> {
        wow_world_core::creature_respawn::save_pending_respawn_time_on_queue_like_cpp(
            self.create_world_map(u32::from(map_id), instance_id)
                .map_mut()
                .creature_respawn_queue_like_cpp_mut(),
            map_id,
            instance_id,
            respawn,
            now,
            now_secs,
        )
    }

    fn persisted_respawn_time_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        object_type: wow_map::SpawnObjectType,
        spawn_id: u64,
    ) -> Option<i64> {
        self.find_map(u32::from(map_id), instance_id)?
            .map()
            .creature_respawn_queue_like_cpp()
            .persisted_respawn_time_like_cpp(object_type, spawn_id)
    }
}
