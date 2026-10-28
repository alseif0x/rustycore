use super::*;

fn effect_row(
    id: u32,
    spell_id: i32,
    difficulty_id: u8,
    effect_index: u32,
    effect: u32,
    aura_type: i32,
    base_points: i32,
    target_1: i16,
) -> wow_data::spell_db2::SpellEffectDb2Entry {
    wow_data::spell_db2::SpellEffectDb2Entry {
        id,
        difficulty_id: i32::from(difficulty_id),
        effect_index: i32::try_from(effect_index).unwrap(),
        effect,
        effect_amplitude: 0.0,
        effect_attributes: 0,
        effect_aura: aura_type as i16,
        effect_aura_period: 0,
        effect_base_points: base_points,
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
        effect_misc_value: [0; 2],
        effect_radius_index: [0; 2],
        effect_spell_class_mask: [0; 4],
        implicit_target: [target_1, 0],
        spell_id: u32::try_from(spell_id).unwrap(),
    }
}

fn misc_row(
    id: u32,
    spell_id: i32,
    difficulty_id: u8,
    attributes: [u32; 15],
) -> wow_data::spell_db2::SpellMiscEntry {
    wow_data::spell_db2::SpellMiscEntry {
        id,
        difficulty_id,
        attributes: attributes.map(|word| word as i32),
        spell_id: u32::try_from(spell_id).unwrap(),
        ..Default::default()
    }
}

fn spell_store(
    misc: Vec<wow_data::spell_db2::SpellMiscEntry>,
    effects: Vec<wow_data::spell_db2::SpellEffectDb2Entry>,
) -> wow_data::SpellStore {
    let keys = misc
        .iter()
        .map(|row| (row.spell_id, row.difficulty_id))
        .chain(effects.iter().map(|row| {
            (
                row.spell_id,
                u8::try_from(row.difficulty_id)
                    .expect("selected SpellEffect difficulty fits its canonical key"),
            )
        }))
        .collect::<Vec<_>>();
    spell_store_with_keys(keys, misc, effects)
}

fn spell_store_with_keys(
    keys: impl IntoIterator<Item = (u32, u8)>,
    misc: Vec<wow_data::spell_db2::SpellMiscEntry>,
    effects: Vec<wow_data::spell_db2::SpellEffectDb2Entry>,
) -> wow_data::SpellStore {
    spell_store_with_seed(
        crate::selected_spell_fixtures::canonical_spell_store(keys),
        misc,
        effects,
    )
}

fn spell_store_with_seed(
    spell_store: wow_data::SpellStore,
    misc: Vec<wow_data::spell_db2::SpellMiscEntry>,
    effects: Vec<wow_data::spell_db2::SpellEffectDb2Entry>,
) -> wow_data::SpellStore {
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
        wow_data::spell_db2::SpellMiscStore::from_entries(misc),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries(effects),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    spell_store.hydrate_effective_core_db2_like_cpp(stores)
}

fn difficulty(id: u32, fallback: u8) -> wow_data::difficulty::DifficultyEntry {
    wow_data::difficulty::DifficultyEntry {
        id,
        instance_type: 0,
        flags: 0,
        fallback_difficulty_id: fallback,
        toggle_difficulty_id: 0,
    }
}

fn active_attributes() -> [u32; 15] {
    [0; 15]
}

fn sickness_base_effects(spell_id: i32) -> Vec<wow_data::spell_db2::SpellEffectDb2Entry> {
    [(0, 80, 10), (1, 79, 20), (2, 159, 30)]
        .into_iter()
        .enumerate()
        .map(|(id, (index, aura, points))| {
            effect_row(
                id as u32 + 1,
                spell_id,
                0,
                index,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                aura,
                points,
                0,
            )
        })
        .collect()
}

