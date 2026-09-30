//! Game-event catalog models.

mod event_state;
mod reports;
mod spawn_indexes;

pub use event_state::{
    GameEventActiveSetLikeCpp, GameEventCheckOutcomeLikeCpp, GameEventConditionApplyOutcomeLikeCpp,
    GameEventConditionCheckOutcomeLikeCpp, GameEventConditionCheckSummaryLikeCpp,
    GameEventConditionLikeCpp, GameEventConditionProgressOutcomeLikeCpp,
    GameEventConditionProgressSummaryLikeCpp, GameEventConditionSaveApplyOutcomeLikeCpp,
    GameEventDataLikeCpp, GameEventDataStoreLikeCpp, GameEventHolidayActiveOutcomeLikeCpp,
    GameEventNextCheckOutcomeLikeCpp, GameEventPrerequisiteInsertOutcomeLikeCpp,
    GameEventQuestCompleteOutcomeLikeCpp, GameEventQuestConditionRecordLikeCpp,
    GameEventStartOutcomeLikeCpp, GameEventStartSummaryLikeCpp, GameEventStateLikeCpp,
    GameEventStopOutcomeLikeCpp, GameEventStopSummaryLikeCpp, GameEventUpdateOutcomeLikeCpp,
    GameEventWorldNextPhaseFinishedLikeCpp, GameEventWorldStateSaveEvidenceLikeCpp,
    GameEventWorldStateUpdateEvidenceLikeCpp, GameEventWorldStateUpdateOutcomeLikeCpp,
    GameEventWorldStateUpdateSkipLikeCpp, GameEventWorldStateUpdateSourceLikeCpp,
    GameEventWorldStateValueSkipReasonLikeCpp,
};
pub(in crate::spawn_store_loader) use event_state::{
    GameEventConditionSaveRowLikeCpp, GameEventSizingLikeCpp,
};
pub use reports::{
    GameEventConditionLoadReportLikeCpp, GameEventConditionSaveLoadReportLikeCpp,
    GameEventDataLoadReportLikeCpp, GameEventModelEquipLoadReportLikeCpp,
    GameEventNpcFlagLoadReportLikeCpp, GameEventNpcVendorCacheUpdateSummaryLikeCpp,
    GameEventNpcVendorLoadReportLikeCpp, GameEventObjectGuidLoadReportLikeCpp,
    GameEventPoolLoadReportLikeCpp, GameEventPrerequisiteLoadReportLikeCpp,
    GameEventQuestConditionLoadReportLikeCpp, GameEventQuestRelationFamilyLoadReportLikeCpp,
    GameEventQuestRelationsLoadReportLikeCpp, GameEventSpawnGuidLoadReportLikeCpp,
};
pub use spawn_indexes::{
    GameEventModelEquipBaselineChangeSummaryLikeCpp,
    GameEventModelEquipBaselineRecordOutcomeLikeCpp, GameEventModelEquipLikeCpp,
    GameEventModelEquipRecordLikeCpp, GameEventNpcFlagRecordLikeCpp, GameEventNpcFlagsLikeCpp,
    GameEventNpcVendorRecordLikeCpp, GameEventNpcVendorsLikeCpp, GameEventPoolIdsLikeCpp,
    GameEventQuestRelationCacheUpdateSummaryLikeCpp, GameEventQuestRelationRecordLikeCpp,
    GameEventQuestRelationsLikeCpp, GameEventSpawnGuidsLikeCpp,
};
