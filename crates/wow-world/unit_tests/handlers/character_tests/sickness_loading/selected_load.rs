use super::*;
use super::selected_restore::{authority, keyed_rows, loaded_sickness};
use std::sync::Arc;

fn saved_effects(
    spell_id: u32,
    caster_guid: ObjectGuid,
    effect_mask: u32,
    amount: i32,
) -> Vec<crate::session::CharacterAuraEffectRowLikeCpp> {
    (0..3)
        .map(|effect_index| crate::session::CharacterAuraEffectRowLikeCpp {
            caster_guid,
            item_guid: ObjectGuid::EMPTY,
            spell_id,
            effect_mask,
            effect_index,
            amount,
            base_amount: amount,
        })
        .collect()
}

fn indexed_effect(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    effect_index: i32,
    aura_type: i16,
    misc_value: i32,
) -> wow_data::spell_db2::SpellEffectDb2Entry {
    wow_data::spell_db2::SpellEffectDb2Entry {
        id,
        difficulty_id: i32::from(difficulty_id),
        effect_index,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: aura_type,
        effect_aura_period: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        effect_chain_amplitude: 0.0,
        effect_chain_targets: 0,
        effect_die_sides: 0,
        effect_item_type: 0,
        effect_mechanic: 0,
        effect_points_per_resource: 0.0,
        effect_pos_facing: 0.0,
        effect_real_points_per_level: 0.0,
        effect_trigger_spell: 0,
        bonus_coefficient_from_ap: 0.0,
        pvp_multiplier: 0.0,
        coefficient: 0.0,
        variance: 0.0,
        resource_coefficient: 0.0,
        group_size_base_points_coefficient: 0.0,
        effect_misc_value: [misc_value, 0],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [0; 2],
        spell_id,
    }
}

fn selected_misc(
    id: u32,
    spell_id: u32,
    difficulty_id: u8,
    duration_index: u16,
    attributes: [u32; 15],
) -> wow_data::SpellMiscEntry {
    wow_data::SpellMiscEntry {
        id,
        spell_id,
        difficulty_id,
        duration_index,
        attributes: attributes.map(|attribute| attribute as i32),
        ..Default::default()
    }
}

fn difficulty(id: u32, fallback_difficulty_id: u8) -> wow_data::difficulty::DifficultyEntry {
    wow_data::difficulty::DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id,
        toggle_difficulty_id: 0,
    }
}

fn install_indexed_sickness_data(
    session: &mut WorldSession,
    misc_rows: Vec<wow_data::SpellMiscEntry>,
    effect_rows: Vec<wow_data::spell_db2::SpellEffectDb2Entry>,
    difficulty_rows: Vec<wow_data::difficulty::DifficultyEntry>,
) {
    let misc_store = wow_data::spell_db2::SpellMiscStore::from_entries(misc_rows.clone());
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
        misc_store,
        wow_data::spell_db2::SpellEffectDb2Store::from_entries(effect_rows),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    let mut spells = wow_data::SpellStore::new().hydrate_effective_core_db2_like_cpp(stores);
    if spells.get(SICKNESS_SPELL_ID).is_none() {
        spells.insert(SICKNESS_SPELL_ID, sickness_catalog_info());
    }
    session.set_spell_store(Arc::new(spells));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_spell_duration_store(Arc::new(
        wow_data::SpellDurationStore::from_entries([wow_data::SpellDurationEntry {
            id: 427,
            duration: -600_000,
            duration_per_level: 0,
            max_duration: 600_000,
        }]),
    ));
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries(
        difficulty_rows,
    )));
}

fn replace_spell_attributes(session: &mut WorldSession, spell_id: i32, attributes: [u32; 15]) {
    let existing = session
        .catalogs
        .spell_store()
        .cloned()
        .expect("selected SpellStore");
    let info = existing
        .get(spell_id)
        .cloned()
        .expect("selected SpellInfo");
    let mut spells = wow_data::SpellStore::new();
    spells.insert(spell_id, info);
    spells.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spells));

    let mut misc_rows = session
        .catalogs
        .spell_catalogs
        .spell_misc_store()
        .expect("selected SpellMisc store")
        .entries_like_cpp()
        .cloned()
        .collect::<Vec<_>>();
    let misc = misc_rows
        .iter_mut()
        .find(|row| row.spell_id == spell_id as u32 && row.difficulty_id == 0)
        .expect("base selected SpellMisc row");
    misc.attributes = attributes.map(|attribute| attribute as i32);
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
}

