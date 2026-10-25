//! Canonical Player coverage for the persisted aura restore path.

use super::*;
use std::sync::Arc;
use wow_core::guid::HighGuid;
use wow_packet::ServerPacket;

#[path = "sickness_loading/selected_restore.rs"]
mod selected_restore;

const SICKNESS_SPELL_ID: i32 = 15_007;
const SICKNESS_VISUAL_ROW_ID: u32 = 245_039;

fn sickness_effect(index: u32, aura: i32, base: i32, misc: i32) -> wow_data::SpellEffectInfo {
    wow_data::SpellEffectInfo {
        effect_index: index,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: aura,
        effect_base_points: base,
        effect_die_sides: 1,
        effect_misc_value_1: misc,
        ..Default::default()
    }
}

fn spell_info(spell_id: i32, effects: Vec<wow_data::SpellEffectInfo>) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: effects
            .first()
            .map_or(0, |effect| effect.effect_base_points),
        effect_bonus_coefficient: 0.0,
        aura_type: effects.first().map(|effect| effect.effect_aura),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects,
    }
}

fn sickness_catalog_info() -> wow_data::SpellInfo {
    spell_info(
        SICKNESS_SPELL_ID,
        vec![
            sickness_effect(
                0,
                wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
                -30,
                -1,
            ),
            sickness_effect(
                1,
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
                -25,
                127,
            ),
            sickness_effect(
                2,
                wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT,
                -3,
                127,
            ),
        ],
    )
}

fn canonical_aura_session(
    spell_infos: impl IntoIterator<Item = wow_data::SpellInfo>,
    resurrection_sickness_spell_id: i32,
) -> (WorldSession, ObjectGuid, flume::Receiver<Vec<u8>>) {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);

    let mut race = chr_race_entry(1, 0);
    race.res_sickness_spell_id = resurrection_sickness_spell_id;
    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([race])));
    let mut spells = wow_data::SpellStore::new();
    let mut misc_rows = Vec::new();
    let mut aura_options_rows = Vec::new();
    for info in spell_infos {
        let spell_id = info.spell_id;
        misc_rows.push(wow_data::SpellMiscEntry {
            id: spell_id as u32,
            spell_id: spell_id as u32,
            duration_index: if spell_id == resurrection_sickness_spell_id {
                427
            } else {
                0
            },
            ..Default::default()
        });
        if spell_id == resurrection_sickness_spell_id {
            aura_options_rows.push(wow_data::SpellAuraOptionsEntry {
                id: spell_id as u32,
                difficulty_id: 0,
                cumulative_aura: 0,
                proc_category_recovery: 0,
                proc_chance: 0,
                proc_charges: 3,
                spell_procs_per_minute_id: 0,
                proc_type_mask: [0; 2],
                spell_id: spell_id as u32,
            });
        }
        spells.insert(spell_id, info);
        spells.insert_spell_misc_attributes_like_cpp(spell_id, [0; 15]);
    }
    session.set_spell_store(Arc::new(spells));
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries(misc_rows)));
    session.set_spell_duration_store(Arc::new(wow_data::SpellDurationStore::from_entries([
        wow_data::SpellDurationEntry {
            id: 427,
            duration: -600_000,
            duration_per_level: 0,
            max_duration: 600_000,
        },
    ])));
    session.set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries(
        aura_options_rows,
    )));
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));
    wow_world_spell::set_player_aura_authority_complete_like_cpp(&mut session, true);
    (session, player_guid, send_rx)
}

fn capture_player_save_request(
    session: &WorldSession,
) -> Option<wow_persistence::PlayerCharacterSaveRequestLikeCpp> {
    let (lifecycle, hub) = crate::session::split_lifecycle_ref(session);
    let owner = wow_world_core::session::PlayerSaveOwnerAccessLikeCpp::new_like_cpp(
        hub.core,
        hub.catalogs.talent_store().map(AsRef::as_ref),
        hub.catalogs.spell_catalogs.spell_store().map(AsRef::as_ref),
        hub.catalogs.chr.races_store.as_deref(),
        hub.catalogs.difficulty_store.as_deref(),
    );
    owner
        .capture_like_cpp(lifecycle.player_save_session_inputs_like_cpp(1_700_000_000))
        .map(|captured| captured.into_parts_like_cpp().0)
}

