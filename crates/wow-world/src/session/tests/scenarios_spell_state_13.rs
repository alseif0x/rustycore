//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn spell_duel_effect_requests_duel_and_sets_challenged_state_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let (mut target_session, _, target_send_rx) = make_session();
    let spell_id = SPELL_DUEL_LIKE_CPP as i32;
    let gameobject_entry = 21680_i32;
    let player_guid = ObjectGuid::create_player(1, 810);
    let target_guid = ObjectGuid::create_player(1, 811);
    let canonical = shared_canonical_map_manager();
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(10.0, 10.0, 0.0, 0.0),
        571,
        0,
    );
    add_canonical_test_player_on_map(
        &canonical,
        target_guid,
        Position::new(12.0, 10.0, 0.0, 0.0),
        571,
        0,
    );
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::new(10.0, 10.0, 0.0, 0.0));
    target_session.set_player_guid(Some(target_guid));

    let registry = Arc::new(PlayerRegistry::default());
    let (target_tx, _target_rx) = flume::bounded(10);
    let mut target_info = broadcast_info(target_guid, target_tx);
    target_info.placement.map_id = 571;
    target_info.placement.position = Position::new(12.0, 10.0, 0.0, 0.0);
    target_info.command_tx = target_session.session_command_tx();
    registry.register_or_replace(target_guid, target_info, Default::default());
    session.set_player_registry(registry);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUEL,
                effect_misc_value_1: gameobject_entry,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, target_guid)
        .await
        .expect("represented EffectDuel should execute");
    target_session
        .process_represented_session_commands_like_cpp()
        .await;

    let packets = drain_server_packet_bytes(&send_rx);
    let opcodes: Vec<_> = packets
        .iter()
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(bytes).server_opcode())
        .collect();
    assert_eq!(
        opcodes,
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::DuelRequested,
            ServerOpcodes::CooldownEvent
        ]
    );
    assert_eq!(
        drain_server_opcodes(&target_send_rx),
        vec![ServerOpcodes::DuelRequested]
    );

    let request = session
        .represented_duel_requests_like_cpp()
        .first()
        .copied()
        .expect("represented duel request");
    assert_eq!(request.target_guid, target_guid);
    assert_eq!(request.gameobject_entry, gameobject_entry as u32);
    assert!(!request.to_the_death);
    assert_eq!(
        target_session.resolved_represented_duel_arbiter_guid_like_cpp(),
        Some(Some(request.arbiter_guid))
    );
    assert_eq!(
        session.mutate_canonical_player_by_guid_like_cpp(player_guid, |player| {
            player.duel_info_like_cpp()
        }),
        Some(Some(wow_entities::PlayerDuelInfoLikeCpp {
            opponent: target_guid,
            state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
        }))
    );
    assert_eq!(
        session.mutate_canonical_player_by_guid_like_cpp(target_guid, |player| {
            player.duel_info_like_cpp()
        }),
        Some(Some(wow_entities::PlayerDuelInfoLikeCpp {
            opponent: player_guid,
            state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
        }))
    );

    let packet = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::DuelRequested)
        })
        .expect("duel requested packet");
    let mut reader = wow_packet::WorldPacket::from_bytes(packet);
    reader.skip_opcode();
    let arbiter_bytes = reader.read_bytes(16).unwrap();
    let mut raw = [0_u8; 16];
    raw.copy_from_slice(&arbiter_bytes);
    assert_eq!(ObjectGuid::from_raw_bytes(&raw), request.arbiter_guid);
    let requester_bytes = reader.read_bytes(16).unwrap();
    raw.copy_from_slice(&requester_bytes);
    assert_eq!(ObjectGuid::from_raw_bytes(&raw), player_guid);
    let account_bytes = reader.read_bytes(16).unwrap();
    raw.copy_from_slice(&account_bytes);
    assert_eq!(
        ObjectGuid::from_raw_bytes(&raw),
        ObjectGuid::create_global(HighGuid::WowAccount, 0, session.account_id as i64)
    );
    assert!(!reader.read_bit().unwrap());
    assert_eq!(reader.remaining(), 0);
}
#[tokio::test]
async fn spell_duel_effect_ignores_target_already_dueling_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = SPELL_DUEL_LIKE_CPP as i32;
    let player_guid = ObjectGuid::create_player(1, 812);
    let target_guid = ObjectGuid::create_player(1, 813);
    let other_guid = ObjectGuid::create_player(1, 814);
    let canonical = shared_canonical_map_manager();
    add_canonical_test_player_on_map(&canonical, player_guid, Position::ZERO, 571, 0);
    add_canonical_test_player_on_map(&canonical, target_guid, Position::ZERO, 571, 0);
    {
        let mut manager = canonical.lock().unwrap();
        manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(target_guid)
            .unwrap()
            .set_duel_info_like_cpp(Some(wow_entities::PlayerDuelInfoLikeCpp {
                opponent: other_guid,
                state: wow_entities::PlayerDuelStateLikeCpp::Challenged,
            }));
    }
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUEL,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUEL,
                effect_misc_value_1: 21680,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, target_guid)
        .await
        .expect("represented EffectDuel target-with-duel should execute as C++ no-op");

    assert!(session.represented_duel_requests_like_cpp().is_empty());
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
    assert_eq!(
        session.mutate_canonical_player_by_guid_like_cpp(player_guid, |player| {
            player.duel_info_like_cpp()
        }),
        Some(None)
    );
}
#[tokio::test]
async fn spell_modify_cooldown_effect_adjusts_trigger_spell_history_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 790_i32;
    let trigger_spell = 791_u32;
    let player_guid = ObjectGuid::create_player(1, 790);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    let now_ms = u64::from(crate::session_rules::game_time_ms_like_cpp());
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .spells
                .history
                .start_cooldown(now_ms, trigger_spell, 0, 30_000, 0, 0, false);
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        cooldown_or_charges_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWN,
            -2_000,
            0,
            i32::try_from(trigger_spell).unwrap(),
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectModifyCooldown should execute");

    let cooldown = session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .spells
                .history
                .cooldown(trigger_spell)
        })
        .flatten()
        .expect("trigger spell cooldown should remain after a -2s delta");
    assert_eq!(cooldown.cooldown_end_ms, now_ms + 28_000);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_modify_charges_effect_restores_consumed_charges_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 792_i32;
    let charge_category = 44_u32;
    let player_guid = ObjectGuid::create_player(1, 792);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 100, 100);
    let now_ms = u64::from(crate::session_rules::game_time_ms_like_cpp());
    session
        .mutate_canonical_player_like_cpp(|player| {
            let history = &mut player.unit_mut().subsystems_mut().spells.history;
            assert!(history.consume_charge(charge_category, now_ms, 10_000, 2));
            assert!(history.consume_charge(charge_category, now_ms + 1, 10_000, 2));
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        cooldown_or_charges_spell_info_like_cpp(
            spell_id,
            wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_CHARGES,
            1,
            i32::try_from(charge_category).unwrap(),
            0,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented EffectModifySpellCharges should execute");

    let consumed = session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .spells
                .history
                .consumed_charges(charge_category)
        })
        .unwrap();
    assert_eq!(consumed, 1);
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_self_resurrect_flat_case_sets_health_and_powers_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 780_i32;
    let player_guid = ObjectGuid::create_player(1, 780);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 0, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
                effect_base_points: -35,
                effect_misc_value_1: 77,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented self-resurrect flat effect should execute");

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 35);
    let powers = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.get_power(PowerType::Mana),
                player.get_power(PowerType::Rage),
                player.get_power(PowerType::Energy),
                player.get_power(PowerType::Focus),
            )
        })
        .unwrap();
    assert_eq!(powers, (35, 77, 0, 100, 0));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_self_resurrect_percent_case_uses_max_health_and_mana_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 781_i32;
    let player_guid = ObjectGuid::create_player(1, 781);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 0, 400);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
                effect_base_points: 25,
                effect_misc_value_1: 999,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented self-resurrect percent effect should execute");

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 100);
    let powers = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.get_power(PowerType::Mana),
                player.get_power(PowerType::Rage),
                player.get_power(PowerType::Energy),
                player.get_power(PowerType::Focus),
            )
        })
        .unwrap();
    assert_eq!(powers, (100, 50, 0, 100, 0));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[tokio::test]
