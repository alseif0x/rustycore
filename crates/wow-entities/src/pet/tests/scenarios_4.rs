//! Pet lifecycle, aura and persistence state regression scenarios, part 4 of 4.
//!
//! Moved out of the pet.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn pet_unlearn_spells_like_cpp_loading_still_batches_without_sending() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.set_loading(true);
    assert!(pet.add_spell(
        100,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal
    ));
    let mut action_bar = [0; MAX_UNIT_ACTION_BAR_INDEX];

    let outcome = pet.unlearn_spells_like_cpp(
        [100],
        false,
        false,
        &mut action_bar,
        |_| 0,
        |spell_id| spell_id,
    );

    assert_eq!(outcome.packet_spell_ids, vec![100]);
    assert!(!outcome.send_session_packet);
}

#[test]
fn pet_declined_names_store_all_cpp_cases() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    let names = PetDeclinedNamesLikeCpp {
        names: ["Mishy", "Mishya", "Mishu", "Mishom", "Mishe"].map(str::to_string),
    };

    pet.set_declined_names(Some(names.clone()));

    assert_eq!(pet.declined_names(), Some(&names));
}

#[test]
fn stable_lookup_matches_cpp_priority_order() {
    let stable = PetStable {
        current_pet_index: Some(1),
        active_pets: vec![Some(pet_info(10, 100)), Some(pet_info(20, 200))],
        stabled_pets: vec![Some(pet_info(30, 300))],
        unslotted_pets: vec![pet_info(40, 400)],
    };

    assert_eq!(
        Pet::get_load_pet_info(&stable, 0, 20, None),
        Some(PetLoadSelection {
            pet_number: 20,
            creature_id: 200,
            slot: 1,
        })
    );
    assert_eq!(
        Pet::get_load_pet_info(&stable, 0, 30, None).unwrap().slot,
        PetSaveMode::stable_slot(0)
    );
    assert_eq!(
        Pet::get_load_pet_info(&stable, 0, 0, Some(PetSaveMode::AsCurrent as i16)),
        Some(PetLoadSelection {
            pet_number: 20,
            creature_id: 200,
            slot: 1,
        })
    );
    assert_eq!(
        Pet::get_load_pet_info(&stable, 400, 0, None).unwrap().slot,
        PetSaveMode::NotInSlot as i16
    );
}

#[test]
fn stable_lookup_preserves_cpp_deleted_save_mode() {
    let stable = PetStable {
        current_pet_index: Some(0),
        active_pets: vec![None],
        stabled_pets: vec![None],
        unslotted_pets: Vec::new(),
    };

    let result = Pet::get_load_pet_info_result_like_cpp(&stable, 0, 999, None);
    assert_eq!(result, PetLoadInfoResult::Deleted);
    assert_eq!(result.selection(), None);
    assert_eq!(result.save_mode(), PetSaveMode::AsDeleted as i16);
    assert_eq!(Pet::get_load_pet_info(&stable, 0, 999, None), None);

    let result =
        Pet::get_load_pet_info_result_like_cpp(&stable, 0, 0, Some(PetSaveMode::AsCurrent as i16));
    assert_eq!(result, PetLoadInfoResult::Deleted);
    assert_eq!(result.save_mode(), PetSaveMode::AsDeleted as i16);
}

#[test]
fn pet_save_slot_helpers_match_cpp_ranges() {
    assert!(PetSaveMode::is_active_slot(0));
    assert!(PetSaveMode::is_active_slot(4));
    assert!(!PetSaveMode::is_active_slot(5));
    assert!(PetSaveMode::is_stabled_slot(5));
    assert!(PetSaveMode::is_stabled_slot(204));
    assert!(!PetSaveMode::is_stabled_slot(205));
}

#[test]
fn permanent_pet_and_xp_factor_match_cpp_shape() {
    let pet = Pet::new(owner_guid(), PetType::Hunter);
    assert!(pet.is_permanent_pet_for(owner_guid(), 1));
    assert!(!pet.is_permanent_pet_for(owner_guid(), 0));
    assert_eq!(Pet::pet_next_level_xp_for_owner_level(10_000), 500);
}
