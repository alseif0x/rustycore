use super::*;
use std::sync::Arc;

const IGNORE_CASTER_MODIFIERS: u32 = 0x2000_0000;
const AURA_UNIQUE: u32 = 0x0000_0800;
const WISP_SPELL: i32 = wow_world_entities::SPELL_WISP_SPIRIT_AURA_LIKE_CPP;

fn sickness_effect_rows(difficulties: &[u8]) -> Vec<wow_data::spell_db2::SpellEffectDb2Entry> {
    difficulties
        .iter()
        .flat_map(|&difficulty_id| {
            [
                (
                    0,
                    wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
                    -40,
                ),
                (
                    1,
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
                    -50,
                ),
                (
                    2,
                    wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT,
                    -60,
                ),
            ]
            .into_iter()
            .map(move |(effect_index, aura, points)| {
                effect_row(
                    u32::from(difficulty_id) * 100 + effect_index + 1,
                    difficulty_id,
                    effect_index,
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    aura,
                    points,
                )
            })
        })
        .collect()
}

fn sickness_misc_row(
    difficulty_id: u8,
    attributes1: u32,
    attributes3: u32,
) -> wow_data::SpellMiscEntry {
    let mut attributes = [0; 15];
    attributes[0] = 0x2C80_0100;
    attributes[1] = attributes1;
    attributes[3] = attributes3;
    wow_data::SpellMiscEntry {
        id: 70_000 + u32::from(difficulty_id),
        spell_id: SICKNESS_SPELL as u32,
        difficulty_id,
        duration_index: 427,
        attributes: attributes.map(|value| value as i32),
        ..Default::default()
    }
}

async fn selected_sickness_session(
    guid_counter: i64,
    map_difficulty: u8,
    visual_difficulty: u8,
    difficulties: Vec<wow_data::DifficultyEntry>,
    misc_difficulties: &[(u8, u32, u32)],
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    selected_sickness_session_with_cooldowns(
        guid_counter,
        map_difficulty,
        visual_difficulty,
        difficulties,
        misc_difficulties,
        Vec::new(),
    )
    .await
}

async fn selected_sickness_session_with_cooldowns(
    guid_counter: i64,
    map_difficulty: u8,
    visual_difficulty: u8,
    difficulties: Vec<wow_data::DifficultyEntry>,
    misc_difficulties: &[(u8, u32, u32)],
    cooldown_rows: Vec<wow_data::spell_db2::SpellCooldownsEntry>,
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let effect_difficulties = misc_difficulties
        .iter()
        .map(|(difficulty, _, _)| *difficulty)
        .collect::<Vec<_>>();
    super::selected_execution_effects::selected_session_with_visual_inputs_and_cooldowns(
        guid_counter,
        map_difficulty,
        sickness_effect_rows(&effect_difficulties),
        difficulties,
        None,
        Some(
            misc_difficulties
                .iter()
                .map(|&(difficulty, attributes1, attributes3)| {
                    sickness_misc_row(difficulty, attributes1, attributes3)
                })
                .collect(),
        ),
        visual_difficulty,
        cooldown_rows,
    )
    .await
}

async fn selected_sickness_session_without_spell_info_key(
    guid_counter: i64,
    map_difficulty: u8,
    visual_difficulty: u8,
    difficulties: Vec<wow_data::DifficultyEntry>,
    misc_difficulties: &[(u8, u32, u32)],
) -> (WorldSession, flume::Receiver<Vec<u8>>, ObjectGuid) {
    let effect_difficulties = misc_difficulties
        .iter()
        .map(|(difficulty, _, _)| *difficulty)
        .collect::<Vec<_>>();
    let misc_rows = misc_difficulties
        .iter()
        .map(|&(difficulty, attributes1, attributes3)| {
            sickness_misc_row(difficulty, attributes1, attributes3)
        })
        .collect();
    selected_session_without_selected_spell_info_key(
        guid_counter,
        map_difficulty,
        sickness_effect_rows(&effect_difficulties),
        difficulties,
        None,
        Some(misc_rows),
        visual_difficulty,
    )
    .await
}

