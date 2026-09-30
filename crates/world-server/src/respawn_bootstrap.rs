// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Persisted respawn loading and canonical map startup installation.

use std::sync::Arc;

use anyhow::{Result, bail};
use tracing::{debug, warn};
use wow_persistence::{
    RespawnPersistenceLoadOutcomeLikeCpp, RespawnPersistencePortLikeCpp,
    RespawnPersistenceRowLikeCpp,
};
use wow_world::conditions::{
    ConditionMapRef, ConditionMapStateSnapshot, is_spawn_group_meeting_map_conditions_like_cpp,
};

use super::{
    PersistedRespawnApplyReportLikeCpp, PersistedRespawnLoadReportLikeCpp,
    PersistedRespawnTimesLikeCpp, SharedCanonicalSpawnMetadataLikeCpp, spawn_store_loader,
};

impl PersistedRespawnTimesLikeCpp {
    pub(super) fn push(&mut self, key: wow_map::MapKey, info: wow_map::RespawnInfoLikeCpp) {
        self.by_map.entry(key).or_default().push(info);
    }

    fn for_map(&self, key: wow_map::MapKey) -> &[wow_map::RespawnInfoLikeCpp] {
        self.by_map.get(&key).map_or(&[], Vec::as_slice)
    }

    pub(super) fn maps_len(&self) -> usize {
        self.by_map.len()
    }

    pub(super) fn respawns_len(&self) -> usize {
        self.by_map.values().map(Vec::len).sum()
    }
}

pub(super) async fn load_persisted_respawn_times_like_cpp(
    respawn_persistence: &dyn RespawnPersistencePortLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
) -> Result<(
    PersistedRespawnTimesLikeCpp,
    PersistedRespawnLoadReportLikeCpp,
)> {
    let rows = match respawn_persistence.load_all_like_cpp().await {
        RespawnPersistenceLoadOutcomeLikeCpp::Loaded(rows) => rows,
        RespawnPersistenceLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("failed to load persisted respawn times: {reason}")
        }
    };
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    let mut report = PersistedRespawnLoadReportLikeCpp::default();

    for row in rows {
        if let Some((key, info)) =
            persisted_respawn_info_from_row_like_cpp(row, canonical_spawn_metadata, &mut report)
        {
            snapshot.push(key, info);
        }
    }

    Ok((snapshot, report))
}

pub(super) fn persisted_respawn_info_from_row_like_cpp(
    row: RespawnPersistenceRowLikeCpp,
    canonical_spawn_metadata: &spawn_store_loader::CanonicalSpawnMetadataLikeCpp,
    report: &mut PersistedRespawnLoadReportLikeCpp,
) -> Option<(wow_map::MapKey, wow_map::RespawnInfoLikeCpp)> {
    report.rows += 1;
    let Ok(object_type_raw) = u8::try_from(row.object_type_raw) else {
        report.invalid_type += 1;
        return None;
    };
    let Some(object_type) = wow_map::SpawnObjectType::from_raw(object_type_raw) else {
        report.invalid_type += 1;
        return None;
    };
    if matches!(object_type, wow_map::SpawnObjectType::AreaTrigger) {
        report.unsupported_area_trigger += 1;
        return None;
    }

    let Some(spawn_data) = canonical_spawn_metadata
        .spawn_store()
        .spawn_data(object_type, row.spawn_id)
    else {
        report.missing_spawn_metadata += 1;
        return None;
    };

    report.loaded += 1;
    Some((
        wow_map::MapKey::new(row.map_id, row.instance_id),
        wow_map::RespawnInfoLikeCpp {
            object_type,
            spawn_id: row.spawn_id,
            entry: spawn_data.id,
            respawn_time: row.respawn_time,
            grid_id: wow_map::compute_grid_coord(
                spawn_data.spawn_point.x,
                spawn_data.spawn_point.y,
            )
            .get_id(),
        },
    ))
}

fn apply_persisted_respawns_to_managed_map_like_cpp(
    managed_map: &mut wow_map::ManagedMap,
    persisted_respawn_times: &PersistedRespawnTimesLikeCpp,
    map_store: &wow_data::MapStore,
) -> PersistedRespawnApplyReportLikeCpp {
    let key = wow_map::MapKey::new(managed_map.map_id(), managed_map.instance_id());
    let respawns = persisted_respawn_times.for_map(key);
    let mut report = PersistedRespawnApplyReportLikeCpp {
        candidates: respawns.len(),
        ..PersistedRespawnApplyReportLikeCpp::default()
    };

    if !matches!(managed_map.kind(), wow_map::ManagedMapKind::World) {
        report.skipped_non_world_map = respawns.len();
        return report;
    }
    if map_store
        .get(managed_map.map_id())
        .is_some_and(|entry| entry.is_instanceable_like_cpp())
    {
        // C++ `Map::LoadRespawnTimes` returns immediately for every
        // `MapEntry::Instanceable()` map. `ManagedMapKind::World` alone is not
        // sufficient because garrisons are represented by that kind too.
        report.skipped_instanceable_map = respawns.len();
        return report;
    }

    for info in respawns {
        match managed_map
            .map_mut()
            .add_respawn_info_like_cpp(info.clone())
        {
            wow_map::AddRespawnInfoOutcomeLikeCpp::Inserted => report.inserted += 1,
            wow_map::AddRespawnInfoOutcomeLikeCpp::ReplacedExisting => {
                report.replaced_existing += 1
            }
            wow_map::AddRespawnInfoOutcomeLikeCpp::RejectedZeroSpawnId => {
                report.rejected_zero_spawn_id += 1;
            }
            wow_map::AddRespawnInfoOutcomeLikeCpp::RejectedUnsupportedType => {
                report.rejected_unsupported_type += 1;
            }
            wow_map::AddRespawnInfoOutcomeLikeCpp::RejectedExistingSoonerOrEqual => {
                report.rejected_existing_sooner_or_equal += 1;
            }
        }
    }

    report
}