fn install_positive_sickness_effects_with_debuff(session: &mut WorldSession) {
    let existing = session
        .catalogs
        .spell_store()
        .cloned()
        .expect("selected SpellStore");
    let mut info = existing
        .get(SICKNESS_SPELL_ID)
        .cloned()
        .expect("selected SpellInfo");
    for (index, base_amount) in [(0, 30), (1, 25), (2, 3)] {
        info.effects
            .iter_mut()
            .find(|effect| effect.effect_index == index)
            .expect("selected sickness effect index")
            .effect_base_points = base_amount;
    }
    let mut attributes = existing
        .misc_attributes_for_difficulty_like_cpp(SICKNESS_SPELL_ID, 0, None)
        .expect("selected SpellInfo attributes");
    attributes[0] = SPELL_ATTR0_AURA_IS_DEBUFF;
    let mut spells = wow_data::SpellStore::new();
    spells.insert(SICKNESS_SPELL_ID, info);
    spells.insert_spell_misc_attributes_like_cpp(SICKNESS_SPELL_ID, attributes);
    session.set_spell_store(Arc::new(spells));

    let mut misc_rows = session
        .catalogs
        .spell_catalogs
        .spell_misc_store()
        .expect("selected SpellMisc store")
        .entries_like_cpp()
        .cloned()
        .collect::<Vec<_>>();
    let misc = misc_rows
        .iter_mut()
        .find(|row| row.spell_id == SICKNESS_SPELL_ID as u32 && row.difficulty_id == 0)
        .expect("selected base SpellMisc row");
    misc.attributes = attributes.map(|attribute| attribute as i32);
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
}

fn loaded_spell(
    session: &WorldSession,
    spell_id: i32,
) -> Option<(
    wow_entities::AuraApplicationLikeCpp,
    wow_entities::AuraPersistenceMetadata,
    wow_entities::AuraCastProvenanceLikeCpp,
)> {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.spell_id == spell_id)
                .map(|aura| {
                    (
                        aura.clone(),
                        auras
                            .aura_persistence_metadata(aura.slot)
                            .expect("selected aura persistence metadata")
                            .clone(),
                        auras.aura_cast_provenance_like_cpp(aura.slot),
                    )
                })
        })
        .flatten()
}

fn selected_snapshot_effects(
    session: &WorldSession,
    spell_id: i32,
) -> Vec<(u32, i32, i32, i32)> {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let slot = auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.spell_id == spell_id)
                .expect("selected aura application")
                .slot;
            auras
                .threat_snapshot_like_cpp(slot)
                .expect("selected aura snapshot")
                .effects()
                .to_vec()
        })
        .expect("canonical Player owner")
}

