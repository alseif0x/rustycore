//! Startup loading and validation for the represented GameEventMgr catalog.
//!
//! The parent facade remains the composition owner; this module only groups
//! the game-event catalog loading family and exposes its entrypoints to that parent.

use super::*;

pub(super) fn load_game_event_pool_ids_like_cpp(
    rows: Vec<GameEventPoolRowLikeCpp>,
    game_event_sizing: GameEventSizingLikeCpp,
    mgr: &PoolMgrLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> GameEventPoolIdsLikeCpp {
    let mut game_event_pools =
        GameEventPoolIdsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);
    for row in rows {
        apply_game_event_pool_row_like_cpp(
            row,
            mgr,
            &mut game_event_pools,
            &mut report.game_event_pools,
        );
    }
    game_event_pools
}
pub(super) fn load_game_events_like_cpp(
    rows: Vec<GameEventDataRowLikeCpp>,
    game_event_sizing: GameEventSizingLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> GameEventDataStoreLikeCpp {
    let mut game_events =
        GameEventDataStoreLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);
    if rows.is_empty() {
        return GameEventDataStoreLikeCpp::default();
    }
    for row in rows {
        apply_game_event_data_row_like_cpp(row, &mut game_events, &mut report.game_events);
    }
    game_events
}
pub(super) fn load_game_event_prerequisites_like_cpp(
    rows: Vec<GameEventPrerequisiteRowLikeCpp>,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) {
    for row in rows {
        apply_game_event_prerequisite_row_like_cpp(
            row,
            game_events,
            &mut report.game_event_prerequisites,
        );
    }
}
pub(super) fn apply_game_event_prerequisite_row_like_cpp(
    row: GameEventPrerequisiteRowLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut GameEventPrerequisiteLoadReportLikeCpp,
) {
    report.rows += 1;
    match game_events.insert_prerequisite_event_like_cpp(row.event_id, row.prerequisite_event) {
        GameEventPrerequisiteInsertOutcomeLikeCpp::Loaded => report.loaded += 1,
        GameEventPrerequisiteInsertOutcomeLikeCpp::Duplicate => report.duplicate_ignored += 1,
        GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangeEvent => {
            report.skipped_out_of_range_event += 1;
        }
        GameEventPrerequisiteInsertOutcomeLikeCpp::NonWorldEvent => {
            report.skipped_non_world_event += 1;
        }
        GameEventPrerequisiteInsertOutcomeLikeCpp::OutOfRangePrerequisite => {
            report.skipped_out_of_range_prerequisite += 1;
        }
    }
}
pub(super) fn load_game_event_conditions_like_cpp(
    rows: Vec<GameEventConditionRowLikeCpp>,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) {
    for row in rows {
        apply_game_event_condition_row_like_cpp(
            row,
            game_events,
            &mut report.game_event_conditions,
        );
    }
}
pub(super) fn apply_game_event_condition_row_like_cpp(
    row: GameEventConditionRowLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut GameEventConditionLoadReportLikeCpp,
) {
    report.rows += 1;
    match game_events.apply_game_event_condition_row_like_cpp(
        row.event_id,
        row.condition_id,
        row.req_num,
        row.max_world_state,
        row.done_world_state,
    ) {
        GameEventConditionApplyOutcomeLikeCpp::Loaded => report.loaded += 1,
        GameEventConditionApplyOutcomeLikeCpp::OutOfRangeEvent => {
            report.skipped_out_of_range += 1;
        }
    }
}
pub(super) async fn load_game_event_condition_saves_like_cpp(
    persistence: &dyn wow_persistence::GameEventPersistencePortLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> Result<()> {
    let rows = match persistence.load_condition_saves_like_cpp().await {
        wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Loaded(rows) => rows,
        wow_persistence::GameEventConditionSaveLoadOutcomeLikeCpp::Failed { reason } => {
            anyhow::bail!("game-event condition-save persistence load failed: {reason}")
        }
    };
    for row in rows {
        apply_game_event_condition_save_row_like_cpp(
            GameEventConditionSaveRowLikeCpp {
                event_id: u16::from(row.event_id),
                condition_id: row.condition_id,
                done: row.done,
            },
            game_events,
            &mut report.game_event_condition_saves,
        );
    }

    Ok(())
}

pub(super) fn apply_game_event_condition_save_row_like_cpp(
    row: GameEventConditionSaveRowLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut GameEventConditionSaveLoadReportLikeCpp,
) {
    report.rows += 1;
    match game_events.apply_game_event_condition_save_row_like_cpp(
        row.event_id,
        row.condition_id,
        row.done,
    ) {
        GameEventConditionSaveApplyOutcomeLikeCpp::Loaded => report.loaded += 1,
        GameEventConditionSaveApplyOutcomeLikeCpp::OutOfRangeEvent => {
            report.skipped_out_of_range_event += 1;
        }
        GameEventConditionSaveApplyOutcomeLikeCpp::MissingCondition => {
            report.skipped_missing_condition += 1;
        }
    }
}

pub(super) fn load_game_event_quest_conditions_like_cpp(
    rows: Vec<GameEventQuestConditionRowLikeCpp>,
    game_events: &GameEventDataStoreLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> BTreeMap<u32, GameEventQuestConditionRecordLikeCpp> {
    let mut quest_conditions = BTreeMap::new();
    for row in rows {
        apply_game_event_quest_condition_row_like_cpp(
            row,
            game_events,
            &mut quest_conditions,
            &mut report.game_event_quest_conditions,
        );
    }
    quest_conditions
}

pub(super) fn apply_game_event_quest_condition_row_like_cpp(
    row: GameEventQuestConditionRowLikeCpp,
    game_events: &GameEventDataStoreLikeCpp,
    quest_conditions: &mut BTreeMap<u32, GameEventQuestConditionRecordLikeCpp>,
    report: &mut GameEventQuestConditionLoadReportLikeCpp,
) {
    report.rows += 1;
    if game_events.event_like_cpp(row.event_id).is_none() {
        report.skipped_out_of_range_event += 1;
        return;
    }

    let previous = quest_conditions.insert(
        row.quest_id,
        GameEventQuestConditionRecordLikeCpp {
            quest_id: row.quest_id,
            event_id: row.event_id,
            condition_id: row.condition_id,
            num: row.num,
        },
    );
    report.loaded += 1;
    if previous.is_some() {
        report.overwrites += 1;
    }
}

pub(super) fn apply_game_event_data_row_like_cpp(
    row: GameEventDataRowLikeCpp,
    game_events: &mut GameEventDataStoreLikeCpp,
    report: &mut GameEventDataLoadReportLikeCpp,
) {
    report.rows += 1;
    if row.event_id == 0 {
        report.skipped_reserved_zero += 1;
        return;
    }

    let Some(event) = game_events.event_mut_like_cpp(row.event_id) else {
        report.skipped_out_of_range += 1;
        return;
    };

    event.event_id = row.event_id;
    event.start = row.start;
    event.end = row.end;
    event.next_start = 0;
    event.occurence = row.occurence;
    event.length = row.length;
    event.holiday_id = row.holiday_id;
    event.holiday_stage = row.holiday_stage;
    event.description = row.description;
    event.state_raw = row.state_raw;
    event.announce = row.announce;
    report.loaded += 1;

    if !event.is_valid_like_cpp() {
        report.invalid_normal_zero_length += 1;
    }
    if event.holiday_id != 0 {
        report.holiday_validation_deferred += 1;
    }
}

pub(super) fn apply_game_event_pool_row_like_cpp(
    row: GameEventPoolRowLikeCpp,
    mgr: &PoolMgrLikeCpp,
    game_event_pools: &mut GameEventPoolIdsLikeCpp,
    report: &mut GameEventPoolLoadReportLikeCpp,
) {
    report.rows += 1;
    if game_event_pools
        .internal_event_id_like_cpp(row.event_id)
        .is_none()
    {
        report.skipped_out_of_range += 1;
        return;
    }
    if !mgr.templates.contains_key(&row.pool_entry) || !mgr.check_pool_like_cpp(row.pool_entry) {
        report.skipped_broken_pool += 1;
        return;
    }
    if game_event_pools.push_pool_id_like_cpp(row.event_id, row.pool_entry) {
        report.loaded += 1;
    }
}

pub(super) fn load_game_event_spawn_guids_like_cpp(
    creature_rows: Vec<GameEventObjectGuidRowLikeCpp>,
    gameobject_rows: Vec<GameEventObjectGuidRowLikeCpp>,
    game_event_sizing: GameEventSizingLikeCpp,
    store: &SpawnStore,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> GameEventSpawnGuidsLikeCpp {
    let mut game_event_spawn_guids =
        GameEventSpawnGuidsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    load_game_event_object_guids_like_cpp(
        creature_rows,
        SpawnObjectType::Creature,
        store,
        &mut game_event_spawn_guids,
        &mut report.game_event_spawn_guids.creature,
    );
    load_game_event_object_guids_like_cpp(
        gameobject_rows,
        SpawnObjectType::GameObject,
        store,
        &mut game_event_spawn_guids,
        &mut report.game_event_spawn_guids.gameobject,
    );

    game_event_spawn_guids
}

pub(super) fn load_game_event_object_guids_like_cpp(
    rows: Vec<GameEventObjectGuidRowLikeCpp>,
    object_type: SpawnObjectType,
    store: &SpawnStore,
    game_event_spawn_guids: &mut GameEventSpawnGuidsLikeCpp,
    report: &mut GameEventObjectGuidLoadReportLikeCpp,
) {
    for row in rows {
        apply_game_event_object_guid_row_like_cpp(
            row,
            object_type,
            store,
            game_event_spawn_guids,
            report,
        );
    }
}

pub(super) fn apply_game_event_object_guid_row_like_cpp(
    row: GameEventObjectGuidRowLikeCpp,
    object_type: SpawnObjectType,
    store: &SpawnStore,
    game_event_spawn_guids: &mut GameEventSpawnGuidsLikeCpp,
    report: &mut GameEventObjectGuidLoadReportLikeCpp,
) {
    report.rows += 1;
    let Some(spawn_data) = store.spawn_data(object_type, row.guid) else {
        report.skipped_missing_spawn_metadata += 1;
        return;
    };
    if game_event_spawn_guids
        .internal_event_id_like_cpp(row.event_id)
        .is_none()
    {
        report.skipped_out_of_range += 1;
        return;
    }
    if spawn_data.pool_id != 0 {
        report.pooled_still_loaded += 1;
    }
    if game_event_spawn_guids.push_guid_like_cpp(object_type, row.event_id, row.guid) {
        report.loaded += 1;
    }
}

pub(super) fn load_creature_equip_template_ids_like_cpp(
    rows: Vec<CreatureEquipmentIdPersistenceRowLikeCpp>,
    report: &mut GameEventModelEquipLoadReportLikeCpp,
) -> BTreeSet<(u32, u8)> {
    let mut equipment_ids = BTreeSet::new();
    for row in rows {
        report.equipment_rows += 1;
        // C++ game_event_model_equip validation calls GetEquipmentInfo only for > 0 ids;
        // id 0 is not a valid template key for that positive-id validation path.
        if row.equipment_id > 0 && equipment_ids.insert((row.creature_id, row.equipment_id)) {
            report.equipment_ids_loaded += 1;
        }
    }
    equipment_ids
}

pub(super) fn load_game_event_model_equip_like_cpp(
    equipment_rows: Vec<CreatureEquipmentIdPersistenceRowLikeCpp>,
    model_rows: Vec<GameEventModelEquipRowLikeCpp>,
    game_event_sizing: GameEventSizingLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> GameEventModelEquipLikeCpp {
    let equipment_ids = load_creature_equip_template_ids_like_cpp(
        equipment_rows,
        &mut report.game_event_model_equip,
    );
    let mut model_equip =
        GameEventModelEquipLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    for row in model_rows {
        apply_game_event_model_equip_row_like_cpp(
            row,
            &equipment_ids,
            &mut model_equip,
            &mut report.game_event_model_equip,
        );
    }
    model_equip
}

pub(super) fn apply_game_event_model_equip_row_like_cpp(
    row: GameEventModelEquipRowLikeCpp,
    equipment_ids: &BTreeSet<(u32, u8)>,
    model_equip: &mut GameEventModelEquipLikeCpp,
    report: &mut GameEventModelEquipLoadReportLikeCpp,
) {
    report.rows += 1;
    if model_equip.records_like_cpp(row.event_id).is_none() {
        report.invalid_event_id += 1;
        return;
    }
    if row.equipment_id > 0 && !equipment_ids.contains(&(row.entry, row.equipment_id)) {
        report.missing_equipment_template += 1;
        return;
    }

    if model_equip.push_record_like_cpp(
        row.event_id,
        GameEventModelEquipRecordLikeCpp {
            spawn_id: row.spawn_id,
            model_id: row.model_id,
            model_id_prev: 0,
            equipment_id: row.equipment_id,
            equipment_id_prev: 0,
        },
    ) {
        report.loaded += 1;
    }
}

pub(super) fn load_game_event_quest_relations_like_cpp(
    creature_rows: Vec<GameEventQuestRelationRowLikeCpp>,
    gameobject_rows: Vec<GameEventQuestRelationRowLikeCpp>,
    game_event_sizing: GameEventSizingLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> GameEventQuestRelationsLikeCpp {
    let mut quest_relations =
        GameEventQuestRelationsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    load_game_event_creature_quest_relations_like_cpp(creature_rows, &mut quest_relations, report);
    load_game_event_gameobject_quest_relations_like_cpp(
        gameobject_rows,
        &mut quest_relations,
        report,
    );

    report.game_event_quest_relations.creature.events_touched = quest_relations
        .creature_records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();
    report.game_event_quest_relations.gameobject.events_touched = quest_relations
        .gameobject_records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();

    quest_relations
}

pub(super) fn load_game_event_creature_quest_relations_like_cpp(
    rows: Vec<GameEventQuestRelationRowLikeCpp>,
    quest_relations: &mut GameEventQuestRelationsLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) {
    for row in rows {
        apply_game_event_creature_quest_relation_row_like_cpp(
            row,
            quest_relations,
            &mut report.game_event_quest_relations.creature,
        );
    }
}

pub(super) fn load_game_event_gameobject_quest_relations_like_cpp(
    rows: Vec<GameEventQuestRelationRowLikeCpp>,
    quest_relations: &mut GameEventQuestRelationsLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) {
    for row in rows {
        apply_game_event_gameobject_quest_relation_row_like_cpp(
            row,
            quest_relations,
            &mut report.game_event_quest_relations.gameobject,
        );
    }
}

pub(super) fn apply_game_event_creature_quest_relation_row_like_cpp(
    row: GameEventQuestRelationRowLikeCpp,
    quest_relations: &mut GameEventQuestRelationsLikeCpp,
    report: &mut GameEventQuestRelationFamilyLoadReportLikeCpp,
) {
    report.rows += 1;
    let event_id = u16::from(row.event_id);
    if quest_relations
        .creature_records_like_cpp(event_id)
        .is_none()
    {
        report.skipped_out_of_range += 1;
        return;
    }

    if quest_relations.push_creature_record_like_cpp(
        event_id,
        GameEventQuestRelationRecordLikeCpp {
            giver_id: row.giver_id,
            quest_id: row.quest_id,
        },
    ) {
        report.loaded += 1;
    }
}

pub(super) fn apply_game_event_gameobject_quest_relation_row_like_cpp(
    row: GameEventQuestRelationRowLikeCpp,
    quest_relations: &mut GameEventQuestRelationsLikeCpp,
    report: &mut GameEventQuestRelationFamilyLoadReportLikeCpp,
) {
    report.rows += 1;
    let event_id = u16::from(row.event_id);
    if quest_relations
        .gameobject_records_like_cpp(event_id)
        .is_none()
    {
        report.skipped_out_of_range += 1;
        return;
    }

    if quest_relations.push_gameobject_record_like_cpp(
        event_id,
        GameEventQuestRelationRecordLikeCpp {
            giver_id: row.giver_id,
            quest_id: row.quest_id,
        },
    ) {
        report.loaded += 1;
    }
}

pub(super) fn load_game_event_npc_flags_like_cpp(
    rows: Vec<GameEventNpcFlagRowLikeCpp>,
    game_event_sizing: GameEventSizingLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> GameEventNpcFlagsLikeCpp {
    let mut npc_flags =
        GameEventNpcFlagsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    for row in rows {
        apply_game_event_npc_flag_row_like_cpp(
            row,
            &mut npc_flags,
            &mut report.game_event_npc_flags,
        );
    }

    report.game_event_npc_flags.events_touched = npc_flags
        .records_by_event_id
        .iter()
        .filter(|records| !records.is_empty())
        .count();

    npc_flags
}

pub(super) fn apply_game_event_npc_flag_row_like_cpp(
    row: GameEventNpcFlagRowLikeCpp,
    npc_flags: &mut GameEventNpcFlagsLikeCpp,
    report: &mut GameEventNpcFlagLoadReportLikeCpp,
) {
    report.rows += 1;
    if npc_flags.records_like_cpp(row.event_id).is_none() {
        report.skipped_out_of_range += 1;
        return;
    }

    if npc_flags.push_record_like_cpp(
        row.event_id,
        GameEventNpcFlagRecordLikeCpp {
            spawn_id: row.spawn_id,
            npcflag: row.npcflag,
        },
    ) {
        report.loaded += 1;
    }
}

pub(super) fn load_game_event_npc_vendors_like_cpp(
    rows: Vec<GameEventNpcVendorRowLikeCpp>,
    game_event_sizing: GameEventSizingLikeCpp,
    store: &SpawnStore,
    npc_flags: &GameEventNpcFlagsLikeCpp,
    report: &mut CanonicalSpawnStoreLoadReport,
) -> GameEventNpcVendorsLikeCpp {
    let mut npc_vendors =
        GameEventNpcVendorsLikeCpp::from_game_event_sizing_like_cpp(game_event_sizing);

    for row in rows {
        apply_game_event_npc_vendor_row_like_cpp(
            row,
            store,
            npc_flags,
            &mut npc_vendors,
            &mut report.game_event_npc_vendors,
        );
    }
    npc_vendors
}

pub(super) fn apply_game_event_npc_vendor_row_like_cpp(
    row: GameEventNpcVendorRowLikeCpp,
    store: &SpawnStore,
    npc_flags: &GameEventNpcFlagsLikeCpp,
    npc_vendors: &mut GameEventNpcVendorsLikeCpp,
    report: &mut GameEventNpcVendorLoadReportLikeCpp,
) {
    report.rows += 1;
    let event_id = u16::from(row.event_id);
    if npc_vendors.records_like_cpp(event_id).is_none() {
        report.skipped_out_of_range += 1;
        return;
    }

    let Some(spawn_data) = store.spawn_data(SpawnObjectType::Creature, row.spawn_id) else {
        report.skipped_missing_creature_spawn_metadata += 1;
        return;
    };

    let event_npc_flag_low32 = npc_flags
        .records_like_cpp(event_id)
        .and_then(|records| {
            records
                .iter()
                .find(|record| record.spawn_id == row.spawn_id)
                .map(|record| record.npcflag as u32)
        })
        .unwrap_or(0);

    if npc_vendors.push_record_like_cpp(
        event_id,
        GameEventNpcVendorRecordLikeCpp {
            spawn_id: row.spawn_id,
            guid: row.spawn_id,
            entry: spawn_data.id,
            item: row.item,
            maxcount: row.maxcount,
            incrtime: row.incrtime,
            extended_cost: row.extended_cost,
            vendor_type: row.vendor_type,
            item_type: row.vendor_type,
            bonus_list_ids: parse_game_event_npc_vendor_bonus_list_ids_like_cpp(
                &row.bonus_list_ids,
            ),
            player_condition_id: row.player_condition_id,
            ignore_filtering: row.ignore_filtering,
            event_npc_flag_low32,
        },
    ) {
        report.loaded += 1;
        report.validation_deferred += 1;
    }
}

pub(super) fn parse_game_event_npc_vendor_bonus_list_ids_like_cpp(raw: &str) -> Vec<i32> {
    raw.split_whitespace()
        .filter_map(|token| token.parse::<i32>().ok())
        .collect()
}
