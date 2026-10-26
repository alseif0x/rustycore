use super::*;
use std::sync::Arc;
use wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;

const IGNORE_CASTER_MODIFIERS: u32 = 0x2000_0000;
const AURA_UNIQUE: u32 = 0x0000_0800;
const PANDEMIC: u32 = 0x0010_0000;
const GHOST_SPELL: i32 = wow_world_entities::SPELL_GHOST_LIKE_CPP;
const WISP_SPELL: i32 = wow_world_entities::SPELL_WISP_SPIRIT_AURA_LIKE_CPP;

fn effects_for(
    spell_id: i32,
    difficulty_id: u8,
    aura_types: [i32; 3],
    base_points: [i32; 3],
    die_sides: [i32; 3],
    misc_values: [i32; 3],
) -> Vec<wow_data::SpellEffectDb2Entry> {
    aura_types
        .into_iter()
        .zip(base_points)
        .zip(die_sides)
        .zip(misc_values)
        .enumerate()
        .map(|(index, (((aura, points), sides), misc))| {
            let mut row = effect_row(
                spell_id as u32 * 1_000 + u32::from(difficulty_id) * 10 + index as u32 + 1,
                difficulty_id,
                index as u32,
                SPELL_EFFECT_APPLY_AURA,
                aura,
                points,
            );
            row.spell_id = spell_id as u32;
            row.effect_die_sides = sides;
            row.effect_misc_value = [misc, 0];
            row
        })
        .collect()
}

fn misc_row(
    spell_id: i32,
    difficulty_id: u8,
    duration_index: u16,
    attributes1: u32,
    attributes3: u32,
    attributes13: u32,
) -> wow_data::SpellMiscEntry {
    let mut attributes = [0_u32; 15];
    attributes[0] = 0x2C80_0100;
    attributes[1] = attributes1;
    attributes[3] = attributes3;
    attributes[13] = attributes13;
    wow_data::SpellMiscEntry {
        id: 80_000 + spell_id as u32 + u32::from(difficulty_id),
        spell_id: spell_id as u32,
        difficulty_id,
        duration_index,
        attributes: attributes.map(|value| value as i32),
        ..Default::default()
    }
}

fn install_aura_options(session: &mut WorldSession, spell_id: i32, rows: &[(u8, u32, i32)]) {
    let mut entries = session
        .catalogs
        .spell_catalogs
        .spell_aura_options_store
        .as_deref()
        .map(|store| {
            store
                .entries_like_cpp()
                .filter(|entry| entry.spell_id != spell_id as u32)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    entries.extend(
        rows.iter()
            .map(|&(difficulty_id, cumulative_aura, proc_charges)| {
                wow_data::SpellAuraOptionsEntry {
                    id: 81_000 + spell_id as u32 + u32::from(difficulty_id),
                    difficulty_id,
                    cumulative_aura,
                    proc_category_recovery: 0,
                    proc_chance: 0,
                    proc_charges,
                    spell_procs_per_minute_id: 0,
                    proc_type_mask: [wow_data::spell::PROC_FLAG_KILL_LIKE_CPP as i32, 0],
                    spell_id: spell_id as u32,
                }
            }),
    );
    session.set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries(
        entries,
    )));
}

fn selected_aura_state(
    session: &WorldSession,
    spell_id: i32,
) -> (
    wow_entities::AuraApplicationLikeCpp,
    wow_entities::AuraPersistenceMetadata,
    wow_entities::AuraCastProvenanceLikeCpp,
    Vec<(u32, i32, i32, i32)>,
) {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let application = auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.spell_id == spell_id)
                .expect("selected AuraApplication")
                .clone();
            let metadata = auras
                .aura_persistence_metadata(application.slot)
                .cloned()
                .expect("selected Aura persistence metadata");
            let provenance = auras.aura_cast_provenance_like_cpp(application.slot);
            let effects = auras
                .threat_snapshot_like_cpp(application.slot)
                .expect("selected Aura effect snapshot")
                .effects()
                .to_vec();
            (application, metadata, provenance, effects)
        })
        .expect("canonical Player")
}