fn install_aura_options(session: &mut WorldSession, entries: &[(u8, i32)]) {
    let mut rows = crate::session::hub_ref(session)
        .catalogs
        .spell_catalogs
        .spell_aura_options_store
        .as_deref()
        .map(|store| {
            store
                .entries_like_cpp()
                .filter(|entry| entry.spell_id != SICKNESS_SPELL as u32)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rows.extend(entries.iter().map(|&(difficulty_id, proc_charges)| {
        wow_data::SpellAuraOptionsEntry {
            id: 71_000 + u32::from(difficulty_id),
            difficulty_id,
            cumulative_aura: 0,
            proc_category_recovery: 0,
            proc_chance: 0,
            proc_charges,
            spell_procs_per_minute_id: 0,
            proc_type_mask: [wow_data::spell::PROC_FLAG_KILL_LIKE_CPP as i32, 0],
            spell_id: SICKNESS_SPELL as u32,
        }
    }));
    session.set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries(
        rows,
    )));
}

fn install_generated_proc_store(session: &mut WorldSession) -> usize {
    let outcome = {
        let catalogs = crate::session::hub_ref(session).catalogs;
        let spells = catalogs
            .spell_catalogs
            .spell_store
            .as_deref()
            .expect("canonical SpellStore");
        let difficulties = catalogs
            .difficulty_store
            .as_deref()
            .expect("DifficultyStore");
        let aura_options = catalogs
            .spell_catalogs
            .spell_aura_options_store
            .as_deref()
            .expect("complete SpellAuraOptions store");
        let class_options = catalogs
            .spell_catalogs
            .spell_class_options_store()
            .map(AsRef::as_ref)
            .expect("complete SpellClassOptions store");
        let serverside_spells = wow_data::ServersideSpellStoreLikeCpp::default();
        let spell_chains = wow_data::SpellChainStoreLikeCpp::default();
        let procs_per_minute = wow_data::SpellProcsPerMinuteStore::from_entries([]);
        wow_data::SpellProcStoreLikeCpp::from_rows_and_stores_like_cpp(
            std::iter::empty::<wow_data::SpellProcRowLikeCpp>(),
            spells,
            &serverside_spells,
            &spell_chains,
            difficulties,
            aura_options,
            class_options,
            &procs_per_minute,
        )
        .expect("build proc entries from canonical selected spell facts")
    };
    let generated_count = outcome.generated_entry_count;
    session.set_spell_proc_store(Arc::new(outcome.store));
    generated_count
}

fn application_charge_for_spell(session: &WorldSession, spell_id: i32) -> (u8, u8) {
    let application = visible_sickness_aura(session, spell_id)
        .into_iter()
        .next()
        .expect("selected sickness AuraApplication");
    let charges = session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .aura_persistence_metadata(application.slot)
                .expect("selected Aura charge metadata")
                .charges
        })
        .expect("canonical Player");
    (application.difficulty_id, charges)
}

fn application_charge(session: &WorldSession) -> (u8, u8) {
    application_charge_for_spell(session, SICKNESS_SPELL)
}

pub(super) async fn cast_full_selected_spell(
    session: &mut WorldSession,
    spell_id: i32,
    target_guid: ObjectGuid,
    original_caster_guid: Option<ObjectGuid>,
) -> Result<(), &'static str> {
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawn_catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    wow_world_spell::execute_server_triggered_spell_like_cpp(
        session,
        generators.item.as_ref(),
        &creature_spawn_catalogs,
        spell_id,
        target_guid,
        wow_entities::SpellCastMetadata {
            triggered_ignores_global_cooldown_like_cpp: true,
            original_caster_guid,
            ..Default::default()
        },
    )
    .await
}

pub(super) async fn cast_full_sickness(
    session: &mut WorldSession,
    target_guid: ObjectGuid,
    original_caster_guid: Option<ObjectGuid>,
) -> Result<(), &'static str> {
    cast_full_selected_spell(session, SICKNESS_SPELL, target_guid, original_caster_guid).await
}

fn install_same_map_foreign_player(session: &WorldSession, low_guid: u64) -> ObjectGuid {
    let guid = ObjectGuid::create_player(
        1,
        i64::try_from(low_guid).expect("foreign Player database GUID fits ObjectGuid"),
    );
    let manager = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("canonical MapManager");
    let mut manager = manager.lock().expect("canonical MapManager");
    let owner = session
        .core
        .player_handle_like_cpp
        .expect("canonical Player handle");
    let map_key = match manager
        .player_residence_like_cpp(owner)
        .expect("canonical Player residence")
    {
        wow_map::PlayerResidenceLikeCpp::Active(key) => key,
        wow_map::PlayerResidenceLikeCpp::Detached => panic!("canonical Player is detached"),
    };
    let mut foreign = Box::new(wow_entities::Player::new(Some(low_guid), false));
    foreign.unit_mut().world_mut().object_mut().create(guid);
    foreign.unit_mut().set_level(44);
    let handle = manager
        .install_detached_player_like_cpp(foreign)
        .expect("foreign Player owner");
    manager
        .attach_player_like_cpp(handle, map_key, Position::ZERO)
        .expect("same-map foreign Player");
    guid
}

