//! #775 — the canonical Player owns the hydration of its trait-config details.
//!
//! C++ fills the Player's trait configs at login and reads them back through
//! its own accessors (`Player::AddTraitConfig`, `Player.h:1836`, and
//! `GetTraitConfig`, `:1837`). The details a login payload carries only
//! describe the rows that were actually loaded, so the owner installs them for
//! every config at once or refuses the hydration whole.

use crate::{PlayerSpellRuntimeState, PlayerTraitConfigDetails, PlayerTraitConfigState};

fn details(create_index: usize) -> PlayerTraitConfigDetails {
    PlayerTraitConfigDetails {
        create_index,
        local_identifier: 7,
        skill_line_id: 0,
        trait_system_id: 0,
        name: "build".into(),
        entries: Vec::new(),
    }
}

fn loaded_runtime(rows: &[(i32, (i32, i32, i32))]) -> PlayerSpellRuntimeState {
    let mut runtime = PlayerSpellRuntimeState::default();
    for (config_id, header) in rows {
        runtime.insert_trait_config_row_like_cpp(*config_id, PlayerTraitConfigState::from(*header));
    }
    runtime.mark_trait_authority_complete_like_cpp(true);
    runtime
}

fn stored_details(runtime: &PlayerSpellRuntimeState, config_id: i32) -> Option<usize> {
    runtime
        .trait_config_rows_like_cpp()
        .get(&config_id)?
        .details
        .as_ref()
        .map(|details| details.create_index)
}

#[test]
fn the_loaded_details_install_on_every_row_like_cpp() {
    let mut runtime = loaded_runtime(&[(1, (0, 71, 2)), (2, (0, 72, 0))]);

    assert!(runtime.install_loaded_trait_config_details_like_cpp(&[
        (1, (0, 71, 2), details(0)),
        (2, (0, 72, 0), details(1)),
    ]));

    assert_eq!(stored_details(&runtime, 1), Some(0));
    assert_eq!(stored_details(&runtime, 2), Some(1));
}

#[test]
fn an_unhydrated_row_set_refuses_the_whole_hydration() {
    let mut runtime = PlayerSpellRuntimeState::default();
    runtime.insert_trait_config_row_like_cpp(1, PlayerTraitConfigState::from((0, 71, 2)));

    assert!(!runtime.install_loaded_trait_config_details_like_cpp(&[(1, (0, 71, 2), details(0))]));
    assert_eq!(stored_details(&runtime, 1), None);
}

#[test]
fn incomplete_trait_entry_rows_refuse_the_hydration() {
    let mut runtime = loaded_runtime(&[(1, (0, 71, 2))]);
    runtime.set_trait_config_authority_for_fixture_like_cpp(true, false, true);

    assert!(!runtime.install_loaded_trait_config_details_like_cpp(&[(1, (0, 71, 2), details(0))]));
    assert_eq!(stored_details(&runtime, 1), None);
}

#[test]
fn a_different_number_of_configs_refuses_the_hydration() {
    let mut runtime = loaded_runtime(&[(1, (0, 71, 2)), (2, (0, 72, 0))]);

    assert!(!runtime.install_loaded_trait_config_details_like_cpp(&[(1, (0, 71, 2), details(0))]));
    assert_eq!(stored_details(&runtime, 1), None);
    assert_eq!(stored_details(&runtime, 2), None);
}

#[test]
fn a_repeated_config_id_refuses_the_hydration() {
    let mut runtime = loaded_runtime(&[(1, (0, 71, 2)), (2, (0, 72, 0))]);

    assert!(!runtime.install_loaded_trait_config_details_like_cpp(&[
        (1, (0, 71, 2), details(0)),
        (1, (0, 71, 2), details(1)),
    ]));
    assert_eq!(stored_details(&runtime, 1), None);
}

#[test]
fn a_header_that_disagrees_with_the_stored_row_refuses_the_hydration() {
    let mut runtime = loaded_runtime(&[(1, (0, 71, 2)), (2, (0, 72, 0))]);

    assert!(!runtime.install_loaded_trait_config_details_like_cpp(&[
        (1, (0, 71, 2), details(0)),
        (2, (0, 99, 0), details(1)),
    ]));
    assert_eq!(stored_details(&runtime, 1), None);
    assert_eq!(stored_details(&runtime, 2), None);
}

#[test]
fn an_unknown_config_id_refuses_the_hydration() {
    let mut runtime = loaded_runtime(&[(1, (0, 71, 2))]);

    assert!(!runtime.install_loaded_trait_config_details_like_cpp(&[(9, (0, 71, 2), details(0))]));
    assert_eq!(stored_details(&runtime, 1), None);
}

#[test]
fn an_empty_authoritative_row_set_accepts_an_empty_hydration() {
    let mut runtime = loaded_runtime(&[]);

    assert!(runtime.install_loaded_trait_config_details_like_cpp(&[]));
    assert!(runtime.trait_config_rows_like_cpp().is_empty());
}

#[test]
fn a_second_hydration_replaces_the_details_it_stored() {
    let mut runtime = loaded_runtime(&[(1, (0, 71, 2))]);
    assert!(runtime.install_loaded_trait_config_details_like_cpp(&[(1, (0, 71, 2), details(0))]));

    let mut later = details(0);
    later.name = "later".into();
    assert!(runtime.install_loaded_trait_config_details_like_cpp(&[(1, (0, 71, 2), later)]));

    assert_eq!(
        runtime.trait_config_rows_like_cpp()[&1]
            .details
            .as_ref()
            .map(|details| details.name.as_str()),
        Some("later")
    );
}
