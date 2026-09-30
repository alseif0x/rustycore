use super::*;

#[test]
fn current_player_item_enchantment_plan_removes_canonical_duration_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_510);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 90_511);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_skill_values_like_cpp(HashMap::from([(333, 80)]));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 905,
            effect_arg: [0; 3],
            effect_points_min: [0; 3],
            effect_scaling_points: [0.0; 3],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 333,
            required_skill_rank: 75,
            item_level: 1,
            charges: 0,
            effect: [ItemEnchantmentType::None as u8; 3],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
    ])));

    session.insert_inventory_item_like_cpp(
        EQUIPMENT_SLOT_MAINHAND,
        InventoryItem {
            guid: item_guid,
            entry_id: 700,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Weapon as u8),
        },
    );
    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 905, 12_000, 0);
    session.insert_inventory_item_object(item.clone());
    session
        .mutate_canonical_player_like_cpp(|player| {
            player.add_enchantment_duration(
                &mut item,
                EnchantmentSlot::EnhancementTemporary,
                12_000,
            );
            assert_eq!(player.enchant_durations().len(), 1);
        })
        .unwrap();

    let plan = session
        .apply_current_player_item_enchantment_plan_like_cpp(
            item_guid,
            EnchantmentSlot::EnhancementTemporary,
            ApplyEnchantmentArgs::remove(),
        )
        .expect("canonical player should receive enchantment remove plan");

    assert!(
        matches!(
        plan.result,
        ApplyEnchantmentResult::Applied {
            apply: false,
            duration_action: Some(ApplyEnchantmentDurationAction::Removed {
                item_guid: removed_guid,
                slot: EnchantmentSlot::EnhancementTemporary,
            }),
            ..
        } if removed_guid == item_guid
        ),
        "unexpected plan: {plan:?}"
    );
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| player.enchant_durations().is_empty())
            .unwrap()
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&item_guid]
            .data()
            .enchantments[EnchantmentSlot::EnhancementTemporary as usize]
            .id,
        905
    );
}
#[test]
fn loaded_equipped_item_enchantments_apply_and_send_durations_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 90_520);
    let player_position = Position::new(1.0, 2.0, 3.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 90_521);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session
        .mutate_canonical_player_like_cpp(|player| player.unit_mut().set_level(80))
        .unwrap();
    session.set_spell_item_enchantment_store(Arc::new(SpellItemEnchantmentStore::from_entries([
        SpellItemEnchantmentEntry {
            id: 903,
            effect_arg: [0; 3],
            effect_points_min: [0; 3],
            effect_scaling_points: [0.0; 3],
            item_visual: 0,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 1,
            charges: 0,
            effect: [ItemEnchantmentType::None as u8; 3],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
        SpellItemEnchantmentEntry {
            id: 904,
            effect_arg: [0; 3],
            effect_points_min: [0; 3],
            effect_scaling_points: [0.0; 3],
            item_visual: 44,
            flags: SpellItemEnchantmentFlags::empty(),
            required_skill_id: 0,
            required_skill_rank: 0,
            item_level: 1,
            charges: 0,
            effect: [ItemEnchantmentType::None as u8; 3],
            condition_id: 0,
            min_level: 1,
            max_level: 0,
        },
    ])));

    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    item.set_enchantment(EnchantmentSlot::EnhancementPermanent, 904, 0, 0);
    item.set_enchantment(EnchantmentSlot::EnhancementTemporary, 903, 6_000, 0);
    session.insert_inventory_item_object(item);

    let outcome = session.apply_loaded_equipped_item_enchantments_like_cpp(item_guid);

    assert_eq!(outcome.plans.len(), 2);
    assert!(outcome.plans.iter().any(|plan| matches!(
        plan.result,
        ApplyEnchantmentResult::Applied {
            item_guid: applied_item_guid,
            slot: EnchantmentSlot::EnhancementPermanent,
            enchantment_id: 904,
            apply: true,
            update_permanent_visible_item: true,
            ..
        } if applied_item_guid == item_guid
    )));
    assert!(outcome.plans.iter().any(|plan| matches!(
        plan.result,
        ApplyEnchantmentResult::Applied {
            item_guid: applied_item_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            enchantment_id: 903,
            apply: true,
            duration_action: Some(ApplyEnchantmentDurationAction::Added(
                PlayerEnchantTimeUpdate {
                    item_guid: duration_item_guid,
                    slot: EnchantmentSlot::EnhancementTemporary,
                    duration_secs: 6,
                }
            )),
            ..
        } if applied_item_guid == item_guid && duration_item_guid == item_guid
    )));
    assert_eq!(
        outcome.visible_item_changes,
        vec![(EQUIPMENT_SLOT_MAINHAND, 700, 0, 44)]
    );
    assert_eq!(
        outcome.duration_updates,
        vec![PlayerEnchantTimeUpdate {
            item_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            duration_secs: 6,
        }]
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.enchant_durations().to_vec()),
        Some(vec![PlayerEnchantDuration {
            item_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            left_duration_ms: 6_000,
        }])
    );
    assert!(
        send_rx.try_recv().is_err(),
        "loaded enchant replay queues packets until after login CREATE"
    );
    session.send_loaded_equipped_item_enchantment_updates_like_cpp(&outcome);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        ItemEnchantTimeUpdate {
            owner_guid: player_guid,
            item_guid,
            duration_left: 6,
            slot: EnchantmentSlot::EnhancementTemporary as u32,
        }
        .to_bytes()
    );
    assert!(
        drain_server_packet_bytes(&send_rx)
            .iter()
            .any(|bytes| WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::UpdateObject)),
        "permanent enchant visual update is emitted after login CREATE"
    );
}
