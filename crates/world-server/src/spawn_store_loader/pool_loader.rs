//! PoolMgr and spawn-group startup loading over canonical spawn metadata.
//!
//! The parent facade remains the startup composition owner; this module groups
//! pool/member validation and spawn-group template construction.

use super::*;

pub(super) async fn load_pool_mgr_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    store: &SpawnStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<PoolMgrLikeCpp> {
    let mut mgr = PoolMgrLikeCpp::new();
    let templates = match persistence.load_pool_templates_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(templates) => templates,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical pool template catalog failed: {reason}")
        }
    };
    if templates.is_empty() {
        return Ok(mgr);
    }
    for row in templates {
        apply_pool_template_row_like_cpp(row, &mut mgr, &mut report.pool_mgr);
    }
    load_pool_member_rows_like_cpp(
        load_pool_members_from_persistence_like_cpp(
            persistence,
            PoolMemberKindPersistenceLikeCpp::Creature,
        )
        .await?,
        store,
        PoolMemberKindLikeCpp::Creature,
        &mut mgr,
        report,
    );
    load_pool_member_rows_like_cpp(
        load_pool_members_from_persistence_like_cpp(
            persistence,
            PoolMemberKindPersistenceLikeCpp::GameObject,
        )
        .await?,
        store,
        PoolMemberKindLikeCpp::GameObject,
        &mut mgr,
        report,
    );
    load_pool_member_rows_like_cpp(
        load_pool_members_from_persistence_like_cpp(
            persistence,
            PoolMemberKindPersistenceLikeCpp::Pool,
        )
        .await?,
        store,
        PoolMemberKindLikeCpp::Pool,
        &mut mgr,
        report,
    );
    apply_pool_map_propagation_like_cpp(&mut mgr, &mut report.pool_mgr);
    apply_pool_final_validation_like_cpp(&mgr, &mut report.pool_mgr);
    load_pool_autospawn_candidates_like_cpp(persistence, &mut mgr, report).await?;
    Ok(mgr)
}
pub(super) async fn load_pool_members_from_persistence_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    kind: PoolMemberKindPersistenceLikeCpp,
) -> Result<Vec<PoolMemberRowLikeCpp>> {
    match persistence.load_pool_members_like_cpp(kind).await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical pool member catalog failed: {reason}")
        }
    }
}
pub(super) fn load_pool_member_rows_like_cpp(
    rows: Vec<PoolMemberRowLikeCpp>,
    store: &SpawnStore,
    kind: PoolMemberKindLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) {
    for row in rows {
        match kind {
            PoolMemberKindLikeCpp::Creature | PoolMemberKindLikeCpp::GameObject => {
                apply_pool_spawn_member_row_like_cpp(row, store, kind, mgr, &mut report.pool_mgr);
            }
            PoolMemberKindLikeCpp::Pool => {
                apply_pool_pool_member_row_like_cpp(row, mgr, &mut report.pool_mgr);
            }
        }
    }
}
pub(super) async fn load_pool_autospawn_candidates_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let rows = match persistence.load_pool_autospawn_candidates_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical pool autospawn catalog failed: {reason}")
        }
    };
    for row in rows {
        apply_pool_autospawn_candidate_row_like_cpp(row, mgr, &mut report.pool_mgr);
    }
    Ok(())
}
pub(super) fn apply_pool_template_row_like_cpp(
    row: PoolTemplateRowLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    report.template_rows += 1;
    mgr.insert_template_like_cpp(row.entry, PoolTemplateDataLikeCpp::new(row.max_limit, -1));
    report.templates_loaded += 1;
}
pub(super) fn apply_pool_spawn_member_row_like_cpp(
    row: PoolMemberRowLikeCpp,
    store: &SpawnStore,
    kind: PoolMemberKindLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    let member_report = match kind {
        PoolMemberKindLikeCpp::Creature => &mut report.creature_members,
        PoolMemberKindLikeCpp::GameObject => &mut report.gameobject_members,
        PoolMemberKindLikeCpp::Pool => {
            unreachable!("pool rows use apply_pool_pool_member_row_like_cpp")
        }
    };
    member_report.rows += 1;

    let spawn_type = match kind {
        PoolMemberKindLikeCpp::Creature => SpawnObjectType::Creature,
        PoolMemberKindLikeCpp::GameObject => SpawnObjectType::GameObject,
        PoolMemberKindLikeCpp::Pool => {
            unreachable!("pool rows use apply_pool_pool_member_row_like_cpp")
        }
    };
    let Some(spawn_data) = store.spawn_data(spawn_type, row.spawn_id) else {
        member_report.skipped_missing_spawn += 1;
        return;
    };
    let Some(template) = mgr.templates.get_mut(&row.pool_spawn_id) else {
        member_report.skipped_missing_template += 1;
        return;
    };
    if !(0.0..=100.0).contains(&row.chance) {
        member_report.skipped_invalid_chance += 1;
        return;
    }

    let map_id = match i32::try_from(spawn_data.map_id) {
        Ok(map_id) => map_id,
        Err(_) => {
            member_report.skipped_map_mismatch += 1;
            return;
        }
    };
    if template.map_id == -1 {
        template.map_id = map_id;
    }
    if template.map_id != map_id {
        member_report.skipped_map_mismatch += 1;
        return;
    }

    let max_limit = template.max_limit;
    let group_map = match kind {
        PoolMemberKindLikeCpp::Creature => &mut mgr.creature_groups,
        PoolMemberKindLikeCpp::GameObject => &mut mgr.gameobject_groups,
        PoolMemberKindLikeCpp::Pool => {
            unreachable!("pool rows use apply_pool_pool_member_row_like_cpp")
        }
    };
    let group = group_map
        .entry(row.pool_spawn_id)
        .or_insert_with(|| PoolGroupLikeCpp::with_pool_id(kind, row.pool_spawn_id));
    group.set_pool_id_like_cpp(row.pool_spawn_id);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(row.spawn_id, row.chance), max_limit);
    let spawn_id = row.spawn_id;
    let _ = mgr.register_spawn_pool_relation_like_cpp(kind, spawn_id, row.pool_spawn_id);
    member_report.loaded += 1;
}

