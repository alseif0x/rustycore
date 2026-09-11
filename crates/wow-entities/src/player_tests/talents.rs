//! #752 — the canonical Player owns its talent, glyph and specialization state.
//!
//! C++ `Player` holds `_talents[MAX_SPECIALIZATIONS]` and `_specializationInfo`
//! and performs every transition itself: `AddTalent` (`Player.cpp:2644`),
//! `SetGlyph` (`:25477`), `ActivateTalentGroup` (`:26894`), `ResetTalents`
//! (`:3505`) and the `_LoadTalents`/`_LoadGlyphs` hydration (`:26623`/`:26573`).
//! These pin the invariants that belong with the owner, independently of the
//! catalog-driven rules that stay in `wow-world`.

use super::*;

use crate::{
    PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP, PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP, PlayerTalentRuntimeState,
};

/// One hydrated runtime with a single known talent, reused by the persistence
/// round trip.
pub(super) fn hydrated_talent_runtime_like_cpp() -> PlayerTalentRuntimeState {
    let mut runtime = PlayerTalentRuntimeState::default();
    runtime.add_talent_like_cpp(0, 42, 1);
    runtime.mark_talents_loaded_like_cpp();
    runtime.mark_glyphs_loaded_like_cpp();
    runtime
}

#[test]
fn a_fresh_runtime_is_unhydrated_and_on_the_first_group_like_cpp() {
    let runtime = PlayerTalentRuntimeState::default();

    assert!(!runtime.talents_loaded_like_cpp());
    assert!(!runtime.glyphs_loaded_like_cpp());
    assert_eq!(runtime.active_group_like_cpp(), 0);
    assert_eq!(runtime.bonus_groups_like_cpp(), 0);
    assert_eq!(runtime.reset_talents_cost_like_cpp(), 0);
    assert_eq!(runtime.reset_talents_time_secs_like_cpp(), 0);
}

#[test]
fn the_active_group_never_leaves_the_cpp_specialization_range() {
    let mut runtime = PlayerTalentRuntimeState::default();
    let last = (PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP - 1) as u8;

    assert_eq!(runtime.set_active_group_like_cpp(1), 1);
    assert_eq!(runtime.active_group_like_cpp(), 1);

    // C++ never addresses a group beyond MAX_SPECIALIZATIONS.
    assert_eq!(runtime.set_active_group_like_cpp(200), last);
    assert_eq!(runtime.active_group_like_cpp(), last);

    assert_eq!(runtime.set_bonus_groups_like_cpp(200), last);
    assert_eq!(runtime.bonus_groups_like_cpp(), last);
}

#[test]
fn adding_a_talent_stores_the_rank_in_its_own_group_like_cpp() {
    let mut runtime = PlayerTalentRuntimeState::default();

    assert!(runtime.add_talent_like_cpp(0, 42, 2));
    assert!(runtime.add_talent_like_cpp(1, 42, 4));

    assert_eq!(runtime.talent_group_like_cpp(0).unwrap().get(&42), Some(&2));
    assert_eq!(runtime.talent_group_like_cpp(1).unwrap().get(&42), Some(&4));

    // C++ `AddTalent` overwrites the stored rank for an already known talent.
    assert!(runtime.add_talent_like_cpp(0, 42, 3));
    assert_eq!(runtime.talent_group_like_cpp(0).unwrap().get(&42), Some(&3));
}

#[test]
fn a_group_outside_the_range_is_refused_rather_than_panicking() {
    let mut runtime = PlayerTalentRuntimeState::default();
    let out_of_range = PLAYER_MAX_SPECIALIZATIONS_LIKE_CPP as u8;

    assert!(!runtime.add_talent_like_cpp(out_of_range, 42, 1));
    assert!(!runtime.set_glyph_like_cpp(out_of_range, 0, 700));
    assert!(runtime.talent_group_like_cpp(out_of_range).is_none());
    assert!(runtime.glyph_like_cpp(out_of_range, 0).is_none());
}

