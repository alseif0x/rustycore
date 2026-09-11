//! #754 — the canonical Player owns its spell runtime and its invariants.
//!
//! C++ `Player` holds `PlayerSpellMap m_spells` (`Player.h:2961`) and
//! `m_overrideSpells` (`:2962`) and performs every transition itself:
//! `AddSpell` (`Player.cpp:2741`), `LearnSpell` (`:3192`), `RemoveSpell`
//! (`:3236`), `AddOverrideSpell` (`:28581`) and `RemoveOverrideSpell`
//! (`:28586`), with `_LoadSpells` (`:18924`) and `_SaveSpells` (`:20399`) at
//! the persistence edges. These pin the invariants that belong with the owner.

use super::*;

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    PlayerKnownSpellRecord, PlayerSpellAcquisitionSnapshotLikeCpp, PlayerSpellLoadState,
    PlayerSpellRuntimeState,
};

fn row(spell_id: i32, state: PlayerSpellLoadState) -> PlayerKnownSpellRecord {
    PlayerKnownSpellRecord {
        spell_id,
        state,
        active: true,
        disabled: false,
        favorite: false,
        dependent: false,
    }
}

#[test]
fn a_fresh_runtime_is_unhydrated_and_empty_like_cpp() {
    let runtime = PlayerSpellRuntimeState::default();

    assert!(runtime.known_spells_like_cpp().is_empty());
    assert!(runtime.rows_like_cpp().is_empty());
    assert!(!runtime.rows_loaded_like_cpp());
    assert!(!runtime.rows_complete_like_cpp());
    assert!(!runtime.trait_definition_ids_complete_like_cpp());
    assert!(!runtime.override_spells_complete_like_cpp());
}

#[test]
fn an_authoritative_empty_load_is_not_an_unhydrated_owner_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();

    runtime.replace_rows_like_cpp(BTreeMap::new(), true);

    // C++ has no such flag because it always has a Player; RustyCore keeps the
    // distinction explicit so a consumer cannot read "no rows" as "not loaded".
    assert!(runtime.rows_loaded_like_cpp());
    assert!(runtime.rows_complete_like_cpp());
    assert!(runtime.rows_like_cpp().is_empty());

    runtime.clear_rows_like_cpp();
    assert!(!runtime.rows_loaded_like_cpp());
    assert!(!runtime.rows_complete_like_cpp());
}

#[test]
fn learning_a_spell_makes_it_known_once_and_clears_its_removal_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.mark_removed_like_cpp(10);

    runtime.learn_known_spell_id_unless_known_like_cpp(10);
    runtime.learn_known_spell_id_unless_known_like_cpp(10);

    assert_eq!(runtime.known_spells_like_cpp(), &[10]);
    assert!(!runtime.removed_known_spells_like_cpp().contains(&10));
}

#[test]
fn forgetting_a_known_spell_tracks_it_as_removed_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.learn_known_spell_id_unless_known_like_cpp(10);
    runtime.set_favorite_like_cpp(10, true);

    let forgotten = runtime.forget_known_spell_like_cpp(10);

    assert!(forgotten.was_known);
    assert!(!forgotten.was_dependent);
    assert!(runtime.known_spells_like_cpp().is_empty());
    assert!(!runtime.favorite_known_spells_like_cpp().contains(&10));
    assert!(runtime.removed_known_spells_like_cpp().contains(&10));
}

#[test]
fn forgetting_a_dependent_spell_is_not_tracked_as_removed_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.learn_known_spell_id_unless_known_like_cpp(10);
    runtime.set_dependent_like_cpp(10, true);

    let forgotten = runtime.forget_known_spell_like_cpp(10);

    // C++ deletes a dependent spell's row outright instead of tombstoning it,
    // so it must not enter the removed set.
    assert!(forgotten.was_known);
    assert!(forgotten.was_dependent);
    assert!(!runtime.removed_known_spells_like_cpp().contains(&10));
    assert!(!runtime.dependent_known_spells_like_cpp().contains(&10));
}