fn sickness_rows(
    player_guid: ObjectGuid,
    remain_time_ms: i32,
) -> (
    crate::session::CharacterAuraRowLikeCpp,
    Vec<crate::session::CharacterAuraEffectRowLikeCpp>,
) {
    let item_guid = ObjectGuid::EMPTY;
    let row = crate::session::CharacterAuraRowLikeCpp {
        caster_guid: player_guid,
        item_guid,
        spell_id: SICKNESS_SPELL_ID as u32,
        effect_mask: 0b111,
        recalculate_mask: 0b111,
        difficulty: 0,
        stack_count: 1,
        max_duration_ms: 600_000,
        remain_time_ms,
        remain_charges: 3,
        cast_item_id: 6948,
        cast_item_level: 80,
    };
    let effects = [(-76, 201), (-76, 202), (-1, 203)]
        .into_iter()
        .enumerate()
        .map(
            |(index, (base_amount, amount))| crate::session::CharacterAuraEffectRowLikeCpp {
                caster_guid: player_guid,
                item_guid,
                spell_id: SICKNESS_SPELL_ID as u32,
                effect_mask: 0b111,
                effect_index: index as u8,
                amount,
                base_amount,
            },
        )
        .collect();
    (row, effects)
}

fn selected_sickness_application(
    caster_guid: ObjectGuid,
    slot: u8,
    effect_mask: u32,
    duration_total: i32,
    duration_remaining: i32,
) -> wow_entities::AuraApplicationLikeCpp {
    wow_entities::AuraApplicationLikeCpp {
        spell_id: SICKNESS_SPELL_ID,
        difficulty_id: 0,
        caster_guid,
        slot,
        duration_total,
        duration_remaining,
        stack_count: 1,
        aura_flags: 0,
        effect_mask,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    }
}

fn insert_runtime_sickness_key(
    session: &mut WorldSession,
    application: wow_entities::AuraApplicationLikeCpp,
    metadata: Option<wow_entities::AuraPersistenceMetadata>,
    effects: Vec<(u32, i32, i32, i32)>,
) {
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let slot = application.slot;
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            auras.insert_runtime_application_like_cpp(application);
            auras.insert_threat_snapshot_like_cpp(
                slot,
                wow_entities::AuraThreatSnapshotLikeCpp::new([0; 2], effects),
            );
            if let Some(metadata) = metadata {
                assert!(auras.set_aura_persistence_metadata(slot, metadata));
            }
            player.set_player_aura_authority_complete_like_cpp(true);
        })
        .expect("canonical Player owner");
}

fn install_sickness_visual(session: &mut WorldSession) {
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [wow_data::SpellXSpellVisualEntry {
                id: SICKNESS_VISUAL_ROW_ID,
                difficulty_id: 0,
                spell_visual_id: 76_543,
                probability: 1.0,
                flags: 0,
                priority: 0,
                spell_icon_file_id: 0,
                active_icon_file_id: 0,
                viewer_unit_condition_id: 0,
                viewer_player_condition_id: 0,
                caster_unit_condition_id: 0,
                caster_player_condition_id: 0,
                spell_id: SICKNESS_SPELL_ID as u32,
            }],
        ))),
        ..Default::default()
    });
}

