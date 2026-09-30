//! One memory-only death/corpse prefix over the existing two storage rails.
//! Pinned C++ a5f8da2e Creature.cpp:419–487,2193–2295; Map.cpp:666–817.
//! This does not execute Creature::Update or activate a canonical producer.
use crate::map::{Map, NoopGridLifecycle, NoopTerrainGridLoader};
use crate::map_manager::{MapManager as LegacyMapManager, PendingRespawn};
use crate::spawn::ActorRespawnPhaseOutcome;
use std::collections::VecDeque;
use std::time::Instant;
use wow_constants::DeathState;

mod backend;
mod models;
use backend::Memory;
use models::{Clocks, MemoryPrefix};
pub use models::{LegacyCreatureRespawnPrefix, RemovedCreatureCorpse};

pub(super) fn prepare_canonical_creature_respawns(
    map: &mut Map<NoopTerrainGridLoader, NoopGridLifecycle>,
    now: Instant,
    conversion_now: Instant,
    conversion_now_secs: i64,
    persistent_world_map: bool,
) -> (ActorRespawnPhaseOutcome, VecDeque<PendingRespawn>) {
    let prefix = prepare(
        Memory::Canonical(map),
        Clocks {
            now,
            conversion_now,
            conversion_now_secs,
        },
        persistent_world_map,
    );
    (prefix.outcome, prefix.ready.into())
}

pub(crate) fn prepare_legacy_creature_respawns(
    manager: &mut LegacyMapManager,
    key: (u16, u32),
    now: Instant,
    conversion_now: Instant,
    conversion_now_secs: i64,
    persistent_world_map: bool,
) -> LegacyCreatureRespawnPrefix {
    let prefix = prepare(
        Memory::Legacy { manager, key },
        Clocks {
            now,
            conversion_now,
            conversion_now_secs,
        },
        persistent_world_map,
    );
    LegacyCreatureRespawnPrefix {
        creatures_seen: prefix.outcome.creatures_seen,
        respawn_db_mutations: prefix.outcome.respawn_db_mutations,
        removed_corpses: prefix.removed_corpses,
        ready: prefix.ready,
    }
}

fn prepare(mut memory: Memory<'_>, clocks: Clocks, persistent_world_map: bool) -> MemoryPrefix {
    let mut prefix = MemoryPrefix::default();
    let Some(key) = memory.key() else {
        prefix.outcome.invalid_map_id = true;
        return prefix;
    };
    let guids = memory.actor_guids();
    prefix.outcome.creatures_seen = guids.len();
    if persistent_world_map {
        for guid in &guids {
            let pending = memory.actor(*guid).and_then(|actor| {
                (!actor.is_alive()
                    && actor.creature.spawn_id() != 0
                    && actor.creature.runtime_state().save_respawn_requested)
                    .then(|| backend::capture_pending(actor, key.0, clocks))
            });
            if let Some(pending) = pending {
                if let Some(save) = memory.save_row(&pending, key, clocks) {
                    prefix.outcome.respawn_db_mutations.push(save);
                }
                // Canonical expects its exclusive Actor; legacy retains its
                // conditional still-present lookup, strictly AFTER saving.
                memory.clear_death_save_request(*guid);
            }
        }
    }

    let despawn_guids: Vec<_> = guids
        .into_iter()
        .filter(|guid| {
            memory.actor(*guid).is_some_and(|actor| {
                !actor.is_alive()
                    && actor.creature.unit().death_state() == DeathState::Corpse
                    && actor.corpse_despawn_due_like_cpp()
            })
        })
        .collect();
    for guid in despawn_guids {
        let Some(captured) = memory.cleanup_and_capture(guid, key.0, clocks) else {
            continue;
        };
        let persistent_spawn = captured.pending.persistent_spawn;
        let spawn_id = captured.pending.spawn_id;
        if persistent_world_map
            && persistent_spawn
            && memory
                .saved_time(spawn_id)
                .is_none_or(|stored| stored < captured.seconds)
        {
            // Preserve the old in-memory replacement and single SQL upsert.
            // Legacy's removed-row Delete value is discarded, never published.
            memory.remove_saved_row(spawn_id);
            if let Some(save) = memory.save_row(&captured.pending, key, clocks) {
                prefix.outcome.respawn_db_mutations.push(save);
            }
        }
        memory.queue(captured.pending);
        if memory.finish_corpse(
            guid,
            captured.info,
            persistent_spawn,
            &mut prefix.removed_corpses,
        ) {
            prefix.outcome.corpses_removed += 1;
        } else {
            prefix.outcome.removal_failures += 1;
        }
    }
    prefix.ready = memory.ready(clocks.now);
    prefix
}

#[cfg(test)]
mod tests;