fn install_sickness_data(
    session: &mut WorldSession,
    spell_id: i32,
    misc: Vec<wow_data::spell_db2::SpellMiscEntry>,
    effects: Vec<wow_data::spell_db2::SpellEffectDb2Entry>,
    difficulties: impl IntoIterator<Item = wow_data::difficulty::DifficultyEntry>,
) {
    session.set_chr_races_store(std::sync::Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([sickness_race_row(spell_id)]),
    ));
    session.set_spell_store(std::sync::Arc::new(spell_store(misc, effects)));
    session.set_difficulty_store(std::sync::Arc::new(
        wow_data::DifficultyStore::from_entries(difficulties),
    ));
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
}

fn application(
    caster: wow_core::ObjectGuid,
    spell_id: i32,
    slot: u8,
    mask: u32,
    difficulty_id: u8,
    max_duration: i32,
    remaining_duration: i32,
) -> wow_entities::AuraApplicationLikeCpp {
    let mut application = sickness_application(
        caster,
        spell_id,
        slot,
        mask,
        max_duration,
        remaining_duration,
        std::time::Instant::now(),
    );
    application.difficulty_id = difficulty_id;
    application
}

#[test]
fn indexed_sickness_save_uses_each_effect_and_application_attributes() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let spell_id = 54_321;

    let mut base_attributes = [0; 15];
    base_attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    let mut channelled_attributes = [0; 15];
    channelled_attributes[1] = wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED;
    let mut passive_attributes = [0; 15];
    passive_attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    let misc = vec![
        misc_row(1, spell_id, 0, base_attributes),
        misc_row(2, spell_id, 3, channelled_attributes),
        misc_row(3, spell_id, 4, active_attributes()),
        misc_row(4, spell_id, 5, passive_attributes),
    ];
    let mut effects = sickness_base_effects(spell_id);
    effects.extend([
        effect_row(4, spell_id, 3, 0, 0, 80, 999, 0),
        effect_row(
            5,
            spell_id,
            3,
            1,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            79,
            220,
            0,
        ),
        effect_row(
            6,
            spell_id,
            4,
            0,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            80,
            410,
            0,
        ),
        effect_row(
            7,
            spell_id,
            5,
            2,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            159,
            999,
            46,
        ),
    ]);
    install_sickness_data(
        &mut session,
        spell_id,
        misc,
        effects,
        [
            difficulty(0, 0),
            difficulty(3, 0),
            difficulty(4, 3),
            difficulty(5, 3),
        ],
    );

    insert_sickness_application(
        &mut session,
        application(player_guid, spell_id, 1, 0b111, 4, -1, -17),
        Some(vec![(1, 80, 111, 0), (2, 79, -222, 0), (4, 159, 333, 0)]),
    );
    insert_sickness_application(
        &mut session,
        application(player_guid, spell_id, 2, 0b001, 3, 60_000, 45_000),
        None,
    );
    insert_sickness_application(
        &mut session,
        application(player_guid, spell_id, 3, 0b100, 5, 60_000, 45_000),
        None,
    );

    let saved = capture_sickness(&mut session)
        .expect("the active application resolves through indexed selected data");
    assert_eq!(saved.families.len(), 1);
    assert_eq!(saved.families[0].auras.len(), 1);
    let aura = &saved.families[0].auras[0];
    assert_eq!(aura.difficulty, 4);
    assert_eq!((aura.max_duration, aura.remaining_duration), (-1, -17));
    assert_eq!(aura.recalculate_mask, 0b111);
    assert_eq!(
        aura.effects,
        vec![
            wow_persistence::PlayerLifecycleAuraEffectSave {
                index: 0,
                amount: 111,
                base_amount: 410,
            },
            wow_persistence::PlayerLifecycleAuraEffectSave {
                index: 1,
                amount: -222,
                base_amount: 220,
            },
            wow_persistence::PlayerLifecycleAuraEffectSave {
                index: 2,
                amount: 333,
                base_amount: 30,
            },
        ]
    );
}