fn sickness_rows(
    old_unique: bool,
    new_unique: bool,
    old_attributes13: u32,
) -> (
    Vec<wow_data::spell_db2::SpellEffectDb2Entry>,
    Vec<wow_data::SpellMiscEntry>,
) {
    let aura_types = [
        wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
        wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT,
    ];
    let mut effects = effects_for(
        SICKNESS_SPELL,
        0,
        aura_types,
        [-40, -50, -60],
        [0, 1, 0],
        [-1, 701, 703],
    );
    effects.extend(effects_for(
        SICKNESS_SPELL,
        3,
        aura_types,
        [-80, -100, -120],
        [0; 3],
        [-1, 902, 903],
    ));
    let old_attributes1 = if old_unique { AURA_UNIQUE } else { 0 };
    let new_attributes1 = if new_unique { AURA_UNIQUE } else { 0 };
    let misc = vec![
        misc_row(
            SICKNESS_SPELL,
            0,
            1,
            old_attributes1,
            IGNORE_CASTER_MODIFIERS,
            old_attributes13,
        ),
        misc_row(
            SICKNESS_SPELL,
            3,
            2,
            new_attributes1,
            IGNORE_CASTER_MODIFIERS,
            0,
        ),
    ];
    (effects, misc)
}

async fn sickness_refresh_session(
    guid_counter: i64,
    old_unique: bool,
    new_unique: bool,
    old_attributes13: u32,
    old_duration: i32,
    new_duration: i32,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let (effects, misc) = sickness_rows(old_unique, new_unique, old_attributes13);
    let (mut session, send_rx, player_guid) =
        super::selected_execution_effects::selected_session_with_raw_misc_rows(
            guid_counter,
            0,
            effects,
            vec![difficulty(0, 3), difficulty(3, 0)],
            misc,
        )
        .await;
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 1,
            duration: old_duration,
            duration_per_level: 0,
            max_duration: old_duration,
        },
        wow_data::SpellDurationEntry {
            id: 2,
            duration: new_duration,
            duration_per_level: 0,
            max_duration: new_duration,
        },
    ])));
    install_aura_options(&mut session, SICKNESS_SPELL, &[(0, 0, 2), (3, 0, 9)]);
    session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp::default()));
    (session, send_rx, player_guid)
}

#[tokio::test]
async fn sickness_refresh_uses_old_policy_duration_and_effect_rows() {
    for (index, old_unique, new_unique) in [(0, true, false), (1, false, true)] {
        let (mut session, _send_rx, player_guid) =
            sickness_refresh_session(46_001 + index, old_unique, new_unique, 0, 60_000, 120_000)
                .await;
        let _packet_peer = keep_recovery_input_open(&mut session);
        super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
            .await
            .expect("create sickness from its difficulty-zero SpellInfo");
        let before = visible_sickness_aura(&session, SICKNESS_SPELL)
            .into_iter()
            .next()
            .expect("initial sickness Aura");
        let before_state = selected_aura_state(&session, SICKNESS_SPELL);
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                let application = player
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .runtime_application_mut_like_cpp(before.slot)
                    .expect("existing sickness Aura");
                application.duration_remaining = 4_000;
            })
            .expect("canonical Player");

        install_player_on_difficulty_map(&mut session, 3);
        super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
            .await
            .expect("refresh uses the old Aura's stack policy");

        let refreshed = visible_sickness_aura(&session, SICKNESS_SPELL)
            .into_iter()
            .next()
            .expect("refreshed sickness Aura");
        assert_eq!(refreshed.slot, before.slot);
        assert_eq!(
            refreshed.difficulty_id, 0,
            "the original raw cast difficulty remains"
        );
        assert_eq!(
            refreshed.duration_total, 60_000,
            "the old duration row wins"
        );
        assert_eq!(
            refreshed.duration_remaining,
            if old_unique { 4_000 } else { 60_000 }
        );

        let (application, metadata, provenance, effects) =
            selected_aura_state(&session, SICKNESS_SPELL);
        assert_eq!(application.slot, before.slot);
        assert_eq!(application.difficulty_id, 0);
        assert_eq!(provenance, before_state.2);
        assert_eq!(metadata.charges, 2, "charge reset uses old AuraOptions");
        assert_eq!(metadata.base_amounts[1], (1, -100));
        let effect = effects.iter().find(|effect| effect.0 == 0b010).unwrap();
        assert_eq!(effect.2, -99, "old DieSides add one to the new base amount");
        assert_eq!(
            effect.3, 701,
            "old misc data remains attached to the Aura effect"
        );
        assert_ne!(before_state.1.base_amounts[1], metadata.base_amounts[1]);
    }
}

