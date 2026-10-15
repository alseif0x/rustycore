// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest dialog: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

pub(in crate::session) fn quest_has_represented_item_objective_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    wow_world_quest::represented_quest_has_item_objective_like_cpp(quest)
}

pub(crate) use wow_world_instances::RepresentedAdventureMapStartQuestLikeCpp;
pub(crate) use wow_world_quest::{
    RepresentedPendingQuestSharingLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedQuestCompleteStatusUpdateLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp, RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestPushResultResponseLikeCpp,
};

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_quest::{
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_social::RepresentedForceDeselectLikeCpp;

pub(crate) use wow_world_quest::RepresentedQuestRewardReputationSourceLikeCpp;
