// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical map tick orchestration.
//!
//! C++ `MapManager::Update` (`Maps/MapManager.cpp:287-318`) advances one shared
//! timer, decides `CanUnload` before updating, joins the map updates and then
//! runs `DelayedUpdate`. #787 splits that sequence where C++ drives each map's
//! world sessions (`Maps/Map.cpp:669-680`), so the coordinator can release
//! every synchronous guard for the session phase and resume the same tick.
//!
//! Separated from `runtime/map.rs` under #787, following that file's recorded
//! split direction: canonical tick orchestration apart from respawn
//! persistence projections and recipient delivery.

pub(crate) mod object_work;
mod respawn_actor;
mod respawn_catalog;
use respawn_catalog::canonical_map_tick_respawn_phase_like_cpp;

use super::map::{
    build_loaded_grid_creature_respawn_record_like_cpp,
    build_loaded_grid_gameobject_respawn_record_like_cpp,
};
use super::tick_summary::{
    CanonicalMapObjectValuesUpdateLikeCpp, CanonicalObjectVisibilityDestroyLikeCpp,
    CanonicalSpawnGroupConditionTickSummaryLikeCpp,
};
use super::*;

/// One admitted map of a split canonical tick and the sessions C++ would drive
/// inside its `Map::Update` (#787).
#[derive(Debug)]
pub(crate) struct CanonicalMapSessionPassPlanLikeCpp {
    pub(crate) plan: wow_map::MapTickPlanLikeCpp,
    /// Per admitted map, its in-world players in `m_mapRefManager` order.
    pub(crate) participants: Vec<CanonicalMapSessionPassMapLikeCpp>,
}

/// One admitted map and the sessions its tick drives, with the identities
/// frozen under the guard that is released immediately afterwards (#787).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalMapSessionPassMapLikeCpp {
    pub(crate) key: wow_map::MapKey,
    /// The incarnation of the map this tick admitted; a map recreated under the
    /// same key during the pass is a different map.
    pub(crate) incarnation: u64,
    pub(crate) participants: Vec<wow_map::MapSessionPassParticipantLikeCpp>,
}

/// A process-unique identity for one canonical map coordinator.
///
/// Two coordinators would produce the same epoch sequence, so the epoch alone
/// cannot tell a session whose request it is holding. C++ has one
/// `MapManager::Update` caller and needs no such identity.
pub(crate) fn canonical_map_coordinator_id_like_cpp() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// C++ `MapManager::Update` up to the point where each map drives its world
/// sessions. The caller must release every synchronous guard before delivering
/// the session requests, and then resume the same tick with
/// [`canonical_map_tick_resume_like_cpp`].
pub(crate) fn canonical_map_tick_begin_like_cpp(
    manager: &mut wow_map::MapManager,
    diff_ms: u32,
) -> Option<CanonicalMapSessionPassPlanLikeCpp> {
    let plan = manager.begin_tick_like_cpp(diff_ms).into_started()?;
    let participants = plan
        .updated_maps_like_cpp()
        .iter()
        .map(|participant| CanonicalMapSessionPassMapLikeCpp {
            key: participant.key,
            incarnation: participant.incarnation,
            participants: manager.map_session_pass_participants_like_cpp(participant.key),
        })
        .collect();
    Some(CanonicalMapSessionPassPlanLikeCpp { plan, participants })
}

/// The rest of the same canonical tick, resumed with the diff saved at the
/// split. Runs exactly once per plan.
pub(crate) fn canonical_map_tick_resume_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    plan: wow_map::MapTickPlanLikeCpp,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp> {
    try_canonical_map_tick_resume(
        manager,
        legacy_manager,
        plan,
        scheduler,
        canonical_spawn_metadata,
        condition_store,
        map_store,
        loaded_grid_creature_respawn_caches,
    ).ok().flatten()
}