#[test]
fn selected_load_resolves_indexed_effects_through_difficulty_chains() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let mut attributes = [0; 15];
    attributes[0] = SPELL_ATTR0_AURA_IS_DEBUFF;
    let misc = vec![selected_misc(
        1,
        SICKNESS_SPELL_ID as u32,
        0,
        427,
        attributes,
    )];
    let effects = vec![
        indexed_effect(1, SICKNESS_SPELL_ID as u32, 0, 0, 80, 10),
        indexed_effect(2, SICKNESS_SPELL_ID as u32, 0, 1, 79, 20),
        indexed_effect(3, SICKNESS_SPELL_ID as u32, 0, 2, 159, 30),
        indexed_effect(4, SICKNESS_SPELL_ID as u32, 3, 1, 79, 21),
        indexed_effect(5, SICKNESS_SPELL_ID as u32, 4, 0, 80, 40),
    ];
    install_indexed_sickness_data(
        &mut session,
        misc,
        effects,
        vec![difficulty(4, 3), difficulty(3, 0), difficulty(0, 0)],
    );
    let (mut aura, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    aura.difficulty = 4;
    aura.recalculate_mask = 0;

    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], saved, 0),
        1
    );
    let loaded = loaded_spell(&session, SICKNESS_SPELL_ID).expect("loaded sickness");
    assert_eq!(loaded.0.difficulty_id, 4, "the saved difficulty is retained");
    assert_eq!(
        selected_snapshot_effects(&session, SICKNESS_SPELL_ID)
            .iter()
            .map(|effect| effect.3)
            .collect::<Vec<_>>(),
        [40, 21, 30],
        "each effect uses its first exact row in the 4 -> 3 -> 0 chain"
    );

    let (mut zero_session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let mut attributes = [0; 15];
    attributes[0] = SPELL_ATTR0_AURA_IS_DEBUFF;
    install_indexed_sickness_data(
        &mut zero_session,
        vec![selected_misc(
            1,
            SICKNESS_SPELL_ID as u32,
            3,
            427,
            attributes,
        )],
        vec![
            indexed_effect(1, SICKNESS_SPELL_ID as u32, 3, 0, 80, 33),
            indexed_effect(2, SICKNESS_SPELL_ID as u32, 3, 1, 79, 34),
            indexed_effect(3, SICKNESS_SPELL_ID as u32, 3, 2, 159, 35),
        ],
        vec![difficulty(0, 3), difficulty(3, 0)],
    );
    let (mut aura, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    aura.recalculate_mask = 0;
    assert_eq!(
        zero_session.load_represented_character_auras_like_cpp([aura], saved, 0),
        1
    );
    assert_eq!(
        selected_snapshot_effects(&zero_session, SICKNESS_SPELL_ID)
            .iter()
            .map(|effect| effect.3)
            .collect::<Vec<_>>(),
        [33, 34, 35],
        "difficulty zero follows its stored fallback when the store is passed"
    );
}

#[test]
fn selected_load_rejects_missing_difficulty_cycle_and_malformed_metadata_before_removal() {
    let (mut missing_row, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (old, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    assert_eq!(
        missing_row.load_represented_character_auras_like_cpp([old], saved, 0),
        1
    );
    let old = loaded_spell(&missing_row, SICKNESS_SPELL_ID).expect("existing sickness");
    missing_row.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));
    assert!(missing_row
        .catalogs
        .difficulty_store()
        .expect("known DifficultyStore")
        .contains(0));
    assert!(!missing_row
        .catalogs
        .difficulty_store()
        .expect("known DifficultyStore")
        .contains(4));
    let (mut candidate, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 80_000);
    candidate.difficulty = 4;
    assert_eq!(
        missing_row.load_represented_character_auras_like_cpp([candidate], saved, 0),
        0
    );
    let current = loaded_spell(&missing_row, SICKNESS_SPELL_ID)
        .expect("existing aura is preserved");
    assert_eq!(current.0.slot, old.0.slot);
    assert_eq!(current.2.cast_id, old.2.cast_id);
    assert_eq!(authority(&missing_row), Some(false));

    let (mut cyclic, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (old, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    assert_eq!(
        cyclic.load_represented_character_auras_like_cpp([old], saved, 0),
        1
    );
    let old = loaded_spell(&cyclic, SICKNESS_SPELL_ID).expect("existing sickness");
    cyclic.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([
        difficulty(4, 3),
        difficulty(3, 4),
        difficulty(0, 0),
    ])));
    let (mut candidate, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 80_000);
    candidate.difficulty = 4;
    assert_eq!(
        cyclic.load_represented_character_auras_like_cpp([candidate], saved, 0),
        0
    );
    let current = loaded_spell(&cyclic, SICKNESS_SPELL_ID).expect("existing aura is preserved");
    assert_eq!(current.0.slot, old.0.slot);
    assert_eq!(current.2.cast_id, old.2.cast_id);
    assert_eq!(authority(&cyclic), Some(false));

    let (mut malformed, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (old, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    assert_eq!(
        malformed.load_represented_character_auras_like_cpp([old], saved, 0),
        1
    );
    let old = loaded_spell(&malformed, SICKNESS_SPELL_ID).expect("existing sickness");
    let mut attributes = [0; 15];
    attributes[0] = SPELL_ATTR0_AURA_IS_DEBUFF;
    install_indexed_sickness_data(
        &mut malformed,
        vec![selected_misc(
            1,
            SICKNESS_SPELL_ID as u32,
            0,
            427,
            attributes,
        )],
        vec![
            indexed_effect(1, SICKNESS_SPELL_ID as u32, 0, 0, 80, 10),
            indexed_effect(2, SICKNESS_SPELL_ID as u32, 0, 1, 79, 20),
            indexed_effect(3, SICKNESS_SPELL_ID as u32, 0, 2, 159, 30),
            indexed_effect(4, SICKNESS_SPELL_ID as u32, 0, 32, 80, 99),
        ],
        vec![difficulty(0, 0)],
    );
    let (candidate, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 80_000);
    assert_eq!(
        malformed.load_represented_character_auras_like_cpp([candidate], saved, 0),
        0
    );
    let current = loaded_spell(&malformed, SICKNESS_SPELL_ID)
        .expect("malformed metadata preserves the existing aura");
    assert_eq!(current.0.slot, old.0.slot);
    assert_eq!(current.2.cast_id, old.2.cast_id);
    assert_eq!(authority(&malformed), Some(false));
}

#[test]
fn selected_load_expiry_uses_whole_debuff_polarity_without_caster_resolution() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    install_positive_sickness_effects_with_debuff(&mut session);
    let store = session
        .catalogs
        .spell_store()
        .expect("selected SpellStore");
    assert!(store
        .get(SICKNESS_SPELL_ID)
        .expect("selected SpellInfo")
        .effects()
        .iter()
        .all(|effect| effect.effect_base_points > 0));
    assert_eq!(
        store
            .misc_attributes_for_difficulty_like_cpp(SICKNESS_SPELL_ID, 0, None)
            .expect("selected SpellInfo attributes")[0]
            & SPELL_ATTR0_AURA_IS_DEBUFF,
        SPELL_ATTR0_AURA_IS_DEBUFF
    );
    assert_eq!(
        session
            .catalogs
            .spell_catalogs
            .spell_misc_store()
            .expect("selected SpellMisc store")
            .entries_like_cpp()
            .find(|row| row.spell_id == SICKNESS_SPELL_ID as u32 && row.difficulty_id == 0)
            .expect("selected base SpellMisc row")
            .attributes[0] as u32
            & SPELL_ATTR0_AURA_IS_DEBUFF,
        SPELL_ATTR0_AURA_IS_DEBUFF
    );
    let missing_caster = ObjectGuid::create_player(1, 99_991);
    let (mut aura, mut saved) = keyed_rows(
        player,
        missing_caster,
        ObjectGuid::EMPTY,
        0b111,
        50_000,
    );
    aura.recalculate_mask = 0;
    for effect in &mut saved {
        effect.amount = 50;
    }
    session.set_creature_aggro_config_like_cpp(Default::default());

    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], saved, 60),
        0
    );
    assert!(loaded_sickness(&session).is_empty());
    assert_eq!(authority(&session), Some(true));
}

