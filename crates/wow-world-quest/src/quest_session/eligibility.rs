// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest availability: status classification and the `SatisfyQuest*` gates,
//! moved from the World `handlers/quest/eligibility.rs` and
//! `handlers/quest/mod.rs` (#1363 slice 6b).

use super::*;

pub fn get_represented_quest_giver_status_with_catalog_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest_info: Option<&wow_data::progression_rewards::QuestInfoStore>,
    source: RepresentedQuestGiverStatusSourceLikeCpp,
) -> u64 {
    let player = host.core().quest_eligibility_access_like_cpp(
        #[cfg(any(test, feature = "test-fixtures"))]
        &host.hub_ref().fixtures.identity.player_race,
        #[cfg(any(test, feature = "test-fixtures"))]
        &host.hub_ref().fixtures.identity.player_class,
        #[cfg(any(test, feature = "test-fixtures"))]
        &host.hub_ref().fixtures.identity.player_level,
        #[cfg(any(test, feature = "test-fixtures"))]
        &host
            .hub_ref()
            .fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_records_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &host
            .hub_ref()
            .fixtures
            .progression
            .reputation_state_like_cpp,
    );
    let conditions = host.player_condition_projection_cx_like_cpp();
    crate::QuestEligibilityCx::new(
        player,
        host.quest_state(),
        host.hub_ref().catalogs,
        &conditions,
        host.world_test_consumer_like_cpp(),
    )
    .get_represented_quest_giver_status_with_catalog_like_cpp(quest_info, source)
}

pub fn represented_quest_is_important_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    represented_quest_dialog_classification_like_cpp(
        quest,
        host.hub_ref().catalogs.quests.info_store.as_deref(),
    )
    .is_important()
}

/// C++ anchor: `Player::CanTakeQuest` (Player.cpp:14090–14102).
/// This keeps the current represented Rust gate order; `SatisfyQuestTimed`
/// remains unrepresented, so this is not a full-parity claim.
pub fn can_take_quest<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    with_quest_eligibility_cx_like_cpp(host, |operation| operation.can_take_quest_like_cpp(quest))
}

/// C++ anchor: `Player::CanSeeStartQuest` (Player.cpp:14073–14085).
///
/// `HandleQuestgiverCompleteQuest` asks this before it answers a dialog
/// request, so the represented bounded projection is exposed here rather
/// than duplicated in the handler.
pub fn can_see_start_quest_represented_bounded_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    with_quest_eligibility_cx_like_cpp(host, |operation| {
        operation.can_see_start_quest_like_cpp(quest)
    })
}

/// Build the represented eligibility operation over the canonical Player
/// access, the quest state and the condition projection.
pub(crate) fn with_quest_eligibility_cx_like_cpp<R, H: QuestSessionHostLikeCpp>(
    host: &H,
    run: impl FnOnce(&crate::QuestEligibilityCx<'_>) -> R,
) -> R {
    let player = host.core().quest_eligibility_access_like_cpp(
        #[cfg(any(test, feature = "test-fixtures"))]
        &host.hub_ref().fixtures.identity.player_race,
        #[cfg(any(test, feature = "test-fixtures"))]
        &host.hub_ref().fixtures.identity.player_class,
        #[cfg(any(test, feature = "test-fixtures"))]
        &host.hub_ref().fixtures.identity.player_level,
        #[cfg(any(test, feature = "test-fixtures"))]
        &host
            .hub_ref()
            .fixtures
            .progression
            .player_skill_test_fixture_like_cpp
            .player_skill_records_like_cpp,
        #[cfg(any(test, feature = "test-fixtures"))]
        &host
            .hub_ref()
            .fixtures
            .progression
            .reputation_state_like_cpp,
    );
    let conditions = host.player_condition_projection_cx_like_cpp();
    run(&crate::QuestEligibilityCx::new(
        player,
        host.quest_state(),
        host.hub_ref().catalogs,
        &conditions,
        host.world_test_consumer_like_cpp(),
    ))
}

pub fn is_quest_disabled_like_cpp<H: QuestSessionHostLikeCpp>(host: &H, quest_id: u32) -> bool {
    crate::quest_is_disabled_like_cpp(host.hub_ref(), quest_id)
}

pub(crate) fn represented_quest_dialog_classification_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
    quest_info: Option<&wow_data::progression_rewards::QuestInfoStore>,
) -> crate::QuestDialogClassificationLikeCpp {
    crate::QuestDialogClassificationLikeCpp::new(
        quest.flags,
        quest.flags_ex,
        quest_info.and_then(|store| store.get(quest.quest_info_id as u32)),
    )
}
