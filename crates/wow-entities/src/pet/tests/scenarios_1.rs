//! Pet lifecycle, aura and persistence state regression scenarios, part 1 of 4.
//!
//! Moved out of the pet.rs root under #636; every test is unchanged.

use super::*;

#[test]
fn pet_constructor_matches_cpp_guardian_base_state() {
    let summon = Pet::new(owner_guid(), PetType::Summon);

    assert!(summon.creature().unit().world().is_world_object());
    assert_eq!(summon.creature().unit().world().name(), "Pet");
    assert_eq!(
        summon.unit_type_mask(),
        UNIT_MASK_SUMMON
            | UNIT_MASK_MINION
            | UNIT_MASK_GUARDIAN
            | UNIT_MASK_PET
            | UNIT_MASK_CONTROLABLE_GUARDIAN
    );
    assert_eq!(summon.owner_guid(), owner_guid());
    assert_eq!(summon.pet_type(), PetType::Summon);
    assert!(summon.is_controlled());
    assert!(!summon.is_temporary_summoned());
    assert_eq!(summon.duration_ms(), 0);
    assert!(!summon.is_loading());
    assert!(!summon.is_removed());
    assert_eq!(summon.focus_regen_timer_ms(), PET_FOCUS_REGEN_INTERVAL_MS);
    assert_eq!(summon.group_update_mask(), 0);
    assert_eq!(summon.specialization(), 0);
    assert!(summon.declined_name().is_none());
    assert!(summon.declined_names().is_none());

    let hunter = Pet::new(owner_guid(), PetType::Hunter);
    assert!((hunter.unit_type_mask() & UNIT_MASK_HUNTER_PET) != 0);
}

#[test]
fn pet_add_to_world_uses_unit_path_and_resets_follow_flags_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(2));
    let charm_info = pet
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .init_charm_info();
    charm_info.command_state = crate::COMMAND_FOLLOW_LIKE_CPP as u8;
    charm_info.is_command_attack = true;
    charm_info.is_command_follow = true;
    charm_info.is_at_stay = true;
    charm_info.is_following = true;
    charm_info.is_returning = true;

    let first = pet.add_to_world_like_cpp();
    assert_eq!(first.guid, pet_guid(2));
    assert!(first.inserted_pet_lookup);
    assert!(first.unit_add_to_world.is_some());
    assert!(first.aim_initialize_represented);
    assert!(first.zone_script_on_creature_create_represented);
    assert!(first.follow_command_flags_reset);
    assert!(pet.creature().unit().world().object().is_in_world());
    let charm_info = pet
        .creature()
        .unit()
        .subsystems()
        .control
        .charm_info
        .as_ref()
        .unwrap();
    assert_eq!(
        charm_info.command_state,
        crate::COMMAND_FOLLOW_LIKE_CPP as u8,
        "C++ clears transient follow flags but does not change CommandState"
    );
    assert!(!charm_info.is_command_attack);
    assert!(!charm_info.is_command_follow);
    assert!(!charm_info.is_at_stay);
    assert!(!charm_info.is_following);
    assert!(!charm_info.is_returning);

    let charm_info = pet
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .charm_info
        .as_mut()
        .unwrap();
    charm_info.is_command_follow = true;
    charm_info.is_following = true;
    let second = pet.add_to_world_like_cpp();
    assert!(!second.inserted_pet_lookup);
    assert!(second.unit_add_to_world.is_none());
    assert!(second.follow_command_flags_reset);
    assert!(
        !pet.creature()
            .unit()
            .subsystems()
            .control
            .charm_info
            .as_ref()
            .unwrap()
            .is_following,
        "C++ follow cleanup is outside the IsInWorld guard"
    );
}

#[test]
fn pet_remove_from_world_uses_unit_path_and_pet_lookup_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(3));
    assert!(pet.remove_from_world_like_cpp().is_none());

    let add = pet.add_to_world_like_cpp();
    assert!(add.unit_add_to_world.is_some());
    let remove = pet.remove_from_world_like_cpp().unwrap();
    assert_eq!(remove.guid, pet_guid(3));
    assert!(remove.unit_remove_from_world.is_some());
    assert!(remove.removed_pet_lookup);
    assert!(!pet.creature().unit().world().object().is_in_world());
    assert!(pet.remove_from_world_like_cpp().is_none());
}