#[test]
fn selected_sickness_recalculates_from_saved_base_and_publishes_full_aura() {
    let (mut session, player_guid, send_rx) =
        canonical_aura_session([sickness_catalog_info()], 15_007);
    install_sickness_visual(&mut session);
    let (row, effects) = sickness_rows(player_guid, 90_000);

    assert_eq!(
        session.load_represented_character_auras_like_cpp([row], effects, 30),
        1
    );
    assert!(send_rx.try_recv().is_err(), "loading an aura stays silent");

    let (application, metadata, provenance) = crate::session::hub_ref(&session)
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let application = auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.spell_id == SICKNESS_SPELL_ID)
                .expect("canonical sickness application")
                .clone();
            let metadata = auras
                .aura_persistence_metadata(application.slot)
                .expect("persisted-only fields stay beside the runtime slot")
                .clone();
            let provenance = auras.aura_cast_provenance_like_cpp(application.slot);
            (application, metadata, provenance)
        })
        .expect("canonical Player owner");

    assert_eq!(
        application
            .represented_effect_amounts
            .iter()
            .map(|effect| effect.amount)
            .collect::<Vec<_>>(),
        [-75, -75, 0],
        "recalculation uses persisted base amounts, not the catalog's -30/-25/-3 values or stale DB amounts"
    );
    assert_eq!(application.duration_total, 600_000);
    assert_eq!(application.duration_remaining, 60_000);
    assert_eq!(application.aura_flags, 0x15);
    assert_eq!(application.effect_mask, 0b111);
    assert_eq!(metadata.base_amounts, [(0, -76), (1, -76), (2, -1)]);
    assert_eq!(
        (
            metadata.item_guid,
            metadata.cast_item_id,
            metadata.cast_item_level,
            metadata.recalculate_mask,
            metadata.charges,
        ),
        (ObjectGuid::EMPTY, 6948, 80, 0b111, 3)
    );
    assert!(!provenance.cast_id.is_empty());
    assert_eq!(provenance.spell_visual_id, SICKNESS_VISUAL_ROW_ID as i32);

    let (state, hub) = crate::session::split_spell_state_ref(&session);
    assert_eq!(
        state.resolved_player_aura_authority_complete_like_cpp(hub),
        Some(true)
    );
    let (_, hub) = crate::session::split_spell_state_ref(&session);
    assert_eq!(
        hub.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
        ),
        Some(vec![(-1, -75)]),
        "the canonical stat projection reads the recalculated AuraEffect amount"
    );
    let (_, hub) = crate::session::split_spell_state_ref(&session);
    assert_eq!(
        hub.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
        ),
        Some(vec![(127, -75)])
    );

    let saved = capture_player_save_request(&session)
        .expect("the real PlayerSaveOwner captures the loaded Player")
        .resurrection_sickness
        .expect("complete selected sickness rows are persisted");
    assert_eq!(saved.spell_id, SICKNESS_SPELL_ID as u32);
    assert_eq!(saved.families.len(), 1);
    assert_eq!(
        saved.families[0].caster_guid_binary,
        player_guid.to_raw_bytes()
    );
    assert_eq!(
        saved.families[0].item_guid_binary,
        ObjectGuid::EMPTY.to_raw_bytes()
    );
    let saved_aura = &saved.families[0].auras[0];
    assert_eq!(saved_aura.max_duration, 600_000);
    assert!((59_000..=60_000).contains(&saved_aura.remaining_duration));
    assert_eq!(
        (saved_aura.cast_item_id, saved_aura.cast_item_level),
        (6948, 80)
    );
    assert_eq!(
        saved_aura
            .effects
            .iter()
            .map(|effect| (effect.amount, effect.base_amount))
            .collect::<Vec<_>>(),
        [(-75, -76), (-75, -76), (0, -1)],
        "the save snapshot sees recalculated live amounts while retaining DB bases"
    );

    let (state, hub) = crate::session::split_spell_state_ref(&session);
    state.send_initial_player_auras_like_cpp(hub);
    let actual = send_rx
        .try_recv()
        .expect("post-add publishes the full aura list");
    let expected = wow_packet::packets::misc::AuraUpdate::full_for(
        player_guid,
        vec![wow_packet::packets::misc::AuraInfoLikeCpp {
            slot: application.slot,
            aura_data: Some(wow_packet::packets::misc::AuraDataInfoLikeCpp {
                cast_id: provenance.cast_id,
                spell_id: SICKNESS_SPELL_ID,
                spell_visual_id: SICKNESS_VISUAL_ROW_ID as i32,
                flags: 0x15,
                active_flags: 0b111,
                caster_guid: player_guid,
                cast_level: 80,
                applications: 0,
                duration_ms: Some(600_000),
                remaining_ms: Some(60_000),
                points: Vec::new(),
            }),
        }],
    )
    .to_bytes();
    assert_eq!(
        actual, expected,
        "post-add retains the full aura fields and cast provenance"
    );
}

