//! Spell handler misc scenarios.
//!
//! Split out of the inline test module under #624; assertions unchanged.

use super::*;

#[tokio::test]
async fn cancel_channelling_interrupts_matching_player_channel_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 8);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(basic_spell_store([12_345]));
    install_active_spell_cast(&mut session, 12_345, cast_id);
    install_canonical_channeled_spell(&mut session, player_guid, 12_345);

    session
        .handle_cancel_channelling(cancel_channelling_packet(12_345, 40))
        .await;

    assert_eq!(canonical_channeled_spell_id(&mut session), None);
    assert!(session.active_spell_cast_snapshot_like_cpp().is_none());
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_channelling_mismatched_spell_preserves_channel_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 9);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(basic_spell_store([67_890]));
    install_active_spell_cast(&mut session, 12_345, cast_id);
    let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

    session
        .handle_cancel_channelling(cancel_channelling_packet(67_890, 40))
        .await;

    assert_eq!(
        canonical_channeled_spell_id(&mut session),
        Some(spell.spell_id)
    );
    assert_eq!(
        session
            .active_spell_cast_snapshot_like_cpp()
            .as_ref()
            .map(|active_cast| active_cast.spell_id),
        Some(12_345)
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_channelling_zero_spell_preserves_channel_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 10);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(basic_spell_store([12_345]));
    install_active_spell_cast(&mut session, 12_345, cast_id);
    let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

    session
        .handle_cancel_channelling(cancel_channelling_packet(0, 40))
        .await;

    assert_eq!(
        canonical_channeled_spell_id(&mut session),
        Some(spell.spell_id)
    );
    assert_eq!(
        session
            .active_spell_cast_snapshot_like_cpp()
            .as_ref()
            .map(|active_cast| active_cast.spell_id),
        Some(12_345)
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_channelling_missing_spellinfo_preserves_channel_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 11);
    install_canonical_player(&mut session, &canonical, player_guid);
    session.set_spell_store(basic_spell_store([]));
    install_active_spell_cast(&mut session, 12_345, cast_id);
    let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

    session
        .handle_cancel_channelling(cancel_channelling_packet(12_345, 40))
        .await;

    assert_eq!(
        canonical_channeled_spell_id(&mut session),
        Some(spell.spell_id)
    );
    assert_eq!(
        session
            .active_spell_cast_snapshot_like_cpp()
            .as_ref()
            .map(|active_cast| active_cast.spell_id),
        Some(12_345)
    );
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_queued_spell_clears_only_pending_request_like_cpp() {
    let (mut session, send_rx) = make_session();
    let active_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7);
    let pending_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 8);
    install_active_spell_cast(&mut session, 12_345, active_cast_id);
    install_pending_spell_cast_request(&mut session, 67_890, pending_cast_id);

    session
        .handle_cancel_queued_spell(WorldPacket::new_empty())
        .await;

    assert_eq!(
        session
            .active_spell_cast_snapshot_like_cpp()
            .as_ref()
            .map(|active_cast| active_cast.spell_id),
        Some(12_345)
    );
    assert!(session.pending_spell_cast_for_test_like_cpp().is_none());

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    assert_eq!(
        cast_failed_fields_like_cpp(&packets[0]),
        (pending_cast_id, 67_890, 32),
        "C++ Player::CancelPendingCastRequest sends SPELL_FAILED_DONT_REPORT for the queued request"
    );
}
#[tokio::test]
async fn cancel_queued_spell_without_pending_request_is_silent_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_cancel_queued_spell(WorldPacket::new_empty())
        .await;

    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_empty_spell_handlers_stay_silent_without_runtime_slots_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_cancel_auto_repeat_spell(WorldPacket::new_empty())
        .await;
    session
        .handle_cancel_growth_aura(WorldPacket::new_empty())
        .await;
    session
        .handle_cancel_mount_aura(WorldPacket::new_empty())
        .await;

    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn cancel_mod_speed_no_control_ignores_non_mover_guid_like_cpp() {
    let (mut session, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    session.set_player_guid(Some(player_guid));
    session.set_spell_store(mod_speed_no_control_spell_store(12_345, false));

    session
        .execute_spell(12_345, player_guid)
        .await
        .expect("represented mod-speed-no-control aura should apply");
    let _ = drain_server_opcodes(&send_rx);

    assert!(
        !session
            .try_handle_cancel_mod_speed_no_control_auras_like_cpp(
                cancel_mod_speed_no_control_packet(other_guid),
            )
            .await
    );

    assert!(session.visible_auras.values().any(|aura| {
        aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModSpeedNoControl)
    }));
}
#[tokio::test]
async fn self_res_unlisted_spell_stays_silent_like_cpp() {
    let (mut session, send_rx) = make_session();
    let spell_id = 20_000;

    session.set_player_guid(Some(ObjectGuid::create_player(1, 20_000)));
    session.set_spell_store(self_res_spell_store(spell_id));
    session.set_player_health_like_cpp(0, 100);

    session.handle_self_res(int32_spell_packet(spell_id)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn self_res_missing_spell_info_keeps_self_res_spell_like_cpp() {
    let (mut session, send_rx) = make_session();
    let spell_id = 20_002;

    session.set_player_guid(Some(ObjectGuid::create_player(1, 20_002)));
    session.set_player_health_like_cpp(0, 100);
    session.add_represented_self_res_spell_like_cpp(spell_id);

    session.handle_self_res(int32_spell_packet(spell_id)).await;

    assert!(!session.player_is_alive_like_cpp());
    assert!(session.has_represented_self_res_spell_like_cpp(spell_id));
    assert!(send_rx.is_empty());
}
#[tokio::test]
async fn queued_spell_with_mana_cost_deducts_when_executed_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 13_354;
    install_canonical_player(&mut session, &canonical, player_guid);
    set_canonical_player_mana_like_cpp(&mut session, 500, 1000);
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(
        spell_store_with_global_cooldown_and_mana_power_cost_like_cpp(spell_id, 1_500, 50, 10.0),
    );
    session.mutate_cast_execution_like_cpp(|state| {
        state.last_cast_time =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1_200))
    });

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert!(
        session
            .pending_spell_cast_for_test_like_cpp()
            .as_ref()
            .is_some_and(|pending| pending.spell_id == spell_id)
    );
    assert!(
        send_rx.is_empty(),
        "C++ Player::_pendingSpellCastRequest queues without executing or spending power"
    );
    assert_eq!(canonical_player_mana_like_cpp(&mut session), 500);

    session.mutate_cast_execution_like_cpp(|state| {
        state.last_cast_time =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1_500))
    });
    session.tick_pending_spell_cast_request_like_cpp().await;

    assert!(session.pending_spell_cast_for_test_like_cpp().is_none());
    assert_eq!(
        canonical_player_mana_like_cpp(&mut session),
        350,
        "queued C++ casts still run Spell::TakePower when the pending request executes"
    );
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&ServerOpcodes::UpdateObject));
    assert!(opcodes.contains(&ServerOpcodes::SpellGo));
    assert!(opcodes.contains(&ServerOpcodes::CooldownEvent));
}
#[tokio::test]
async fn mana_cost_spell_on_non_mana_class_does_not_spend_primary_power_like_cpp() {
    let (mut session, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 42);
    let spell_id = 13_355;
    install_canonical_player(&mut session, &canonical, player_guid);
    assert!(session.sync_canonical_player_primary_power_like_cpp(PowerType::Rage, 500, 1000, 0));
    session.set_known_spells_like_cpp(vec![spell_id]);
    session.set_spell_store(spell_store_with_mana_power_cost_like_cpp(spell_id, 50, 0.0));

    session
        .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
        .await;

    assert_eq!(
        canonical_player_power_like_cpp(&mut session, PowerType::Rage),
        500,
        "C++ GetPowerIndexByClass has no Mana slot for warrior, so mana-cost spells cannot spend Rage as Mana"
    );
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 2);
    assert_eq!(
        &packets[0][..2],
        &(ServerOpcodes::SpellPrepare as u16).to_le_bytes()
    );
    assert_eq!(
        cast_failed_reason_like_cpp(&packets[1]),
        SpellCastResult::NoPower as i32
    );
}
#[test]
fn item_money_loot_generation_matches_cpp_boundary_branches() {
    let mut rng = StdRng::seed_from_u64(0xC0FFEE);

    assert_eq!(
        wow_loot::generate_money_loot_with_rate_like_cpp(0, 0, 1.0, &mut rng),
        0
    );
    let swapped = normalize_item_money_loot_bounds_like_cpp(120, 100);
    assert_eq!(swapped, (100, 120));
    let swapped_roll =
        wow_loot::generate_money_loot_with_rate_like_cpp(swapped.0, swapped.1, 1.0, &mut rng);
    assert!((100..=120).contains(&swapped_roll));
    assert_eq!(
        wow_loot::generate_money_loot_with_rate_like_cpp(100, 100, 1.0, &mut rng),
        100
    );
    let swapped_rate_roll =
        wow_loot::generate_money_loot_with_rate_like_cpp(swapped.0, swapped.1, 2.5, &mut rng);
    assert!((250..=300).contains(&swapped_rate_roll));

    let small_range = wow_loot::generate_money_loot_with_rate_like_cpp(100, 200, 1.0, &mut rng);
    assert!((100..=200).contains(&small_range));

    let wide_range =
        wow_loot::generate_money_loot_with_rate_like_cpp(1_000, 100_000, 1.0, &mut rng);
    assert_eq!(wide_range & 0xFF, 0);
    assert!((((1_000 >> 8) << 8)..=((100_000 >> 8) << 8)).contains(&wide_range));
}
#[tokio::test]
async fn item_template_addon_port_preserves_found_rows_and_request_order() {
    let fixture = ItemTemplateAddonCatalogPortFixtureLikeCpp::new(
        [ItemTemplateAddonMoneyOutcomeLikeCpp::Found(
            wow_persistence::ItemTemplateAddonMoneyRowLikeCpp {
                min_money: Some(120),
                max_money: Some(100),
            },
        )],
        [ItemTemplateAddonLootMetadataOutcomeLikeCpp::Found(
            wow_persistence::ItemTemplateAddonLootMetadataRowLikeCpp {
                flags_cu: ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
                quest_log_item_id: 77,
            },
        )],
    );
    let (mut session, _send_rx) = make_session();
    session.set_item_template_addon_catalog_persistence_port_like_cpp(fixture.clone());

    assert_eq!(
        session
            .load_item_template_addon_money_loot_like_cpp(1001)
            .await,
        (100, 120)
    );
    assert_eq!(
        session
            .load_item_template_addon_loot_metadata_like_cpp(1002)
            .await,
        ItemTemplateAddonLootMetadataLikeCpp {
            flags_cu: ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
            quest_log_item_id: 77,
        }
    );
    assert_eq!(
        *fixture.money_requests.lock().unwrap(),
        [ItemTemplateAddonCatalogRequestLikeCpp { item_entry: 1001 }]
    );
    assert_eq!(
        *fixture.metadata_requests.lock().unwrap(),
        [ItemTemplateAddonCatalogRequestLikeCpp { item_entry: 1002 }]
    );
}
#[tokio::test]
async fn item_template_addon_port_preserves_absent_malformed_and_failed_defaults() {
    let fixture = ItemTemplateAddonCatalogPortFixtureLikeCpp::new(
        [
            ItemTemplateAddonMoneyOutcomeLikeCpp::Found(
                wow_persistence::ItemTemplateAddonMoneyRowLikeCpp {
                    min_money: Some(10),
                    max_money: None,
                },
            ),
            ItemTemplateAddonMoneyOutcomeLikeCpp::Missing,
            ItemTemplateAddonMoneyOutcomeLikeCpp::Failed {
                reason: "world read failed".into(),
            },
        ],
        [
            ItemTemplateAddonLootMetadataOutcomeLikeCpp::Missing,
            ItemTemplateAddonLootMetadataOutcomeLikeCpp::Failed {
                reason: "world read failed".into(),
            },
        ],
    );
    let (mut session, _send_rx) = make_session();
    session.set_item_template_addon_catalog_persistence_port_like_cpp(fixture);

    for item_entry in [2001, 2002, 2003] {
        assert_eq!(
            session
                .load_item_template_addon_money_loot_like_cpp(item_entry)
                .await,
            (0, 0)
        );
    }
    for item_entry in [2004, 2005] {
        assert_eq!(
            session
                .load_item_template_addon_loot_metadata_like_cpp(item_entry)
                .await,
            ItemTemplateAddonLootMetadataLikeCpp::default()
        );
    }

    let (session_without_port, _send_rx) = make_session();
    assert_eq!(
        session_without_port
            .load_item_template_addon_money_loot_like_cpp(2006)
            .await,
        (0, 0)
    );
    assert_eq!(
        session_without_port
            .load_item_template_addon_loot_metadata_like_cpp(2007)
            .await,
        ItemTemplateAddonLootMetadataLikeCpp::default()
    );
}
#[test]
fn loot_rate_helpers_match_cpp_roll_inputs() {
    let mut rng = StdRng::seed_from_u64(0xA11CE);
    assert!(roll_chance_with_rate_like_cpp(100.0, 0.0, &mut rng));
    assert!(roll_chance_with_rate_like_cpp(60.0, 2.0, &mut rng));
    assert_eq!(referenced_loot_max_count_like_cpp(3, 2.0), 6);
    assert_eq!(referenced_loot_max_count_like_cpp(3, 0.5), 1);
}
#[test]
fn item_loot_quest_status_custom_metadata_matches_cpp_gate() {
    assert!(item_loot_quest_status_allows_like_cpp(
        true, true, true, false
    ));
    assert!(item_loot_quest_status_allows_like_cpp(
        false, false, false, false
    ));
    assert!(!item_loot_quest_status_allows_like_cpp(
        false, true, false, false
    ));
    assert!(!item_loot_quest_status_allows_like_cpp(
        false, false, true, false
    ));
    assert!(item_loot_quest_status_allows_like_cpp(
        false, true, false, true
    ));

    let metadata = ItemTemplateAddonLootMetadataLikeCpp {
        flags_cu: ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP,
        quest_log_item_id: 25,
    };
    assert!(metadata.ignores_quest_status());
    assert_eq!(metadata.quest_log_item_id, 25);
}
#[test]
fn loot_entry_follow_rules_metadata_matches_cpp_constructor() {
    let normal = loot_entry_flags_for_row_metadata_like_cpp(
        false,
        ItemFlags::empty(),
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    );
    assert!(normal.follow_loot_rules);
    assert!(!normal.needs_quest);
    assert!(!normal.freeforall);

    let quest_only_without_custom = loot_entry_flags_for_row_metadata_like_cpp(
        true,
        ItemFlags::empty(),
        ItemTemplateAddonLootMetadataLikeCpp::default(),
    );
    assert!(!quest_only_without_custom.follow_loot_rules);
    assert!(quest_only_without_custom.needs_quest);

    let quest_only_following_rules = loot_entry_flags_for_row_metadata_like_cpp(
        true,
        ItemFlags::MULTI_DROP,
        ItemTemplateAddonLootMetadataLikeCpp {
            flags_cu: ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
            quest_log_item_id: 0,
        },
    );
    assert!(quest_only_following_rules.follow_loot_rules);
    assert!(quest_only_following_rules.freeforall);
    assert!(quest_only_following_rules.needs_quest);
}
#[test]
fn loot_entry_free_for_all_view_metadata_matches_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let normal = LootEntry {
        loot_list_id: 0,
        item_id: 25,
        quantity: 1,
        random_properties_id: 0,
        random_properties_seed: 0,
        item_context: 0,
        flags: LootEntryFlags::default(),
        allowed_looters: vec![player_guid],
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    };
    assert_eq!(normal.free_for_all_ui_type_like_cpp(), 4);
    assert!(normal.is_over_threshold_like_cpp());
    assert!(normal.visible_in_represented_free_for_all_view_like_cpp(player_guid));
    assert!(!normal.visible_in_represented_free_for_all_view_like_cpp(other_guid));

    let ffa = LootEntry {
        flags: LootEntryFlags {
            freeforall: true,
            needs_quest: true,
            ..Default::default()
        },
        ..normal.clone()
    };
    assert_eq!(ffa.free_for_all_ui_type_like_cpp(), 4);
    assert!(!ffa.is_over_threshold_like_cpp());
    assert!(ffa.flags.needs_quest);
    assert!(!ffa.is_looted_for_player_like_cpp(player_guid));
    let mut looted_ffa = ffa.clone();
    looted_ffa.add_allowed_looter_like_cpp(other_guid);
    looted_ffa.mark_looted_for_player_like_cpp(player_guid);
    assert!(!looted_ffa.taken);
    assert!(looted_ffa.is_looted_for_player_like_cpp(player_guid));
    assert!(!looted_ffa.is_looted_for_player_like_cpp(other_guid));
    assert!(!looted_ffa.fully_looted_like_cpp());
    looted_ffa.mark_looted_for_player_like_cpp(other_guid);
    assert!(looted_ffa.fully_looted_like_cpp());

    let under_threshold = LootEntry {
        flags: LootEntryFlags {
            under_threshold: true,
            ..Default::default()
        },
        ..normal
    };
    assert!(!under_threshold.is_over_threshold_like_cpp());

    let taken = LootEntry {
        taken: true,
        ..under_threshold
    };
    assert!(!taken.visible_in_represented_free_for_all_view_like_cpp(player_guid));
}
#[test]
fn add_loot_item_stacks_caps_like_cpp_max_nr_loot_items() {
    let mut loot_items = Vec::new();
    add_loot_item_stacks_like_cpp(&mut loot_items, 25, 45, 20, Default::default());
    assert_eq!(loot_items.len(), 3);
    assert_eq!(loot_items[0].quantity, 20);
    assert_eq!(loot_items[1].quantity, 20);
    assert_eq!(loot_items[2].quantity, 5);
    assert_eq!(loot_items[2].loot_list_id, 2);
    assert_eq!(loot_items[2].random_properties_id, 0);
    assert_eq!(loot_items[2].random_properties_seed, 0);
    assert_eq!(loot_items[2].item_context, 0);

    let mut capped = Vec::new();
    add_loot_item_stacks_like_cpp(&mut capped, 25, 100, 1, Default::default());
    assert_eq!(capped.len(), 18);
    assert_eq!(capped[17].loot_list_id, 17);
}
#[test]
fn stored_loot_item_persistence_skips_missing_template_and_currency_tokens_like_cpp() {
    // template missing -> no persist
    assert!(!stored_loot_item_should_persist_like_cpp(
        false,
        BagFamilyMask::NONE
    ));

    // normal template -> persist
    assert!(stored_loot_item_should_persist_like_cpp(
        true,
        BagFamilyMask::NONE
    ));

    // currency token -> no persist (C++ ItemTemplate::IsCurrencyToken)
    assert!(!stored_loot_item_should_persist_like_cpp(
        true,
        BagFamilyMask::CURRENCY_TOKENS
    ));

    // currency token combined with other families still no persist
    assert!(!stored_loot_item_should_persist_like_cpp(
        true,
        BagFamilyMask::CURRENCY_TOKENS | BagFamilyMask::HERBS
    ));
}