#[tokio::test]
async fn ghost_and_wisp_refresh_copy_only_new_base_points_from_selected_effects() {
    use wow_data::spell::aura_types as aura;

    let mut effects = Vec::new();
    let ghost_types = [
        aura::SPELL_AURA_GHOST,
        aura::SPELL_AURA_MOD_INCREASE_SPEED,
        aura::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
    ];
    effects.extend(effects_for(
        GHOST_SPELL,
        0,
        ghost_types,
        [-20, 49, 49],
        [1; 3],
        [0; 3],
    ));
    effects.extend(effects_for(
        GHOST_SPELL,
        3,
        ghost_types,
        [-30, 89, 89],
        [0; 3],
        [101; 3],
    ));
    let wisp_types = [
        aura::SPELL_AURA_MOD_INCREASE_SPEED,
        aura::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        aura::SPELL_AURA_TRANSFORM,
    ];
    effects.extend(effects_for(
        WISP_SPELL,
        0,
        wisp_types,
        [74, 74, -1],
        [1; 3],
        [0, 0, 12_861],
    ));
    effects.extend(effects_for(
        WISP_SPELL,
        3,
        wisp_types,
        [84, 84, -5],
        [0; 3],
        [201, 202, 99_999],
    ));
    let mut ghost_old = misc_row(GHOST_SPELL, 0, 21, 0x0000_0400, 0x0010_0000, 0);
    ghost_old.attributes[0] = 0x2D80_0100;
    let mut ghost_new = misc_row(GHOST_SPELL, 3, 21, AURA_UNIQUE, 0x0010_0000, 0);
    ghost_new.attributes[0] = 0x2D80_0100;
    let mut wisp_old = misc_row(WISP_SPELL, 0, 21, 0x1000_0400, 0x0010_0000, 0);
    wisp_old.attributes[0] = 0x2D80_0100;
    let mut wisp_new = misc_row(WISP_SPELL, 3, 21, 0x1000_0C00, 0x0010_0000, 0);
    wisp_new.attributes[0] = 0x2D80_0100;
    let (mut session, _send_rx, player_guid) =
        super::selected_execution_effects::selected_session_with_raw_misc_rows(
            46_011,
            0,
            effects,
            vec![difficulty(0, 3), difficulty(3, 0)],
            vec![ghost_old, ghost_new, wisp_old, wisp_new],
        )
        .await;
    install_aura_options(&mut session, GHOST_SPELL, &[(0, 0, 0), (3, 0, 9)]);
    install_aura_options(&mut session, WISP_SPELL, &[(0, 0, 0), (3, 0, 8)]);
    session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp::default()));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 21,
            duration: -1,
            duration_per_level: 0,
            max_duration: -1,
        },
    ])));
    let _packet_peer = keep_recovery_input_open(&mut session);

    let ghost_before = selected_aura_state(&session, GHOST_SPELL);
    super::selected_charges::cast_full_selected_spell(&mut session, WISP_SPELL, player_guid, None)
        .await
        .expect("create Wisp from its difficulty-zero selected effects");
    let wisp_before = selected_aura_state(&session, WISP_SPELL);
    install_player_on_difficulty_map(&mut session, 3);
    for spell_id in [GHOST_SPELL, WISP_SPELL] {
        super::selected_charges::cast_full_selected_spell(
            &mut session,
            spell_id,
            player_guid,
            None,
        )
        .await
        .expect("refresh selected Ghost or Wisp from the new difficulty");
    }

    let ghost_after = selected_aura_state(&session, GHOST_SPELL);
    assert_eq!(ghost_after.0.slot, ghost_before.0.slot);
    assert_eq!(ghost_after.0.difficulty_id, 0);
    assert_eq!(ghost_after.2, ghost_before.2);
    assert_eq!(ghost_after.1.base_amounts[0], (0, -30));
    assert_eq!(
        ghost_after.3.iter().find(|effect| effect.0 == 1).unwrap().2,
        -29
    );
    assert_eq!(
        ghost_after.3.iter().find(|effect| effect.0 == 1).unwrap().3,
        0
    );

    let wisp_after = selected_aura_state(&session, WISP_SPELL);
    assert_eq!(wisp_after.0.slot, wisp_before.0.slot);
    assert_eq!(wisp_after.0.difficulty_id, 0);
    assert_eq!(wisp_after.2, wisp_before.2);
    assert_eq!(wisp_after.1.base_amounts[0], (0, 84));
    assert_eq!(
        wisp_after.3.iter().find(|effect| effect.0 == 1).unwrap().2,
        85
    );
    assert_eq!(
        wisp_after.3.iter().find(|effect| effect.0 == 1).unwrap().3,
        0
    );
    assert_eq!(
        wisp_after.3.iter().find(|effect| effect.0 == 4).unwrap().3,
        12_861
    );
}