#[test]
fn selected_sickness_capture_reconciles_foreign_and_item_keys() {
    let (mut session, player_guid, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let foreign_caster = ObjectGuid::create_player(1, 99);
    let finite_item = ObjectGuid::create_item(1, 88);
    let permanent_item = ObjectGuid::create_item(1, 89);
    insert_runtime_sickness_key(
        &mut session,
        selected_sickness_application(foreign_caster, 7, 0b001, 600_000, 550_000),
        Some(wow_entities::AuraPersistenceMetadata {
            item_guid: finite_item,
            cast_item_id: 6_948,
            cast_item_level: 80,
            recalculate_mask: 0b001,
            charges: 3,
            base_amounts: vec![(0, -76)],
        }),
        vec![(0b001, 80, -75, 0)],
    );
    insert_runtime_sickness_key(
        &mut session,
        selected_sickness_application(player_guid, 3, 0b010, 600_000, 550_000),
        None,
        vec![(0b010, 79, -25, 0)],
    );
    insert_runtime_sickness_key(
        &mut session,
        selected_sickness_application(foreign_caster, 9, 0b100, 0, 0),
        Some(wow_entities::AuraPersistenceMetadata {
            item_guid: permanent_item,
            cast_item_id: 1_234,
            cast_item_level: 40,
            recalculate_mask: 0b100,
            charges: 2,
            base_amounts: vec![(2, -1)],
        }),
        vec![(0b100, 159, -1, 0)],
    );

    let request = capture_player_save_request(&session).expect("complete owner snapshot");
    let saved = request
        .resurrection_sickness
        .expect("complete selected spell keyset");
    assert_eq!(saved.spell_id, SICKNESS_SPELL_ID as u32);
    assert_eq!(
        saved.families.len(),
        2,
        "known permanent item key is skipped"
    );
    assert!(saved.families.windows(2).all(|pair| {
        (pair[0].caster_guid_binary, pair[0].item_guid_binary)
            < (pair[1].caster_guid_binary, pair[1].item_guid_binary)
    }));

    let foreign_family = saved
        .families
        .iter()
        .find(|family| {
            family.caster_guid_binary == foreign_caster.to_raw_bytes()
                && family.item_guid_binary == finite_item.to_raw_bytes()
        })
        .expect("finite item family keeps its foreign caster and item key");
    let foreign_aura = &foreign_family.auras[0];
    assert_eq!(
        (
            foreign_aura.max_duration,
            foreign_aura.charges,
            foreign_aura.recalculate_mask,
            foreign_aura.cast_item_id,
            foreign_aura.cast_item_level,
        ),
        (600_000, 3, 1, 6_948, 80)
    );
    assert!((549_000..=550_000).contains(&foreign_aura.remaining_duration));
    assert_eq!(foreign_aura.effects[0].amount, -75);
    assert_eq!(foreign_aura.effects[0].base_amount, -76);

    let self_family = saved
        .families
        .iter()
        .find(|family| {
            family.caster_guid_binary == player_guid.to_raw_bytes()
                && family.item_guid_binary == ObjectGuid::EMPTY.to_raw_bytes()
        })
        .expect("self empty-item key remains in the complete spell snapshot");
    assert_eq!(self_family.auras[0].effects[0].amount, -25);
    assert!(!saved.families.iter().any(|family| {
        family.caster_guid_binary == foreign_caster.to_raw_bytes()
            && family.item_guid_binary == permanent_item.to_raw_bytes()
    }));
}

#[test]
fn selected_sickness_capture_holds_unknown_catalog_and_rejects_duplicate_family_masks() {
    let (mut missing_attributes, _, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let mut spells = wow_data::SpellStore::new();
    spells.insert(SICKNESS_SPELL_ID, sickness_catalog_info());
    missing_attributes.set_spell_store(Arc::new(spells));
    wow_world_spell::set_player_aura_authority_complete_like_cpp(&mut missing_attributes, true);
    assert!(
        capture_player_save_request(&missing_attributes)
            .unwrap()
            .resurrection_sickness
            .is_none()
    );

    let (mut duplicate, player_guid, _) =
        canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    insert_runtime_sickness_key(
        &mut duplicate,
        selected_sickness_application(player_guid, 2, 1, 60_000, 50_000),
        None,
        vec![(1, 80, -75, 0)],
    );
    insert_runtime_sickness_key(
        &mut duplicate,
        selected_sickness_application(player_guid, 3, 1, 60_000, 50_000),
        None,
        vec![(1, 80, -75, 0)],
    );
    assert!(
        capture_player_save_request(&duplicate)
            .unwrap()
            .resurrection_sickness
            .is_none()
    );
}

#[test]
fn selected_sickness_capture_skips_known_foreign_limit_n_and_holds_unknown_targets() {
    let (mut session, _, _) = canonical_aura_session([sickness_catalog_info()], SICKNESS_SPELL_ID);
    let foreign_caster = ObjectGuid::create_player(1, 99);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(SICKNESS_SPELL_ID, sickness_catalog_info());
    let mut attributes = [0; 15];
    attributes[5] = 0x20;
    spell_store.insert_spell_misc_attributes_like_cpp(SICKNESS_SPELL_ID, attributes);
    session.set_spell_store(Arc::new(spell_store));
    insert_runtime_sickness_key(
        &mut session,
        selected_sickness_application(foreign_caster, 4, 1, 60_000, 50_000),
        None,
        vec![(1, 80, -75, 0)],
    );
    let saved = capture_player_save_request(&session)
        .unwrap()
        .resurrection_sickness
        .expect("known source exclusion still completes the spell snapshot");
    assert!(saved.families.is_empty());

    let mut unsupported = sickness_catalog_info();
    unsupported.effects[0].implicit_target_1 = 6;
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(SICKNESS_SPELL_ID, unsupported);
    spell_store.insert_spell_misc_attributes_like_cpp(SICKNESS_SPELL_ID, [0; 15]);
    session.set_spell_store(Arc::new(spell_store));
    assert!(
        capture_player_save_request(&session)
            .unwrap()
            .resurrection_sickness
            .is_none()
    );
}

#[test]
fn aura_restore_matches_raw_caster_and_item_keys_and_scopes_recalculation() {
    let sickness = sickness_catalog_info();
    let buff_id = 90_010;
    let non_selected_buff = spell_info(
        buff_id,
        vec![sickness_effect(
            0,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            -30,
            127,
        )],
    );
    let (mut session, player_guid, _) =
        canonical_aura_session([sickness, non_selected_buff], 15_007);
    let item_guid = ObjectGuid::create_world_object(HighGuid::Item, 0, 1, 571, 0, 1, 800);
    let raw_empty_caster_row = crate::session::CharacterAuraRowLikeCpp {
        caster_guid: ObjectGuid::EMPTY,
        item_guid,
        spell_id: buff_id as u32,
        effect_mask: 1,
        recalculate_mask: 0,
        difficulty: 0,
        stack_count: 1,
        max_duration_ms: -1,
        remain_time_ms: -1,
        remain_charges: 0,
        cast_item_id: 1234,
        cast_item_level: 40,
    };
    let selected_scope_row = crate::session::CharacterAuraRowLikeCpp {
        caster_guid: player_guid,
        item_guid: ObjectGuid::EMPTY,
        spell_id: buff_id as u32,
        effect_mask: 1,
        recalculate_mask: 1,
        difficulty: 0,
        stack_count: 1,
        max_duration_ms: -1,
        remain_time_ms: -1,
        remain_charges: 0,
        cast_item_id: 0,
        cast_item_level: -1,
    };
    let effect = |caster_guid, item_guid, amount, base_amount| {
        crate::session::CharacterAuraEffectRowLikeCpp {
            caster_guid,
            item_guid,
            spell_id: buff_id as u32,
            effect_mask: 1,
            effect_index: 0,
            amount,
            base_amount,
        }
    };

    assert_eq!(
        session.load_represented_character_auras_like_cpp(
            [raw_empty_caster_row, selected_scope_row],
            [
                effect(ObjectGuid::EMPTY, item_guid, 17, -76),
                effect(ObjectGuid::EMPTY, ObjectGuid::EMPTY, 777, -99),
                effect(player_guid, item_guid, 888, -99),
                effect(player_guid, ObjectGuid::EMPTY, 33, -44),
            ],
            0,
        ),
        2
    );

    let (raw_caster_aura, scoped_recalc_aura, metadata) = crate::session::hub_ref(&session)
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            let raw_caster_aura = auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.caster_guid.is_empty())
                .expect("empty C++ caster GUID is preserved")
                .clone();
            let scoped_recalc_aura = auras
                .runtime_applications_like_cpp()
                .values()
                .find(|aura| aura.caster_guid == player_guid)
                .expect("self-cast non-selected buff")
                .clone();
            let metadata = auras
                .aura_persistence_metadata(raw_caster_aura.slot)
                .expect("loaded item provenance")
                .clone();
            (raw_caster_aura, scoped_recalc_aura, metadata)
        })
        .expect("canonical Player owner");
    assert_eq!(raw_caster_aura.caster_guid, ObjectGuid::EMPTY);
    assert_eq!(raw_caster_aura.aura_flags & 0x1, 0);
    assert_eq!(raw_caster_aura.represented_effect_amounts[0].amount, 17);
    assert_eq!(
        (
            metadata.item_guid,
            metadata.cast_item_id,
            metadata.cast_item_level,
        ),
        (item_guid, 1234, 40),
        "effect rows with a different item or caster do not replace the complete key"
    );
    assert_eq!(scoped_recalc_aura.represented_effect_amounts[0].amount, 33);
    let (state, hub) = crate::session::split_spell_state_ref(&session);
    assert_eq!(
        state.resolved_player_aura_authority_complete_like_cpp(hub),
        Some(false),
        "unsupported recalculation on another buff preserves DB state but cannot authorize sickness replacement"
    );
}

