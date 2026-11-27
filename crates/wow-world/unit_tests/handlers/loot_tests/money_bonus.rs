use super::*;
use wow_data::spell::aura_types::SPELL_AURA_MOD_MONEY_GAIN;

const BONUS_SPELL: i32 = 61001;

#[tokio::test]
async fn loot_money_late_threshold_keeps_newer_progress_and_original_award_date() {
    use wow_persistence::LootMoneyCriteriaProgress;
    let (mut first, rx, _, _, _, _, _) = two_sessions_with_authoritative_creature_loot_like_cpp(
        authoritative_test_loot_like_cpp(10, false),
    );
    register_looter(&mut first, &Arc::new(PlayerRegistry::default()), 0);
    drain_server_opcodes_like_cpp(&rx);
    let newest = LootMoneyCriteriaProgress {
        counter: 10_000_007,
        date: 1_800_000_120,
        new_achievements: vec![1177],
        achievement_date: Some(1_800_000_120),
    };
    let earlier = LootMoneyCriteriaProgress {
        counter: 1_000_000,
        date: 1_800_000_000,
        new_achievements: vec![1176],
        achievement_date: Some(1_800_000_000),
    };
    let hub = crate::session::hub_ref(&first);
    hub.apply_loot_money_criteria(&newest);
    drain_server_opcodes_like_cpp(&rx);
    hub.apply_loot_money_criteria(&earlier);
    hub.apply_loot_money_criteria(&earlier);
    assert_eq!(
        drain_server_opcodes_like_cpp(&rx),
        [wow_constants::ServerOpcodes::AchievementEarned as u16]
    );
    let result = hub.loot_achievement_login_data();
    assert_eq!(result.progress[0].quantity, newest.counter);
    assert_eq!(result.earned.len(), 2);
    assert_eq!(result.earned[1].id, 1176);
    assert_eq!(
        result.earned[1].packed_date,
        wow_core::pack_session_wow_time(1_800_000_000, 0)
    );
}

#[tokio::test]
async fn loot_money_criteria_gate_and_base_share_belong_to_each_recipient() {
    for restricted_local in [false, true] {
        let (mut first, first_rx, mut second, second_rx, _, first_guid, second_guid) =
            two_sessions_with_authoritative_creature_loot_like_cpp(
                authoritative_test_loot_like_cpp(23, false),
            );
        let registry = Arc::new(PlayerRegistry::default());
        install_bonus_catalog(&mut first);
        second.core.account_state.cannot_earn_achievements = !restricted_local;
        register_looter(&mut first, &registry, 20);
        register_looter(&mut second, &registry, 50);
        first
            .core
            .with_owned_player_mut_like_cpp(|p| p.set_game_master_like_cpp(restricted_local))
            .unwrap();
        install_group_loot_group(&mut first, first_guid, second_guid);
        first.clear_loot_money_persistence_test_result_like_cpp();
        first.set_group_loot_money_persistence_port_like_cpp(Arc::new(
            GroupLootMoneyPortFixtureLikeCpp {
                calls: Arc::new(AtomicUsize::new(0)),
                chest: None,
            },
        ));
        first.handle_loot_money(loot_money_packet()).await;
        second.process_represented_session_commands_like_cpp().await;
        for (session, rx, eligible) in [
            (&first, &first_rx, !restricted_local),
            (&second, &second_rx, restricted_local),
        ] {
            let ops = drain_server_opcodes_like_cpp(rx);
            let payout = ops
                .iter()
                .position(|op| *op == ServerOpcodes::LootMoneyNotify as u16)
                .unwrap();
            let criterion = ops
                .iter()
                .position(|op| *op == ServerOpcodes::CriteriaUpdate as u16);
            assert_eq!(criterion.is_some(), eligible);
            if let Some(position) = criterion {
                assert!(position < payout);
            }
            let snapshot = crate::session::hub_ref(session).loot_achievement_login_data();
            assert_eq!(
                snapshot.progress.first().map(|row| row.quantity),
                eligible.then_some(11)
            );
        }
    }
}