#[test]
fn pet_debug_info_appends_pet_type_and_number_like_cpp() {
    let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
    hunter
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .init_charm_info()
        .pet_number = 42;
    assert_eq!(
        hunter.debug_info_with_guardian_like_cpp("GuardianDebug"),
        Some("GuardianDebug\nPetType: 1 PetNumber: 42".to_string())
    );

    let mut summon = Pet::new(owner_guid(), PetType::Summon);
    summon
        .creature_mut()
        .unit_mut()
        .subsystems_mut()
        .control
        .init_charm_info()
        .pet_number = 7;
    assert_eq!(
        summon.debug_info_with_guardian_like_cpp("G"),
        Some("G\nPetType: 0 PetNumber: 7".to_string()),
        "C++ uses std::to_string(getPetType()), so the enum is numeric"
    );
}

#[test]
fn pet_debug_info_requires_charm_info_like_cpp_assumption() {
    let pet = Pet::new(owner_guid(), PetType::Hunter);
    assert_eq!(pet.debug_info_with_guardian_like_cpp("GuardianDebug"), None);
}

#[test]
fn pet_power_index_delegates_to_creature_bridge() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);

    assert_eq!(pet.get_power_index(PowerType::Mana), Some(0));
    assert_eq!(pet.get_power_index(PowerType::ComboPoints), Some(2));
    assert_eq!(pet.get_power_index(PowerType::Energy), None);

    pet.creature_mut().set_power_type(PowerType::Focus);
    assert_eq!(pet.get_power_index(PowerType::Focus), Some(0));
    assert_eq!(pet.get_power_index(PowerType::Mana), None);
}

#[test]
fn pet_type_duration_group_and_focus_state_follow_cpp_shape() {
    let mut pet = Pet::new(owner_guid(), PetType::Max);

    assert!(!pet.is_controlled());
    pet.set_pet_type(PetType::Hunter);
    assert!(pet.is_controlled());
    assert!((pet.unit_type_mask() & UNIT_MASK_HUNTER_PET) != 0);

    pet.set_duration(100);
    assert!(pet.is_temporary_summoned());
    assert!(!pet.tick_focus_regen_timer(1_000));
    assert_eq!(pet.focus_regen_timer_ms(), 3_000);
    assert!(pet.tick_focus_regen_timer(3_000));
    assert_eq!(pet.focus_regen_timer_ms(), PET_FOCUS_REGEN_INTERVAL_MS);

    pet.set_group_update_flag(0x1);
    pet.set_group_update_flag(0x4);
    assert_eq!(pet.group_update_mask(), 0x5);
    pet.reset_group_update_flag();
    assert_eq!(pet.group_update_mask(), 0);
}

#[test]
fn pet_group_update_flags_follow_cpp_owner_group_gate() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);

    assert_eq!(
        pet.set_group_update_flag_like_cpp(GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP, false),
        None,
        "C++ Pet::SetGroupUpdateFlag mutates only when owner has a group"
    );
    assert_eq!(pet.group_update_mask(), 0);

    assert_eq!(
        pet.set_group_update_flag_like_cpp(GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP, true),
        Some(PetGroupUpdateOutcomeLikeCpp {
            group_update_mask: GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP,
            owner_group_flag: Some(GROUP_UPDATE_FLAG_PET_LIKE_CPP),
        })
    );
    assert_eq!(
        pet.group_update_mask(),
        GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP
    );

    assert_eq!(
        pet.reset_group_update_flag_like_cpp(false),
        PetGroupUpdateOutcomeLikeCpp {
            group_update_mask: GROUP_UPDATE_FLAG_PET_NONE_LIKE_CPP,
            owner_group_flag: None,
        }
    );
    assert_eq!(pet.group_update_mask(), 0);

    pet.set_group_update_flag_like_cpp(GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP, true);
    assert_eq!(
        pet.reset_group_update_flag_like_cpp(true),
        PetGroupUpdateOutcomeLikeCpp {
            group_update_mask: GROUP_UPDATE_FLAG_PET_NONE_LIKE_CPP,
            owner_group_flag: Some(GROUP_UPDATE_FLAG_PET_LIKE_CPP),
        }
    );
}