#[test]
fn forgetting_an_unknown_spell_changes_nothing_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();

    let forgotten = runtime.forget_known_spell_like_cpp(10);

    assert!(!forgotten.was_known);
    assert!(runtime.removed_known_spells_like_cpp().is_empty());
}

#[test]
fn marking_a_learned_spell_dependent_mirrors_onto_its_row_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();
    let mut known = row(10, PlayerSpellLoadState::Unchanged);
    known.favorite = true;
    runtime.replace_rows_like_cpp(BTreeMap::from([(10, known)]), true);
    runtime.set_favorite_like_cpp(10, true);

    runtime.mark_dependent_learned_spell_like_cpp(10);

    assert!(runtime.dependent_known_spells_like_cpp().contains(&10));
    assert!(!runtime.favorite_known_spells_like_cpp().contains(&10));
    let stored = runtime.rows_like_cpp().get(&10).expect("row");
    assert!(stored.dependent);
    assert!(!stored.favorite);
}

#[test]
fn a_dependent_mark_does_not_touch_rows_that_are_not_authoritative_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();
    let known = row(10, PlayerSpellLoadState::Unchanged);
    runtime.insert_row_like_cpp(10, known);

    runtime.mark_dependent_learned_spell_like_cpp(10);

    assert!(runtime.dependent_known_spells_like_cpp().contains(&10));
    // The rows are not an authoritative set here, so they are left alone.
    assert!(!runtime.rows_like_cpp().get(&10).expect("row").dependent);
}

#[test]
fn overrides_erase_their_entry_once_the_last_replacement_is_gone_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();

    runtime.add_override_spell_like_cpp(50, 10);
    runtime.add_override_spell_like_cpp(50, 20);
    assert_eq!(
        runtime.override_spells_like_cpp().get(&50),
        Some(&BTreeSet::from([10, 20]))
    );

    runtime.remove_override_spell_like_cpp(50, 10);
    assert_eq!(
        runtime.override_spells_like_cpp().get(&50),
        Some(&BTreeSet::from([20]))
    );

    // C++ `RemoveOverrideSpell` erases the whole entry with its last member.
    runtime.remove_override_spell_like_cpp(50, 20);
    assert!(runtime.override_spells_like_cpp().get(&50).is_none());

    // Removing an override that was never registered is a no-op.
    runtime.remove_override_spell_like_cpp(99, 1);
    assert!(runtime.override_spells_like_cpp().is_empty());
}

#[test]
fn a_negative_override_id_is_refused_before_the_map_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();

    runtime.add_override_spell_like_cpp(-1, 10);
    runtime.add_override_spell_like_cpp(50, 0);

    assert!(runtime.override_spells_like_cpp().is_empty());
}

#[test]
fn installing_an_acquisition_snapshot_marks_every_part_authoritative_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();

    runtime.install_acquisition_snapshot_like_cpp(PlayerSpellAcquisitionSnapshotLikeCpp {
        known_spells: vec![10],
        rows: BTreeMap::from([(10, row(10, PlayerSpellLoadState::Unchanged))]),
        dependent_known_spells: BTreeSet::from([10]),
        removed_known_spells: BTreeSet::from([20]),
        favorite_known_spells: BTreeSet::from([10]),
        trait_definition_ids: BTreeMap::from([(10, 100)]),
        override_spells: BTreeMap::from([(50, BTreeSet::from([10]))]),
    });

    assert!(runtime.rows_loaded_like_cpp());
    assert!(runtime.rows_complete_like_cpp());
    assert!(runtime.trait_definition_ids_complete_like_cpp());
    assert!(runtime.override_spells_complete_like_cpp());
    assert_eq!(runtime.known_spells_like_cpp(), &[10]);
    assert_eq!(runtime.trait_definition_ids_like_cpp().get(&10), Some(&100));
}