fn install_bonus_catalog(session: &mut WorldSession) {
    let mut store =
        crate::selected_spell_fixtures::canonical_spell_store([(BONUS_SPELL as u32, 0)]);
    let apply = wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA;
    store.insert(
        BONUS_SPELL,
        wow_data::SpellInfo {
            spell_id: BONUS_SPELL,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: apply,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(SPELL_AURA_MOD_MONEY_GAIN),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect: apply,
                effect_aura: SPELL_AURA_MOD_MONEY_GAIN,
                effect_misc_value_1: 1,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(store));
    session.set_spell_group_store(Default::default());
    session.set_spell_group_stack_rule_store(Default::default());
    session.set_spell_chain_store(Default::default());
}

fn register_looter(session: &mut WorldSession, registry: &Arc<PlayerRegistry>, percent: i32) {
    let guid = session.player_guid().unwrap();
    let mut player = Box::new(wow_entities::Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player
        .unit_mut()
        .world_mut()
        .set_name(&format!("Looter{}", guid.counter()));
    player.unit_mut().set_max_health(100);
    player.unit_mut().set_health(100);
    player.insert_player_visible_aura_like_cpp(AuraApplication {
        spell_id: BONUS_SPELL,
        difficulty_id: 0,
        caster_guid: guid,
        slot: 3,
        duration_total: 30_000,
        duration_remaining: 30_000,
        stack_count: 1,
        aura_flags: 1,
        effect_mask: 1,
        aura_interrupt_flags: 0,
        aura_interrupt_flags2: 0,
        represented_effect: None,
        represented_amount: percent,
        represented_effect_amounts: vec![wow_entities::RepresentedAuraEffectAmountLikeCpp {
            effect_index: 0,
            amount: percent,
        }],
        represented_misc_value: Some(1),
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    });
    player.apply_player_threat_aura_like_cpp(
        BONUS_SPELL as u32,
        guid,
        3,
        wow_entities::AuraThreatSnapshotLikeCpp::new(
            [0, 0],
            vec![(1, SPELL_AURA_MOD_MONEY_GAIN, percent, 1)],
        ),
    );
    player.set_player_aura_authority_complete_like_cpp(true);
    let handle = {
        let mut manager = session
            .core
            .canonical_map_manager
            .as_ref()
            .unwrap()
            .lock()
            .unwrap();
        let handle = manager.install_detached_player_like_cpp(player).unwrap();
        manager
            .attach_player_like_cpp(handle, wow_map::MapKey::new(0, 0), Position::ZERO)
            .unwrap();
        handle
    };
    session.core.player_handle_like_cpp = Some(handle);
    session.core.player_identity_bootstrap_like_cpp = None;
    session.core.state = crate::session::SessionState::LoggedIn;
    session.set_player_registry(Arc::clone(registry));
    session.register_in_player_registry();
    assert!(registry.loot_presence(guid).unwrap().is_in_world);
}

#[tokio::test]
async fn loot_money_bonus_is_frozen_per_recipient_before_credit_and_publication() {
    let (mut first, first_rx, mut second, second_rx, _, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            23, false,
        ));
    let registry = Arc::new(PlayerRegistry::default());
    install_bonus_catalog(&mut first);
    register_looter(&mut first, &registry, 20);
    register_looter(&mut second, &registry, 50);
    install_group_loot_group(&mut first, first_guid, second_guid);
    for rx in [&first_rx, &second_rx] {
        drain_server_opcodes_like_cpp(rx);
    }

    second
        .core
        .with_owned_player_mut_like_cpp(|p| p.set_player_aura_authority_complete_like_cpp(false))
        .unwrap();
    first.handle_loot_money(loot_money_packet()).await;
    assert_eq!(
        (first.player_gold_like_cpp(), second.player_gold_like_cpp()),
        (0, 0)
    );
    for rx in [&first_rx, &second_rx] {
        let opcodes = drain_server_opcodes_like_cpp(rx);
        assert!(!opcodes.contains(&(wow_constants::ServerOpcodes::CoinRemoved as u16)));
        assert!(!opcodes.contains(&(wow_constants::ServerOpcodes::LootMoneyNotify as u16)));
    }
    second
        .core
        .with_owned_player_mut_like_cpp(|p| p.set_player_aura_authority_complete_like_cpp(true))
        .unwrap();
    first.handle_loot_money(loot_money_packet()).await;
    // A change after commit cannot change the already-earned remote share.
    second
        .core
        .with_owned_player_mut_like_cpp(|p| {
            p.remove_player_threat_aura_like_cpp(BONUS_SPELL as u32, second_guid, 3, 1);
            assert!(p.remove_player_visible_aura_like_cpp(3).is_some());
        })
        .unwrap();
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(
        (first.player_gold_like_cpp(), second.player_gold_like_cpp()),
        (13, 16)
    );
    for (rx, expected_bonus) in [(&first_rx, 2), (&second_rx, 5)] {
        recv_packet_with_opcode(rx, wow_constants::ServerOpcodes::CoinRemoved);
        let mut packet = recv_packet_with_opcode(rx, wow_constants::ServerOpcodes::LootMoneyNotify);
        assert_eq!(packet.read_uint64().unwrap(), 11);
        assert_eq!(packet.read_uint64().unwrap(), expected_bonus);
        assert!(!packet.read_bit().unwrap());
    }
    first.handle_loot_money(loot_money_packet()).await;
    second.process_represented_session_commands_like_cpp().await;
    assert_eq!(
        (first.player_gold_like_cpp(), second.player_gold_like_cpp()),
        (13, 16)
    );
}