#[test]
fn pet_set_display_id_marks_pet_model_group_update_only_for_controlled_pets_like_cpp() {
    let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
    let outcome = hunter.set_display_id_like_cpp(12_345, true, true);
    assert_eq!(
        outcome,
        PetSetDisplayIdOutcomeLikeCpp {
            model_id: 12_345,
            set_native: true,
            group_update: Some(PetGroupUpdateOutcomeLikeCpp {
                group_update_mask: GROUP_UPDATE_FLAG_PET_MODEL_ID_LIKE_CPP,
                owner_group_flag: Some(GROUP_UPDATE_FLAG_PET_LIKE_CPP),
            }),
        }
    );
    assert_eq!(hunter.creature().unit().data().display_id, 12_345);
    assert_eq!(hunter.creature().unit().data().native_display_id, 12_345);

    let mut uncontrolled = Pet::new(owner_guid(), PetType::Max);
    let outcome = uncontrolled.set_display_id_like_cpp(22_222, false, true);
    assert_eq!(
        outcome,
        PetSetDisplayIdOutcomeLikeCpp {
            model_id: 22_222,
            set_native: false,
            group_update: None,
        },
        "C++ Pet::SetDisplayId returns before SetGroupUpdateFlag when !isControlled()"
    );
    assert_eq!(uncontrolled.group_update_mask(), 0);
    assert_eq!(uncontrolled.creature().unit().data().display_id, 22_222);
    assert_eq!(uncontrolled.creature().unit().data().native_display_id, 0);
}

#[test]
fn pet_have_in_diet_matches_cpp_food_type_and_family_mask() {
    assert!(
        !Pet::have_in_diet_like_cpp(0, true, Some(0xFFFF)),
        "C++ returns false before template lookup when item->FoodType is zero"
    );
    assert!(
        !Pet::have_in_diet_like_cpp(1, false, Some(0xFFFF)),
        "C++ returns false when GetCreatureTemplate() is missing"
    );
    assert!(
        !Pet::have_in_diet_like_cpp(1, true, None),
        "C++ returns false when CreatureFamily lookup is missing"
    );
    assert!(Pet::have_in_diet_like_cpp(1, true, Some(0b0001)));
    assert!(Pet::have_in_diet_like_cpp(4, true, Some(0b1000)));
    assert!(!Pet::have_in_diet_like_cpp(4, true, Some(0b0100)));
    assert!(
        !Pet::have_in_diet_like_cpp(33, true, Some(u32::MAX)),
        "C++ food masks are u32; out-of-range represented input fails closed"
    );
}

#[test]
fn pet_native_object_scale_matches_cpp_hunter_family_scale() {
    let family = PetFamilyScaleLikeCpp {
        min_scale: 0.75,
        min_scale_level: 10,
        max_scale: 1.25,
        max_scale_level: 50,
    };

    assert_eq!(
        Pet::native_object_scale_like_cpp(PetType::Summon, 40, 1.5, Some(family)),
        1.5,
        "C++ applies CreatureFamily scaling only to hunter pets"
    );
    assert_eq!(
        Pet::native_object_scale_like_cpp(PetType::Hunter, 40, 1.5, None),
        1.5,
        "C++ falls back to Guardian::GetNativeObjectScale when CreatureFamily lookup is missing"
    );
    assert_eq!(
        Pet::native_object_scale_like_cpp(
            PetType::Hunter,
            40,
            1.5,
            Some(PetFamilyScaleLikeCpp {
                min_scale: 0.0,
                ..family
            })
        ),
        1.5,
        "C++ requires CreatureFamilyEntry::MinScale > 0.0"
    );

    assert_eq!(
        Pet::native_object_scale_like_cpp(PetType::Hunter, 8, 1.5, Some(family)),
        family.min_scale
    );
    assert_eq!(
        Pet::native_object_scale_like_cpp(PetType::Hunter, 50, 1.5, Some(family)),
        family.max_scale
    );
    assert_eq!(
        Pet::native_object_scale_like_cpp(PetType::Hunter, 30, 1.5, Some(family)),
        0.95,
        "C++ divides the middle interpolation by MaxScaleLevel, not by the min-max span"
    );
}