#[test]
fn offline_expiration_is_a_complete_empty_restore() {
    let (mut session, player_guid, _) = canonical_aura_session([sickness_catalog_info()], 15_007);
    let (row, effects) = sickness_rows(player_guid, 10_000);
    assert_eq!(
        session.load_represented_character_auras_like_cpp([row], effects, 11),
        0
    );

    let no_runtime_sickness = crate::session::hub_ref(&session)
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .values()
                .all(|aura| aura.spell_id != SICKNESS_SPELL_ID)
        })
        .expect("canonical Player owner");
    assert!(no_runtime_sickness);
    let (state, hub) = crate::session::split_spell_state_ref(&session);
    assert_eq!(
        state.resolved_player_aura_authority_complete_like_cpp(hub),
        Some(true),
        "legitimate offline expiry must not turn a complete read into a destructive-save hold"
    );
    let saved = capture_player_save_request(&session)
        .expect("the real PlayerSaveOwner captures the Player after legitimate expiry")
        .resurrection_sickness
        .expect("complete empty sickness group authorizes its scoped delete");
    assert!(saved.families.is_empty());
}

#[test]
fn missing_difficulty_catalog_keeps_sickness_persistence_incomplete() {
    let (mut session, player_guid, _) = canonical_aura_session([sickness_catalog_info()], 15_007);
    let (mut row, effects) = sickness_rows(player_guid, 90_000);
    row.difficulty = 1;

    assert_eq!(
        session.load_represented_character_auras_like_cpp([row], effects, 0),
        0
    );
    let (state, hub) = crate::session::split_spell_state_ref(&session);
    assert_eq!(
        state.resolved_player_aura_authority_complete_like_cpp(hub),
        Some(false),
        "unknown difficulty data must preserve the stored sickness rows"
    );
    let request = capture_player_save_request(&session)
        .expect("the rest of the Player request remains capturable");
    assert!(request.resurrection_sickness.is_none());
}