#[test]
fn invalidating_the_auxiliary_snapshots_leaves_their_contents_alone_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.set_trait_definition_id_like_cpp(10, Some(100));
    runtime.add_override_spell_like_cpp(50, 10);
    runtime.set_acquisition_snapshot_completeness_like_cpp(true, true);

    runtime.set_acquisition_snapshot_completeness_like_cpp(false, false);

    assert!(!runtime.trait_definition_ids_complete_like_cpp());
    assert!(!runtime.override_spells_complete_like_cpp());
    // Only the authority changed: the values a later load reconciles remain.
    assert_eq!(runtime.trait_definition_ids_like_cpp().get(&10), Some(&100));
    assert!(runtime.override_spells_like_cpp().contains_key(&50));
}

#[test]
fn a_saved_rebase_drops_removed_rows_and_recomputes_the_derived_sets_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();
    let mut dependent = row(10, PlayerSpellLoadState::New);
    dependent.dependent = true;
    let mut favorite = row(20, PlayerSpellLoadState::Changed);
    favorite.favorite = true;
    let mut disabled = row(30, PlayerSpellLoadState::New);
    disabled.disabled = true;
    let temporary = row(40, PlayerSpellLoadState::Temporary);
    let removed = row(50, PlayerSpellLoadState::Removed);
    runtime.replace_rows_like_cpp(
        BTreeMap::from([
            (10, dependent),
            (20, favorite),
            (30, disabled),
            (40, temporary),
            (50, removed),
        ]),
        true,
    );
    runtime.mark_removed_like_cpp(50);
    runtime.set_trait_definition_id_like_cpp(50, Some(500));
    runtime.set_trait_definition_id_like_cpp(10, Some(100));

    runtime.rebase_onto_saved_rows_like_cpp();

    // The removed row is gone; every other row settles, except the temporary
    // one C++ never persists.
    assert!(runtime.rows_like_cpp().get(&50).is_none());
    assert_eq!(
        runtime.rows_like_cpp().get(&10).expect("row").state,
        PlayerSpellLoadState::Unchanged
    );
    assert_eq!(
        runtime.rows_like_cpp().get(&40).expect("row").state,
        PlayerSpellLoadState::Temporary
    );
    assert!(runtime.removed_known_spells_like_cpp().is_empty());
    assert_eq!(
        runtime.dependent_known_spells_like_cpp(),
        &BTreeSet::from([10])
    );
    assert_eq!(
        runtime.favorite_known_spells_like_cpp(),
        &BTreeSet::from([20])
    );
    // A disabled spell is not client-known.
    assert_eq!(runtime.known_spells_like_cpp(), &[10, 20, 40]);
    // The trait definition of a row that is gone goes with it.
    assert!(runtime.trait_definition_ids_like_cpp().get(&50).is_none());
    assert_eq!(runtime.trait_definition_ids_like_cpp().get(&10), Some(&100));
}

#[test]
fn replacing_the_known_list_prunes_every_derived_entry_like_cpp() {
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.replace_known_spells_like_cpp(vec![10, 20]);
    runtime.set_dependent_like_cpp(10, true);
    runtime.set_dependent_like_cpp(20, true);
    runtime.set_favorite_like_cpp(20, true);
    runtime.set_trait_definition_id_like_cpp(20, Some(200));
    runtime.mark_removed_like_cpp(30);

    runtime.replace_known_spells_and_prune_derived_like_cpp(vec![10]);

    assert_eq!(runtime.known_spells_like_cpp(), &[10]);
    assert!(runtime.removed_known_spells_like_cpp().is_empty());
    assert_eq!(
        runtime.dependent_known_spells_like_cpp(),
        &BTreeSet::from([10])
    );
    assert!(runtime.favorite_known_spells_like_cpp().is_empty());
    assert!(runtime.trait_definition_ids_like_cpp().is_empty());
}

#[test]
fn the_player_owns_the_spell_runtime_for_its_whole_lifetime_like_cpp() {
    let mut player = Player::new(Some(1), false);

    player
        .gameplay_state_mut()
        .spells
        .learn_known_spell_id_unless_known_like_cpp(10);
    player
        .gameplay_state_mut()
        .spells
        .add_override_spell_like_cpp(50, 10);

    let runtime = player.spell_runtime_like_cpp();
    assert_eq!(runtime.known_spells_like_cpp(), &[10]);
    assert!(runtime.override_spells_like_cpp().contains_key(&50));
}