#[test]
fn pet_is_permanent_pet_for_matches_cpp_owner_class_and_creature_type() {
    assert!(Pet::is_permanent_pet_for_like_cpp(
        PetType::Hunter,
        Class::Warrior,
        CreatureType::Critter
    ));
    assert!(Pet::is_permanent_pet_for_like_cpp(
        PetType::Summon,
        Class::Warlock,
        CreatureType::Demon
    ));
    assert!(Pet::is_permanent_pet_for_like_cpp(
        PetType::Summon,
        Class::DeathKnight,
        CreatureType::Undead
    ));
    assert!(Pet::is_permanent_pet_for_like_cpp(
        PetType::Summon,
        Class::Mage,
        CreatureType::Elemental
    ));

    assert!(
        !Pet::is_permanent_pet_for_like_cpp(
            PetType::Summon,
            Class::Warlock,
            CreatureType::Elemental
        ),
        "C++ requires warlock summon pets to use CREATURE_TYPE_DEMON"
    );
    assert!(
        !Pet::is_permanent_pet_for_like_cpp(
            PetType::Summon,
            Class::DeathKnight,
            CreatureType::Demon
        ),
        "C++ requires death knight summon pets to use CREATURE_TYPE_UNDEAD"
    );
    assert!(
        !Pet::is_permanent_pet_for_like_cpp(PetType::Summon, Class::Mage, CreatureType::Demon),
        "C++ requires mage summon pets to use CREATURE_TYPE_ELEMENTAL"
    );
    assert!(!Pet::is_permanent_pet_for_like_cpp(
        PetType::Summon,
        Class::Shaman,
        CreatureType::Elemental
    ));
    assert!(!Pet::is_permanent_pet_for_like_cpp(
        PetType::Max,
        Class::Hunter,
        CreatureType::Beast
    ));
}

#[test]
fn pet_duration_update_matches_cpp_temporary_summon_expiry_modes() {
    let mut summon = Pet::new(owner_guid(), PetType::Summon);
    summon.set_duration(100);
    assert_eq!(
        summon.update_duration_like_cpp(40),
        PetDurationUpdateOutcome::Active
    );
    assert_eq!(summon.duration_ms(), 60);
    assert_eq!(
        summon.update_duration_like_cpp(60),
        PetDurationUpdateOutcome::Expired {
            save_mode: PetSaveMode::NotInSlot
        }
    );
    assert_eq!(summon.duration_ms(), 0);

    let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
    hunter.set_duration(1);
    assert_eq!(
        hunter.update_duration_like_cpp(1),
        PetDurationUpdateOutcome::Expired {
            save_mode: PetSaveMode::AsDeleted
        }
    );
}

#[test]
fn pet_duration_update_skips_cpp_removed_loading_and_permanent_states() {
    let mut permanent = Pet::new(owner_guid(), PetType::Summon);
    assert_eq!(
        permanent.update_duration_like_cpp(100),
        PetDurationUpdateOutcome::Active
    );

    let mut removed = Pet::new(owner_guid(), PetType::Summon);
    removed.set_duration(100);
    removed.set_removed(true);
    assert_eq!(
        removed.update_duration_like_cpp(100),
        PetDurationUpdateOutcome::Skipped
    );
    assert_eq!(removed.duration_ms(), 100);

    let mut loading = Pet::new(owner_guid(), PetType::Summon);
    loading.set_duration(100);
    loading.set_loading(true);
    assert_eq!(
        loading.update_duration_like_cpp(100),
        PetDurationUpdateOutcome::Skipped
    );
    assert_eq!(loading.duration_ms(), 100);
}

