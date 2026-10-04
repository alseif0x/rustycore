// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical quest-completion state transition used by objective progress.

use wow_world_core::session::QuestObjectiveAccessLikeCpp;
use wow_world_loot::LootState;

use super::{
    RepresentedQuestCompleteStatusUpdateLikeCpp, SessionQuestState,
    objective_progress::invalidate_player_quest_status_authority_like_cpp,
};

pub(crate) fn complete_represented_quest_status_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    quest_state: &mut SessionQuestState,
    quest_id: u32,
    tracking_event_auto_reward: bool,
    world_test_consumer: bool,
) -> bool {
    invalidate_player_quest_status_authority_like_cpp(owner, quest_state, world_test_consumer);
    #[cfg(any(test, feature = "test-fixtures"))]
    let fixture_owner = world_test_consumer && owner.owner_handle_absent_like_cpp();

    #[cfg(any(test, feature = "test-fixtures"))]
    let canonical = if fixture_owner {
        let mut state = quest_state.player_quest_gameplay_fixture_like_cpp();
        let completed = state
            .status_mut_like_cpp(quest_id)
            .and_then(|status| {
                (status.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP).then(|| {
                    let old_status = status.status;
                    status.status = wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP;
                    old_status
                })
            });
        quest_state.apply_player_quest_gameplay_fixture_like_cpp(state);
        Some(completed)
    } else {
        owner.complete_quest_status_like_cpp(
            quest_id,
            wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
        )
    };

    #[cfg(not(any(test, feature = "test-fixtures")))]
    let canonical = owner.complete_quest_status_like_cpp(
        quest_id,
        wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
    );

    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer
        && !fixture_owner
        && canonical.is_some()
        && let Some(state) = owner.player_quest_gameplay_snapshot_like_cpp()
    {
        quest_state.apply_player_quest_core_compatibility_like_cpp(&state);
    }

    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = world_test_consumer;

    let Some(Some(old_status)) = canonical else {
        return false;
    };

    quest_state.record_represented_quest_complete_status_update_like_cpp(
        RepresentedQuestCompleteStatusUpdateLikeCpp {
            quest_id,
            old_status,
            new_status: wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            send_quest_update_called: true,
            quest_slot_state_complete_represented: true,
            quest_slot_state_live_update_unrepresented: true,
            visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
            spell_area_runtime_unrepresented: true,
            tracking_event_auto_reward_unrepresented: tracking_event_auto_reward,
            quest_tracker_complete_time_unrepresented: true,
            script_status_change_unrepresented: true,
        },
    );
    true
}

/// Finish the original quest-completion registry publication sequence.
pub(crate) fn sync_quest_completion_registry_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    loot: &LootState,
    #[cfg(any(test, feature = "test-fixtures"))] spell_state: &wow_world_spell::SessionSpellState,
    #[cfg(any(test, feature = "test-fixtures"))] quest_state: &SessionQuestState,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_vehicle_and_pet: (
        &Option<wow_entities::Vehicle>,
        &Option<i32>,
        &Option<u32>,
        &Option<wow_core::ObjectGuid>,
    ),
    #[cfg(any(test, feature = "test-fixtures"))] fixture_position: &Option<wow_core::Position>,
    #[cfg(any(test, feature = "test-fixtures"))] fixture_health: &u32,
    #[cfg(any(test, feature = "test-fixtures"))] fixture_max_health: &u32,
    #[cfg(any(test, feature = "test-fixtures"))] fixture_alive: &bool,
    #[cfg(any(test, feature = "test-fixtures"))] fixture_level: &u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_transport: &Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))] world_test_consumer: bool,
) {
    let Some((position, control)) = owner.player_registry_sync_participants_like_cpp(
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_position,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_level,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_transport,
    ) else {
        return;
    };

    let sync = crate::PlayerRegistrySyncContext::new(
        position,
        control,
        loot,
        #[cfg(any(test, feature = "test-fixtures"))]
        wow_world_core::session::RegistrySyncInputs::new_like_cpp(
            fixture_health,
            fixture_max_health,
            fixture_alive,
        ),
    );
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer {
        let sync = sync.with_fixture_hydration(crate::PlayerRegistryHydrationContext::new(
            owner.player_registry_hydration_access_like_cpp(),
            spell_state,
            quest_state,
            fixture_vehicle_and_pet,
            true,
        ));
        sync.sync();
        return;
    }
    sync.sync();
}
