//! #767 — the canonical Player owns its equipment sets and their invariants.
//!
//! C++ `Player` holds `EquipmentSetContainer _equipmentSets` (`Player.h:3050`)
//! and performs every transition itself: `_LoadEquipmentSets`
//! (`Player.cpp:16907`), `SetEquipmentSet` (`:26376`) with its
//! `EQUIPMENT_SET_NEW`/`EQUIPMENT_SET_CHANGED` rule at `:26406`,
//! `_SaveEquipmentSets` (`:26409`) and `DeleteEquipmentSet` (`:26524`).

use crate::{
    PlayerEquipmentSetLikeCpp, PlayerEquipmentSetUpdateStateLikeCpp, PlayerEquipmentSetsLikeCpp,
};

fn set(
    guid: u64,
    set_id: u32,
    state: PlayerEquipmentSetUpdateStateLikeCpp,
) -> PlayerEquipmentSetLikeCpp {
    let mut equipment_set = PlayerEquipmentSetLikeCpp::equipment(set_id, -1, state);
    equipment_set.guid = guid;
    equipment_set
}

fn outfit(
    guid: u64,
    set_id: u32,
    state: PlayerEquipmentSetUpdateStateLikeCpp,
) -> PlayerEquipmentSetLikeCpp {
    let mut equipment_set = PlayerEquipmentSetLikeCpp::transmog(set_id, -1, state);
    equipment_set.guid = guid;
    equipment_set
}

fn loaded_with(rows: Vec<PlayerEquipmentSetLikeCpp>) -> PlayerEquipmentSetsLikeCpp {
    let mut sets = PlayerEquipmentSetsLikeCpp::default();
    for row in rows {
        sets.install_loaded_set_like_cpp(row);
    }
    sets.mark_loaded_like_cpp();
    sets
}

#[test]
fn a_fresh_owner_is_empty_and_unhydrated_like_cpp() {
    let sets = PlayerEquipmentSetsLikeCpp::default();

    assert!(sets.sets_like_cpp().is_empty());
    assert!(!sets.is_loaded_like_cpp());
    assert_eq!(sets.set_like_cpp(1), None);
    assert_eq!(sets.set_state_like_cpp(1), None);
}

#[test]
fn an_empty_loaded_owner_is_authoritative_and_a_cleared_one_is_not() {
    let mut sets = loaded_with(vec![set(
        1,
        7,
        PlayerEquipmentSetUpdateStateLikeCpp::Unchanged,
    )]);
    assert!(sets.is_loaded_like_cpp());

    sets.clear_like_cpp();

    assert!(sets.sets_like_cpp().is_empty());
    assert!(!sets.is_loaded_like_cpp());
}

#[test]
fn a_loaded_row_is_stored_under_its_own_guid_like_cpp() {
    let sets = loaded_with(vec![set(
        42,
        7,
        PlayerEquipmentSetUpdateStateLikeCpp::Unchanged,
    )]);

    assert_eq!(sets.set_like_cpp(42).map(|row| row.set_id), Some(7));
    assert_eq!(
        sets.sets_like_cpp().keys().copied().collect::<Vec<_>>(),
        [42]
    );
}

#[test]
fn a_created_set_starts_new_like_cpp() {
    let mut sets = loaded_with(vec![]);

    sets.create_set_like_cpp(set(5, 1, PlayerEquipmentSetUpdateStateLikeCpp::Unchanged));

    assert_eq!(
        sets.set_state_like_cpp(5),
        Some(PlayerEquipmentSetUpdateStateLikeCpp::New)
    );
}

#[test]
fn editing_an_unknown_guid_is_refused_like_cpp_set_equipment_set() {
    let mut sets = loaded_with(vec![set(
        1,
        7,
        PlayerEquipmentSetUpdateStateLikeCpp::Unchanged,
    )]);

    assert!(!sets.update_set_like_cpp(set(2, 8, PlayerEquipmentSetUpdateStateLikeCpp::Unchanged)));
    assert_eq!(sets.set_like_cpp(2), None);
    assert_eq!(sets.sets_like_cpp().len(), 1);
}

#[test]
fn editing_a_new_set_keeps_it_new_so_it_is_inserted_once_like_cpp() {
    let mut sets = loaded_with(vec![]);
    sets.create_set_like_cpp(set(5, 1, PlayerEquipmentSetUpdateStateLikeCpp::Unchanged));

    let mut edited = set(5, 1, PlayerEquipmentSetUpdateStateLikeCpp::Unchanged);
    edited.set_name = "later".into();
    assert!(sets.update_set_like_cpp(edited));

    let row = sets.set_like_cpp(5).expect("stored set");
    assert_eq!(row.state, PlayerEquipmentSetUpdateStateLikeCpp::New);
    assert_eq!(row.set_name, "later");
}

#[test]
fn editing_a_stored_set_marks_it_changed_like_cpp() {
    let mut sets = loaded_with(vec![set(
        1,
        7,
        PlayerEquipmentSetUpdateStateLikeCpp::Unchanged,
    )]);

    assert!(sets.update_set_like_cpp(set(1, 7, PlayerEquipmentSetUpdateStateLikeCpp::Unchanged)));

    assert_eq!(
        sets.set_state_like_cpp(1),
        Some(PlayerEquipmentSetUpdateStateLikeCpp::Changed)
    );
}