async fn spell_self_resurrect_skips_alive_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 782_i32;
    let player_guid = ObjectGuid::create_player(1, 782);
    configure_self_resurrect_canonical_player_like_cpp(&mut session, player_guid, 40, 100);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
                effect_base_points: -35,
                effect_misc_value_1: 77,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("alive player self-resurrect should be a C++ no-op");

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(session.player_health_like_cpp(), 40);
    let powers = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.unit().data().health,
                player.get_power(PowerType::Mana),
                player.get_power(PowerType::Rage),
                player.get_power(PowerType::Energy),
                player.get_power(PowerType::Focus),
            )
        })
        .unwrap();
    assert_eq!(powers, (40, 25, 50, 30, 40));
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
#[tokio::test]
async fn spell_stuck_teleports_home_and_sends_hearthstone_cooldown_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_map_store(crate::teleport_test_fixtures::world_maps([571, 0]));
    let spell_id = 783_i32;
    let player_guid = ObjectGuid::create_player(1, 783);
    let start = Position::new(10.0, 20.0, 30.0, 1.0);
    let home = Position::new(100.0, 200.0, 40.0, 2.0);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "StuckHome".to_string(),
        start,
        571,
        1,
        1,
        80,
        0,
    ));
    session.set_player_health_like_cpp(100, 100);
    let _ = session.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
        map_id: 0,
        area_id: 12,
        position: home,
    });
    set_stuck_spell_store_like_cpp(&mut session, spell_id);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented stuck spell should execute");

    assert_eq!(session.pending_teleport_like_cpp(), Some((0, home)));
    assert!(
        session
            .spell_last_cast_time_like_cpp(8690)
            .flatten()
            .is_some(),
        "C++ EffectStuck starts Hearthstone cooldown after successful home teleport"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::CancelCombat,
            ServerOpcodes::TransferPending,
            ServerOpcodes::SuspendToken,
            ServerOpcodes::CooldownEvent,
            ServerOpcodes::CooldownEvent,
        ]
    );
}
#[test]
fn login_pet_talent_reset_clears_pet_spells_and_specs_without_clearing_flag_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    const AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP: u16 = 0x010;

    session.set_represented_at_login_flags_like_cpp(AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP);
    session.load_represented_pet_stable_rows_like_cpp(
        42,
        [
            character_pet_stable_row_like_cpp(42, PetSaveMode::active_slot(0), 1),
            character_pet_stable_row_like_cpp(43, PetSaveMode::stable_slot(0), 1),
            character_pet_stable_row_like_cpp(44, PetSaveMode::NotInSlot as i16, 1),
        ],
    );
    session.load_represented_pet_spell_rows_like_cpp(
        42,
        [CharacterPetSpellRowLikeCpp {
            spell_id: 123,
            active: ActiveState::Enabled as u8,
        }],
    );
    session.load_represented_pet_spell_cooldown_rows_like_cpp(
        42,
        [CharacterPetSpellCooldownRowLikeCpp {
            spell_id: 123,
            cooldown_end_unix_secs: 1_000,
            category_id: 7,
            category_end_unix_secs: 2_000,
        }],
    );
    session.load_represented_pet_spell_charge_rows_like_cpp(
        42,
        [CharacterPetSpellChargeRowLikeCpp {
            category_id: 7,
            recharge_start_unix_secs: 1_000,
            recharge_end_unix_secs: 2_000,
        }],
    );

    assert!(session.apply_represented_login_pet_talent_reset_like_cpp());

    assert!(
        session
            .pet_load_query_holder_rows_like_cpp
            .spells
            .is_empty(),
        "C++ deletes pet_spell rows for all pets owned by the player"
    );
    assert_eq!(
        session.represented_pet_stable_like_cpp.active_pets[0]
            .as_ref()
            .map(|pet| pet.specialization_id),
        Some(0)
    );
    assert_eq!(
        session.represented_pet_stable_like_cpp.stabled_pets[0]
            .as_ref()
            .map(|pet| pet.specialization_id),
        Some(0)
    );
    assert_eq!(
        session.represented_pet_stable_like_cpp.unslotted_pets[0].specialization_id,
        0
    );
    assert!(
        !session
            .pet_load_query_holder_rows_like_cpp
            .spell_cooldowns
            .is_empty(),
        "C++ AT_LOGIN_RESET_PET_TALENTS does not delete pet_spell_cooldown rows in this block"
    );
    assert!(
        !session
            .pet_load_query_holder_rows_like_cpp
            .spell_charges
            .is_empty(),
        "C++ AT_LOGIN_RESET_PET_TALENTS does not delete pet_spell_charges rows in this block"
    );
    assert_eq!(
        session.represented_at_login_flags_like_cpp(),
        AT_LOGIN_RESET_PET_TALENTS_LIKE_CPP,
        "C++ does not call RemoveAtLoginFlag for AT_LOGIN_RESET_PET_TALENTS in CharacterHandler"
    );
    assert!(
        session
            .represented_at_login_flag_removals_like_cpp()
            .is_empty()
    );
}
#[tokio::test]
async fn first_login_cast_spells_use_player_create_mode_before_other_first_login_work_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 0xC501);
    session.player_guid = Some(player_guid);
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    session.set_player_create_mode_like_cpp(wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP);
    session.set_player_create_cast_spell_store_like_cpp(Arc::new(
        first_login_cast_spell_store_like_cpp(70_001, 70_002),
    ));
    session.set_spell_store(Arc::new(first_login_noop_spell_store_like_cpp([
        70_001, 70_002,
    ])));

    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawn_catalogs = session.creature_spawn_catalogs_for_test_like_cpp();
    let player_bootstrap = session.player_bootstrap_catalogs_for_test_like_cpp();
    let cast_count = session
        .apply_represented_first_login_cast_spells_with_catalogs_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            &player_bootstrap,
        )
        .await;

    assert_eq!(
        cast_count, 1,
        "C++ indexes PlayerInfo::castSpells with Player::GetCreateMode"
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent],
        "first-login cast spells execute through the represented spell path before explored/reputation branches"
    );
}
#[tokio::test]
async fn first_login_cast_spells_noop_without_store_or_player_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    assert_eq!(
        session
            .apply_represented_first_login_cast_spells_like_cpp()
            .await,
        0
    );
    assert!(send_rx.try_recv().is_err());

    session.player_guid = Some(ObjectGuid::create_player(1, 0xC502));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    assert_eq!(
        session
            .apply_represented_first_login_cast_spells_like_cpp()
            .await,
        0
    );
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn start_all_spells_applies_custom_player_create_spells_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    session.set_player_create_custom_spell_store_like_cpp(Arc::new(
        player_create_custom_spell_store_like_cpp(),
    ));
    let mut known_spells = vec![80_001];

    assert_eq!(
        session.apply_represented_start_all_spells_like_cpp(&mut known_spells),
        0,
        "C++ Player::LearnCustomSpells is gated by CONFIG_START_ALL_SPELLS"
    );
    assert_eq!(known_spells, vec![80_001]);
    assert!(
        session
            .represented_dependent_known_spells_like_cpp()
            .is_empty()
    );

    let mut player_bootstrap = session.player_bootstrap_catalogs_for_test_like_cpp();
    player_bootstrap.start_all_spells = true;
    assert_eq!(
        session.apply_represented_start_all_spells_with_catalogs_like_cpp(
            &player_bootstrap,
            &mut known_spells,
        ),
        1,
        "C++ AddSpell semantics do not duplicate an already known spell"
    );
    assert_eq!(known_spells, vec![80_001, 80_002]);
    assert!(
        session
            .represented_dependent_known_spells_like_cpp()
            .contains(&80_001),
        "C++ AddSpell(... dependent=true) marks even an already-known custom spell as dependent"
    );
    assert!(
        session
            .represented_dependent_known_spells_like_cpp()
            .contains(&80_002)
    );
}
#[test]
fn start_all_spells_uses_loaded_race_class_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_loaded_player_identity_like_cpp(0, 2, 1, 1, 0);
    session.set_player_create_custom_spell_store_like_cpp(Arc::new(
        player_create_custom_spell_store_like_cpp(),
    ));
    session.set_start_all_spells_like_cpp(true);
    let mut known_spells = Vec::new();

    assert_eq!(
        session.apply_represented_start_all_spells_like_cpp(&mut known_spells),
        1
    );
    assert_eq!(known_spells, vec![80_003]);
    assert!(
        session
            .represented_dependent_known_spells_like_cpp()
            .contains(&80_003)
    );
}
#[test]
fn load_represented_pet_spell_rows_filters_zero_spell_like_cpp() {
    let (mut session, _, _send_rx) = make_session();

    let loaded = session.load_represented_pet_spell_rows_like_cpp(
        42,
        [
            CharacterPetSpellRowLikeCpp {
                spell_id: 0,
                active: ActiveState::Enabled as u8,
            },
            CharacterPetSpellRowLikeCpp {
                spell_id: 123,
                active: ActiveState::Enabled as u8,
            },
        ],
    );

    assert_eq!(loaded, 1);
    assert_eq!(
        session
            .pet_load_query_holder_rows_like_cpp
            .spells
            .get(&42)
            .and_then(|spells| spells.first())
            .map(|spell| spell.spell_id),
        Some(123)
    );
}
#[test]
fn load_represented_pet_spell_cooldown_rows_filters_unknown_spell_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        1_234,
        gameobject_summon_spell_info_like_cpp(1_234, 0, Vec::new()),
    );
    session.set_spell_store(Arc::new(spell_store));

    let loaded = session.load_represented_pet_spell_cooldown_rows_like_cpp(
        42,
        [
            CharacterPetSpellCooldownRowLikeCpp {
                spell_id: 0,
                cooldown_end_unix_secs: 10,
                category_id: 9,
                category_end_unix_secs: 11,
            },
            CharacterPetSpellCooldownRowLikeCpp {
                spell_id: 1_234,
                cooldown_end_unix_secs: 12,
                category_id: 9,
                category_end_unix_secs: 13,
            },
            CharacterPetSpellCooldownRowLikeCpp {
                spell_id: 9_999,
                cooldown_end_unix_secs: 14,
                category_id: 10,
                category_end_unix_secs: 15,
            },
        ],
    );

    assert_eq!(loaded, 1);
    assert_eq!(
        session
            .pet_load_query_holder_rows_like_cpp
            .spell_cooldowns
            .get(&42)
            .and_then(|cooldowns| cooldowns.first())
            .map(|cooldown| cooldown.spell_id),
        Some(1_234)
    );
}
#[test]
fn load_represented_pet_spell_charge_rows_preserves_db_order_like_cpp() {
    let (mut session, _, _send_rx) = make_session();
    session.set_spell_category_store(Arc::new(SpellCategoryStore::from_entries([
        wow_data::SpellCategoryEntry {
            id: 88,
            name: "represented category".to_string(),
            flags: 0,
            uses_per_week: 0,
            max_charges: 2,
            charge_recovery_time: 30_000,
            type_mask: 0,
        },
    ])));

    let loaded = session.load_represented_pet_spell_charge_rows_like_cpp(
        42,
        [
            CharacterPetSpellChargeRowLikeCpp {
                category_id: 0,
                recharge_start_unix_secs: 1,
                recharge_end_unix_secs: 2,
            },
            CharacterPetSpellChargeRowLikeCpp {
                category_id: 88,
                recharge_start_unix_secs: 3,
                recharge_end_unix_secs: 4,
            },
            CharacterPetSpellChargeRowLikeCpp {
                category_id: 88,
                recharge_start_unix_secs: 5,
                recharge_end_unix_secs: 6,
            },
            CharacterPetSpellChargeRowLikeCpp {
                category_id: 99,
                recharge_start_unix_secs: 7,
                recharge_end_unix_secs: 8,
            },
        ],
    );

    assert_eq!(loaded, 2);
    let rows = session
        .pet_load_query_holder_rows_like_cpp
        .spell_charges
        .get(&42)
        .expect("represented pet charge rows");
    assert_eq!(rows[0].recharge_end_unix_secs, 4);
    assert_eq!(rows[1].recharge_end_unix_secs, 6);
}