pub(crate) fn try_canonical_map_tick_resume(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    plan: wow_map::MapTickPlanLikeCpp,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Result<
    Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp>,
    object_work::CanonicalObjectResumeFailure,
> {
    object_work::try_resume(manager, legacy_manager, plan, scheduler,
        canonical_spawn_metadata, condition_store, map_store,
        loaded_grid_creature_respawn_caches)
}

pub(crate) fn canonical_map_update_tick_set_inactive_like_cpp(
    manager: &mut wow_map::MapManager,
    legacy_manager: Option<&SharedMapManager>,
    diff_ms: u32,
    scheduler: &mut CanonicalRespawnConditionSchedulerLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    condition_store: &wow_data::ConditionEntriesByTypeStore,
    map_store: &wow_data::MapStore,
    loaded_grid_creature_respawn_caches: &LoadedGridCreatureRespawnCachesLikeCpp,
) -> Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp> {
    let plan = canonical_map_tick_begin_like_cpp(manager, diff_ms)?;
    canonical_map_tick_resume_like_cpp(
        manager,
        legacy_manager,
        plan.plan,
        scheduler,
        canonical_spawn_metadata,
        condition_store,
        map_store,
        loaded_grid_creature_respawn_caches,
    )
}



fn canonical_map_tick_tail_like_cpp(
    manager: &mut wow_map::MapManager,
    mut summary: CanonicalSpawnGroupConditionTickSummaryLikeCpp,
    map_store: &wow_data::MapStore,
) -> Option<CanonicalSpawnGroupConditionTickSummaryLikeCpp> {
    summary.player_visibility_refresh_intents =
        manager.take_player_visibility_refresh_intents_like_cpp();
    let map_incarnations = {
        let mut incarnations = std::collections::HashMap::new();
        manager.do_for_all_maps(|managed_map| {
            let key = wow_map::MapKey::new(managed_map.map_id(), managed_map.instance_id());
            if let Some(incarnation) = manager.map_incarnation_like_cpp(key) {
                incarnations.insert(key, incarnation);
            }
        });
        incarnations
    };
    manager.do_for_all_maps_mut(|managed_map| {
        let map_id = managed_map.map_id();
        let instance_id = managed_map.instance_id();
        let map_incarnation = map_incarnations
            .get(&wow_map::MapKey::new(map_id, instance_id))
            .copied()
            .unwrap_or_default();
        summary.object_visibility_destroys.extend(
            managed_map
                .map_mut()
                .take_object_visibility_destroy_recipients_like_cpp()
                .into_iter()
                .map(|intent| CanonicalObjectVisibilityDestroyLikeCpp {
                    map_id,
                    instance_id,
                    map_incarnation,
                    object_guid: intent.object_guid,
                    recipient_guids: intent.recipient_guids,
                }),
        );
        append_map_object_values_updates_like_cpp(&mut summary, managed_map);
        summary.expired_pvp_combat_refs.extend(
            managed_map
                .last_expired_pvp_combat_refs_like_cpp()
                .iter()
                .map(|(owner, target)| {
                    (
                        managed_map.map_id(),
                        managed_map.instance_id(),
                        *owner,
                        *target,
                    )
                }),
        );
        let map_kind = managed_map.kind();
        let map_id = managed_map.map_id();
        let instance_id = managed_map.instance_id();
        let map_is_instanceable = map_store
            .get(map_id)
            .is_some_and(|entry| entry.is_instanceable_like_cpp());
        for info in managed_map
            .last_game_objects_update_summary()
            .respawn_db_saves
        {
            match queue_respawn_db_save_like_cpp(
                map_kind,
                map_is_instanceable,
                map_id,
                instance_id,
                info,
            ) {
                RespawnDbSaveQueueOutcomeLikeCpp::Queued(save) => {
                    summary.respawn_db_save_queued += 1;
                    summary.respawn_db_saves.push(save);
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedNonWorldMap => {
                    summary.respawn_db_save_skipped_non_world_map += 1;
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedInstanceableMap => {
                    summary.respawn_db_save_skipped_instanceable_map += 1;
                }
                RespawnDbSaveQueueOutcomeLikeCpp::SkippedInvalidMapId => {
                    summary.respawn_db_save_skipped_invalid_map_id += 1;
                }
            }
        }
    });
    let has_work = !summary.respawn_db_saves.is_empty()
        || !summary.expired_pvp_combat_refs.is_empty()
        || !summary.player_visibility_refresh_intents.is_empty()
        || !summary.object_values_updates.is_empty()
        || !summary.object_visibility_destroys.is_empty()
        || !summary.respawn_db_deletes.is_empty()
        || summary.maps_evaluated > 0;
    has_work.then_some(summary)
}

/// Convert the typed snapshots captured by `Map::SendObjectUpdates` while the
/// map guard was held into owned packet bytes. The caller delivers the result
/// only after releasing every synchronous map/persistence guard.
fn append_map_object_values_updates_like_cpp(
    summary: &mut CanonicalSpawnGroupConditionTickSummaryLikeCpp,
    managed_map: &wow_map::ManagedMap,
) {
    let Ok(map_id) = u16::try_from(managed_map.map_id()) else {
        return;
    };
    let instance_id = managed_map.instance_id();
    let updates = managed_map.last_send_object_updates_summary_like_cpp();

    // Player/Unit snapshots have viewer-dependent field visibility and are
    // consumed exclusively by the Session-side P3.10 adapter.  Routing their
    // unfiltered bytes through this generic rail would duplicate delivery and
    // expose owner-only Player fields to observers.  Keep the generic rail for
    // object families whose packet is viewer-independent until each earns its
    // own typed consumer.

    for update in updates.game_object_values_updates {
        let Some(packet) =
            wow_world::entity_update_bridge::game_object_values_update_to_update_object(
                update.guid,
                map_id,
                &update.values_update,
            )
        else {
            continue;
        };
        summary
            .object_values_updates
            .push(CanonicalMapObjectValuesUpdateLikeCpp {
                map_id,
                instance_id,
                object_guid: update.guid,
                packet_bytes: packet.to_bytes(),
                unit_values_update: None,
            });
    }

    for update in updates.corpse_values_updates {
        let Some(packet) = wow_world::entity_update_bridge::corpse_values_update_to_update_object(
            update.guid,
            map_id,
            &update.values_update,
        ) else {
            continue;
        };
        summary
            .object_values_updates
            .push(CanonicalMapObjectValuesUpdateLikeCpp {
                map_id,
                instance_id,
                object_guid: update.guid,
                packet_bytes: packet.to_bytes(),
                unit_values_update: None,
            });
    }

    for update in updates.area_trigger_values_updates {
        let Some(packet) =
            wow_world::entity_update_bridge::area_trigger_values_update_to_update_object(
                update.guid,
                map_id,
                &update.values_update,
            )
        else {
            continue;
        };
        summary
            .object_values_updates
            .push(CanonicalMapObjectValuesUpdateLikeCpp {
                map_id,
                instance_id,
                object_guid: update.guid,
                packet_bytes: packet.to_bytes(),
                unit_values_update: None,
            });
    }

    for update in updates.scene_object_values_updates {
        let Some(packet) =
            wow_world::entity_update_bridge::scene_object_values_update_to_update_object(
                update.guid,
                map_id,
                &update.values_update,
            )
        else {
            continue;
        };
        summary
            .object_values_updates
            .push(CanonicalMapObjectValuesUpdateLikeCpp {
                map_id,
                instance_id,
                object_guid: update.guid,
                packet_bytes: packet.to_bytes(),
                unit_values_update: None,
            });
    }

    for update in updates.conversation_values_updates {
        let Some(packet) =
            wow_world::entity_update_bridge::conversation_values_update_to_update_object(
                update.guid,
                map_id,
                &update.values_update,
            )
        else {
            continue;
        };
        summary
            .object_values_updates
            .push(CanonicalMapObjectValuesUpdateLikeCpp {
                map_id,
                instance_id,
                object_guid: update.guid,
                packet_bytes: packet.to_bytes(),
                unit_values_update: None,
            });
    }
}