#[test]
fn indexed_sickness_save_uses_known_difficulty_zero_fallback() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let spell_id = 54_322;
    let effects = [(0, 80, 310), (1, 79, 320), (2, 159, 330)]
        .into_iter()
        .enumerate()
        .map(|(id, (index, aura, points))| {
            effect_row(
                id as u32 + 1,
                spell_id,
                3,
                index,
                wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                aura,
                points,
                0,
            )
        })
        .collect::<Vec<_>>();
    let attributes = active_attributes();
    let misc = vec![misc_row(1, spell_id, 3, attributes)];
    let mut store = spell_store(misc, effects);
    store.insert(
        spell_id,
        sickness_spell_info(spell_id, &[(0, 80, 10), (1, 79, 20), (2, 159, 30)]),
    );
    session.set_chr_races_store(std::sync::Arc::new(
        wow_data::character_progression::ChrRacesStore::from_entries([sickness_race_row(spell_id)]),
    ));
    session.set_spell_store(std::sync::Arc::new(store));
    session.set_difficulty_store(std::sync::Arc::new(
        wow_data::DifficultyStore::from_entries([difficulty(0, 3)]),
    ));
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.unit_mut().set_race(2);
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    insert_sickness_application(
        &mut session,
        application(player_guid, spell_id, 1, 0b111, 0, 60_000, 45_000),
        Some(vec![(1, 80, 11, 0), (2, 79, 22, 0), (4, 159, 33, 0)]),
    );

    let saved =
        capture_sickness(&mut session).expect("difficulty zero follows its known overlay fallback");
    assert_eq!(saved.families.len(), 1);
    assert_eq!(
        saved.families[0].auras[0]
            .effects
            .iter()
            .map(|effect| effect.base_amount)
            .collect::<Vec<_>>(),
        [310, 320, 330]
    );
}

#[test]
fn indexed_sickness_inert_effect_withholds_the_selected_snapshot() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let spell_id = 54_323;
    let mut effects = sickness_base_effects(spell_id);
    effects.push(effect_row(4, spell_id, 4, 0, 0, 80, 999, 0));
    install_sickness_data(
        &mut session,
        spell_id,
        vec![
            misc_row(1, spell_id, 0, active_attributes()),
            misc_row(2, spell_id, 4, active_attributes()),
        ],
        effects,
        [difficulty(0, 0), difficulty(4, 0)],
    );
    insert_sickness_application(
        &mut session,
        application(player_guid, spell_id, 1, 0b001, 4, 60_000, 45_000),
        Some(vec![(1, 80, 11, 0)]),
    );

    assert_eq!(capture_sickness(&mut session), None);
}

#[test]
fn indexed_sickness_missing_and_cyclic_fallbacks_withhold_the_snapshot() {
    for (spell_id, misc, difficulties) in [
        (
            54_324,
            vec![misc_row(1, 54_324, 0, active_attributes())],
            vec![difficulty(0, 0), difficulty(4, 3)],
        ),
        (
            54_325,
            vec![
                misc_row(1, 54_325, 0, active_attributes()),
                misc_row(2, 54_325, 4, active_attributes()),
            ],
            vec![difficulty(0, 0), difficulty(4, 3), difficulty(3, 4)],
        ),
    ] {
        let (mut session, _, _) = make_session();
        let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
        install_sickness_data(
            &mut session,
            spell_id,
            misc,
            sickness_base_effects(spell_id),
            difficulties,
        );
        insert_sickness_application(
            &mut session,
            application(player_guid, spell_id, 1, 0b001, 4, 60_000, 45_000),
            Some(vec![(1, 80, 11, 0)]),
        );

        assert_eq!(capture_sickness(&mut session), None);
    }
}

