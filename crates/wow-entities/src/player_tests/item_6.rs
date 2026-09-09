//! Item scenarios for [`super`].
//!
//! Split out of player_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn merge_bag_item_stack_object_rejects_empty_or_mismatched_slot() {
    let owner = ObjectGuid::create_player(1, 42);
    let bag_guid = ObjectGuid::create_item(1, 850);
    let expected = ObjectGuid::create_item(1, 851);
    let actual = ObjectGuid::create_item(1, 852);
    let mut player = Player::new(None, false);
    let mut bag = Bag::default();
    let mut existing = Item::default();
    let mut incoming = Item::default();

    bag.try_initialize_created_state(crate::BagCreateInfo {
        guid: bag_guid,
        item_id: 100,
        context: ItemContext::None,
        owner: Some(owner),
        max_durability: 0,
        container_slots: 4,
    })
    .unwrap();
    existing.object_mut().create(actual);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 4)
        .unwrap();

    assert_eq!(
        player.merge_bag_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &bag,
            2,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::EmptyBagItemSlot {
            bag: INVENTORY_SLOT_BAG_START,
            slot: 2,
        })
    );

    player
        .store_bag_item(INVENTORY_SLOT_BAG_START, 2, expected)
        .unwrap();
    assert_eq!(
        player.merge_bag_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &bag,
            2,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::MismatchedBagItemGuid {
            bag: INVENTORY_SLOT_BAG_START,
            slot: 2,
            expected,
            actual: ObjectGuid::EMPTY,
        })
    );

    bag.store_item(2, &mut existing);
    assert_eq!(
        player.merge_bag_item_stack_object(
            INVENTORY_SLOT_BAG_START,
            &bag,
            2,
            &mut existing,
            &mut incoming,
            1,
        ),
        Err(PlayerStorageError::MismatchedBagItemGuid {
            bag: INVENTORY_SLOT_BAG_START,
            slot: 2,
            expected,
            actual,
        })
    );
}
#[test]
fn player_get_item_by_guid_scans_everywhere_except_buyback_like_cpp_for_each_item() {
    let mut player = Player::new(None, false);
    player.set_inventory_slot_count(INVENTORY_DEFAULT_SIZE);

    let inventory_item = ObjectGuid::create_item(1, 10);
    let bank_item = ObjectGuid::create_item(1, 11);
    let reagent_bag = ObjectGuid::create_item(1, 12);
    let reagent_item = ObjectGuid::create_item(1, 13);
    let buyback = ObjectGuid::create_item(1, 14);

    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, inventory_item)
        .unwrap();
    player
        .store_top_level_item(BANK_SLOT_ITEM_START, bank_item)
        .unwrap();
    player
        .store_top_level_item(REAGENT_BAG_SLOT_START, reagent_bag)
        .unwrap();
    player
        .register_bag_storage(REAGENT_BAG_SLOT_START, reagent_bag, 3)
        .unwrap();
    player
        .store_bag_item(REAGENT_BAG_SLOT_START, 1, reagent_item)
        .unwrap();
    player
        .store_top_level_item(BUYBACK_SLOT_START, buyback)
        .unwrap();

    assert_eq!(
        player.get_item_by_guid(inventory_item),
        Some(inventory_item)
    );
    assert_eq!(player.get_item_by_guid(bank_item), Some(bank_item));
    assert_eq!(player.get_item_by_guid(reagent_item), Some(reagent_item));
    assert_eq!(player.get_item_by_guid(buyback), None);

    let mut visited = Vec::new();
    let completed = player.for_each_item_guid(ItemSearchLocation::INVENTORY, |guid| {
        visited.push(guid);
        ItemSearchCallbackResult::Continue
    });
    assert!(completed);
    assert!(visited.contains(&inventory_item));
    assert!(!visited.contains(&bank_item));
}
#[test]
fn add_item_to_buyback_slot_object_matches_cpp_price_time_and_replacement() {
    let mut player = Player::new(None, false);
    let mut overwritten = item_with_guid_entry(1100, 7000);
    overwritten.set_count(3);
    overwritten.force_state(ItemUpdateState::Unchanged);
    let old_proto = ItemStorageTemplate {
        sell_price: 11,
        ..ItemStorageTemplate::regular_item(7000, 20)
    };

    let old_slot = player
        .add_item_to_buyback_slot_object(&overwritten, Some(&old_proto), 2000, 1000, None)
        .unwrap();
    assert_eq!(old_slot, BUYBACK_SLOT_START);
    assert_eq!(
        player.get_item_from_buyback_slot(old_slot),
        Some(overwritten.object().guid())
    );
    assert_eq!(player.active_data().buyback_price[0], 33);
    assert_eq!(player.active_data().buyback_timestamp[0], 109000);

    player.set_buyback_timestamp(0, 50);
    for slot in BUYBACK_SLOT_START + 1..BUYBACK_SLOT_END {
        let guid = ObjectGuid::create_item(1, 2000 + slot as i64);
        player.add_item_to_buyback_slot(guid, 1, 100 + slot as i64);
    }

    overwritten.object_mut().add_to_world();
    let mut replacement = item_with_guid_entry(1101, 7001);
    replacement.set_count(4);
    let replacement_proto = ItemStorageTemplate {
        sell_price: 9,
        ..ItemStorageTemplate::regular_item(7001, 20)
    };

    let replaced_slot = player
        .add_item_to_buyback_slot_object(
            &replacement,
            Some(&replacement_proto),
            5000,
            3000,
            Some(&mut overwritten),
        )
        .unwrap();

    assert_eq!(replaced_slot, old_slot);
    assert!(!overwritten.object().is_in_world());
    assert_eq!(overwritten.update_state(), ItemUpdateState::Removed);
    assert_eq!(
        player.get_item_from_buyback_slot(replaced_slot),
        Some(replacement.object().guid())
    );
    assert_eq!(player.active_data().buyback_price[0], 36);
    assert_eq!(player.active_data().buyback_timestamp[0], 110000);
    assert_eq!(
        player.inventory().current_buyback_slot,
        BUYBACK_SLOT_START + 1
    );
}
#[test]
fn remove_item_from_buyback_slot_object_matches_cpp_item_side_effects() {
    let mut player = Player::new(None, false);
    let mut item = item_with_guid_entry(1010, 6948);
    item.force_state(ItemUpdateState::Unchanged);
    item.object_mut().add_to_world();

    let slot = player.add_item_to_buyback_slot(item.object().guid(), 123, 456);
    assert_eq!(
        player
            .remove_item_from_buyback_slot_object(slot, Some(&mut item), true)
            .unwrap(),
        Some(item.object().guid())
    );

    assert!(!item.object().is_in_world());
    assert_eq!(item.update_state(), ItemUpdateState::Removed);
    assert_eq!(player.get_item_from_buyback_slot(slot), None);
    assert_eq!(
        player.active_data().inv_slots[slot as usize],
        ObjectGuid::EMPTY
    );
    assert_eq!(player.active_data().buyback_price[0], 0);
    assert_eq!(player.active_data().buyback_timestamp[0], 0);

    let mut keep_state_item = item_with_guid_entry(1011, 6949);
    keep_state_item.force_state(ItemUpdateState::Unchanged);
    keep_state_item.object_mut().add_to_world();
    let keep_slot = player.add_item_to_buyback_slot(keep_state_item.object().guid(), 200, 500);

    player
        .remove_item_from_buyback_slot_object(keep_slot, Some(&mut keep_state_item), false)
        .unwrap();
    assert!(!keep_state_item.object().is_in_world());
    assert_eq!(keep_state_item.update_state(), ItemUpdateState::Unchanged);
}
#[test]
fn remove_item_from_buyback_slot_object_rejects_mismatched_item_ref() {
    let mut player = Player::new(None, false);
    let expected = ObjectGuid::create_item(1, 1020);
    let mut actual = item_with_guid_entry(1021, 6948);

    let slot = player.add_item_to_buyback_slot(expected, 123, 456);
    assert_eq!(
        player.remove_item_from_buyback_slot_object(slot, Some(&mut actual), true),
        Err(PlayerStorageError::MismatchedItemGuid {
            slot,
            expected,
            actual: actual.object().guid(),
        })
    );
    assert_eq!(player.get_item_from_buyback_slot(slot), Some(expected));
    assert_eq!(player.active_data().buyback_price[0], 123);
    assert_eq!(player.active_data().buyback_timestamp[0], 456);
}
#[test]
fn soulbound_tradeable_item_set_matches_cpp_add_remove_and_update() {
    let mut player = Player::new(None, false);
    let mut keep = item_with_guid_entry(1200, 7000);
    keep.set_owner_guid(player.guid());
    let mut expired = item_with_guid_entry(1201, 7001);
    expired.set_owner_guid(player.guid());
    expired.set_create_played_time(10);
    let mut wrong_owner = item_with_guid_entry(1202, 7002);
    wrong_owner.set_owner_guid(ObjectGuid::create_player(1, 99));
    let missing = item_with_guid_entry(1203, 7003);
    let removed_directly = item_with_guid_entry(1204, 7004);

    player.add_tradeable_item(&keep);
    player.add_tradeable_item(&expired);
    player.add_tradeable_item(&wrong_owner);
    player.add_tradeable_item(&missing);
    player.add_tradeable_item(&removed_directly);
    player.remove_tradeable_item(&removed_directly);

    assert!(
        player
            .soulbound_tradeable_items()
            .contains(&keep.object().guid())
    );
    assert!(
        !player
            .soulbound_tradeable_items()
            .contains(&removed_directly.object().guid())
    );

    let removed = player.update_soulbound_trade_items(&[
        SoulboundTradeableItemRef::from_item(&keep, 7_200),
        SoulboundTradeableItemRef::from_item(&expired, 7_211),
        SoulboundTradeableItemRef::new(
            wrong_owner.object().guid(),
            wrong_owner.owner_guid(),
            false,
        ),
    ]);

    assert!(
        player
            .soulbound_tradeable_items()
            .contains(&keep.object().guid())
    );
    assert_eq!(player.soulbound_tradeable_items().len(), 1);
    assert!(removed.contains(&expired.object().guid()));
    assert!(removed.contains(&wrong_owner.object().guid()));
    assert!(removed.contains(&missing.object().guid()));
    assert!(!removed.contains(&removed_directly.object().guid()));
}
#[test]
fn item_duration_list_matches_cpp_add_remove_and_update_plan() {
    let mut player = Player::new(None, false);
    let mut item = item_with_guid_entry(1210, 7100);
    assert_eq!(player.add_item_durations(&item), None);
    assert!(player.item_durations().is_empty());

    item.set_expiration(900);
    assert_eq!(
        player.add_item_durations(&item),
        Some(PlayerItemTimeUpdate {
            item_guid: item.object().guid(),
            expiration: 900,
        })
    );
    player.add_item_durations(&item);
    assert_eq!(
        player.item_durations(),
        &[item.object().guid(), item.object().guid()]
    );

    assert!(player.remove_item_durations(&item));
    assert_eq!(player.item_durations(), &[item.object().guid()]);

    assert!(
        player
            .update_item_duration_plan(
                &[ItemDurationRef::new(item.object().guid(), 900, false)],
                300,
                true,
            )
            .is_empty()
    );
    assert_eq!(
        player.update_item_duration_plan(
            &[ItemDurationRef::new(item.object().guid(), 900, false)],
            300,
            false,
        ),
        vec![UpdateItemDurationAction::UpdateExpiration {
            item_guid: item.object().guid(),
            expiration: 600,
        }]
    );
    assert_eq!(
        player.update_item_duration_plan(
            &[ItemDurationRef::new(item.object().guid(), 900, true)],
            900,
            true,
        ),
        vec![UpdateItemDurationAction::Expire {
            item_guid: item.object().guid(),
        }]
    );
    assert_eq!(
        player.update_item_duration_plan(&[], 1, false),
        vec![UpdateItemDurationAction::MissingItem {
            item_guid: item.object().guid(),
        }]
    );
}
#[test]
fn item_stat_bonus_actions_match_cpp_apply_item_bonuses_stat_loop() {
    let stats = [
        (ItemModType::Strength as i8, 12),
        (ItemModType::HitRating as i8, 5),
        (-1, 99),
        (ItemModType::SpellPower as i8, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
    ];

    assert_eq!(
        item_stat_bonus_actions_like_cpp(&stats, true),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 12,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitMelee,
                amount: 5,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitRanged,
                amount: 5,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitSpell,
                amount: 5,
                apply: true,
            },
        ],
    );
}
#[test]
fn item_scaling_stat_bonus_actions_match_cpp_scaled_stat_loop() {
    let mut stat_ids = [-1; 10];
    stat_ids[0] = ItemModType::Strength as i32;
    stat_ids[1] = ItemModType::HitRating as i32;
    stat_ids[2] = ItemModType::SpellPower as i32;
    let mut bonuses = [0; 10];
    bonuses[0] = 5_000;
    bonuses[1] = 2_500;
    bonuses[2] = 0;

    assert_eq!(
        item_scaling_stat_bonus_actions_like_cpp(&stat_ids, &bonuses, 200, true),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 100,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitMelee,
                amount: 50,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitRanged,
                amount: 50,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HitSpell,
                amount: 50,
                apply: true,
            },
        ],
        "C++ computes val = getssdMultiplier(mask) * ScalingStatDistribution::Bonus[i] / 10000"
    );
}
#[test]
fn item_stat_bonus_actions_cover_cpp_item_bonus_only_stat_cases() {
    let stats = [
        (ItemModType::AgiStrInt as i8, 7),
        (ItemModType::ExtraArmor as i8, 40),
        (ItemModType::FireResistance as i8, 9),
        (ItemModType::HasteMeleeRating as i8, 3),
        (ItemModType::HasteRangedRating as i8, 4),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
        (-1, 0),
    ];

    assert_eq!(
        item_stat_bonus_actions_like_cpp(&stats, true),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatAgility,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 7,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Agility),
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatStrength,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 7,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Strength),
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::StatIntellect,
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 7,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UpdateStatBuffMod(Stats::Intellect),
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Armor,
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount: 40,
                apply: true,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(SpellSchools::Fire as u32),
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 9,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteMelee,
                amount: 3,
                apply: true,
            },
            ApplyEnchantmentEffectAction::RatingModifier {
                rating: ApplyEnchantmentCombatRating::HasteRanged,
                amount: 4,
                apply: true,
            },
        ],
    );
}
#[test]
fn item_resistance_bonus_actions_match_cpp_template_resistance_loop() {
    let mut resistances = [0i16; 7];
    resistances[SpellSchools::Normal as usize] = 120;
    resistances[SpellSchools::Holy as usize] = 3;
    resistances[SpellSchools::Fire as usize] = 7;

    assert_eq!(
        item_resistance_bonus_actions_like_cpp(&resistances, false),
        vec![
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(SpellSchools::Normal as u32),
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 120,
                apply: false,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(SpellSchools::Holy as u32),
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 3,
                apply: false,
            },
            ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(SpellSchools::Fire as u32),
                modifier: ApplyEnchantmentUnitModifier::BaseValue,
                amount: 7,
                apply: false,
            },
        ],
    );
}
#[test]
fn item_shield_block_bonus_action_matches_cpp_direct_update_field_assignment() {
    assert_eq!(
        item_shield_block_bonus_action_like_cpp(42, true, true),
        Some(ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 42 }),
    );
    assert_eq!(
        item_shield_block_bonus_action_like_cpp(42, true, false),
        Some(ApplyEnchantmentEffectAction::SetShieldBlockValue { amount: 0 }),
    );
    assert_eq!(
        item_shield_block_bonus_action_like_cpp(42, false, true),
        None
    );
    assert_eq!(item_shield_block_bonus_action_like_cpp(0, true, true), None);
}
#[test]
fn item_weapon_damage_actions_match_cpp_direct_apply_weapon_damage() {
    assert_eq!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryType::Weapon,
            12.0,
            18.0,
            2600,
            true,
            false,
            true,
            false,
            true,
        ),
        vec![
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::BaseAttack,
                bound: WeaponDamageBoundLikeCpp::Min,
                amount_bits: 12.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::BaseAttack,
                bound: WeaponDamageBoundLikeCpp::Max,
                amount_bits: 18.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseAttackTime {
                attack_type: WeaponAttackType::BaseAttack,
                time_ms: 2600,
            },
            ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                attack_type: WeaponAttackType::BaseAttack,
            },
        ],
    );

    assert_eq!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryType::Weapon,
            12.0,
            18.0,
            2600,
            false,
            false,
            true,
            false,
            true,
        ),
        vec![
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::BaseAttack,
                bound: WeaponDamageBoundLikeCpp::Min,
                amount_bits: BASE_MINDAMAGE.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::BaseAttack,
                bound: WeaponDamageBoundLikeCpp::Max,
                amount_bits: BASE_MAXDAMAGE.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseAttackTime {
                attack_type: WeaponAttackType::BaseAttack,
                time_ms: 2000,
            },
            ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                attack_type: WeaponAttackType::BaseAttack,
            },
        ],
    );
}
#[test]
fn item_weapon_damage_actions_match_cpp_attack_slot_and_gate_rules() {
    assert_eq!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryType::RangedRight,
            20.0,
            30.0,
            1800,
            true,
            false,
            true,
            true,
            true,
        ),
        vec![
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::RangedAttack,
                bound: WeaponDamageBoundLikeCpp::Min,
                amount_bits: 20.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::RangedAttack,
                bound: WeaponDamageBoundLikeCpp::Max,
                amount_bits: 30.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::UpdateDamagePhysical {
                attack_type: WeaponAttackType::RangedAttack,
            },
        ],
        "C++ skips SetBaseAttackTime when the shapeshift form has CombatRoundTime"
    );
    assert_eq!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_OFFHAND,
            InventoryType::WeaponOffhand,
            9.0,
            11.0,
            0,
            true,
            false,
            true,
            false,
            false,
        ),
        vec![
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::OffAttack,
                bound: WeaponDamageBoundLikeCpp::Min,
                amount_bits: 9.0f32.to_bits(),
            },
            ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
                attack_type: WeaponAttackType::OffAttack,
                bound: WeaponDamageBoundLikeCpp::Max,
                amount_bits: 11.0f32.to_bits(),
            },
        ],
        "C++ skips UpdateDamagePhysical when CanModifyStats is false"
    );
    assert!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_MAINHAND,
            InventoryType::Weapon,
            12.0,
            18.0,
            2600,
            true,
            false,
            false,
            false,
            true,
        )
        .is_empty()
    );
    assert!(
        item_weapon_damage_actions_like_cpp(
            EQUIPMENT_SLOT_HEAD,
            InventoryType::Weapon,
            12.0,
            18.0,
            2600,
            true,
            false,
            true,
            false,
            true,
        )
        .is_empty()
    );
}
#[test]
fn send_new_item_plan_matches_cpp_packet_fields_and_delivery() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);

    let mut item = item_with_guid_entry(12510, 9001);
    item.set_count(3);
    item.set_slot(7);
    item.set_container_guid_and_slot(ObjectGuid::create_item(1, 700), 4);
    item.set_property_seed(4567);
    item.set_random_properties_id(-89);
    item.set_modifier(ItemModifier::BattlePetSpeciesId, 123);
    item.set_modifier(ItemModifier::BattlePetBreedData, 0x1A00_00BC);
    item.set_modifier(ItemModifier::BattlePetLevel, 25);

    let mut args = SendNewItemArgs::new(3, true, false);
    args.quantity_in_inventory = 9;
    assert_eq!(
        player.send_new_item_plan(Some(&item), SendNewItemTemplateRef::new(777, false), args,),
        Some(SendNewItemPlan {
            player_guid,
            item_guid: item.object().guid(),
            item_entry: 9001,
            item_instance: SendNewItemInstancePlan {
                item_id: 9001,
                random_properties_seed: 4567,
                random_properties_id: -89,
                modifications: vec![
                    SendNewItemModifier {
                        value: 123,
                        modifier_type: ItemModifier::BattlePetSpeciesId as u8,
                    },
                    SendNewItemModifier {
                        value: 0x1A00_00BC,
                        modifier_type: ItemModifier::BattlePetBreedData as u8,
                    },
                    SendNewItemModifier {
                        value: 25,
                        modifier_type: ItemModifier::BattlePetLevel as u8,
                    },
                ],
            },
            slot: 4,
            slot_in_bag: 7,
            quest_log_item_id: 777,
            quantity: 3,
            quantity_in_inventory: 9,
            battle_pet_species_id: 123,
            battle_pet_breed_id: 0xBC,
            battle_pet_breed_quality: 0x1A,
            battle_pet_level: 25,
            pushed: true,
            created: false,
            display_text: SendNewItemDisplayText::Normal,
            dungeon_encounter_id: 0,
            is_encounter_loot: false,
            delivery: SendNewItemDelivery::Direct,
        })
    );

    let mut encounter_args = SendNewItemArgs::new(1, false, true);
    encounter_args.broadcast = true;
    encounter_args.player_in_group = true;
    encounter_args.dungeon_encounter_id = 615;
    encounter_args.quantity_in_inventory = 10;
    assert_eq!(
        player
            .send_new_item_plan(
                Some(&item),
                SendNewItemTemplateRef::new(0, false),
                encounter_args,
            )
            .unwrap()
            .delivery,
        SendNewItemDelivery::GroupBroadcast
    );
    let encounter = player
        .send_new_item_plan(
            Some(&item),
            SendNewItemTemplateRef::new(0, true),
            encounter_args,
        )
        .unwrap();
    assert_eq!(encounter.slot_in_bag, -1);
    assert_eq!(
        encounter.display_text,
        SendNewItemDisplayText::EncounterLoot
    );
    assert!(encounter.is_encounter_loot);
    assert_eq!(encounter.dungeon_encounter_id, 615);
    assert_eq!(encounter.delivery, SendNewItemDelivery::Direct);

    assert_eq!(
        player.send_new_item_plan(None, SendNewItemTemplateRef::new(0, false), args),
        None
    );
}