#[tokio::test]
async fn non_timer_refresh_does_not_require_proc_or_duration_catalogs() {
    let (mut session, _send_rx, _) =
        sickness_refresh_session(46_021, true, false, 0, 60_000, 120_000).await;
    session.catalogs.spell_catalogs.spell_proc_store = None;
    session.catalogs.spell_catalogs.spell_duration_store = None;
    let player_guid = session.player_guid().expect("canonical Player GUID");
    let spell_info = session
        .catalogs
        .spell_store()
        .and_then(|store| store.get(SICKNESS_SPELL))
        .cloned()
        .expect("canonical base sickness payload");
    let _packet_peer = keep_recovery_input_open(&mut session);

    session
        .player_aura_application_cx_like_cpp()
        .apply_resurrection_sickness_aura(
            SICKNESS_SPELL,
            player_guid,
            &spell_info,
            60_000,
            0,
            AURA_UNIQUE,
            0,
            Ok((
                0x0011,
                wow_entities::AuraCastProvenanceLikeCpp::default(),
                7,
            )),
        )
        .expect("create the source Aura without duration or proc reads");
    let before = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("initial sickness Aura");
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(before.slot)
                .expect("initial sickness Aura")
                .duration_remaining = 4_000;
        })
        .expect("canonical Player");

    session
        .player_aura_application_cx_like_cpp()
        .apply_resurrection_sickness_aura(
            SICKNESS_SPELL,
            player_guid,
            &spell_info,
            -1,
            1,
            0,
            0,
            Err("fresh creation must be ignored on a matching Aura"),
        )
        .expect("old non-timer policy ignores fresh stack and duration facts");
    let after = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("refreshed sickness Aura");
    assert_eq!(after.duration_total, 60_000);
    assert_eq!(after.duration_remaining, 4_000);
    assert_eq!(selected_aura_state(&session, SICKNESS_SPELL).1.charges, 7);
}