#[test]
fn ghost_wisp_save_uses_indexed_rows_and_application_attributes() {
    let (mut session, _, _) = make_session();
    let player_guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let ghost_spell = 8326;
    let wisp_spell = 20_584;
    let mut ghost_base_attributes = [0; 15];
    ghost_base_attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    let misc = vec![
        misc_row(1, ghost_spell, 0, ghost_base_attributes),
        misc_row(2, ghost_spell, 4, active_attributes()),
        misc_row(3, wisp_spell, 0, active_attributes()),
    ];
    let ghost_effects = [
        (0, wow_data::spell::aura_types::SPELL_AURA_GHOST, 10),
        (
            1,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            20,
        ),
        (
            2,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            30,
        ),
    ];
    let wisp_effects = [
        (
            0,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            40,
        ),
        (
            1,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            50,
        ),
        (2, wow_data::spell::aura_types::SPELL_AURA_TRANSFORM, 60),
    ];
    let mut effects = Vec::new();
    for (id, (index, aura, points)) in ghost_effects.into_iter().enumerate() {
        effects.push(effect_row(
            id as u32 + 1,
            ghost_spell,
            0,
            index,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            aura,
            points,
            0,
        ));
    }
    effects.extend([
        effect_row(
            4,
            ghost_spell,
            3,
            1,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            320,
            0,
        ),
        effect_row(
            5,
            ghost_spell,
            4,
            0,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            wow_data::spell::aura_types::SPELL_AURA_GHOST,
            410,
            0,
        ),
    ]);
    for (id, (index, aura, points)) in wisp_effects.into_iter().enumerate() {
        effects.push(effect_row(
            id as u32 + 6,
            wisp_spell,
            0,
            index,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            aura,
            points,
            0,
        ));
    }
    session.set_spell_store(std::sync::Arc::new(spell_store_with_keys(
        [
            (ghost_spell as u32, 0),
            (ghost_spell as u32, 4),
            (wisp_spell as u32, 0),
        ],
        misc,
        effects,
    )));
    session.set_difficulty_store(std::sync::Arc::new(
        wow_data::DifficultyStore::from_entries([
            difficulty(0, 0),
            difficulty(3, 0),
            difficulty(4, 3),
        ]),
    ));
    session
        .with_owned_player_mut_like_cpp(|player| {
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .unwrap();
    insert_sickness_application(
        &mut session,
        application(player_guid, ghost_spell, 1, 0b111, 4, 60_000, 45_000),
        Some(vec![
            (1, wow_data::spell::aura_types::SPELL_AURA_GHOST, 11, 0),
            (
                2,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                22,
                0,
            ),
            (
                4,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                33,
                0,
            ),
        ]),
    );
    insert_sickness_application(
        &mut session,
        application(player_guid, wisp_spell, 2, 0b111, 0, 60_000, 45_000),
        Some(vec![
            (
                1,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
                44,
                0,
            ),
            (
                2,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
                55,
                0,
            ),
            (4, wow_data::spell::aura_types::SPELL_AURA_TRANSFORM, 66, 0),
        ]),
    );

    let request = session
        .prepare_player_save_like_cpp(1_700_000_000)
        .expect("canonical Player save captures the selected Ghost/Wisp keyset")
        .request;
    let families = request
        .ghost_wisp
        .expect("the selected family catalog and aura authority are complete");
    let ghost = families
        .iter()
        .find(|family| family.spell_id == ghost_spell as u32)
        .expect("Ghost family remains present");
    assert_eq!(ghost.auras.len(), 1);
    assert_eq!(ghost.auras[0].difficulty, 4);
    assert_eq!(
        ghost.auras[0]
            .effects
            .iter()
            .map(|effect| effect.base_amount)
            .collect::<Vec<_>>(),
        [410, 320, 30]
    );
    let wisp = families
        .iter()
        .find(|family| family.spell_id == wisp_spell as u32)
        .expect("Wisp family remains present");
    assert_eq!(wisp.auras.len(), 1);
    assert_eq!(wisp.auras[0].recalculate_mask, 0b011);
    assert_eq!(
        wisp.auras[0]
            .effects
            .iter()
            .map(|effect| effect.base_amount)
            .collect::<Vec<_>>(),
        [40, 50, 60]
    );
}
