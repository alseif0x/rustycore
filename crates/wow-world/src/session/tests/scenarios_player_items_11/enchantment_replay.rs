use super::*;

#[test]
fn loaded_socket_enchantment_replay_enforces_prismatic_and_gem_skills_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_530);
    let item_guid = ObjectGuid::create_item(1, 90_531);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(
        &canonical,
        player_guid,
        Position::new(1.0, 2.0, 3.0, 0.0),
        571,
        0,
    );
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    let enchantment = |id, required_skill_id, required_skill_rank| SpellItemEnchantmentEntry {
        id,
        effect_arg: [0; 3],
        effect_points_min: [0; 3],
        effect_scaling_points: [0.0; 3],
        item_visual: 0,
        flags: SpellItemEnchantmentFlags::empty(),
        required_skill_id,
        required_skill_rank,
        item_level: 1,
        charges: 0,
        effect: [ItemEnchantmentType::None as u8; 3],
        condition_id: 0,
        min_level: 1,
        max_level: 0,
    };
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        enchantment(913, 0, 0),
        enchantment(914, 755, 350),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_templates(std::iter::empty::<(u32, ItemSparseTemplateEntry)>())
            .with_socket_templates([
                (
                    700,
                    ItemSocketTemplateEntry {
                        socket_types: [0, 2, 2],
                        required_skill_id: 0,
                        required_skill_rank: 0,
                    },
                ),
                (
                    800,
                    ItemSocketTemplateEntry {
                        socket_types: [0; 3],
                        required_skill_id: 202,
                        required_skill_rank: 300,
                    },
                ),
            ]),
    ));
    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    item.set_gems(vec![SocketedGem {
        item_id: 800,
        context: 0,
        bonus_list_ids: Vec::new(),
    }]);
    item.set_enchantment(EnchantmentSlot::EnhancementSocket, 913, 0, 0);
    item.set_enchantment(EnchantmentSlot::EnhancementSocketPrismatic, 914, 0, 0);
    session.insert_inventory_item_object(item);

    session.set_player_skill_values_like_cpp(HashMap::from([(755, 349), (202, 299)]));
    let prismatic_blocked = session
        .apply_current_player_item_enchantment_plan_like_cpp(
            item_guid,
            EnchantmentSlot::EnhancementSocket,
            ApplyEnchantmentArgs::apply(),
        )
        .unwrap();
    assert!(matches!(
        prismatic_blocked.result,
        ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::PrismaticRequiredSkillTooLow)
    ));

    session.set_player_skill_values_like_cpp(HashMap::from([(755, 350), (202, 299)]));
    let gem_blocked = session
        .apply_current_player_item_enchantment_plan_like_cpp(
            item_guid,
            EnchantmentSlot::EnhancementSocket,
            ApplyEnchantmentArgs::apply(),
        )
        .unwrap();
    assert!(matches!(
        gem_blocked.result,
        ApplyEnchantmentResult::Skipped(ApplyEnchantmentSkipReason::GemRequiredSkillTooLow)
    ));

    session.set_player_skill_values_like_cpp(HashMap::from([(755, 350), (202, 300)]));
    let allowed = session
        .apply_current_player_item_enchantment_plan_like_cpp(
            item_guid,
            EnchantmentSlot::EnhancementSocket,
            ApplyEnchantmentArgs::apply(),
        )
        .unwrap();
    assert!(matches!(
        allowed.result,
        ApplyEnchantmentResult::Applied { .. }
    ));
}
#[test]
fn loaded_broken_equipped_item_skips_enchantment_replay_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 90_526);
    let item_guid = ObjectGuid::create_item(1, 90_527);
    session.set_player_guid(Some(player_guid));

    let mut item = session.make_inventory_item_object(
        item_guid,
        703,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    item.set_max_durability(100);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);
    assert!(item.is_broken());
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(outcome.plans.is_empty());
    assert!(outcome.duration_updates.is_empty());
    assert!(!outcome.send_stat_update);
    assert!(outcome.visible_item_changes.is_empty());
    assert!(outcome.effect_actions.is_empty());
    assert!(outcome.unrepresented_effect_actions.is_empty());
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn loaded_top_level_bag_skips_enchantment_replay_like_cpp_item_is_equipped() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 90_532);
    let item_guid = ObjectGuid::create_item(1, 90_533);
    session.set_player_guid(Some(player_guid));

    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);
    assert!(!item.is_equipped());
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(outcome.plans.is_empty());
    assert!(outcome.duration_updates.is_empty());
    assert!(!outcome.send_stat_update);
    assert!(outcome.visible_item_changes.is_empty());
    assert!(outcome.effect_actions.is_empty());
    assert!(outcome.unrepresented_effect_actions.is_empty());
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn loaded_disarmed_mainhand_skips_enchantment_replay_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_528);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 90_529);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_unit_flags_like_cpp(UnitFlags::DISARMED);
        })
        .unwrap();

    let mut item = session.make_inventory_item_object(
        item_guid,
        704,
        player_guid,
        1,
        1,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(outcome.plans.is_empty());
    assert!(outcome.duration_updates.is_empty());
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.enchant_durations().len()),
        Some(0)
    );
}
#[test]
fn loaded_equipped_item_enchantments_apply_effect_actions_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_522);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 90_523);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 906,
            effect_arg: [ItemModType::Health as u32, 0, 0],
            effect_points_min: [17, 0, 0],
            effect_scaling_points: [0.0; 3],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 1,
            charges: 0,
            effect: [
                ItemEnchantmentType::Stat as u8,
                ItemEnchantmentType::None as u8,
                ItemEnchantmentType::None as u8,
            ],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
    ])));

    let mut item = session.make_inventory_item_object(
        item_guid,
        701,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 906, 0, 0);
    session.insert_inventory_item_object(item.clone());

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert!(matches!(
        outcome.plans.first().map(|plan| plan.result),
        Some(ApplyEnchantmentResult::Applied {
            enchantment_id: 906,
            apply: true,
            effects_allowed: true,
            ..
        })
    ));
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().health_base,
        17
    );
    assert_eq!(outcome.effect_actions.len(), 1);
    assert!(matches!(
        outcome.effect_actions[0].action,
        ApplyEnchantmentEffectAction::UnitModifier { .. }
    ));
    assert!(outcome.unrepresented_effect_actions.is_empty());
    assert!(outcome.send_stat_update);
    assert!(
        send_rx.try_recv().is_err(),
        "loaded enchant stat update is queued until after login CREATE"
    );
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(
        packets.len(),
        1,
        "only the permanent-enchant visual delta remains; stat bonuses are folded into the full login snapshot"
    );
    assert_eq!(
        WorldPacket::from_bytes(&packets[0]).server_opcode(),
        Some(ServerOpcodes::UpdateObject)
    );

    session.clear_all_inventory_runtime_like_cpp();
    assert!(session.represented_item_bonus_actions_like_cpp().is_empty());
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().health_base,
        0,
        "logout discards the old Player's item modifier state"
    );

    session.insert_inventory_item_object(item);
    let relogin = session.apply_initial_loaded_item_mods_like_cpp(&[item_guid]);
    assert!(relogin.enchantments.send_stat_update);
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().health_base,
        17,
        "relogin replays the new Player's enchantment once instead of carrying or doubling the previous bonus"
    );
}
