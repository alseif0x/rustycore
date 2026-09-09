//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn load_represented_pet_aura_rows_filters_unknown_spell_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        7_777,
        gameobject_summon_spell_info_like_cpp(7_777, 0, Vec::new()),
    );
    session.set_spell_store(Arc::new(spell_store));

    let loaded = session.load_represented_pet_aura_rows_like_cpp(
        42,
        [
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 0,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_777,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 9_999,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
        ],
    );

    assert_eq!(loaded, 1);
    assert_eq!(
        session
            .pet_load_query_holder_rows_like_cpp
            .auras
            .get(&42)
            .and_then(|auras| auras.first())
            .map(|aura| aura.spell_id),
        Some(7_777)
    );
}
#[test]
fn represented_pet_aura_offline_remain_time_matches_cpp_arithmetic() {
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(5_000, 3, true, true),
        Some(2_000)
    );
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(3_000, 3, true, true),
        None,
        "C++ skips when remainTime / IN_MILLISECONDS <= timediff"
    );
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(-1, 99, true, true),
        Some(-1),
        "C++ permanent auras do not tick offline"
    );
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(5_000, 3, true, false),
        Some(5_000),
        "positive auras without SPELL_ATTR4_AURA_EXPIRES_OFFLINE keep their saved remainTime"
    );
    assert_eq!(
        adjusted_represented_pet_aura_remain_time_like_cpp(5_000, 3, false, false),
        Some(2_000),
        "C++ also ticks negative auras offline; represented SpellInfo::IsPositive is not wired yet"
    );
}
#[test]
fn load_represented_pet_aura_rows_ticks_attr4_offline_auras_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in [7_701, 7_702, 7_703] {
        spell_store.insert(
            spell_id,
            gameobject_summon_spell_info_like_cpp(spell_id, 0, Vec::new()),
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    let mut expires_offline_attrs = [0; 15];
    expires_offline_attrs[4] = wow_data::spell::attributes::SPELL_ATTR4_AURA_EXPIRES_OFFLINE as i32;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        wow_data::SpellMiscEntry {
            id: 1,
            attributes: expires_offline_attrs,
            difficulty_id: 0,
            casting_time_index: 0,
            duration_index: 0,
            range_index: 0,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_id: 7_701,
        },
        wow_data::SpellMiscEntry {
            id: 2,
            attributes: expires_offline_attrs,
            difficulty_id: 0,
            casting_time_index: 0,
            duration_index: 0,
            range_index: 0,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_id: 7_702,
        },
    ])));

    let loaded = session.load_represented_pet_aura_rows_with_timediff_like_cpp(
        42,
        [
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_701,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_702,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 3_000,
                remain_charges: 0,
            },
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_703,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
        ],
        3,
    );

    assert_eq!(loaded, 2);
    let auras = session
        .pet_load_query_holder_rows_like_cpp
        .auras
        .get(&42)
        .expect("represented pet auras");
    assert_eq!(auras[0].spell_id, 7_701);
    assert_eq!(auras[0].remain_time_ms, 2_000);
    assert_eq!(
        auras[1].spell_id, 7_703,
        "C++ skips expired offline aura rows and keeps normal positive rows"
    );
    assert_eq!(auras[1].remain_time_ms, 5_000);
}
#[test]
fn load_represented_pet_aura_rows_normalizes_proc_charges_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in [7_777, 8_888, 9_999] {
        spell_store.insert(
            spell_id,
            gameobject_summon_spell_info_like_cpp(spell_id, 0, Vec::new()),
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_aura_options_store(Arc::new(wow_data::SpellAuraOptionsStore::from_entries(
        [
            wow_data::SpellAuraOptionsEntry {
                id: 1,
                difficulty_id: 0,
                cumulative_aura: 0,
                proc_category_recovery: 0,
                proc_chance: 0,
                proc_charges: 3,
                spell_procs_per_minute_id: 0,
                proc_type_mask: [0; 2],
                spell_id: 7_777,
            },
            wow_data::SpellAuraOptionsEntry {
                id: 2,
                difficulty_id: 2,
                cumulative_aura: 0,
                proc_category_recovery: 0,
                proc_chance: 0,
                proc_charges: 5,
                spell_procs_per_minute_id: 0,
                proc_type_mask: [0; 2],
                spell_id: 8_888,
            },
        ],
    )));

    let loaded = session.load_represented_pet_aura_rows_like_cpp(
        42,
        [
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_777,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 8_888,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 2,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 2,
            },
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 9_999,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 7,
            },
        ],
    );

    assert_eq!(loaded, 3);
    let auras = session
        .pet_load_query_holder_rows_like_cpp
        .auras
        .get(&42)
        .expect("represented pet auras");
    assert_eq!(
        auras[0].remain_charges, 3,
        "C++ Pet::_LoadAuras replaces zero remainCharges with SpellInfo::ProcCharges"
    );
    assert_eq!(
        auras[1].remain_charges, 2,
        "C++ Pet::_LoadAuras preserves non-zero remainCharges when ProcCharges exists"
    );
    assert_eq!(
        auras[2].remain_charges, 0,
        "C++ Pet::_LoadAuras clears remainCharges when SpellInfo::ProcCharges is zero"
    );
}
#[test]
fn load_represented_pet_aura_rows_filters_unknown_difficulty_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in [7_001, 7_002, 7_003] {
        spell_store.insert(
            spell_id,
            gameobject_summon_spell_info_like_cpp(spell_id, 0, Vec::new()),
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_ids([2])));

    let loaded = session.load_represented_pet_aura_rows_like_cpp(
        42,
        [
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_001,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 0,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_002,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 2,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
            CharacterPetAuraRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_003,
                effect_mask: 1,
                recalculate_mask: 0,
                difficulty: 9,
                stack_count: 1,
                max_duration_ms: 10_000,
                remain_time_ms: 5_000,
                remain_charges: 0,
            },
        ],
    );

    assert_eq!(loaded, 2);
    let loaded_spell_ids: Vec<_> = session
        .pet_load_query_holder_rows_like_cpp
        .auras
        .get(&42)
        .expect("represented pet auras")
        .iter()
        .map(|aura| aura.spell_id)
        .collect();
    assert_eq!(
        loaded_spell_ids,
        vec![7_001, 7_002],
        "C++ Pet::_LoadAuras keeps DIFFICULTY_NONE and known difficulties, skips unknown non-zero difficulties"
    );
}
#[test]
fn load_represented_pet_aura_effect_rows_filters_bad_effect_index_like_cpp() {
    let (mut session, _, _send_rx) = make_session();

    let loaded = session.load_represented_pet_aura_effect_rows_like_cpp(
        42,
        [
            CharacterPetAuraEffectRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_777,
                effect_mask: 1,
                effect_index: 31,
                amount: 50,
                base_amount: 40,
            },
            CharacterPetAuraEffectRowLikeCpp {
                caster_guid: ObjectGuid::EMPTY,
                spell_id: 7_777,
                effect_mask: 1,
                effect_index: 32,
                amount: 60,
                base_amount: 50,
            },
        ],
    );

    assert_eq!(loaded, 1);
    assert_eq!(
        session
            .pet_load_query_holder_rows_like_cpp
            .aura_effects
            .get(&42)
            .and_then(|effects| effects.first())
            .map(|effect| effect.effect_index),
        Some(31)
    );
}
#[test]
fn resummon_pet_validates_action_bar_spells_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 8_424);
    let position = Position::new(17.0, 27.0, 37.0, 1.2);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TemporaryPetActionBarValidation".to_string(),
        position,
        571,
        1,
        3,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, position, 571, 0);
    session.represented_temporary_unsummoned_pet_number_like_cpp = 42;
    let mut stable = represented_hunter_pet_stable_like_cpp(42, 500);
    stable.active_pets[0].as_mut().unwrap().action_bar =
        "7 2 7 1 7 0 193 1111 193 3333 129 2222 193 4444 6 2 6 1 6 0".to_string();
    session.set_represented_pet_stable_like_cpp(stable);

    let mut spell_store = wow_data::SpellStore::new();
    for spell_id in [1_111, 2_222, 4_444] {
        spell_store.insert(
            spell_id,
            gameobject_summon_spell_info_like_cpp(spell_id, 0, Vec::new()),
        );
    }
    session.set_spell_store(Arc::new(spell_store));

    let mut passive_misc = spell_misc_entry_like_cpp(1_111, 1_111, 0);
    passive_misc.attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE as i32;
    let mut no_autocast_misc = spell_misc_entry_like_cpp(2_222, 2_222, 0);
    no_autocast_misc.attributes[1] = wow_data::spell::attributes::SPELL_ATTR1_NO_AUTOCAST_AI as i32;
    let autocastable_misc = spell_misc_entry_like_cpp(4_444, 4_444, 0);
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([
        passive_misc,
        no_autocast_misc,
        autocastable_misc,
    ])));
    let catalogs = &session.spell_catalogs;
    let spell_misc_store = catalogs
        .spell_misc_store()
        .expect("spell misc store should be installed");
    assert!(!spell_misc_store.is_autocastable_like_cpp(1_111));
    assert!(!spell_misc_store.is_autocastable_like_cpp(2_222));
    assert!(spell_misc_store.is_autocastable_like_cpp(4_444));

    session.resummon_pet_temporary_unsummoned_if_any_like_cpp();

    let pet_guid = session
        .represented_pet_guid_like_cpp()
        .expect("represented active pet guid");
    let manager = canonical.lock().unwrap();
    let pet = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_pet(pet_guid)
        .expect("canonical represented pet");
    let charm_info = pet
        .creature()
        .unit()
        .subsystems()
        .control
        .charm_info
        .as_ref()
        .expect("pet charm info");
    assert_eq!(
        charm_info.action_bar[3],
        wow_entities::make_unit_action_button_like_cpp(1_111, wow_entities::ACT_PASSIVE_LIKE_CPP),
        "C++ LoadPetActionBar forces passive spells to ACT_PASSIVE"
    );
    assert_eq!(
        charm_info.action_bar[4],
        wow_entities::make_unit_action_button_like_cpp(0, wow_entities::ACT_PASSIVE_LIKE_CPP),
        "C++ LoadPetActionBar clears unknown spell buttons"
    );
    assert_eq!(
        charm_info.action_bar[5],
        wow_entities::make_unit_action_button_like_cpp(2_222, wow_entities::ACT_PASSIVE_LIKE_CPP),
        "C++ LoadPetActionBar forces SPELL_ATTR1_NO_AUTOCAST_AI to ACT_PASSIVE"
    );
    assert_eq!(
        charm_info.action_bar[6],
        wow_entities::make_unit_action_button_like_cpp(4_444, wow_entities::ACT_ENABLED_LIKE_CPP)
    );
}
#[tokio::test]
async fn teleport_to_far_map_interrupts_non_melee_spell_casts_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 805);
    let source_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(110.0, 210.0, 40.0, 2.5);
    let melee_spell = wow_entities::CurrentSpellRef::new(61_805, Some(player_guid), None)
        .with_cast_time_ms(1_000);
    let generic_spell = wow_entities::CurrentSpellRef::new(61_806, Some(player_guid), None)
        .with_cast_time_ms(1_500);
    let channeled_spell = wow_entities::CurrentSpellRef::new(61_807, Some(player_guid), None)
        .with_state(wow_constants::SpellState::Delayed);
    let autorepeat_spell = wow_entities::CurrentSpellRef::new(61_808, Some(player_guid), None);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportInterruptSpells".to_string(),
        source_position,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_active_spell_cast_like_cpp(Some(SpellCastState {
        spell_id: 61_806,
        target_guid: player_guid,
        target_data: wow_entities::SpellCastTargetsLikeCpp::default(),
        cast_id: ObjectGuid::EMPTY,
        cast_start_time: Instant::now(),
        cast_time_ms: 10_000,
        spell_visual: wow_entities::SpellCastVisualLikeCpp::default(),
        metadata: SpellCastMetadata::default(),
    }));
    add_canonical_test_player_on_map(&canonical, player_guid, source_position, 571, 0);
    session.mutate_canonical_player_like_cpp(|player| {
        let unit = player.unit_mut();
        unit.set_current_cast_spell(wow_entities::CurrentSpellSlot::Melee, melee_spell);
        unit.set_current_cast_spell(wow_entities::CurrentSpellSlot::Generic, generic_spell);
        unit.set_current_cast_spell(wow_entities::CurrentSpellSlot::Channeled, channeled_spell);
        unit.set_current_cast_spell(wow_entities::CurrentSpellSlot::Autorepeat, autorepeat_spell);
    });

    session.teleport_to(0, destination).await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert_eq!(session.pending_teleport_like_cpp(), Some((0, destination)));
    assert!(
        session.active_spell_cast_snapshot_like_cpp().is_none(),
        "C++ Player::TeleportTo interrupts non-melee casts before transfer"
    );
    session.mutate_canonical_player_like_cpp(|player| {
        let unit = player.unit();
        assert_eq!(
            unit.current_spell(wow_entities::CurrentSpellSlot::Melee),
            Some(melee_spell),
            "C++ InterruptNonMeleeSpells must not interrupt CURRENT_MELEE_SPELL"
        );
        assert_eq!(
            unit.current_spell(wow_entities::CurrentSpellSlot::Generic),
            None
        );
        assert_eq!(
            unit.current_spell(wow_entities::CurrentSpellSlot::Channeled),
            None
        );
        assert_eq!(
            unit.current_spell(wow_entities::CurrentSpellSlot::Autorepeat),
            None
        );
    });
}
#[tokio::test]
async fn teleport_to_preflight_abort_preserves_non_melee_spell_casts_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 806);
    let source_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(111.0, 211.0, 41.0, 2.6);
    let generic_spell = wow_entities::CurrentSpellRef::new(61_809, Some(player_guid), None)
        .with_cast_time_ms(1_500);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportRejectPreservesSpell".to_string(),
        source_position,
        DEATH_KNIGHT_START_MAP_LIKE_CPP,
        1,
        CLASS_DEATH_KNIGHT_LIKE_CPP,
        58,
        0,
    ));
    session.set_active_spell_cast_like_cpp(Some(SpellCastState {
        spell_id: 61_809,
        target_guid: player_guid,
        target_data: wow_entities::SpellCastTargetsLikeCpp::default(),
        cast_id: ObjectGuid::EMPTY,
        cast_start_time: Instant::now(),
        cast_time_ms: 10_000,
        spell_visual: wow_entities::SpellCastVisualLikeCpp::default(),
        metadata: SpellCastMetadata::default(),
    }));
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        source_position,
        u32::from(DEATH_KNIGHT_START_MAP_LIKE_CPP),
        0,
    );
    session.mutate_canonical_player_like_cpp(|player| {
        player
            .unit_mut()
            .set_current_cast_spell(wow_entities::CurrentSpellSlot::Generic, generic_spell);
    });

    session.teleport_to(571, destination).await;

    assert_eq!(
        send_rx.try_recv().expect("SMSG_TRANSFER_ABORTED"),
        wow_packet::packets::misc::TransferAborted {
            map_id: 571,
            arg: 1,
            map_difficulty_x_condition_id: 0,
            transfer_abort: TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP,
        }
        .to_bytes()
    );
    assert!(
        session.active_spell_cast_snapshot_like_cpp().is_some(),
        "C++ Player::TeleportTo returns before spell interruption on preflight aborts"
    );
    session.mutate_canonical_player_like_cpp(|player| {
        assert_eq!(
            player
                .unit()
                .current_spell(wow_entities::CurrentSpellSlot::Generic),
            Some(generic_spell)
        );
    });
}
#[tokio::test]
async fn teleport_to_spell_option_preserves_non_melee_spell_casts_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 810);
    let source_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let destination = Position::new(115.0, 215.0, 45.0, 3.0);
    let generic_spell = wow_entities::CurrentSpellRef::new(61_810, Some(player_guid), None)
        .with_cast_time_ms(1_500);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 1,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.expansion = 1;
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TeleportSpellOption".to_string(),
        source_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, source_position, 571, 0);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    session.set_active_spell_cast_like_cpp(Some(SpellCastState {
        spell_id: 61_810,
        target_guid: player_guid,
        target_data: wow_entities::SpellCastTargetsLikeCpp::default(),
        cast_id: ObjectGuid::EMPTY,
        cast_start_time: Instant::now(),
        cast_time_ms: 10_000,
        spell_visual: wow_entities::SpellCastVisualLikeCpp::default(),
        metadata: SpellCastMetadata::default(),
    }));
    session.mutate_canonical_player_like_cpp(|player| {
        player
            .unit_mut()
            .set_current_cast_spell(wow_entities::CurrentSpellSlot::Generic, generic_spell);
    });

    session
        .teleport_to_with_options(0, destination, TELE_TO_SPELL_LIKE_CPP)
        .await;

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken
        ]
    );
    assert!(
        session.active_spell_cast_snapshot_like_cpp().is_some(),
        "C++ Player::TeleportTo skips InterruptNonMeleeSpells when TELE_TO_SPELL is set"
    );
    assert_eq!(
        session.with_owned_player_like_cpp(|player| {
            player
                .unit()
                .current_spell(wow_entities::CurrentSpellSlot::Generic)
        }),
        Some(Some(generic_spell)),
        "the exact detached Player must retain its active spell"
    );
}