fn install_wisp_aura_options(session: &mut WorldSession, proc_charges: i32) {
    let mut rows = session
        .catalogs
        .spell_catalogs
        .spell_aura_options_store
        .as_deref()
        .map(|store| {
            store
                .entries_like_cpp()
                .filter(|entry| entry.spell_id != WISP_SPELL as u32)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    rows.push(wow_data::SpellAuraOptionsEntry {
        id: 73_000,
        difficulty_id: 0,
        cumulative_aura: 0,
        proc_category_recovery: 0,
        proc_chance: 0,
        proc_charges,
        spell_procs_per_minute_id: 0,
        proc_type_mask: [wow_data::spell::PROC_FLAG_KILL_LIKE_CPP as i32, 0],
        spell_id: WISP_SPELL as u32,
    });
    session.set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries(
        rows,
    )));
}

#[tokio::test]
async fn selected_charge_fresh_uses_actual_info_key_and_preserves_signed_charge_bits() {
    for (index, proc_charges, expected_charges) in [(0, 256, 0), (1, -1, u8::MAX)] {
        let (mut session, _send_rx, player_guid) = selected_sickness_session(
            45_001 + index,
            0,
            3,
            vec![difficulty(0, 3), difficulty(3, 0)],
            &[(3, 0x88, 0)],
        )
        .await;
        install_aura_options(&mut session, &[(3, proc_charges)]);
        session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp::default()));

        let spells = session.catalogs.spell_store().expect("selected SpellStore");
        assert!(spells.contains_spell_info_exact_like_cpp(SICKNESS_SPELL as u32, 3));
        assert!(!spells.contains_spell_info_exact_like_cpp(SICKNESS_SPELL as u32, 0));

        let _packet_peer = keep_recovery_input_open(&mut session);
        cast_full_sickness(&mut session, player_guid, None)
            .await
            .expect("fresh selected sickness cast uses the fallback SpellInfo key");

        assert_eq!(
            application_charge(&session),
            (0, expected_charges),
            "the Aura keeps raw difficulty zero while charges use actual SpellInfo difficulty three"
        );
    }
}

#[tokio::test]
async fn selected_full_sickness_skips_cooldown_event_and_preserves_history() {
    let base_cooldown = wow_data::spell_db2::SpellCooldownsEntry {
        id: 74_001,
        difficulty_id: 0,
        category_recovery_time: 0,
        recovery_time: 30_000,
        start_recovery_time: 0,
        spell_id: SICKNESS_SPELL as u32,
    };
    let scenarios = [
        (
            45_601,
            0,
            vec![difficulty(0, 0)],
            vec![(0, 0x88, IGNORE_CASTER_MODIFIERS)],
            0,
            vec![base_cooldown],
        ),
        (
            45_602,
            3,
            vec![difficulty(0, 3), difficulty(3, 0)],
            vec![(3, 0x88, IGNORE_CASTER_MODIFIERS)],
            3,
            Vec::new(),
        ),
    ];

    for (
        guid_counter,
        visual_difficulty,
        difficulties,
        misc_difficulties,
        info_difficulty,
        cooldown_rows,
    ) in scenarios
    {
        let (mut session, send_rx, player_guid) = selected_sickness_session_with_cooldowns(
            guid_counter,
            0,
            visual_difficulty,
            difficulties,
            &misc_difficulties,
            cooldown_rows,
        )
        .await;
        install_aura_options(&mut session, &[(info_difficulty, 0)]);
        session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp::default()));

        let spell_store = session.catalogs.spell_store().expect("selected SpellStore");
        assert!(
            spell_store.contains_spell_info_exact_like_cpp(SICKNESS_SPELL as u32, info_difficulty)
        );
        assert_eq!(
            spell_store.contains_spell_info_exact_like_cpp(SICKNESS_SPELL as u32, 0),
            info_difficulty == 0
        );
        if info_difficulty == 0 {
            assert_eq!(
                spell_store
                    .get(SICKNESS_SPELL)
                    .expect("Base0 SpellInfo payload")
                    .recovery_time_ms,
                30_000
            );
        }

        let existing_cooldown = wow_entities::SpellCooldown {
            spell_id: SICKNESS_SPELL as u32,
            item_id: 19,
            cooldown_end_ms: 123_456,
            category_id: 7,
            category_end_ms: 654_321,
            on_hold: true,
        };
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                let history = &mut player.unit_mut().subsystems_mut().spells.history;
                history.cooldowns_loaded = true;
                history
                    .cooldowns
                    .insert(SICKNESS_SPELL as u32, existing_cooldown);
            })
            .expect("canonical Player");

        let _packet_peer = keep_recovery_input_open(&mut session);
        cast_full_sickness(&mut session, player_guid, None)
            .await
            .expect("selected FULL sickness cast completes");

        let packets = drain_packets(&send_rx);
        assert!(
            packets
                .iter()
                .any(|packet| packet_opcode(packet) == ServerOpcodes::SpellGo as u16)
        );
        assert!(
            packets
                .iter()
                .all(|packet| packet_opcode(packet) != ServerOpcodes::CooldownEvent as u16)
        );
        let cooldown_after = session
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .spells
                    .history
                    .cooldown(SICKNESS_SPELL as u32)
            })
            .expect("canonical Player");
        assert_eq!(cooldown_after, Some(existing_cooldown));
    }
}