#[test]
fn a_glyph_slot_outside_the_range_is_refused_like_cpp() {
    let mut runtime = PlayerTalentRuntimeState::default();
    let out_of_range = PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP as u8;

    assert!(runtime.set_glyph_like_cpp(0, 0, 700));
    assert_eq!(runtime.glyph_like_cpp(0, 0), Some(700));

    assert!(!runtime.set_glyph_like_cpp(0, out_of_range, 800));
    assert!(runtime.glyph_like_cpp(0, out_of_range).is_none());
}

#[test]
fn taking_a_group_requires_hydrated_talents_like_cpp() {
    let mut runtime = PlayerTalentRuntimeState::default();
    assert!(runtime.add_talent_like_cpp(0, 42, 1));

    // Unhydrated: an empty group must not be reported as authoritative.
    assert!(runtime.take_talent_group_like_cpp(0).is_none());
    assert_eq!(runtime.talent_group_like_cpp(0).unwrap().len(), 1);

    runtime.mark_talents_loaded_like_cpp();
    let taken = runtime
        .take_talent_group_like_cpp(0)
        .expect("hydrated group");
    assert_eq!(taken.get(&42), Some(&1));
    assert!(runtime.talent_group_like_cpp(0).unwrap().is_empty());
    assert!(runtime.take_talent_group_like_cpp(9).is_none());
}

#[test]
fn clearing_talents_empties_every_group_and_returns_to_unhydrated_like_cpp() {
    let mut runtime = PlayerTalentRuntimeState::default();
    runtime.mark_talents_loaded_like_cpp();
    assert!(runtime.add_talent_like_cpp(0, 42, 1));
    assert!(runtime.add_talent_like_cpp(2, 43, 3));

    runtime.clear_talents_like_cpp();

    assert!(!runtime.talents_loaded_like_cpp());
    assert!(
        runtime
            .talent_groups_like_cpp()
            .all(|group| group.is_empty())
    );
}

#[test]
fn clearing_glyphs_empties_every_slot_and_returns_to_unhydrated_like_cpp() {
    let mut runtime = PlayerTalentRuntimeState::default();
    runtime.mark_glyphs_loaded_like_cpp();
    assert!(runtime.set_glyph_like_cpp(1, 2, 700));

    runtime.clear_glyphs_like_cpp();

    assert!(!runtime.glyphs_loaded_like_cpp());
    assert!(
        runtime
            .glyph_groups_like_cpp()
            .all(|slots| slots.iter().all(|glyph| *glyph == 0))
    );
}

#[test]
fn a_completed_reset_installs_its_groups_and_records_cost_and_time_like_cpp() {
    let mut runtime = PlayerTalentRuntimeState::default();
    runtime.mark_talents_loaded_like_cpp();
    assert!(runtime.add_talent_like_cpp(0, 42, 1));

    let mut post = runtime.talent_groups_snapshot_like_cpp();
    post[0].clear();
    runtime.replace_talent_groups_like_cpp(post);
    runtime.set_reset_talents_state_like_cpp(100_000, 12_345);

    assert!(runtime.talent_group_like_cpp(0).unwrap().is_empty());
    // A reset keeps the hydration flag: the groups are authoritative and empty.
    assert!(runtime.talents_loaded_like_cpp());
    assert_eq!(runtime.reset_talents_cost_like_cpp(), 100_000);
    assert_eq!(runtime.reset_talents_time_secs_like_cpp(), 12_345);
}

#[test]
fn the_player_owns_the_runtime_for_its_whole_lifetime_like_cpp() {
    let mut player = Player::new(Some(1), false);

    player
        .gameplay_state_mut()
        .talents
        .set_reset_talents_state_like_cpp(500_000, 98_765);
    assert!(
        player
            .gameplay_state_mut()
            .talents
            .add_talent_like_cpp(1, 77, 5)
    );

    let runtime = player.talent_runtime_like_cpp();
    assert_eq!(runtime.reset_talents_cost_like_cpp(), 500_000);
    assert_eq!(runtime.reset_talents_time_secs_like_cpp(), 98_765);
    assert_eq!(runtime.talent_group_like_cpp(1).unwrap().get(&77), Some(&5));
}