#[test]
fn selected_load_normalizes_full_proc_charges_with_source_truncation() {
    for (full_charges, saved_charges, expected) in
        [(256, 9, 9), (256, 0, 0), (-1, 0, 255), (0, 9, 0)]
    {
        let (mut session, player, _) =
            canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
        session.set_spell_aura_options_store(Arc::new(
            wow_data::SpellAuraOptionsStore::from_entries([wow_data::SpellAuraOptionsEntry {
                id: 1,
                difficulty_id: 0,
                cumulative_aura: 0,
                proc_category_recovery: 0,
                proc_chance: 0,
                proc_charges: full_charges,
                spell_procs_per_minute_id: 0,
                proc_type_mask: [0; 2],
                spell_id: SICKNESS_SPELL_ID as u32,
            }]),
        ));
        let (mut aura, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
        aura.recalculate_mask = 0;
        aura.remain_charges = saved_charges;
        assert_eq!(
            session.load_represented_character_auras_like_cpp([aura], saved, 0),
            1
        );
        assert_eq!(
            loaded_sickness(&session)[0].1.charges,
            expected,
            "full ProcCharges {full_charges} and saved charges {saved_charges}"
        );
    }
}

#[test]
fn selected_load_preflights_metadata_and_caster_before_conflict_removal() {
    let (mut missing_metadata, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (old, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    assert_eq!(
        missing_metadata.load_represented_character_auras_like_cpp([old], saved, 0),
        1
    );
    let old = loaded_spell(&missing_metadata, SICKNESS_SPELL_ID).expect("existing sickness");
    missing_metadata.catalogs.spell_catalogs.spell_misc_store = None;
    let (candidate, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 80_000);
    assert_eq!(
        missing_metadata.load_represented_character_auras_like_cpp([candidate], saved, 0),
        0
    );
    let current = loaded_spell(&missing_metadata, SICKNESS_SPELL_ID)
        .expect("missing selected metadata preserves the existing aura");
    assert_eq!(current.0.slot, old.0.slot);
    assert_eq!(current.2.cast_id, old.2.cast_id);
    assert_eq!(authority(&missing_metadata), Some(false));

    let (mut missing_caster, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (old, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    assert_eq!(
        missing_caster.load_represented_character_auras_like_cpp([old], saved, 0),
        1
    );
    let old = loaded_spell(&missing_caster, SICKNESS_SPELL_ID).expect("existing sickness");
    let foreign = ObjectGuid::create_player(1, 99_992);
    let (candidate, saved) = keyed_rows(player, foreign, ObjectGuid::EMPTY, 0b111, 80_000);
    assert_eq!(
        missing_caster.load_represented_character_auras_like_cpp([candidate], saved, 0),
        0
    );
    let current = loaded_spell(&missing_caster, SICKNESS_SPELL_ID)
        .expect("missing caster facts preserve the existing aura");
    assert_eq!(current.0.slot, old.0.slot);
    assert_eq!(current.2.cast_id, old.2.cast_id);
    assert_eq!(authority(&missing_caster), Some(false));
}

#[test]
fn selected_load_removes_conflicts_before_known_save_exclusions() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let (old, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 90_000);
    assert_eq!(
        session.load_represented_character_auras_like_cpp([old], saved, 0),
        1
    );
    let mut attributes = [0; 15];
    attributes[0] = SPELL_ATTR0_AURA_IS_DEBUFF;
    attributes[1] = wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED;
    replace_spell_attributes(&mut session, SICKNESS_SPELL_ID, attributes);
    let (candidate, saved) = keyed_rows(player, player, ObjectGuid::EMPTY, 0b111, 80_000);

    assert_eq!(
        session.load_represented_character_auras_like_cpp([candidate], saved, 0),
        0
    );
    assert!(loaded_sickness(&session).is_empty());
    assert_eq!(authority(&session), Some(true));
}

fn wisp_info() -> wow_data::SpellInfo {
    spell_info(
        20_584,
        vec![
            sickness_effect(
                0,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                0,
                0,
            ),
            sickness_effect(
                1,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                0,
                0,
            ),
            sickness_effect(
                2,
                wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
                0,
                0,
            ),
        ],
    )
}

#[test]
fn wisp_load_uses_selected_negative_flags_and_self_provenance() {
    let (mut session, player, _) = canonical_aura_session([wisp_info()], 0);
    let mut attributes = [0; 15];
    attributes[0] = SPELL_ATTR0_AURA_IS_DEBUFF;
    replace_spell_attributes(&mut session, 20_584, attributes);
    let mut misc_rows = session
        .catalogs
        .spell_catalogs
        .spell_misc_store()
        .expect("Wisp SpellMisc store")
        .entries_like_cpp()
        .cloned()
        .collect::<Vec<_>>();
    let wisp_misc = misc_rows
        .iter_mut()
        .find(|row| row.spell_id == 20_584)
        .expect("Wisp SpellMisc row");
    wisp_misc.duration_index = 428;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 428,
            duration: -1,
            duration_per_level: 0,
            max_duration: -1,
        },
    ])));
    session
        .core
        .with_owned_player_mut_like_cpp(|owner| owner.unit_mut().set_level(55))
        .expect("canonical Player owner");
    let aura = crate::session::CharacterAuraRowLikeCpp {
        caster_guid: player,
        item_guid: ObjectGuid::EMPTY,
        spell_id: 20_584,
        effect_mask: 0b011,
        recalculate_mask: 0,
        difficulty: 0,
        stack_count: 1,
        max_duration_ms: -1,
        remain_time_ms: -1,
        remain_charges: 0,
        cast_item_id: 0,
        cast_item_level: 0,
    };
    let saved = saved_effects(20_584, player, 0b011, 7)
        .into_iter()
        .filter(|effect| effect.effect_index < 2)
        .collect::<Vec<_>>();

    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], saved, 0),
        1
    );
    let loaded = loaded_spell(&session, 20_584).expect("loaded Wisp");
    assert_eq!(loaded.0.aura_flags, 0x0011);
    assert_eq!(loaded.2.caster_level, Some(55));
    assert_eq!(loaded.0.aura_flags & 0x0004, 0);
    assert_eq!(authority(&session), Some(true));
}