#[tokio::test]
async fn selected_charge_cast_rejects_hydrated_spell_without_canonical_info_key() {
    let (mut session, send_rx, player_guid) = selected_sickness_session_without_spell_info_key(
        45_151,
        0,
        3,
        vec![difficulty(0, 3), difficulty(3, 0)],
        &[(0, 0x88, 0), (3, 0x88, 0)],
    )
    .await;
    let spell_store = session.catalogs.spell_store().expect("selected SpellStore");
    assert!(spell_store.get(SICKNESS_SPELL).is_some());
    let attributes = spell_store
        .misc_attributes_for_exact_difficulty(SICKNESS_SPELL, 3)
        .expect("hydrated selected sickness SpellMisc row");
    assert_eq!(attributes[3], 0);
    assert_eq!(
        spell_store
            .effect_rows_for_exact_difficulty(SICKNESS_SPELL, 3)
            .expect("hydrated selected sickness SpellEffect rows")
            .len(),
        3,
    );
    assert!(!spell_store.contains_spell_info_any_difficulty_like_cpp(SICKNESS_SPELL as u32));

    let _packet_peer = keep_recovery_input_open(&mut session);
    assert_eq!(
        cast_full_sickness(&mut session, player_guid, None).await,
        Err("Spell not found"),
        "hydrated payload and DB2 rows do not substitute for a canonical SpellInfo key"
    );
    assert!(
        drain_packets(&send_rx).is_empty(),
        "no SpellGo is published"
    );
    assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
}

#[tokio::test]
async fn selected_charge_transient_payload_rejects_channelled_or_incomplete_shape_before_spell_go()
{
    for (index, channelled, omit_third_effect) in [(0_i64, false, true), (1, true, false)] {
        let mut effects = sickness_effect_rows(&[3]);
        if omit_third_effect {
            effects.retain(|effect| effect.effect_index != 2);
        }
        let attributes1 = if channelled {
            wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED
        } else {
            0x88
        };
        let (mut session, send_rx, player_guid) = selected_session_with_visual_inputs(
            45_161 + index,
            0,
            effects,
            vec![difficulty(0, 3), difficulty(3, 0)],
            None,
            Some(vec![sickness_misc_row(3, attributes1, 0)]),
            3,
        )
        .await;
        let _packet_peer = keep_recovery_input_open(&mut session);
        let expected_error = if channelled {
            "Selected transient SpellInfo cannot represent a channelled cast"
        } else {
            "Selected spell does not have the supported three-effect Aura shape"
        };
        assert_eq!(
            cast_full_sickness(&mut session, player_guid, None).await,
            Err(expected_error)
        );
        assert!(
            drain_packets(&send_rx).is_empty(),
            "unsupported projection sends no SpellGo"
        );
        assert!(visible_sickness_aura(&session, SICKNESS_SPELL).is_empty());
    }
}