#[tokio::test]
async fn pandemic_refresh_uses_old_attr13_and_rejects_overflow_before_mutation() {
    for (index, current_remaining, expected) in [(0, 900, 1_300), (1, 2_000, 2_000)] {
        let (mut session, _send_rx, player_guid) =
            sickness_refresh_session(46_025 + index, false, false, PANDEMIC, 1_000, 5_000).await;
        let _packet_peer = keep_recovery_input_open(&mut session);
        super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
            .await
            .expect("create old periodic sickness Aura");
        let before = visible_sickness_aura(&session, SICKNESS_SPELL)
            .into_iter()
            .next()
            .expect("periodic sickness Aura");
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .runtime_application_mut_like_cpp(before.slot)
                    .expect("periodic sickness Aura")
                    .duration_remaining = current_remaining;
            })
            .expect("canonical Player");
        install_player_on_difficulty_map(&mut session, 3);
        super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
            .await
            .expect("pandemic duration comes from the old Aura's Attr13 and base duration");
        let after = visible_sickness_aura(&session, SICKNESS_SPELL)
            .into_iter()
            .next()
            .expect("refreshed periodic sickness Aura");
        assert_eq!(after.duration_total, expected);
        assert_eq!(after.duration_remaining, expected);
    }

    let (mut session, _send_rx, player_guid) =
        sickness_refresh_session(46_027, false, false, PANDEMIC, i32::MAX, 5_000).await;
    let _packet_peer = keep_recovery_input_open(&mut session);
    super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("create sickness before testing duration overflow");
    let before = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("sickness Aura before overflow");
    let before_state = selected_aura_state(&session, SICKNESS_SPELL);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(before.slot)
                .expect("sickness Aura before overflow")
                .duration_remaining = i32::MAX;
        })
        .expect("canonical Player");
    install_player_on_difficulty_map(&mut session, 3);
    assert!(
        super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
            .await
            .is_err(),
        "overflow is rejected before the Aura refresh mutates canonical state"
    );
    let after = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("overflow leaves the sickness Aura installed");
    let after_state = selected_aura_state(&session, SICKNESS_SPELL);
    assert_eq!(after.slot, before.slot);
    assert_eq!(after.difficulty_id, 0);
    assert_eq!(after.duration_total, i32::MAX);
    assert_eq!(after.duration_remaining, i32::MAX);
    assert_eq!(after_state.1, before_state.1);
    assert_eq!(after_state.3, before_state.3);
}

async fn sparse_sickness_session(
    guid_counter: i64,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    Vec<wow_data::SpellMiscEntry>,
    Vec<wow_data::spell_db2::SpellEffectDb2Entry>,
) {
    let effects = effects_for(
        SICKNESS_SPELL,
        3,
        [
            wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT,
        ],
        [-40, -50, -60],
        [0; 3],
        [-1, 127, 127],
    );
    let misc = vec![misc_row(
        SICKNESS_SPELL,
        3,
        1,
        0,
        IGNORE_CASTER_MODIFIERS,
        0,
    )];
    let (session, send_rx, player_guid) = super::selected_session_with_visual_inputs(
        guid_counter,
        0,
        effects.clone(),
        vec![difficulty(0, 3), difficulty(3, 0)],
        None,
        Some(misc.clone()),
        3,
    )
    .await;
    (session, send_rx, player_guid, misc, effects)
}

fn install_sparse_sickness_duration_and_charges(session: &mut WorldSession) {
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 1,
            duration: 10_000,
            duration_per_level: 0,
            max_duration: 10_000,
        },
    ])));
    install_aura_options(session, SICKNESS_SPELL, &[(3, 0, 0)]);
    session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp::default()));
}