pub(super) fn apply_pool_pool_member_row_like_cpp(
    row: PoolMemberRowLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    report.pool_members.rows += 1;
    let Ok(child_pool_id) = u32::try_from(row.spawn_id) else {
        report.pool_members.skipped_child_id_overflow += 1;
        return;
    };
    if !mgr.templates.contains_key(&row.pool_spawn_id) {
        report.pool_members.skipped_missing_template += 1;
        return;
    }
    if !mgr.templates.contains_key(&child_pool_id) {
        report.pool_members.skipped_missing_spawn += 1;
        return;
    }
    if row.pool_spawn_id == child_pool_id {
        report.circular_relations += 1;
        report.pool_members.skipped_missing_spawn += 1;
        return;
    }
    if !(0.0..=100.0).contains(&row.chance) {
        report.pool_members.skipped_invalid_chance += 1;
        return;
    }

    let max_limit = mgr
        .templates
        .get(&row.pool_spawn_id)
        .map(|template| template.max_limit)
        .unwrap_or(0);
    let group = mgr.pool_groups.entry(row.pool_spawn_id).or_insert_with(|| {
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, row.pool_spawn_id)
    });
    group.set_pool_id_like_cpp(row.pool_spawn_id);
    group.add_entry_like_cpp(
        PoolObjectLikeCpp::new(u64::from(child_pool_id), row.chance),
        max_limit,
    );
    let _ = mgr.register_child_pool_relation_like_cpp(u64::from(child_pool_id), row.pool_spawn_id);
    report.pool_members.loaded += 1;
}