#[tokio::test]
async fn selected_charge_generated_exact_zero_stops_nonzero_proc_fallback() {
    let (mut session, _send_rx, player_guid) = selected_sickness_session(
        45_201,
        4,
        3,
        vec![difficulty(0, 0), difficulty(3, 0), difficulty(4, 3)],
        &[(3, 0x88, 0), (4, 0x88, 0)],
    )
    .await;
    install_aura_options(&mut session, &[(3, 9), (4, 0)]);
    assert!(install_generated_proc_store(&mut session) >= 2);

    let proc_store = crate::session::hub_ref(&session)
        .catalogs
        .spell_catalogs
        .spell_proc_store
        .as_deref()
        .expect("generated ProcStore");
    assert_eq!(
        proc_store
            .spell_proc_entry_like_cpp(SICKNESS_SPELL as u32, 3)
            .expect("generated fallback proc entry")
            .charges,
        9
    );
    assert_eq!(
        proc_store
            .spell_proc_entry_like_cpp(SICKNESS_SPELL as u32, 4)
            .expect("generated exact proc entry")
            .charges,
        0
    );

    let _packet_peer = keep_recovery_input_open(&mut session);
    cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("generated proc entry supplies selected sickness charges");
    assert_eq!(application_charge(&session), (4, 0));
}

#[tokio::test]
async fn selected_wisp_charge_generation_uses_transform_proc_metadata() {
    let (mut session, send_rx, _realm_rx, _port) = logged_in_released_ghost(45_251).await;
    let player_guid = session.player_guid().expect("canonical Player GUID");
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([0])));
    install_wisp_aura_options(&mut session, 9);
    session.set_creature_aggro_config_like_cpp(crate::session::CreatureAggroConfigLikeCpp {
        spell_x_spell_visual_store: Some(Arc::new(wow_data::SpellXSpellVisualStore::from_entries(
            [],
        ))),
        ..Default::default()
    });
    assert!(install_generated_proc_store(&mut session) >= 1);

    let generated = session
        .catalogs
        .spell_catalogs
        .spell_proc_store
        .as_deref()
        .expect("generated Wisp ProcStore")
        .spell_proc_entry_like_cpp(WISP_SPELL as u32, 0)
        .expect("Wisp transform generates its implicit proc entry");
    assert_eq!(generated.charges, 9);

    let _packet_peer = keep_recovery_input_open(&mut session);
    cast_full_selected_spell(&mut session, WISP_SPELL, player_guid, None)
        .await
        .expect("full-mask Wisp cast applies through its selected aura consumer");
    let packets = drain_packets(&send_rx);
    assert!(
        packets
            .iter()
            .all(|packet| packet_opcode(packet) != ServerOpcodes::CooldownEvent as u16)
    );
    assert_eq!(application_charge_for_spell(&session, WISP_SPELL), (0, 9));
}

