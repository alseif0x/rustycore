//! Forward existing quest reputation and controller-owner operations.

pub fn record_quest_reward_reputation_for_test(
    session: &mut crate::session::WorldSession,
    quest: &wow_data::quest::QuestTemplate,
) {
    session.record_represented_quest_reward_reputation_like_cpp(quest);
}

/// Opt in to the original detached unit-fixture attachment sequence.
pub fn attach_player_controller_for_test(
    session: &mut crate::session::WorldSession,
    guid: wow_core::ObjectGuid,
    name: String,
    position: wow_core::Position,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    session.attach_player_controller_for_fixture(crate::session::SessionPlayerController::new(
        guid, name, position, map_id, race, class, level, gender,
    ));
}

pub fn ensure_world_map_for_current_player_for_test(
    session: &mut crate::session::WorldSession,
) -> Option<wow_map::CreateMapDecision> {
    session.ensure_canonical_world_map_for_current_player_like_cpp()
}
/// Apply one mutation through the existing canonical reputation access.
pub fn mutate_quest_reputation_for_test<R>(
    session: &mut crate::session::WorldSession,
    mutate: impl FnOnce(&mut wow_progression::ReputationMgrMutLikeCpp<'_>) -> R,
) -> Option<R> {
    session.mutate_reputation_mgr_like_cpp(mutate)
}
