// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::time::Instant;

use wow_constants::PowerType;
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_entities::{
    ActiveState, AuraApplicationLikeCpp, CharmInfoState, Pet, PetSpellState, PetSpellType,
    PetStable, PetStableInfo, PetType, ReactState,
};
use wow_persistence::{
    PlayerPetSaveTransactionLikeCpp, PlayerPetSpellChargeLoadRowLikeCpp,
    PlayerPetSpellCooldownLoadRowLikeCpp, PlayerPetSpellSaveOperationLikeCpp,
    PlayerPetStableLoadRowLikeCpp,
};

use super::*;

const PET_NUMBER: u32 = 73;
const PET_ENTRY: u32 = 500;
const ACTIVE_SLOT: u32 = 2;

fn player_guid() -> ObjectGuid {
    ObjectGuid::create_global(HighGuid::Player, 0, 42)
}

fn fixture(owner: ObjectGuid) -> (Pet, PetStable) {
    let mut pet = Pet::new(owner, PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(PET_ENTRY);
    pet.creature_mut().unit_mut().world_mut().set_name("Misha");
    pet.creature_mut()
        .unit_mut()
        .set_native_display_id_like_cpp(12_345);
    pet.creature_mut().unit_mut().set_level(43);
    pet.creature_mut().unit_mut().set_max_health(1_000);
    pet.creature_mut().unit_mut().set_health(888);
    pet.creature_mut().set_power_type(PowerType::Mana);
    pet.creature_mut()
        .unit_mut()
        .set_max_power(PowerType::Mana, 500);
    pet.creature_mut()
        .unit_mut()
        .set_power(PowerType::Mana, 222);
    pet.creature_mut().set_react_state(ReactState::Defensive);
    pet.set_pet_experience(777);
    pet.set_specialization(12);
    pet.set_created_by_spell_id_like_cpp(9_001);
    pet.creature_mut()
        .unit_mut()
        .replace_all_pet_flags_like_cpp(UNIT_PET_FLAG_CAN_BE_RENAMED_LIKE_CPP);

    let mut charm_info = CharmInfoState::default();
    charm_info.pet_number = PET_NUMBER;
    charm_info.action_bar[3] =
        wow_entities::make_unit_action_button_like_cpp(11_111, wow_entities::ACT_ENABLED_LIKE_CPP);
    pet.creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .charm_info = Some(charm_info);

    let mut stable = PetStable {
        current_pet_index: Some(ACTIVE_SLOT),
        ..PetStable::default()
    };
    stable.active_pets.resize(ACTIVE_SLOT as usize + 1, None);
    stable.active_pets[ACTIVE_SLOT as usize] = Some(PetStableInfo {
        pet_number: PET_NUMBER,
        creature_id: PET_ENTRY,
        ..PetStableInfo::default()
    });

    (pet, stable)
}

fn state_transaction(
    prepared: &PreparedPetSave,
) -> (
    &u32,
    &Vec<PlayerPetSpellSaveOperationLikeCpp>,
    &Vec<PlayerPetSpellCooldownLoadRowLikeCpp>,
    &Vec<PlayerPetSpellChargeLoadRowLikeCpp>,
) {
    let PlayerPetSaveTransactionLikeCpp::AurasSpellsAndHistory {
        pet_number,
        auras,
        aura_effects,
        spells,
        cooldowns,
        charges,
    } = &prepared.state_transaction
    else {
        panic!("the first native save phase must contain aura, spell and history rows");
    };
    assert!(auras.is_empty());
    assert!(aura_effects.is_empty());
    (pet_number, spells, cooldowns, charges)
}

#[test]
fn prepare_pet_save_projects_fill_pet_info_and_current_slot_like_cpp() {
    let (pet, stable) = fixture(player_guid());
    let now = 1_700_000_123;

    let prepared = prepare_pet_save(&pet, &stable, Some(2), now).expect("valid live Pet");

    assert_eq!(prepared.pet_number, PET_NUMBER);
    assert_eq!(prepared.info.name, "Misha");
    assert_eq!(
        prepared.info.action_bar,
        Pet::generate_action_bar_data_like_cpp(
            &pet.creature()
                .unit()
                .subsystems()
                .control
                .charm_info
                .as_ref()
                .unwrap()
                .action_bar
        )
    );
    assert_eq!(prepared.info.pet_number, PET_NUMBER);
    assert_eq!(prepared.info.creature_id, PET_ENTRY);
    assert_eq!(prepared.info.display_id, 12_345);
    assert_eq!(prepared.info.level, 43);
    assert_eq!(prepared.info.experience, 777);
    assert_eq!(prepared.info.react_state, ReactState::Aggressive);
    assert_eq!(prepared.info.was_renamed, false);
    assert_eq!(prepared.info.health, 888);
    assert_eq!(prepared.info.mana, 222);
    assert_eq!(prepared.info.last_save_time, now);
    assert_eq!(prepared.info.created_by_spell_id, 9_001);
    assert_eq!(prepared.info.pet_type, PetType::Hunter);
    assert_eq!(prepared.info.specialization_id, 12);

    let PlayerPetSaveTransactionLikeCpp::PetRow { owner_guid, row } = &prepared.row_transaction
    else {
        panic!("the second native save phase must write the Pet row");
    };
    assert_eq!(*owner_guid, 42);
    assert_eq!(
        row,
        &PlayerPetStableLoadRowLikeCpp {
            pet_number: PET_NUMBER,
            creature_id: PET_ENTRY,
            display_id: 12_345,
            level: 43,
            experience: 777,
            react_state: ReactState::Aggressive as u8,
            slot: ACTIVE_SLOT as i16,
            name: "Misha".to_owned(),
            was_renamed: false,
            health: 888,
            mana: 222,
            action_bar: prepared.info.action_bar.clone(),
            last_save_time: now,
            created_by_spell_id: 9_001,
            pet_type: PetType::Hunter as u8,
            specialization_id: 12,
        }
    );

    let (pet_number, spells, cooldowns, charges) = state_transaction(&prepared);
    assert_eq!(*pet_number, PET_NUMBER);
    assert!(spells.is_empty());
    assert!(cooldowns.is_empty());
    assert!(charges.is_empty());

    let mut pet = pet;
    pet.creature_mut()
        .unit_mut()
        .replace_all_pet_flags_like_cpp(0);
    assert!(
        prepare_pet_save(&pet, &stable, None, now)
            .expect("valid live Pet")
            .info
            .was_renamed
    );
}

#[test]
fn prepare_pet_save_projects_native_spell_states_without_acknowledging_them() {
    let (mut pet, stable) = fixture(player_guid());
    pet.add_spell(
        100,
        ActiveState::Enabled,
        PetSpellState::Changed,
        PetSpellType::Normal,
    );
    pet.add_spell(
        101,
        ActiveState::Passive,
        PetSpellState::New,
        PetSpellType::Normal,
    );
    pet.add_spell(
        102,
        ActiveState::Disabled,
        PetSpellState::Removed,
        PetSpellType::Normal,
    );
    pet.add_spell(
        103,
        ActiveState::Enabled,
        PetSpellState::New,
        PetSpellType::Family,
    );
    pet.add_spell(
        104,
        ActiveState::Disabled,
        PetSpellState::Unchanged,
        PetSpellType::Normal,
    );

    let prepared = prepare_pet_save(&pet, &stable, None, 17).expect("valid live Pet");
    let (_, spells, _, _) = state_transaction(&prepared);
    assert_eq!(
        spells,
        &vec![
            PlayerPetSpellSaveOperationLikeCpp::Delete { spell_id: 100 },
            PlayerPetSpellSaveOperationLikeCpp::Insert {
                spell_id: 100,
                active: ActiveState::Enabled as u8,
            },
            PlayerPetSpellSaveOperationLikeCpp::Insert {
                spell_id: 101,
                active: ActiveState::Passive as u8,
            },
            PlayerPetSpellSaveOperationLikeCpp::Delete { spell_id: 102 },
        ]
    );
    assert_eq!(
        pet.spells().get(&100).unwrap().state,
        PetSpellState::Changed
    );
    assert_eq!(pet.spells().get(&101).unwrap().state, PetSpellState::New);
    assert_eq!(
        pet.spells().get(&102).unwrap().state,
        PetSpellState::Removed
    );
    assert_eq!(pet.spells().get(&103).unwrap().state, PetSpellState::New);
}

#[test]
fn prepare_pet_save_maps_history_rows_skips_on_hold_and_converts_milliseconds() {
    let (mut pet, stable) = fixture(player_guid());
    let history = &mut pet
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .spells
        .history;
    history.add_cooldown(200, 0, 12_345, 9, 67_890, false);
    history.add_cooldown(201, 0, 22_222, 10, 88_888, true);
    assert!(history.add_charge_state_like_cpp(44, 10_999, 20_001));
    assert!(history.add_charge_state_like_cpp(44, 20_001, 30_999));

    let prepared = prepare_pet_save(&pet, &stable, None, 17).expect("valid live Pet");
    let (_, _, cooldowns, charges) = state_transaction(&prepared);
    assert_eq!(
        cooldowns,
        &vec![PlayerPetSpellCooldownLoadRowLikeCpp {
            spell_id: 200,
            cooldown_end_unix_secs: 12,
            category_id: 9,
            category_end_unix_secs: 67,
        }]
    );
    assert_eq!(
        charges,
        &vec![
            PlayerPetSpellChargeLoadRowLikeCpp {
                category_id: 44,
                recharge_start_unix_secs: 10,
                recharge_end_unix_secs: 20,
            },
            PlayerPetSpellChargeLoadRowLikeCpp {
                category_id: 44,
                recharge_start_unix_secs: 20,
                recharge_end_unix_secs: 30,
            },
        ]
    );
}

#[test]
fn prepare_pet_save_rejects_native_aura_state_in_both_local_views() {
    let (mut pet, stable) = fixture(player_guid());
    pet.creature_mut()
        .unit_mut()
        .subsystems_mut()
        .auras
        .removed_auras_count = 1;
    assert_eq!(
        prepare_pet_save(&pet, &stable, None, 17),
        Err("pet save requires an empty native aura container")
    );

    let (mut pet, stable) = fixture(player_guid());
    pet.creature_mut()
        .unit_mut()
        .subsystems_mut()
        .auras
        .insert_runtime_application_like_cpp(AuraApplicationLikeCpp {
            spell_id: 123,
            difficulty_id: 0,
            caster_guid: ObjectGuid::EMPTY,
            slot: 4,
            duration_total: 1_000,
            duration_remaining: 500,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 0,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        });
    assert_eq!(
        prepare_pet_save(&pet, &stable, None, 17),
        Err("pet save requires an empty native aura container")
    );
}

#[test]
fn prepare_pet_save_rejects_current_stable_mismatch_and_invalid_react_state() {
    let (pet, mut stable) = fixture(player_guid());
    stable.active_pets[ACTIVE_SLOT as usize]
        .as_mut()
        .unwrap()
        .pet_number += 1;
    assert_eq!(
        prepare_pet_save(&pet, &stable, None, 17),
        Err("pet save CharmInfo number does not match the current stable pet")
    );

    let (pet, stable) = fixture(player_guid());
    assert_eq!(
        prepare_pet_save(&pet, &stable, Some(3), 17),
        Err("pet save temporary react state is invalid")
    );
}

#[test]
fn prepare_pet_save_checks_native_pet_admission_guards() {
    let (pet, stable) = fixture(ObjectGuid::EMPTY);
    assert_eq!(
        prepare_pet_save(&pet, &stable, None, 17),
        Err("pet save requires a Player owner")
    );

    let (mut pet, stable) = fixture(player_guid());
    pet.set_pet_type(PetType::Max);
    assert_eq!(
        prepare_pet_save(&pet, &stable, None, 17),
        Err("pet save requires a fully controlled pet")
    );

    let (mut pet, stable) = fixture(player_guid());
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(0);
    assert_eq!(
        prepare_pet_save(&pet, &stable, None, 17),
        Err("pet save requires a nonzero creature entry")
    );
}