#[tokio::test]
async fn sparse_nonzero_key_sickness_expires_through_canonical_removal() {
    let (mut session, send_rx, player_guid, _, _) = sparse_sickness_session(46_031).await;
    install_sparse_sickness_duration_and_charges(&mut session);
    let _packet_peer = keep_recovery_input_open(&mut session);
    super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("selected sickness resolves its sparse actual key for admission");
    let aura = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("sparse selected sickness Aura");
    assert_eq!(aura.difficulty_id, 0);
    assert!(
        session
            .catalogs
            .spell_store()
            .expect("SpellStore")
            .contains_spell_info_exact_like_cpp(SICKNESS_SPELL as u32, 3)
    );
    assert!(
        !session
            .catalogs
            .spell_store()
            .expect("SpellStore")
            .contains_spell_info_exact_like_cpp(SICKNESS_SPELL as u32, 0)
    );
    while send_rx.try_recv().is_ok() {}

    let result = wow_world_spell::tick_selected_aura_lifetimes(&mut session, 10_000)
        .expect("sparse selected sickness resolves the same key before removal");
    assert_eq!(result.expired, 1);
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
    assert_eq!(
        drain_packets(&send_rx)
            .iter()
            .filter(|bytes| packet_opcode(bytes) == ServerOpcodes::AuraUpdate as u16)
            .count(),
        1
    );
}

fn rehydrate_sickness_payload_without_any_key(
    session: &mut WorldSession,
    misc: Vec<wow_data::SpellMiscEntry>,
    effects: Vec<wow_data::spell_db2::SpellEffectDb2Entry>,
) {
    let keys = session
        .catalogs
        .spell_store()
        .expect("SpellStore")
        .spell_info_keys_in_order_like_cpp()
        .into_iter()
        .filter(|(spell_id, _)| *spell_id != SICKNESS_SPELL as u32)
        .collect::<Vec<_>>();
    let mut payload_misc = misc.clone();
    payload_misc.extend(misc.iter().cloned().map(|mut row| {
        row.id += 1_000;
        row.difficulty_id = 0;
        row
    }));
    let mut payload_effects = effects.clone();
    payload_effects.extend(effects.iter().cloned().map(|mut row| {
        row.id += 1_000;
        row.difficulty_id = 0;
        row
    }));
    let stores = wow_data::EffectiveCoreSpellDb2StoresLikeCpp::new(
        wow_data::spell_db2::SpellCategoriesStore::from_entries([]),
        wow_data::spell_db2::SpellMiscStore::from_entries(payload_misc),
        wow_data::spell_db2::SpellEffectDb2Store::from_entries(payload_effects),
        wow_data::spell_db2::SpellShapeshiftStore::from_entries([]),
        wow_data::spell_db2::SpellInterruptsStore::from_entries([]),
        wow_data::spell_db2::SpellCastTimesStore::from_entries([]),
        wow_data::spell_db2::SpellCooldownsStore::from_entries([]),
        wow_data::spell_db2::SpellCastingRequirementsStore::from_entries([]),
        wow_data::spell_db2::SpellPowerStore::from_entries([]),
        wow_data::spell_db2::SpellPowerDifficultyStore::from_entries([]),
    );
    session.set_spell_store(Arc::new(
        crate::selected_spell_fixtures::canonical_spell_store(keys)
            .hydrate_effective_core_db2_like_cpp(stores),
    ));
}

#[tokio::test]
async fn selected_sickness_removal_requires_the_original_canonical_key() {
    let (mut session, send_rx, player_guid, misc, effects) = sparse_sickness_session(46_041).await;
    install_sparse_sickness_duration_and_charges(&mut session);
    let _packet_peer = keep_recovery_input_open(&mut session);
    super::selected_charges::cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("create selected sickness with its canonical key");
    let aura = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("active selected sickness Aura");
    while send_rx.try_recv().is_ok() {}

    rehydrate_sickness_payload_without_any_key(&mut session, misc, effects);
    let spell_store = session.catalogs.spell_store().expect("hydrated SpellStore");
    assert!(spell_store.get(SICKNESS_SPELL).is_some());
    assert!(!spell_store.contains_spell_info_any_difficulty_like_cpp(SICKNESS_SPELL as u32));
    let result = session
        .player_aura_application_cx_like_cpp()
        .remove_aura_like_cpp(aura.slot);
    assert!(
        result.is_err(),
        "missing key blocks removal before publication or mutation"
    );
    assert_eq!(visible_sickness_aura(&session, SICKNESS_SPELL).len(), 1);
    assert!(
        drain_packets(&send_rx)
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::AuraUpdate as u16)
    );
}
