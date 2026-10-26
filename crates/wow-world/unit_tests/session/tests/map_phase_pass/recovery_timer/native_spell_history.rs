use super::*;
use std::collections::VecDeque;
use std::sync::Arc;
use wow_entities::{SpellChargeState, SpellCooldown, SpellHistory};

const UPDATE_TIME_MS: u64 = 50_000;
const UPDATE_TIME_NS: u128 = 50_000_000_000;
// C++ represents on-hold deadlines with +MONTH (`SpellHistory.cpp:35,424-428`).
const ON_HOLD_DELAY_MS: u64 = 30 * 24 * 60 * 60 * 1_000;

fn history_for_player(session: &WorldSession, handle: wow_map::PlayerHandle) -> SpellHistory {
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .clone();
    manager
        .lock()
        .expect("canonical MapManager lock")
        .with_player_like_cpp(handle, |player| {
            player.unit().subsystems().spells.history.clone()
        })
        .expect("canonical Player history")
}

fn replace_player_history(
    session: &WorldSession,
    handle: wow_map::PlayerHandle,
    history: SpellHistory,
) {
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .clone();
    manager
        .lock()
        .expect("canonical MapManager lock")
        .with_player_mut_like_cpp(handle, |player| {
            player.unit_mut().subsystems_mut().spells.history = history;
        })
        .expect("canonical Player history");
}

fn expired_reincarnation_history(unix_time_ms: u64) -> SpellHistory {
    let mut history = SpellHistory::default();
    history.cooldowns.insert(
        21_169,
        SpellCooldown {
            spell_id: 21_169,
            item_id: 0,
            cooldown_end_ms: unix_time_ms - 1,
            category_id: 91,
            category_end_ms: unix_time_ms - 1,
            on_hold: false,
        },
    );
    history.category_cooldowns.insert(91, 21_169);
    history
}

fn reset_history(session: &mut WorldSession) {
    let (state, mut hub) = crate::session::split_spell_state_mut(session);
    state.reset_represented_character_spell_cooldowns_like_cpp(&mut hub);
}

#[tokio::test]
async fn admitted_player_update_maintains_loaded_native_history_and_reset_index() {
    let (mut session, admission, _, _) = logged_in_recovery_session_at(UPDATE_TIME_NS);
    let handle = admission.handle;

    session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
    session.record_loaded_character_spell_cooldown_like_cpp(21_169, 0, 49, 91, 49);
    session.record_loaded_character_spell_cooldown_like_cpp(700, 0, 50, 92, 50);
    session.record_loaded_character_spell_cooldown_like_cpp(701, 0, 51, 0, 0);
    session.record_loaded_character_spell_cooldown_like_cpp(704, 0, 60, 97, 60);
    session.record_loaded_character_spell_cooldown_like_cpp(704, 0, 51, 97, 51);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let history = &mut player.unit_mut().subsystems_mut().spells.history;
            history.cooldowns.insert(
                702,
                SpellCooldown {
                    spell_id: 702,
                    item_id: 0,
                    cooldown_end_ms: UPDATE_TIME_MS + ON_HOLD_DELAY_MS,
                    category_id: 93,
                    category_end_ms: UPDATE_TIME_MS + ON_HOLD_DELAY_MS,
                    on_hold: true,
                },
            );
            history.category_cooldowns.insert(93, 702);
            history.charges.insert(
                94,
                VecDeque::from([
                    SpellChargeState {
                        recharge_start_ms: UPDATE_TIME_MS - 2,
                        recharge_end_ms: UPDATE_TIME_MS - 1,
                    },
                    SpellChargeState {
                        recharge_start_ms: UPDATE_TIME_MS - 1,
                        recharge_end_ms: UPDATE_TIME_MS,
                    },
                ]),
            );
            history.charges.insert(
                95,
                VecDeque::from([SpellChargeState {
                    recharge_start_ms: UPDATE_TIME_MS,
                    recharge_end_ms: UPDATE_TIME_MS + 1,
                }]),
            );
        })
        .expect("canonical Player history");

    let before = history_for_player(&session, handle);
    assert_eq!(before.category_cooldowns.get(&91), Some(&21_169));
    assert_eq!(before.category_cooldowns.get(&92), Some(&700));
    assert!(!before.category_cooldowns.contains_key(&0));
    assert_eq!(before.cooldowns[&704].cooldown_end_ms, 51_000);
    assert_eq!(before.category_cooldowns.get(&97), Some(&704));

    let reply = run_recovery_request(
        &mut session,
        admission,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;
    assert_eq!(reply.status, Some(PlayerRecoveryStatus::Completed));

    let after = history_for_player(&session, handle);
    assert!(!after.cooldowns.contains_key(&21_169));
    assert!(!after.category_cooldowns.contains_key(&91));
    assert!(after.cooldowns.contains_key(&700));
    assert_eq!(after.category_cooldowns.get(&92), Some(&700));
    assert!(after.cooldowns.contains_key(&701));
    assert!(!after.category_cooldowns.contains_key(&0));
    assert!(after.cooldowns[&702].on_hold);
    assert_eq!(
        after.cooldowns[&702].cooldown_end_ms,
        UPDATE_TIME_MS + ON_HOLD_DELAY_MS
    );
    assert_eq!(
        after.cooldowns[&702].category_end_ms,
        UPDATE_TIME_MS + ON_HOLD_DELAY_MS
    );
    assert!(after.category_cooldowns.contains_key(&93));
    assert!(after.charges[&94].is_empty());
    assert_eq!(after.charges[&95].len(), 1);

    session.mark_represented_character_spell_cooldowns_loaded_like_cpp();
    session.record_loaded_character_spell_cooldown_like_cpp(703, 0, 55, 96, 55);
    assert_eq!(
        history_for_player(&session, handle)
            .category_cooldowns
            .get(&96),
        Some(&703)
    );
    reset_history(&mut session);
    let reset = history_for_player(&session, handle);
    assert!(reset.cooldowns.is_empty());
    assert!(reset.category_cooldowns.is_empty());
    assert!(!reset.cooldowns_loaded);
}