#[test]
fn pet_corpse_update_matches_cpp_hunter_corpse_keep_and_remove() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut().set_corpse_delay(15, false);
    pet.creature_mut()
        .set_death_state_runtime(DeathState::JustDied, 1_000);

    assert_eq!(pet.creature().unit().death_state(), DeathState::Corpse);
    assert_eq!(pet.creature().corpse_remove_time(), 1_015);
    assert_eq!(
        pet.update_corpse_like_cpp(1_014),
        PetCorpseUpdateOutcome::KeepCorpse
    );
    assert_eq!(
        pet.update_corpse_like_cpp(1_015),
        PetCorpseUpdateOutcome::Remove {
            save_mode: PetSaveMode::NotInSlot
        }
    );
}

#[test]
fn pet_corpse_update_removes_non_hunter_corpses_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Summon);
    pet.creature_mut().set_corpse_delay(15, false);
    pet.creature_mut()
        .set_death_state_runtime(DeathState::JustDied, 1_000);

    assert_eq!(
        pet.update_corpse_like_cpp(1_001),
        PetCorpseUpdateOutcome::Remove {
            save_mode: PetSaveMode::NotInSlot
        }
    );
}

#[test]
fn pet_corpse_update_skips_cpp_removed_loading_and_non_corpse_states() {
    let pet = Pet::new(owner_guid(), PetType::Hunter);
    assert_eq!(
        pet.update_corpse_like_cpp(1_000),
        PetCorpseUpdateOutcome::NotCorpse
    );

    let mut removed = Pet::new(owner_guid(), PetType::Hunter);
    removed.set_removed(true);
    assert_eq!(
        removed.update_corpse_like_cpp(1_000),
        PetCorpseUpdateOutcome::Skipped
    );

    let mut loading = Pet::new(owner_guid(), PetType::Hunter);
    loading.set_loading(true);
    assert_eq!(
        loading.update_corpse_like_cpp(1_000),
        PetCorpseUpdateOutcome::Skipped
    );
}

#[test]
fn pet_alive_owner_link_removes_lost_owner_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(10));

    assert_eq!(
        pet.update_alive_owner_link_like_cpp(false, false, Some(pet_guid(10))),
        PetAliveOwnerUpdateOutcome::RemoveLostOwner {
            save_mode: PetSaveMode::NotInSlot,
            return_reagent: true
        },
        "C++ removes when pet is outside owner visibility range and not possessed"
    );
    assert_eq!(
        pet.update_alive_owner_link_like_cpp(true, false, None),
        PetAliveOwnerUpdateOutcome::RemoveLostOwner {
            save_mode: PetSaveMode::NotInSlot,
            return_reagent: true
        },
        "C++ removes controlled pets when owner->GetPetGUID() is empty"
    );
}

#[test]
fn pet_alive_owner_link_removes_unlinked_controlled_pet_like_cpp() {
    let mut summon = Pet::new(owner_guid(), PetType::Summon);
    summon
        .creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(11));

    assert_eq!(
        summon.update_alive_owner_link_like_cpp(true, false, Some(pet_guid(12))),
        PetAliveOwnerUpdateOutcome::RemoveUnlinkedControlled {
            save_mode: PetSaveMode::NotInSlot,
            unexpected_hunter: false
        }
    );

    let mut hunter = Pet::new(owner_guid(), PetType::Hunter);
    hunter
        .creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(13));
    assert_eq!(
        hunter.update_alive_owner_link_like_cpp(true, false, Some(pet_guid(14))),
        PetAliveOwnerUpdateOutcome::RemoveUnlinkedControlled {
            save_mode: PetSaveMode::NotInSlot,
            unexpected_hunter: true
        },
        "C++ ASSERTs this unexpected hunter-pet unlink case before Remove(PET_SAVE_NOT_IN_SLOT)"
    );
}