#[tokio::test]
async fn malformed_aura_guid_keeps_save_authority_incomplete() {
    let (mut session, player_guid, _) = canonical_aura_session([sickness_catalog_info()], 15_007);
    let mut malformed_caster = player_guid.to_raw_bytes().to_vec();
    malformed_caster.pop();
    let aura = wow_persistence::PlayerCharacterAuraLoadRowLikeCpp {
        caster_guid_binary: malformed_caster,
        item_guid_binary: ObjectGuid::EMPTY.to_raw_bytes().to_vec(),
        spell_id: 15_007,
        effect_mask: 0b111,
        recalculate_mask: 0b111,
        difficulty: 0,
        stack_count: 1,
        max_duration_ms: 600_000,
        remain_time_ms: 90_000,
        remain_charges: 0,
        cast_item_id: 0,
        cast_item_level: -1,
    };
    let port = CollectionLoadPortLikeCpp::for_login_auxiliary([
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuras(vec![aura]),
        ),
        PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::CharacterAuraEffects(Vec::new()),
        ),
    ]);
    let lifecycle_port: Arc<dyn PlayerLifecyclePortLikeCpp> = port.clone();

    wow_world_character::load_character_auras_for_login_like_cpp(
        &mut session,
        &lifecycle_port,
        player_guid,
        30,
    )
    .await;

    assert_eq!(
        port.login_auxiliary_requests(),
        [
            PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuras {
                player_guid: player_guid.counter() as u64,
            },
            PlayerLoginAuxiliaryLoadRequestLikeCpp::CharacterAuraEffects {
                player_guid: player_guid.counter() as u64,
            },
        ]
    );
    let (state, hub) = crate::session::split_spell_state_ref(&session);
    assert_eq!(
        state.resolved_player_aura_authority_complete_like_cpp(hub),
        Some(false),
        "an invalid raw key is skipped without authorizing an empty sickness replacement"
    );
    let request = capture_player_save_request(&session)
        .expect("the canonical owner still captures the rest of the Player request");
    assert!(
        request.resurrection_sickness.is_none(),
        "invalid key decoding cannot turn the loaded absence into a sickness delete"
    );
}
