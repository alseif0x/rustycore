use super::*;

#[tokio::test]
async fn repair_all_inventory_item_durability_uses_guild_bank_limit_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let weapon_guid = ObjectGuid::create_item(1, 900);
    let bag_guid = ObjectGuid::create_item(1, 901);
    let armor_guid = ObjectGuid::create_item(1, 902);
    session.set_player_guid(Some(player_guid));
    session.set_player_gold_like_cpp(500);
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: 100,
            class_id: ItemClass::Armor as u8,
            subclass_id: ItemSubClassArmor::Shield as u8,
            material: 0,
            inventory_type: InventoryType::Shield as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 101,
            class_id: ItemClass::Armor as u8,
            subclass_id: 4,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: 200,
            class_id: ItemClass::Container as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::Bag as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    let sparse = |inventory_type: InventoryType, max_durability: u32| ItemSparseTemplateEntry {
        flags: [0; 4],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 1,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 1.0,
        price_random_value: 0.0,
        max_durability,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0; 2],
        required_reputation_faction: 0,
        allowable_class: 0,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: if inventory_type == InventoryType::Bag {
            4
        } else {
            0
        },
        inventory_type: inventory_type as i8,
    };
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            [(
                100,
                ItemStatEntry {
                    stats: [
                        (ItemModType::Strength as i8, 12),
                        (ItemModType::HitRating as i8, 5),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                    ],
                    resistances: [17, 0, 7, 0, 0, 0, 0],
                    armor: 17,
                },
            )],
            [
                (100, sparse(InventoryType::Shield, 50)),
                (101, sparse(InventoryType::Chest, 13)),
                (200, sparse(InventoryType::Bag, 0)),
            ],
            [
                (
                    100,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 57,
                        quality: ItemQuality::Rare as i8,
                        inventory_type: InventoryType::Shield as i8,
                    },
                ),
                (
                    101,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 57,
                        quality: ItemQuality::Rare as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
                (
                    200,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 57,
                        quality: ItemQuality::Normal as i8,
                        inventory_type: InventoryType::Bag as i8,
                    },
                ),
            ],
        ),
    ));
    session.set_durability_costs_store(Arc::new(DurabilityCostsStore::from_entries([
        DurabilityCostsEntry {
            id: 57,
            weapon_sub_class_cost: std::array::from_fn(|_| 0),
            armor_sub_class_cost: std::array::from_fn(|i| {
                if i == ItemSubClassArmor::Shield as usize {
                    13
                } else if i == 4 {
                    5
                } else {
                    0
                }
            }),
        },
    ])));
    let mut shield_block_rows = vec![ShieldBlockRegularEntryLikeCpp::default(); 57];
    shield_block_rows[56].superior = 42.0;
    session.set_shield_block_regular_game_table(Arc::new(
        ShieldBlockRegularGameTableLikeCpp::from_rows(shield_block_rows),
    ));
    session.set_durability_quality_store(Arc::new(DurabilityQualityStore::from_entries([
        DurabilityQualityEntry {
            id: (ItemQuality::Rare as u32 + 1) * 2,
            data: 1.25,
        },
    ])));
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            EQUIPMENT_SLOT_OFFHAND,
            InventoryItem {
                guid: weapon_guid,
                entry_id: 100,
                db_guid: weapon_guid.counter() as u64,
                inventory_type: Some(InventoryType::Shield as u8),
            },
        );
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            INVENTORY_SLOT_BAG_START,
            InventoryItem {
                guid: bag_guid,
                entry_id: 200,
                db_guid: bag_guid.counter() as u64,
                inventory_type: Some(InventoryType::Bag as u8),
            },
        );
    let weapon = session.make_inventory_item_object(
        weapon_guid,
        100,
        player_guid,
        1,
        40,
        ItemContext::None,
        EQUIPMENT_SLOT_OFFHAND,
    );
    let bag = session.make_inventory_item_object(
        bag_guid,
        200,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_BAG_START,
    );
    let mut armor = session.make_inventory_item_object(
        armor_guid,
        101,
        player_guid,
        1,
        10,
        ItemContext::None,
        0,
    );
    armor.set_container_guid_and_slot(bag_guid, 0);
    session.insert_inventory_item_object(weapon);
    session.insert_inventory_item_object(bag);
    session.insert_inventory_item_object(armor);

    session.set_represented_guild_repair_bank_state_like_cpp(Some(
        RepresentedGuildRepairBankStateLikeCpp {
            available_repair_money: 290,
            withdraw_repair_money_allowed: true,
        },
    ));
    assert!(
        session
            .repair_all_inventory_item_durability_with_guild_bank_like_cpp(0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 500);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&armor_guid]
            .data()
            .durability,
        13
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        50
    );
    assert_eq!(
        session.represented_guild_repair_bank_withdraws_like_cpp(),
        &[RepresentedGuildRepairBankWithdrawLikeCpp {
            amount: 290,
            repair: true,
            success: true,
        }]
    );

    session
        .inventory_item_objects
        .get_mut(&weapon_guid)
        .unwrap()
        .set_durability(0);
    session
        .inventory_item_objects
        .get_mut(&armor_guid)
        .unwrap()
        .set_durability(10);
    session.set_represented_guild_repair_bank_state_like_cpp(Some(
        RepresentedGuildRepairBankStateLikeCpp {
            available_repair_money: 2_000,
            withdraw_repair_money_allowed: false,
        },
    ));
    assert!(
        session
            .repair_all_inventory_item_durability_with_guild_bank_like_cpp(0.8, 2.0)
            .await
    );
    assert_eq!(session.player_gold_like_cpp(), 500);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&armor_guid]
            .data()
            .durability,
        13
    );
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        50
    );
    assert_eq!(
        session
            .represented_guild_repair_bank_withdraws_like_cpp()
            .last(),
        Some(&RepresentedGuildRepairBankWithdrawLikeCpp {
            amount: 1330,
            repair: true,
            success: false,
        })
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[RepresentedItemModsReapplyEventLikeCpp {
            item_guid: weapon_guid,
            slot: EQUIPMENT_SLOT_OFFHAND,
            apply: true,
        }],
        "C++ guild-bank repair also calls DurabilityRepair for each selected item before the final guild withdrawal"
    );
    assert_eq!(
        session.represented_item_bonus_actions_like_cpp().len(),
        8,
        "represented guild-bank repair records the same static _ApplyItemBonuses action plan for the broken equipped item"
    );
    assert!(
        session
            .represented_item_bonus_actions_like_cpp()
            .iter()
            .any(|action| matches!(
                action.action,
                ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 42 }
            )),
        "C++ _ApplyItemBonuses sets ActivePlayerData::ShieldBlock for repaired armor shields"
    );
}