#[test]
fn pet_remove_plan_delegates_to_owner_remove_pet_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Summon);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(16));

    assert_eq!(
        pet.remove_plan_like_cpp(PetSaveMode::NotInSlot, true),
        PetRemovePlanLikeCpp {
            owner_guid: owner_guid(),
            pet_guid: pet_guid(16),
            save_mode: PetSaveMode::NotInSlot,
            return_reagent: true,
        },
        "C++ Pet::Remove(mode, returnreagent) delegates GetOwner()->RemovePet(this, mode, returnreagent)"
    );
    assert_eq!(
        pet.remove_plan_like_cpp(PetSaveMode::AsDeleted, false),
        PetRemovePlanLikeCpp {
            owner_guid: owner_guid(),
            pet_guid: pet_guid(16),
            save_mode: PetSaveMode::AsDeleted,
            return_reagent: false,
        }
    );
}

#[test]
fn pet_update_remove_outcomes_convert_to_remove_plan_like_cpp() {
    let mut lost_owner = Pet::new(owner_guid(), PetType::Hunter);
    lost_owner
        .creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(17));
    let PetAliveOwnerUpdateOutcome::RemoveLostOwner {
        save_mode,
        return_reagent,
    } = lost_owner.update_alive_owner_link_like_cpp(false, false, Some(pet_guid(17)))
    else {
        panic!("expected C++ lost-owner branch to request Pet::Remove");
    };
    assert_eq!(
        lost_owner.remove_plan_like_cpp(save_mode, return_reagent),
        PetRemovePlanLikeCpp {
            owner_guid: owner_guid(),
            pet_guid: pet_guid(17),
            save_mode: PetSaveMode::NotInSlot,
            return_reagent: true,
        }
    );

    let mut expired = Pet::new(owner_guid(), PetType::Summon);
    expired
        .creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(18));
    expired.set_duration(1);
    let PetDurationUpdateOutcome::Expired { save_mode } = expired.update_duration_like_cpp(1)
    else {
        panic!("expected C++ duration branch to request Pet::Remove");
    };
    assert_eq!(
        expired.remove_plan_like_cpp(save_mode, false),
        PetRemovePlanLikeCpp {
            owner_guid: owner_guid(),
            pet_guid: pet_guid(18),
            save_mode: PetSaveMode::NotInSlot,
            return_reagent: false,
        }
    );
}

#[test]
fn pet_alive_owner_link_keeps_valid_alive_and_skips_non_alive_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(15));

    assert_eq!(
        pet.update_alive_owner_link_like_cpp(true, false, Some(pet_guid(15))),
        PetAliveOwnerUpdateOutcome::Keep
    );
    assert_eq!(
        pet.update_alive_owner_link_like_cpp(false, true, Some(pet_guid(15))),
        PetAliveOwnerUpdateOutcome::Keep,
        "C++ allows out-of-range possessed pets through the distance branch"
    );

    pet.creature_mut()
        .set_death_state_runtime(DeathState::JustDied, 1_000);
    assert_eq!(
        pet.update_alive_owner_link_like_cpp(true, false, Some(pet_guid(15))),
        PetAliveOwnerUpdateOutcome::NotAlive
    );
}

#[test]
fn pet_set_death_state_hunter_corpse_clears_lootable_and_skinnable_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(1));
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .replace_all_dynamic_flags(0x44);
    pet.creature_mut()
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::SKINNABLE | UnitFlags::IN_COMBAT);

    let outcome = pet.set_death_state_like_cpp(DeathState::JustDied, 1_000);

    assert_eq!(pet.creature().unit().death_state(), DeathState::Corpse);
    assert!(outcome.cleared_hunter_corpse_flags);
    assert!(!outcome.cast_pet_auras_current);
    assert_eq!(pet.creature().unit().world().object().dynamic_flags(), 0);
    assert!(
        !pet.creature()
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::SKINNABLE),
        "C++ Pet::setDeathState(CORPSE) removes UNIT_FLAG_SKINNABLE for hunter pets"
    );
    assert!(
        pet.creature()
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::IN_COMBAT),
        "Pet-specific C++ branch removes SKINNABLE only; broader combat cleanup belongs to Creature/Unit"
    );
}

