use super::*;

#[test]
fn represented_item_mods_records_weapon_damage_without_stat_entry_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 900);
    session.set_player_guid(Some(player_guid));
    session.set_item_store(Arc::new(ItemStore::from_records([represented_test_item_record_like_cpp(
        100,
        InventoryType::Weapon,
        ItemClass::Weapon,
        7,
    )])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_weapon_templates([(
        100,
        ItemWeaponTemplateEntry {
            dmg_variance: 1.0,
            item_delay: 2600,
            min_damage: [12, 0, 0, 0, 0],
            max_damage: [18, 0, 0, 0, 0],
            damage_damage_type: 0,
        },
    )])));
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryItem {
                guid: item_guid,
                entry_id: 100,
                db_guid: item_guid.counter() as u64,
                inventory_type: Some(InventoryType::Weapon as u8),
            },
        );
    let item = session.make_inventory_item_object(
        item_guid,
        100,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(item);

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_MAINHAND, true);

    assert_eq!(
        session.represented_item_bonus_actions_like_cpp(),
        &[
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    bound: wow_entities::WeaponDamageBoundLikeCpp::Min,
                    amount_bits: 12.0f32.to_bits(),
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    bound: wow_entities::WeaponDamageBoundLikeCpp::Max,
                    amount_bits: 18.0f32.to_bits(),
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseAttackTime {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    time_ms: 2600,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                },
            },
        ],
        "C++ Player::_ApplyItemBonuses reaches _ApplyWeaponDamage even when ItemSparse has no stat modifiers"
    );
}
#[test]
fn represented_item_mods_apply_scaling_weapon_dps_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 901);
    session.set_player_guid(Some(player_guid));
    session.set_player_level_like_cpp(80);
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        scaling_stat_distribution_id: 77,
        scaling_stat_value: 0x0000_0200,
        ..represented_test_item_record_like_cpp(
            101,
            InventoryType::Weapon,
            ItemClass::Weapon,
            7,
        )
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_weapon_templates([(
        101,
        ItemWeaponTemplateEntry {
            dmg_variance: 1.0,
            item_delay: 2000,
            min_damage: [1, 0, 0, 0, 0],
            max_damage: [2, 0, 0, 0, 0],
            damage_damage_type: 0,
        },
    )])));
    session.set_scaling_stat_distribution_store(Arc::new(
        ScalingStatDistributionStore::from_entries([ScalingStatDistributionEntry {
            id: 77,
            player_level_to_item_level_curve_id: 0,
            min_level: 10,
            max_level: 20,
            bonus: [0; 10],
            stat_id: [0; 10],
        }]),
    ));
    session.set_scaling_stat_values_store(Arc::new(ScalingStatValuesStore::from_entries([
        ScalingStatValuesEntry {
            id: 20,
            char_level: 20,
            weapon_dps_1h: 100,
            weapon_dps_2h: 0,
            spellcaster_dps_1h: 0,
            spellcaster_dps_2h: 0,
            ranged_dps: 0,
            wand_dps: 0,
            spell_power: 0,
            shoulder_budget: 0,
            trinket_budget: 0,
            weapon_budget_1h: 0,
            primary_budget: 0,
            ranged_budget: 0,
            tertiary_budget: 0,
            cloth_shoulder_armor: 0,
            leather_shoulder_armor: 0,
            mail_shoulder_armor: 0,
            plate_shoulder_armor: 0,
            cloth_cloak_armor: 0,
            cloth_chest_armor: 0,
            leather_chest_armor: 0,
            mail_chest_armor: 0,
            plate_chest_armor: 0,
        },
    ])));
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryItem {
                guid: item_guid,
                entry_id: 101,
                db_guid: item_guid.counter() as u64,
                inventory_type: Some(InventoryType::Weapon as u8),
            },
        );
    let item = session.make_inventory_item_object(
        item_guid,
        101,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(item);

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_MAINHAND, true);

    assert_eq!(
        session.represented_item_bonus_actions_like_cpp(),
        &[
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    bound: wow_entities::WeaponDamageBoundLikeCpp::Min,
                    amount_bits: 140.0f32.to_bits(),
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    bound: wow_entities::WeaponDamageBoundLikeCpp::Max,
                    amount_bits: 260.0f32.to_bits(),
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::SetBaseAttackTime {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                    time_ms: 2000,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_MAINHAND,
                action: ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                    attack_type: wow_constants::WeaponAttackType::BaseAttack,
                },
            },
        ],
        "C++ clamps player level to ScalingStatDistribution range and replaces weapon min/max from ScalingStatValues::getDPSMod"
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize],
        [140.0, 260.0],
        "represented runtime state now applies the planned SetBaseWeaponDamage actions"
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .base_attack_time[wow_constants::WeaponAttackType::BaseAttack as usize],
        2000
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .damage_physical_updates,
        &[wow_constants::WeaponAttackType::BaseAttack]
    );
    let stat_changes =
        represented_player_stat_changes_like_cpp(&session.represented_item_bonus_state_like_cpp());
    assert_eq!(stat_changes.min_damage, 140.0);
    assert_eq!(stat_changes.max_damage, 260.0);
    assert_eq!(
        stat_changes.min_ranged_damage, 0.0,
        "the represented packet projection does not invent ranged/offhand values when C++ only changed BASE_ATTACK"
    );
}