fn install_mixed_sickness_metadata(session: &mut WorldSession) {
    let existing = session
        .catalogs
        .spell_store()
        .cloned()
        .expect("sickness SpellStore");
    let mut info = existing
        .get(SICKNESS_SPELL_ID)
        .cloned()
        .expect("sickness SpellInfo");
    for (index, points) in [(0, 80), (1, -79), (2, 159)] {
        let effect = info
            .effects
            .iter_mut()
            .find(|effect| effect.effect_index == index)
            .expect("selected sickness effect index");
        effect.effect_base_points = points;
        effect.polarity_metadata = Some(wow_data::SpellEffectPolarityMetadata::default());
    }
    let mut attributes = [0; 15];
    let mut spells = wow_data::SpellStore::new();
    spells.insert(SICKNESS_SPELL_ID, info);
    spells.insert_spell_misc_attributes_like_cpp(SICKNESS_SPELL_ID, attributes);
    session.set_spell_store(Arc::new(spells));
    let mut misc_rows = session
        .catalogs
        .spell_catalogs
        .spell_misc_store()
        .expect("sickness SpellMisc store")
        .entries_like_cpp()
        .cloned()
        .collect::<Vec<_>>();
    let misc = misc_rows
        .iter_mut()
        .find(|row| row.spell_id == SICKNESS_SPELL_ID as u32 && row.difficulty_id == 0)
        .expect("sickness base SpellMisc row");
    misc.attributes = attributes.map(|attribute| attribute as i32);
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_spell_class_options_store(Arc::new(
        wow_data::SpellClassOptionsStore::from_entries([]),
    ));
    session.catalogs.factions.template_store = Some(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([]),
    ));
}