#[test]
fn pet_set_death_state_non_hunter_corpse_keeps_pet_specific_flags_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Summon);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(2));
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .replace_all_dynamic_flags(0x44);
    pet.creature_mut()
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::SKINNABLE);

    let outcome = pet.set_death_state_like_cpp(DeathState::JustDied, 1_000);

    assert_eq!(pet.creature().unit().death_state(), DeathState::Corpse);
    assert!(!outcome.cleared_hunter_corpse_flags);
    assert_eq!(pet.creature().unit().world().object().dynamic_flags(), 0x44);
    assert!(
        pet.creature()
            .unit()
            .unit_flags_like_cpp()
            .contains(UnitFlags::SKINNABLE)
    );
}

#[test]
fn pet_set_death_state_alive_requests_current_pet_auras_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid(3));

    let outcome = pet.set_death_state_like_cpp(DeathState::Alive, 1_000);

    assert_eq!(pet.creature().unit().death_state(), DeathState::Alive);
    assert!(!outcome.cleared_hunter_corpse_flags);
    assert!(
        outcome.cast_pet_auras_current,
        "C++ Pet::setDeathState(ALIVE) calls CastPetAuras(true)"
    );
}

#[test]
fn pet_focus_regen_timer_preserves_cpp_overshoot_and_lag_reset() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);

    assert!(pet.tick_focus_regen_timer(4_500));
    assert_eq!(pet.focus_regen_timer_ms(), 3_500);

    assert!(pet.tick_focus_regen_timer(7_500));
    assert_eq!(pet.focus_regen_timer_ms(), 1);

    assert!(pet.tick_focus_regen_timer(10_000));
    assert_eq!(pet.focus_regen_timer_ms(), PET_FOCUS_REGEN_INTERVAL_MS);
}

#[test]
fn pet_regenerate_focus_matches_cpp_base_amount_and_clamps() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut().set_power_type(PowerType::Focus);
    pet.creature_mut()
        .unit_mut()
        .set_max_power(PowerType::Focus, 100);
    pet.creature_mut()
        .unit_mut()
        .set_power(PowerType::Focus, 40);

    assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, true), 24);
    assert_eq!(pet.creature().unit().get_power(PowerType::Focus), 64);

    assert_eq!(pet.regenerate_focus_like_cpp(2.0, 0.5, 5, true), 28);
    assert_eq!(pet.creature().unit().get_power(PowerType::Focus), 92);

    assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, true), 8);
    assert_eq!(pet.creature().unit().get_power(PowerType::Focus), 100);
    assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, true), 0);
}

#[test]
fn pet_regenerate_focus_honors_cpp_power_guards() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);

    assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, true), 0);

    pet.creature_mut().set_power_type(PowerType::Focus);
    pet.creature_mut()
        .unit_mut()
        .set_max_power(PowerType::Focus, 100);
    pet.creature_mut()
        .unit_mut()
        .set_power(PowerType::Focus, 40);

    assert_eq!(pet.regenerate_focus_like_cpp(1.0, 1.0, 0, false), 0);
    assert_eq!(pet.creature().unit().get_power(PowerType::Focus), 40);
}

#[test]
fn pet_give_level_updates_hunter_xp_and_levelup_hooks_like_cpp() {
    let mut pet = Pet::new(owner_guid(), PetType::Hunter);
    pet.creature_mut().unit_mut().set_level(10);
    pet.set_pet_experience(50);
    pet.set_pet_next_level_experience(500);

    let outcome = pet.give_pet_level_like_cpp(11, |level| u32::from(level) * 1_000);

    assert_eq!(
        outcome,
        PetLevelUpdateOutcome {
            changed: true,
            reset_experience: true,
            refresh_stats: true,
            init_levelup_spells: true
        }
    );
    assert_eq!(pet.creature().level(), 11);
    assert_eq!(pet.pet_experience(), 0);
    assert_eq!(pet.pet_next_level_experience(), 550);

    let unchanged = pet.give_pet_level_like_cpp(11, |_| 999);
    assert_eq!(
        unchanged,
        PetLevelUpdateOutcome {
            changed: false,
            reset_experience: false,
            refresh_stats: false,
            init_levelup_spells: false
        }
    );
}
