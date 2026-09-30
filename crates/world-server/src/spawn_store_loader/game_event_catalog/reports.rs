//! Load reports for game-event catalog data.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventDataLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_reserved_zero: usize,
    pub skipped_out_of_range: usize,
    pub invalid_normal_zero_length: usize,
    pub holiday_validation_deferred: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventPrerequisiteLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range_event: usize,
    pub skipped_non_world_event: usize,
    pub skipped_out_of_range_prerequisite: usize,
    pub duplicate_ignored: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventConditionLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventConditionSaveLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range_event: usize,
    pub skipped_missing_condition: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestConditionLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range_event: usize,
    pub overwrites: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventPoolLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
    pub skipped_broken_pool: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventObjectGuidLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_missing_spawn_metadata: usize,
    pub skipped_out_of_range: usize,
    pub pooled_still_loaded: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventSpawnGuidLoadReportLikeCpp {
    pub creature: GameEventObjectGuidLoadReportLikeCpp,
    pub gameobject: GameEventObjectGuidLoadReportLikeCpp,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventModelEquipLoadReportLikeCpp {
    pub equipment_rows: usize,
    pub equipment_ids_loaded: usize,
    pub rows: usize,
    pub loaded: usize,
    pub invalid_event_id: usize,
    pub missing_equipment_template: usize,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestRelationFamilyLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
    pub events_touched: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventQuestRelationsLoadReportLikeCpp {
    pub creature: GameEventQuestRelationFamilyLoadReportLikeCpp,
    pub gameobject: GameEventQuestRelationFamilyLoadReportLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcFlagLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
    pub events_touched: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcVendorLoadReportLikeCpp {
    pub rows: usize,
    pub loaded: usize,
    pub skipped_out_of_range: usize,
    pub skipped_missing_creature_spawn_metadata: usize,
    pub validation_deferred: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameEventNpcVendorCacheUpdateSummaryLikeCpp {
    pub event_id: u16,
    pub activate: bool,
    pub missing_event_bucket: bool,
    pub records_seen: usize,
    pub items_added: usize,
    pub items_removed: usize,
    pub remove_misses: usize,
    pub no_match: usize,
}