/// Equip one 50-max-durability weapon in the mainhand for durability scenarios.
fn equip_durability_test_weapon_like_cpp(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    weapon_guid: ObjectGuid,
    durability: u32,
) {
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: 300,
        class_id: ItemClass::Weapon as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::Weapon as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            [(
                300,
                ItemStatEntry {
                    stats: [
                        (ItemModType::Strength as i8, 12),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                        (-1, 0),
                    ],
                    resistances: [0; 7],
                    armor: 0,
                },
            )],
            [(
                300,
                ItemSparseTemplateEntry {
                    flags: [0; 4],
                    bag_family: 0,
                    start_quest_id: 0,
                    stackable: 1,
                    max_count: 0,
                    lock_id: 0,
                    required_reputation_rank: 0,
                    sell_price: 0,
                    buy_price: 0,
                    vendor_stack_count: 1,
                    price_variance: 1.0,
                    price_random_value: 0.0,
                    max_durability: 50,
                    other_faction_item_id: 0,
                    content_tuning_id: 0,
                    player_level_to_item_level_curve_id: 0,
                    limit_category: 0,
                    instance_bound: 0,
                    zone_bound: [0; 2],
                    required_reputation_faction: 0,
                    allowable_class: 0,
                    required_expansion: 0,
                    bonding: ItemBondingType::None as u8,
                    container_slots: 0,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
            [(
                300,
                ItemRandomPropertyTemplateEntry {
                    item_level: 57,
                    quality: ItemQuality::Rare as i8,
                    inventory_type: InventoryType::Weapon as i8,
                },
            )],
        ),
    ));
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryItem {
                guid: weapon_guid,
                entry_id: 300,
                db_guid: weapon_guid.counter() as u64,
                inventory_type: Some(InventoryType::Weapon as u8),
            },
        );
    let weapon = session.make_inventory_item_object(
        weapon_guid,
        300,
        player_guid,
        1,
        durability,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(weapon);
}

fn durability_spell_store_like_cpp(
    spell_id: i32,
    effect: u32,
    damage: i32,
    slot: i32,
) -> wow_data::SpellStore {
    let mut store = wow_data::SpellStore::new();
    store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: effect,
            effect_base_points: damage,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect,
                effect_base_points: damage,
                effect_misc_value_1: slot,
                ..Default::default()
            }],
        },
    );
    store
}

#[test]
fn durability_points_loss_breaks_and_removes_equipped_item_mods_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 43);
    let weapon_guid = ObjectGuid::create_item(1, 903);
    session.set_player_guid(Some(player_guid));
    // 5 of 50 durability: the minimum one-point loss breaks the item.
    equip_durability_test_weapon_like_cpp(&mut session, player_guid, weapon_guid, 5);

    let affected = session.apply_represented_durability_loss_all_like_cpp(0.1, false);

    assert_eq!(affected, 1);
    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        0
    );
    assert_eq!(
        session.represented_item_mod_reapply_events_like_cpp(),
        &[RepresentedItemModsReapplyEventLikeCpp {
            item_guid: weapon_guid,
            slot: EQUIPMENT_SLOT_MAINHAND,
            apply: false,
        }],
        "C++ `DurabilityPointsLoss` removes equipped item mods before the durability write"
    );
}