#[test]
fn assigning_a_spec_matches_only_an_equipment_set_like_cpp() {
    let mut sets = loaded_with(vec![
        outfit(1, 7, PlayerEquipmentSetUpdateStateLikeCpp::Unchanged),
        set(2, 7, PlayerEquipmentSetUpdateStateLikeCpp::Unchanged),
    ]);

    assert!(sets.assign_set_to_spec_like_cpp(7, 2));

    assert_eq!(
        sets.set_like_cpp(2).map(|row| row.assigned_spec_index),
        Some(2)
    );
    assert_eq!(
        sets.set_state_like_cpp(2),
        Some(PlayerEquipmentSetUpdateStateLikeCpp::Changed)
    );
    assert_eq!(
        sets.set_like_cpp(1).map(|row| row.assigned_spec_index),
        Some(-1)
    );
    assert_eq!(
        sets.set_state_like_cpp(1),
        Some(PlayerEquipmentSetUpdateStateLikeCpp::Unchanged)
    );
}

#[test]
fn assigning_a_spec_to_a_new_set_keeps_it_new_like_cpp() {
    let mut sets = loaded_with(vec![set(1, 7, PlayerEquipmentSetUpdateStateLikeCpp::New)]);

    assert!(sets.assign_set_to_spec_like_cpp(7, 3));

    assert_eq!(
        sets.set_like_cpp(1).map(|row| row.assigned_spec_index),
        Some(3)
    );
    assert_eq!(
        sets.set_state_like_cpp(1),
        Some(PlayerEquipmentSetUpdateStateLikeCpp::New)
    );
}

#[test]
fn assigning_a_spec_without_a_matching_set_id_changes_nothing_like_cpp() {
    let mut sets = loaded_with(vec![set(
        1,
        7,
        PlayerEquipmentSetUpdateStateLikeCpp::Unchanged,
    )]);

    assert!(!sets.assign_set_to_spec_like_cpp(9, 1));

    assert_eq!(
        sets.set_like_cpp(1).map(|row| row.assigned_spec_index),
        Some(-1)
    );
}

#[test]
fn deleting_a_new_set_erases_it_and_a_stored_one_is_tombstoned_like_cpp() {
    let mut sets = loaded_with(vec![
        set(1, 7, PlayerEquipmentSetUpdateStateLikeCpp::New),
        set(2, 8, PlayerEquipmentSetUpdateStateLikeCpp::Unchanged),
    ]);

    assert!(sets.delete_set_like_cpp(1));
    assert!(sets.delete_set_like_cpp(2));

    assert_eq!(sets.set_like_cpp(1), None);
    assert_eq!(
        sets.set_state_like_cpp(2),
        Some(PlayerEquipmentSetUpdateStateLikeCpp::Deleted)
    );
}

#[test]
fn deleting_an_unknown_set_reports_nothing_removed_like_cpp() {
    let mut sets = loaded_with(vec![set(
        1,
        7,
        PlayerEquipmentSetUpdateStateLikeCpp::Unchanged,
    )]);

    assert!(!sets.delete_set_like_cpp(9));
    assert_eq!(sets.sets_like_cpp().len(), 1);
}

#[test]
fn a_completed_save_drops_deleted_rows_and_returns_the_rest_to_unchanged_like_cpp() {
    let mut sets = loaded_with(vec![
        set(1, 7, PlayerEquipmentSetUpdateStateLikeCpp::New),
        set(2, 8, PlayerEquipmentSetUpdateStateLikeCpp::Changed),
        set(3, 9, PlayerEquipmentSetUpdateStateLikeCpp::Deleted),
        set(4, 10, PlayerEquipmentSetUpdateStateLikeCpp::Unchanged),
    ]);

    sets.mark_sets_saved_like_cpp();

    assert_eq!(sets.set_like_cpp(3), None);
    for guid in [1, 2, 4] {
        assert_eq!(
            sets.set_state_like_cpp(guid),
            Some(PlayerEquipmentSetUpdateStateLikeCpp::Unchanged)
        );
    }
    assert!(sets.is_loaded_like_cpp());
}

#[test]
fn an_acknowledged_save_settles_only_rows_that_did_not_change_since() {
    let mut sets = loaded_with(vec![
        set(1, 7, PlayerEquipmentSetUpdateStateLikeCpp::New),
        set(2, 8, PlayerEquipmentSetUpdateStateLikeCpp::New),
    ]);
    let saved = sets.snapshot_like_cpp();
    let mut edited = set(2, 8, PlayerEquipmentSetUpdateStateLikeCpp::New);
    edited.set_name = "later".into();
    assert!(sets.update_set_like_cpp(edited));

    sets.acknowledge_saved_sets_like_cpp(saved);

    assert_eq!(
        sets.set_state_like_cpp(1),
        Some(PlayerEquipmentSetUpdateStateLikeCpp::Unchanged)
    );
    assert_eq!(
        sets.set_state_like_cpp(2),
        Some(PlayerEquipmentSetUpdateStateLikeCpp::Changed)
    );
}

#[test]
fn an_acknowledged_save_tombstones_a_row_that_disappeared_since() {
    let mut sets = loaded_with(vec![set(1, 7, PlayerEquipmentSetUpdateStateLikeCpp::New)]);
    let saved = sets.snapshot_like_cpp();
    assert!(sets.delete_set_like_cpp(1));
    assert_eq!(sets.set_like_cpp(1), None);

    sets.acknowledge_saved_sets_like_cpp(saved);

    assert_eq!(
        sets.set_state_like_cpp(1),
        Some(PlayerEquipmentSetUpdateStateLikeCpp::Deleted)
    );
}

#[test]
fn an_acknowledged_delete_erases_the_row_it_confirmed() {
    let mut sets = loaded_with(vec![set(
        1,
        7,
        PlayerEquipmentSetUpdateStateLikeCpp::Unchanged,
    )]);
    assert!(sets.delete_set_like_cpp(1));
    let saved = sets.snapshot_like_cpp();

    sets.acknowledge_saved_sets_like_cpp(saved);

    assert_eq!(sets.set_like_cpp(1), None);
}
