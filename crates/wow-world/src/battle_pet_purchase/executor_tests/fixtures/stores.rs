//! Stores packets.
//!
//! Separated from fixtures.rs under #709.

use super::*;

pub(crate) fn saga_species_store_like_cpp() -> Arc<BattlePetSpeciesStore> {
    Arc::new(BattlePetSpeciesStore::from_entries([
        BattlePetSpeciesEntry {
            id: SAGA_SPECIES,
            description: String::new(),
            source_text: String::new(),
            creature_id: 99,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 0,
            flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        },
        BattlePetSpeciesEntry {
            id: LEGACY_UNIQUE_SPECIES,
            description: String::new(),
            source_text: String::new(),
            creature_id: 100,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 0,
            flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP
                | wow_data::BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        },
    ]))
}

pub(crate) fn saga_stat_stores_like_cpp() -> (
    Arc<BattlePetBreedQualityStore>,
    Arc<BattlePetBreedStateStore>,
    Arc<BattlePetSpeciesStateStore>,
) {
    (
        Arc::new(BattlePetBreedQualityStore::from_entries([
            BattlePetBreedQualityEntry {
                id: 1,
                state_multiplier: 1.0,
                quality_enum: 1,
            },
        ])),
        Arc::new(BattlePetBreedStateStore::from_entries([
            BattlePetBreedStateEntry {
                id: 1,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 500,
                battle_pet_breed_id: 7,
            },
            BattlePetBreedStateEntry {
                id: 2,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
                value: 300,
                battle_pet_breed_id: 7,
            },
            BattlePetBreedStateEntry {
                id: 3,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
                value: 200,
                battle_pet_breed_id: 7,
            },
        ])),
        Arc::new(BattlePetSpeciesStateStore::from_entries([
            BattlePetSpeciesStateEntry {
                id: 1,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 100,
                battle_pet_species_id: SAGA_SPECIES,
            },
            BattlePetSpeciesStateEntry {
                id: 2,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 100,
                battle_pet_species_id: LEGACY_UNIQUE_SPECIES,
            },
        ])),
    )
}

pub(crate) fn saga_selection_like_cpp(species: u32) -> BattlePetTrainerSelectionLikeCpp {
    BattlePetTrainerSelectionLikeCpp {
        species,
        breed: 7,
        quality: 1,
        display_id: 123,
        level: 1,
    }
}

pub(crate) fn saga_trainer_guid_like_cpp() -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 100, 1)
}

pub(crate) fn saga_registry_like_cpp(
    persistence: Arc<FakeSagaPersistenceLikeCpp>,
) -> Arc<BattlePetAccountRegistryLikeCpp> {
    let (qualities, breed_states, species_states) = saga_stat_stores_like_cpp();
    Arc::new(
        BattlePetAccountRegistryLikeCpp::new_with_persistence_like_cpp(
            persistence,
            saga_species_store_like_cpp(),
            qualities,
            breed_states,
            species_states,
            Arc::new(wow_data::BattlePetXpGameTableLikeCpp::from_rows([])),
            REALM_ID,
            VIRTUAL_REALM,
        ),
    )
}

pub(crate) fn store_handle_like_cpp(
    store: &Arc<FakeBattlePetPurchaseStoreLikeCpp>,
) -> Arc<dyn BattlePetPurchaseStoreLikeCpp> {
    store.clone()
}

pub(crate) fn saga_offer_like_cpp(price: u32) -> PreparedBattlePetTrainerOfferLikeCpp {
    PreparedBattlePetTrainerOfferLikeCpp {
        source_spell_id: SAGA_SPELL_ID,
        effective_price: price,
        species_id: SAGA_SPECIES,
    }
}

pub(crate) fn saga_learn_effect_like_cpp(
    record_id: u32,
    wrapper_spell_id: u32,
    learned_spell_id: u32,
) -> wow_data::SpellAcquisitionEffectLikeCpp {
    wow_data::SpellAcquisitionEffectLikeCpp {
        record_id,
        spell_id_raw: i64::from(wrapper_spell_id),
        difficulty_id_raw: 0,
        effect_index_raw: 1,
        effect_type_raw: 36, // C++ SPELL_EFFECT_LEARN_SPELL
        effect_aura_raw: 0,
        effect_mechanic_raw: 0,
        effect_attributes_raw: 0,
        effect_base_points_raw: 0,
        effect_die_sides_raw: 0,
        effect_chain_targets_raw: 0,
        effect_points_per_resource_bits: 0.0_f32.to_bits(),
        effect_real_points_per_level_bits: 0.0_f32.to_bits(),
        effect_coefficient_bits: 0.0_f32.to_bits(),
        effect_variance_bits: 0.0_f32.to_bits(),
        effect_trigger_spell_raw: i64::from(learned_spell_id),
        effect_item_type_raw: 0,
        effect_misc_value_raw: [0, 0],
        implicit_target_raw: [1, 0],
    }
}

pub(crate) fn saga_summon_effect_like_cpp(
    spell_id: u32,
) -> wow_data::SpellAcquisitionEffectLikeCpp {
    wow_data::SpellAcquisitionEffectLikeCpp {
        record_id: 1,
        spell_id_raw: i64::from(spell_id),
        difficulty_id_raw: 0,
        effect_index_raw: 0,
        effect_type_raw: 28, // C++ SPELL_EFFECT_SUMMON
        effect_aura_raw: 0,
        effect_mechanic_raw: 0,
        effect_attributes_raw: 0,
        effect_base_points_raw: 0,
        effect_die_sides_raw: 0,
        effect_chain_targets_raw: 0,
        effect_points_per_resource_bits: 0.0_f32.to_bits(),
        effect_real_points_per_level_bits: 0.0_f32.to_bits(),
        effect_coefficient_bits: 0.0_f32.to_bits(),
        effect_variance_bits: 0.0_f32.to_bits(),
        effect_trigger_spell_raw: 0,
        effect_item_type_raw: 0,
        effect_misc_value_raw: [99, i64::from(SAGA_SUMMON_PROPERTIES_ID)],
        implicit_target_raw: [1, 0],
    }
}

pub(crate) fn insert_saga_trainer_creature_like_cpp(
    manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(SAGA_CREATURE_ENTRY);
    creature.unit_mut().world_mut().set_map(0, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::new(1.0, 0.0, 0.0, 0.0));
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(
        1,
        35,
        wow_constants::unit::NPCFlags1::TRAINER.bits()
            | wow_constants::unit::NPCFlags1::TRAINER_CLASS.bits()
            | wow_constants::unit::NPCFlags1::TRAINER_PROFESSION.bits(),
        0,
    );
    creature.unit_mut().world_mut().object_mut().add_to_world();
    manager
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .expect("canonical test map")
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}