#[test]
fn selected_load_resolves_foreign_mixed_polarity_from_the_map_relation() {
    let (mut session, player, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    install_mixed_sickness_metadata(&mut session);
    let owner = session
        .core
        .player_handle_like_cpp
        .expect("canonical sickness Player handle");
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical MapManager");
    let foreign = ObjectGuid::create_player(1, 99_993);
    let group = ObjectGuid::create_group(99_994);
    let caster_handle = {
        let mut manager = manager.lock().expect("canonical MapManager");
        let mut caster = Box::new(wow_entities::Player::new(Some(99_993), false));
        caster.unit_mut().world_mut().object_mut().create(foreign);
        caster.unit_mut().set_level(44);
        caster.unit_mut().set_unit_flags_like_cpp(
            wow_constants::unit::UnitFlags::PLAYER_CONTROLLED,
        );
        let caster_handle = manager
            .install_detached_player_like_cpp(caster)
            .expect("foreign caster Player");
        manager
            .attach_player_like_cpp(
                caster_handle,
                wow_map::MapKey::new(571, 0),
                wow_core::Position::ZERO,
            )
            .expect("same-map foreign caster");
        manager
            .with_player_mut_like_cpp(owner, |target| {
                target.unit_mut().set_unit_flags_like_cpp(
                    wow_constants::unit::UnitFlags::PLAYER_CONTROLLED,
                );
                target.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
                    group_guid: group,
                    leader_guid: player,
                    role_mask: 0,
                    subgroup: 0,
                }));
            })
            .expect("canonical target Player");
        manager
            .with_player_mut_like_cpp(caster_handle, |source| {
                source.set_group_like_cpp(Some(wow_entities::PlayerGroupState {
                    group_guid: group,
                    leader_guid: player,
                    role_mask: 0,
                    subgroup: 0,
                }));
            })
            .expect("foreign caster Player");
        caster_handle
    };
    assert!(manager
        .lock()
        .expect("canonical MapManager")
        .with_player_like_cpp(caster_handle, |caster| caster.guid() == foreign)
        .expect("foreign caster owner"));
    let (aura, saved) = keyed_rows(player, foreign, ObjectGuid::EMPTY, 0b111, 90_000);

    assert_eq!(
        session.load_represented_character_auras_like_cpp([aura], saved, 0),
        1
    );
    let loaded = loaded_spell(&session, SICKNESS_SPELL_ID).expect("loaded mixed sickness");
    assert_eq!(loaded.0.caster_guid, foreign);
    assert_eq!(loaded.0.aura_flags, 0x0102);
    assert_eq!(loaded.2.caster_level, Some(44));
    assert_eq!(authority(&session), Some(true));
}
