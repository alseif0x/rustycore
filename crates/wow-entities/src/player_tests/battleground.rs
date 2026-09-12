//! #783 — the canonical Player owns its battleground state.
//!
//! C++ keeps this in `Player::m_bgData` (`Player.h:2821`, `BGData` at `:976`)
//! and reads it back through `InBattleground` (`:2335`), `GetBattlegroundId`
//! (`:2337`) and `GetBattlegroundTypeId` (`:2338`), with
//! `Player::SetArenaTeamIdInvited` (`:1956`) beside it.

use crate::{
    PlayerBattlegroundQueueSlotLikeCpp, PlayerBattlegroundQueueTypeIdLikeCpp,
    PlayerBattlegroundState,
};

fn queue_type() -> PlayerBattlegroundQueueTypeIdLikeCpp {
    PlayerBattlegroundQueueTypeIdLikeCpp {
        battlemaster_list_id: 1,
        queue_type: 0,
        rated: false,
        team_size: 0,
    }
}

fn slot(slot: u32, invited_instance_guid: u32) -> PlayerBattlegroundQueueSlotLikeCpp {
    PlayerBattlegroundQueueSlotLikeCpp {
        slot,
        queue_type_id: queue_type(),
        invited_instance_guid,
    }
}

#[test]
fn a_fresh_battleground_state_has_no_battleground_and_no_queue() {
    let state = PlayerBattlegroundState::default();

    assert!(!state.in_battleground_like_cpp());
    assert_eq!(state.battleground_type_id_like_cpp(), None);
    assert_eq!(state.battleground_map_id_like_cpp(), None);
    assert_eq!(state.battleground_status_like_cpp(), None);
    assert!(state.has_no_queue_slot_like_cpp());
    assert_eq!(state.arena_team_id_invited_like_cpp(), 0);
}

#[test]
fn a_zero_type_is_the_cpp_battleground_type_none() {
    let mut state = PlayerBattlegroundState::default();
    state.set_battleground_type_id_like_cpp(7);
    assert!(state.in_battleground_like_cpp());

    state.set_battleground_type_id_like_cpp(0);

    assert!(!state.in_battleground_like_cpp());
    assert_eq!(state.battleground_type_id_like_cpp(), None);
}

#[test]
fn the_entry_context_records_the_type_and_its_map_together() {
    let mut state = PlayerBattlegroundState::default();

    state.set_battleground_context_like_cpp(7, 30);

    assert_eq!(state.battleground_type_id_like_cpp(), Some(7));
    assert_eq!(state.battleground_map_id_like_cpp(), Some(30));
}

#[test]
fn a_zero_map_clears_the_recorded_battleground_map() {
    let mut state = PlayerBattlegroundState::default();
    state.set_battleground_context_like_cpp(7, 30);

    state.set_battleground_context_like_cpp(7, 0);

    assert_eq!(state.battleground_type_id_like_cpp(), Some(7));
    assert_eq!(state.battleground_map_id_like_cpp(), None);
}

#[test]
fn one_slot_holds_one_queue_like_cpp() {
    let mut state = PlayerBattlegroundState::default();

    state.install_queue_slot_like_cpp(slot(2, 0));
    state.install_queue_slot_like_cpp(slot(2, 99));

    assert_eq!(state.queue_slots_like_cpp().len(), 1);
    assert_eq!(state.queue_slots_like_cpp()[0].invited_instance_guid, 99);
}

#[test]
fn different_slots_are_kept_side_by_side_like_cpp() {
    let mut state = PlayerBattlegroundState::default();

    state.install_queue_slot_like_cpp(slot(0, 0));
    state.install_queue_slot_like_cpp(slot(2, 99));

    assert_eq!(state.queue_slots_like_cpp().len(), 2);
    assert!(!state.has_no_queue_slot_like_cpp());
    assert_eq!(
        state
            .queue_slots_like_cpp()
            .iter()
            .map(|queued| queued.slot)
            .collect::<Vec<_>>(),
        [0, 2]
    );
}

#[test]
fn the_arena_invitation_is_independent_of_the_battleground_like_cpp() {
    let mut state = PlayerBattlegroundState::default();
    state.set_battleground_context_like_cpp(7, 30);

    state.set_arena_team_id_invited_like_cpp(100);
    state.set_battleground_type_id_like_cpp(0);

    assert_eq!(state.arena_team_id_invited_like_cpp(), 100);
}

#[test]
fn the_status_is_recorded_on_its_own() {
    let mut state = PlayerBattlegroundState::default();

    state.set_battleground_status_like_cpp(Some(3));
    assert_eq!(state.battleground_status_like_cpp(), Some(3));

    state.set_battleground_status_like_cpp(None);
    assert_eq!(state.battleground_status_like_cpp(), None);
    assert!(!state.in_battleground_like_cpp());
}

#[test]
fn the_represented_parts_rebuild_the_state_the_mirror_holds() {
    let state = PlayerBattlegroundState::from_represented_parts_like_cpp(
        Some(7),
        Some(30),
        Some(3),
        vec![slot(2, 99)],
        100,
    );

    assert!(state.in_battleground_like_cpp());
    assert_eq!(state.battleground_type_id_like_cpp(), Some(7));
    assert_eq!(state.battleground_map_id_like_cpp(), Some(30));
    assert_eq!(state.battleground_status_like_cpp(), Some(3));
    assert_eq!(state.queue_slots_like_cpp().len(), 1);
    assert_eq!(state.arena_team_id_invited_like_cpp(), 100);
}