pub(super) fn install_canonical_spawn_group_initializer_like_cpp(
    manager: &mut wow_map::MapManager,
    canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    condition_store: Arc<wow_data::ConditionEntriesByTypeStore>,
    persisted_respawn_times: Arc<PersistedRespawnTimesLikeCpp>,
    map_store: Arc<wow_data::MapStore>,
) {
    manager.set_spawn_group_initializer_like_cpp(move |managed_map| {
        let map_id = managed_map.map_id();
        let instance_id = managed_map.instance_id();
        let difficulty_id = u32::from(managed_map.map().spawn_mode());
        let Ok(canonical_spawn_metadata) = canonical_spawn_metadata.lock() else {
            warn!(
                map_id,
                instance_id,
                difficulty_id,
                "CanonicalSpawnMetadataLikeCpp mutex poisoned; skipping InitSpawnGroupState hook"
            );
            return;
        };

        let pool_init_report = managed_map.map_mut().init_pools_for_map_like_cpp(
            canonical_spawn_metadata.pool_mgr_like_cpp(),
            |_kind, _pool_id| 0.0,
            |_candidates, count| (0..count).collect(),
        );
        if pool_init_report.attempted() > 0 || pool_init_report.error_count() > 0 {
            debug!(
                map_id,
                instance_id,
                difficulty_id,
                attempted = pool_init_report.attempted(),
                planned = pool_init_report.planned(),
                errors = pool_init_report.error_count(),
                spawn_one_actions = pool_init_report.spawn_one_actions(),
                respawn_one_actions = pool_init_report.respawn_one_actions(),
                despawn_one_actions = pool_init_report.despawn_one_actions(),
                "Applied represented C++ PoolMgr::InitPoolsForMap autospawn plans to map-owned pool data before LoadRespawnTimes; live entity side effects remain report-only"
            );
        }
        for error in &pool_init_report.errors {
            warn!(
                map_id,
                instance_id,
                difficulty_id,
                pool_id = error.pool_id,
                error = ?error.error,
                "PoolMgr::InitPoolsForMap represented autospawn planning failed for pool; leaving entity side effects unexecuted"
            );
        }

        let respawn_report = apply_persisted_respawns_to_managed_map_like_cpp(
            managed_map,
            persisted_respawn_times.as_ref(),
            map_store.as_ref(),
        );
        if respawn_report.candidates > 0 {
            debug!(
                map_id,
                instance_id,
                difficulty_id,
                candidates = respawn_report.candidates,
                inserted = respawn_report.inserted,
                replaced_existing = respawn_report.replaced_existing,
                rejected_zero_spawn_id = respawn_report.rejected_zero_spawn_id,
                rejected_unsupported_type = respawn_report.rejected_unsupported_type,
                rejected_existing_sooner_or_equal = respawn_report.rejected_existing_sooner_or_equal,
                skipped_non_world_map = respawn_report.skipped_non_world_map,
                skipped_instanceable_map = respawn_report.skipped_instanceable_map,
                "Applied C++ startup LoadRespawnTimes snapshot to canonical map before InitSpawnGroupState"
            );
        }

        let groups = canonical_spawn_metadata.spawn_group_templates_for_map_like_cpp(map_id);
        if groups.is_empty() {
            debug!(
                map_id,
                instance_id,
                difficulty_id,
                "InitSpawnGroupState hook found no spawn groups for map"
            );
            return;
        }

        let group_templates = groups
            .iter()
            .map(|(_group_id, template)| *template)
            .collect::<Vec<_>>();
        let map_ref = ConditionMapRef::new(map_id, instance_id);
        let map_state = ConditionMapStateSnapshot {
            active_event_ids: &[],
            world_states: &[],
            difficulty_id,
            instance_data: &[],
            instance_data64: &[],
            boss_states: &[],
            scenario_step_id: None,
        };
        let changes =
            managed_map
                .map_mut()
                .init_spawn_group_state_like_cpp(group_templates, |group| {
                    is_spawn_group_meeting_map_conditions_like_cpp(
                        condition_store.as_ref(),
                        group.group_id,
                        map_ref,
                        Some(map_state),
                        &[],
                    )
                });
        let toggled = changes
            .iter()
            .filter(|(_group_id, change)| {
                matches!(change, wow_map::SpawnGroupActiveChange::Toggled)
            })
            .count();
        debug!(
            map_id,
            instance_id,
            difficulty_id,
            groups_evaluated = changes.len(),
            toggled,
            "Applied C++ InitSpawnGroupState hook to canonical map"
        );
    });
}