#[tokio::test]
async fn admitted_player_update_expires_loaded_cooldown_one_nanosecond_after_ms_deadline() {
    let (mut session, admission, _, _) = logged_in_recovery_session_at(UPDATE_TIME_NS + 1);
    let handle = admission.handle;
    session.record_loaded_character_spell_cooldown_like_cpp(21_169, 0, 50, 98, 50);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .spells
                .history
                .charges
                .insert(
                    99,
                    VecDeque::from([SpellChargeState {
                        recharge_start_ms: UPDATE_TIME_MS,
                        recharge_end_ms: UPDATE_TIME_MS + 1,
                    }]),
                );
        })
        .expect("canonical Player history");

    let reply = run_recovery_request(
        &mut session,
        admission,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;
    assert_eq!(reply.status, Some(PlayerRecoveryStatus::Completed));

    let after = history_for_player(&session, handle);
    assert!(!after.cooldowns.contains_key(&21_169));
    assert!(!after.category_cooldowns.contains_key(&98));
    assert_eq!(after.charges[&99].len(), 1);
}

#[tokio::test]
async fn history_update_remains_applied_when_later_selected_aura_work_refuses() {
    let (mut session, admission, _, _) = logged_in_recovery_session_at(UPDATE_TIME_NS);
    let sickness_spell = super::selected_recovery::install_selected_sickness_snapshot(&mut session);
    session.set_chr_races_store(Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([]),
    ));
    session.record_loaded_character_spell_cooldown_like_cpp(21_169, 0, 49, 91, 49);

    let reply = run_recovery_request(
        &mut session,
        admission,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;

    assert_eq!(reply.status, Some(PlayerRecoveryStatus::Unavailable));
    let history = history_for_player(&session, admission.handle);
    assert!(!history.cooldowns.contains_key(&21_169));
    assert!(!history.category_cooldowns.contains_key(&91));
    assert_eq!(death_timer(&session), 300_000);
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .values()
                    .find(|aura| aura.spell_id == sickness_spell)
                    .expect("selected sickness application remains present")
                    .duration_remaining
            })
            .expect("canonical Player remains available"),
        120_000,
    );
}

#[tokio::test]
async fn stale_map_incarnation_does_not_maintain_native_history() {
    let (mut session, admission, _, _) = logged_in_recovery_session_at(UPDATE_TIME_NS);
    let initial = expired_reincarnation_history(UPDATE_TIME_MS);
    replace_player_history(&session, admission.handle, initial.clone());
    let stale = PlayerRecoveryAdmission {
        map_incarnation: admission.map_incarnation + 1,
        ..admission
    };

    let reply = run_recovery_request(
        &mut session,
        stale,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;

    assert_eq!(
        reply.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(history_for_player(&session, admission.handle), initial);
}

#[tokio::test]
async fn stale_residence_does_not_maintain_native_history() {
    let (mut session, admission, _, _) = logged_in_recovery_session_at(UPDATE_TIME_NS);
    let initial = expired_reincarnation_history(UPDATE_TIME_MS);
    replace_player_history(&session, admission.handle, initial.clone());
    let stale = PlayerRecoveryAdmission {
        residence_revision: admission.residence_revision + 1,
        ..admission
    };

    let reply = run_recovery_request(
        &mut session,
        stale,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;

    assert_eq!(
        reply.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(history_for_player(&session, admission.handle), initial);
}

#[tokio::test]
async fn stale_handle_does_not_maintain_reinstalled_player_history() {
    let (mut session, admission, _, _) = logged_in_recovery_session_at(UPDATE_TIME_NS);
    let initial = expired_reincarnation_history(UPDATE_TIME_MS);
    replace_player_history(&session, admission.handle, initial.clone());
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .expect("canonical MapManager")
        .clone();
    let replacement_handle = {
        let mut manager = manager.lock().expect("canonical MapManager lock");
        let player = manager
            .retire_player_like_cpp(admission.handle)
            .expect("the admitted Player can be reinstalled");
        let replacement = manager
            .install_detached_player_like_cpp(player)
            .expect("the Player receives a new owner generation");
        manager
            .attach_player_like_cpp(
                replacement,
                admission.map_key,
                wow_core::Position::new(1.0, 2.0, 3.0, 0.5),
            )
            .expect("the replacement generation returns to its admitted Map");
        replacement
    };

    let reply = run_recovery_request(
        &mut session,
        admission,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;

    assert_eq!(
        reply.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(history_for_player(&session, replacement_handle), initial);
}

#[tokio::test]
async fn replay_with_a_later_timestamp_does_not_expire_native_history() {
    let (mut session, admission, _, _) = logged_in_recovery_session_at(UPDATE_TIME_NS);
    session.record_loaded_character_spell_cooldown_like_cpp(21_169, 0, 50, 91, 50);

    let first = run_recovery_request(
        &mut session,
        admission,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;
    assert_eq!(first.status, Some(PlayerRecoveryStatus::Completed));
    assert!(
        history_for_player(&session, admission.handle)
            .cooldowns
            .contains_key(&21_169)
    );

    let replay = PlayerRecoveryAdmission {
        unix_time_ns: UPDATE_TIME_NS + 1_000_000_000,
        ..admission
    };
    let reply = run_recovery_request(
        &mut session,
        replay,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;

    assert_eq!(
        reply.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert!(
        history_for_player(&session, admission.handle)
            .cooldowns
            .contains_key(&21_169)
    );
}