#[tokio::test]
async fn durability_damage_spell_effect_reduces_equipped_items_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 90_200_i32;
    let player_guid = ObjectGuid::create_player(1, 44);
    let weapon_guid = ObjectGuid::create_item(1, 904);
    session.set_player_guid(Some(player_guid));
    equip_durability_test_weapon_like_cpp(&mut session, player_guid, weapon_guid, 50);
    session.set_spell_store(Arc::new(durability_spell_store_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE,
        7,
        -1,
    )));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented durability-damage spell should execute");

    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        43,
        "C++ EffectDurabilityDamage slot < 0 calls DurabilityPointsLossAll"
    );
    assert_eq!(
        durability_execute_log_row_like_cpp(&send_rx, player_guid, spell_id),
        (-1, -1),
        "C++ logs ItemID -1 and Amount -1 for the all-items branch"
    );
}

/// C++ `Spell::EffectDurabilityDamage` (`SpellEffects.cpp:4336-4340`) logs
/// `ExecuteLogEffectDurabilityDamage(effect, unitTarget, item->GetEntry(), slot)`.
#[tokio::test]
async fn durability_damage_spell_effect_logs_the_item_entry_and_slot_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 90_202_i32;
    let player_guid = ObjectGuid::create_player(1, 46);
    let weapon_guid = ObjectGuid::create_item(1, 906);
    session.set_player_guid(Some(player_guid));
    equip_durability_test_weapon_like_cpp(&mut session, player_guid, weapon_guid, 50);
    let slot = i32::from(EQUIPMENT_SLOT_MAINHAND);
    session.set_spell_store(Arc::new(durability_spell_store_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE,
        7,
        slot,
    )));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented durability-damage spell should execute");

    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        43
    );
    assert_eq!(
        durability_execute_log_row_like_cpp(&send_rx, player_guid, spell_id),
        (300, slot),
        "C++ logs the equipped item entry as ItemID and the slot as Amount"
    );
}

/// Decode the single `DurabilityDamageTargets` row of a cast's execute log.
fn durability_execute_log_row_like_cpp(
    send_rx: &flume::Receiver<Vec<u8>>,
    player_guid: ObjectGuid,
    spell_id: i32,
) -> (i32, i32) {
    let packets = crate::session::tests::drain_server_packet_bytes(send_rx);
    assert_eq!(
        packets
            .iter()
            .map(|bytes| {
                wow_packet::WorldPacket::from_bytes(bytes)
                    .server_opcode()
                    .expect("server opcode")
            })
            .collect::<Vec<_>>(),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::SpellExecuteLog,
            ServerOpcodes::CooldownEvent
        ]
    );
    let log_bytes = packets
        .iter()
        .find(|bytes| {
            wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
                == Some(ServerOpcodes::SpellExecuteLog)
        })
        .expect("execute log packet");
    let mut log = wow_packet::WorldPacket::from_bytes(log_bytes);
    log.read_uint16().expect("opcode");
    assert_eq!(log.read_packed_guid().expect("caster"), player_guid);
    assert_eq!(log.read_int32().expect("spell id"), spell_id);
    assert_eq!(log.read_uint32().expect("effect count"), 1);
    assert_eq!(
        log.read_int32().expect("effect"),
        i32::try_from(wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE).unwrap()
    );
    assert_eq!(log.read_uint32().expect("power drain count"), 0);
    assert_eq!(log.read_uint32().expect("extra attacks count"), 0);
    assert_eq!(log.read_uint32().expect("durability count"), 1);
    for _ in 0..3 {
        assert_eq!(log.read_uint32().expect("empty list count"), 0);
    }
    let victim = log.read_packed_guid().expect("victim");
    assert_eq!(victim, player_guid);
    (
        log.read_int32().expect("item id"),
        log.read_int32().expect("amount"),
    )
}

#[tokio::test]
async fn durability_damage_pct_spell_effect_reduces_the_targeted_slot_like_cpp() {
    let (mut session, _, _) = make_session();
    let spell_id = 90_201_i32;
    let player_guid = ObjectGuid::create_player(1, 45);
    let weapon_guid = ObjectGuid::create_item(1, 905);
    session.set_player_guid(Some(player_guid));
    equip_durability_test_weapon_like_cpp(&mut session, player_guid, weapon_guid, 50);
    session.set_spell_store(Arc::new(durability_spell_store_like_cpp(
        spell_id,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE_PCT,
        20,
        i32::from(EQUIPMENT_SLOT_MAINHAND),
    )));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("represented durability-damage-pct spell should execute");

    assert_eq!(
        session.inventory_item_objects_like_cpp()[&weapon_guid]
            .data()
            .durability,
        40,
        "C++ EffectDurabilityDamagePCT slot >= 0 calls DurabilityLoss on that slot"
    );
}