pub(super) fn apply_pool_map_propagation_like_cpp(
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    let pool_ids = mgr.templates.keys().copied().collect::<Vec<_>>();
    for pool_id in pool_ids {
        let mut checked = std::collections::HashSet::new();
        let mut current = pool_id;
        while let Some(parent) = mgr.child_pool_to_parent.get(&current).copied() {
            let child_map_id = mgr
                .templates
                .get(&current)
                .map_or(-1, |template| template.map_id);
            if child_map_id != -1 {
                if let Some(parent_template) = mgr.templates.get_mut(&parent) {
                    if parent_template.map_id == -1 {
                        parent_template.map_id = child_map_id;
                    }
                    if parent_template.map_id != child_map_id {
                        mgr.remove_child_pool_relation_like_cpp(current, parent);
                        report.map_mismatches += 1;
                        report.relation_removals += 1;
                        report.pool_members.loaded = report.pool_members.loaded.saturating_sub(1);
                        break;
                    }
                }
            }

            checked.insert(current);
            if checked.contains(&parent) {
                mgr.remove_child_pool_relation_like_cpp(current, parent);
                report.circular_relations += 1;
                report.relation_removals += 1;
                report.pool_members.loaded = report.pool_members.loaded.saturating_sub(1);
                break;
            }
            current = parent;
        }
    }
}

pub(super) fn apply_pool_final_validation_like_cpp(
    mgr: &PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    for (&pool_id, template) in &mgr.templates {
        if mgr.is_empty_like_cpp(pool_id) {
            report.empty_pools += 1;
        } else if template.map_id == -1 {
            report.missing_map_after_non_empty += 1;
        }
    }
}

pub(super) fn apply_pool_autospawn_candidate_row_like_cpp(
    row: PoolAutospawnCandidateRowLikeCpp,
    mgr: &mut PoolMgrLikeCpp,
    report: &mut PoolMgrLoadReportLikeCpp,
) {
    report.autospawn_rows += 1;
    if mgr.is_empty_like_cpp(row.pool_entry) {
        report.autospawn_skipped_empty += 1;
        return;
    }
    if !mgr.check_pool_like_cpp(row.pool_entry) {
        report.autospawn_skipped_broken += 1;
        return;
    }
    if row.child_pool_id != 0 {
        let _mother_pool_id = row.mother_pool_id;
        report.autospawn_skipped_child += 1;
        return;
    }
    if let Some(template) = mgr.templates.get(&row.pool_entry) {
        mgr.add_auto_spawn_pool_like_cpp(template.map_id, row.pool_entry);
        report.autospawn_loaded += 1;
    }
}

pub fn spawn_group_templates_for_spawn_store(
    store: &wow_data::SpawnGroupTemplateStore,
) -> BTreeMap<u32, SpawnGroupTemplateData> {
    let mut templates = BTreeMap::new();
    for template in store.iter() {
        let map_id = match template.group_id {
            0 | 1 => 0,
            _ => SPAWNGROUP_MAP_UNSET,
        };
        templates.insert(
            template.group_id,
            SpawnGroupTemplateData {
                group_id: template.group_id,
                name: template.name.clone(),
                map_id,
                flags: SpawnGroupFlags(template.flags),
            },
        );
    }

    templates
        .entry(0)
        .or_insert_with(SpawnGroupTemplateData::default_group);
    templates
        .entry(1)
        .or_insert_with(SpawnGroupTemplateData::legacy_group);
    templates
}

pub(super) async fn load_spawn_group_members_like_cpp(
    persistence: &dyn CanonicalSpawnCatalogPersistencePortLikeCpp,
) -> Result<Vec<SpawnGroupMemberRow>> {
    match persistence.load_spawn_group_members_like_cpp().await {
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows
            .into_iter()
            .map(
                |row: SpawnGroupMemberPersistenceRowLikeCpp| SpawnGroupMemberRow {
                    group_id: row.group_id,
                    spawn_type: row.spawn_type,
                    spawn_id: row.spawn_id,
                },
            )
            .collect()),
        CanonicalSpawnCatalogLoadOutcomeLikeCpp::Failed { reason } => {
            bail!("canonical spawn-group member catalog failed: {reason}")
        }
    }
}