#[tokio::test]
async fn selected_charge_refresh_uses_original_key_and_ignores_fresh_modifier_failure() {
    let (mut session, _send_rx, player_guid) = selected_sickness_session(
        45_301,
        0,
        0,
        vec![difficulty(0, 3), difficulty(3, 0), difficulty(4, 3)],
        &[
            (0, 0x88, IGNORE_CASTER_MODIFIERS),
            (3, 0x88, IGNORE_CASTER_MODIFIERS),
            (4, 0x88, 0),
        ],
    )
    .await;
    install_aura_options(&mut session, &[(0, 5), (3, 5), (4, 9)]);
    let mut class_rows = session
        .catalogs
        .spell_catalogs
        .spell_class_options_store()
        .map(|store| {
            store
                .entries_like_cpp()
                .filter(|entry| entry.spell_id != SICKNESS_SPELL)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    class_rows.push(wow_data::SpellClassOptionsEntry {
        id: 72_000,
        spell_id: SICKNESS_SPELL,
        modal_next_spell: 0,
        spell_class_set: 3,
        spell_class_mask: [1, 0, 0, 0],
    });
    session.set_spell_class_options_store(Arc::new(
        wow_data::SpellClassOptionsStore::from_entries(class_rows),
    ));
    session.set_spell_label_store(Arc::new(wow_data::SpellLabelStore::from_entries([
        wow_data::SpellLabelEntry {
            id: 72_001,
            label_id: 81,
            spell_id: SICKNESS_SPELL as u32,
        },
    ])));
    assert!(install_generated_proc_store(&mut session) >= 2);

    let spell_store = session.catalogs.spell_store().expect("selected SpellStore");
    for difficulty_id in [0, 3, 4] {
        assert!(
            spell_store.contains_spell_info_exact_like_cpp(SICKNESS_SPELL as u32, difficulty_id,)
        );
    }
    let proc_store = session
        .catalogs
        .spell_catalogs
        .spell_proc_store
        .as_deref()
        .expect("generated ProcStore");
    for (difficulty_id, charges) in [(0_u8, 5_u32), (3, 5), (4, 9)] {
        assert_eq!(
            proc_store
                .spell_proc_entry_like_cpp(SICKNESS_SPELL as u32, u32::from(difficulty_id))
                .expect("generated exact selected sickness proc entry")
                .charges,
            charges,
        );
    }

    let _packet_peer = keep_recovery_input_open(&mut session);
    cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("initial selected sickness uses the ignored-modifier Info");
    assert_eq!(application_charge(&session), (0, 5));
    install_player_on_difficulty_map(&mut session, 4);
    cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("matching refresh keeps working after fresh modifier admission fails");

    assert_eq!(
        application_charge(&session),
        (0, 5),
        "refresh looks up charges from its stored raw difficulty, not the new cast's key"
    );
}

#[tokio::test]
async fn selected_charge_non_timer_refresh_needs_no_proc_store_and_keeps_old_charges() {
    let (mut session, _send_rx, player_guid) = selected_sickness_session(
        45_401,
        0,
        3,
        vec![difficulty(0, 3), difficulty(3, 0), difficulty(4, 3)],
        &[
            (3, 0x88 | AURA_UNIQUE, IGNORE_CASTER_MODIFIERS),
            (4, 0x88 | AURA_UNIQUE, IGNORE_CASTER_MODIFIERS),
        ],
    )
    .await;
    install_aura_options(&mut session, &[(3, 5), (4, 9)]);
    assert!(install_generated_proc_store(&mut session) >= 2);

    let _packet_peer = keep_recovery_input_open(&mut session);
    cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("initial selected sickness cast");
    let application = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("initial selected sickness Aura");
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .runtime_application_mut_like_cpp(application.slot)
                .expect("initial sickness Aura")
                .duration_remaining = 10_000;
        })
        .expect("canonical Player");
    session.catalogs.spell_catalogs.spell_proc_store = None;
    install_player_on_difficulty_map(&mut session, 4);

    cast_full_sickness(&mut session, player_guid, None)
        .await
        .expect("non-timer refresh does not consult the missing ProcStore");

    let refreshed = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("refreshed sickness Aura");
    assert_eq!(refreshed.duration_remaining, 10_000);
    assert_eq!(application_charge(&session), (0, 5));
}

#[tokio::test]
async fn selected_charge_failure_keeps_conflicting_foreign_aura() {
    let (mut session, _send_rx, player_guid) = selected_sickness_session(
        45_501,
        0,
        3,
        vec![difficulty(0, 3), difficulty(3, 0)],
        &[(3, 0x88, IGNORE_CASTER_MODIFIERS)],
    )
    .await;
    install_aura_options(&mut session, &[(3, 7)]);
    session.set_spell_proc_store(Arc::new(wow_data::SpellProcStoreLikeCpp::default()));
    let foreign_caster = install_same_map_foreign_player(&session, 45_502);
    let _packet_peer = keep_recovery_input_open(&mut session);
    cast_full_sickness(&mut session, player_guid, Some(foreign_caster))
        .await
        .expect("real selected cast installs the same-map original-caster Aura");
    let old = visible_sickness_aura(&session, SICKNESS_SPELL)
        .into_iter()
        .next()
        .expect("foreign sickness Aura");
    assert_eq!(old.caster_guid, foreign_caster);
    session.catalogs.spell_catalogs.spell_proc_store = None;

    assert_eq!(
        cast_full_sickness(&mut session, player_guid, None).await,
        Err("Missing canonical SpellProc store for selected aura charges")
    );
    let after = visible_sickness_aura(&session, SICKNESS_SPELL);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].slot, old.slot);
    assert_eq!(after[0].caster_guid, foreign_caster);
    assert_eq!(application_charge(&session), (0, 7));
}
