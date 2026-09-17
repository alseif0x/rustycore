//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn equipment_stats_use_one_canonical_contribution_path_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_001);
    let item_guid = ObjectGuid::create_item(1, 61_001);
    let item_id = 61_001;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            item_id,
            ItemStatEntry {
                stats: std::array::from_fn(|index| {
                    if index == 0 {
                        (wow_constants::ItemModType::Strength as i8, 10)
                    } else {
                        (wow_constants::ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )));
    let item = session.make_inventory_item_object(
        item_guid,
        item_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::EQUIPMENT_SLOT_CHEST,
    );
    session.insert_inventory_item_object(item);
    session.insert_inventory_item_like_cpp(
        wow_entities::EQUIPMENT_SLOT_CHEST,
        InventoryItem {
            guid: item_guid,
            entry_id: item_id,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::Chest as u8),
        },
    );
    // A post-login equip applies `_ApplyItemBonuses` to the Player owner once.
    assert!(
        session
            .resolved_inventory_item_object_like_cpp(item_guid)
            .is_some()
    );
    let changed = session.apply_inventory_item_store_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_CHEST,
        item_guid,
    );
    assert!(changed);
    assert_eq!(
        (
            session.player_race_like_cpp(),
            session.player_class_like_cpp(),
            session.player_level_like_cpp()
        ),
        (1, 5, 80)
    );
    assert!(session.player_stats().is_some());
    let _ = session.send_stat_update();
    let equipped = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("equipped stat projection");
    assert_eq!(
        equipped.stats[wow_constants::Stats::Strength as usize],
        20,
        "C++ Player::_ApplyItemBonuses contributes the item exactly once"
    );

    // C++ `Player::UpdateExpertise` derives `MainhandExpertise` from the
    // combat-rating bonus; `RangedExpertise`/`CombatRatingExpertise` are never
    // written by C++ and keep their zero create value. The values must be
    // published on the same Player snapshot as the rest of the equipment
    // projection.
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::RatingModifier {
            rating: wow_entities::ApplyEnchantmentCombatRating::Expertise,
            amount: 46,
            apply: true,
        }
    ));
    let _ = session.send_stat_update();
    let equipped = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("equipped expertise projection");
    assert_eq!(equipped.mainhand_expertise, 46.0);
    assert_eq!(equipped.offhand_expertise, 46.0);
    assert_eq!(equipped.ranged_expertise, 0.0);
    assert_eq!(equipped.combat_rating_expertise, 0.0);

    session.apply_inventory_item_remove_side_effects_like_cpp(
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_CHEST,
        item_guid,
        &[],
    );
    let _ = session.send_stat_update();
    let unequipped = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("unequipped stat projection");
    assert_eq!(
        unequipped.stats[wow_constants::Stats::Strength as usize],
        10
    );
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::RatingModifier {
            rating: wow_entities::ApplyEnchantmentCombatRating::Expertise,
            amount: 46,
            apply: false,
        }
    ));

    // `_ApplyAllItemMods` must reject a broken item before it reaches the
    // canonical accumulator, exactly as the C++ `Item::IsBroken` gate does.
    assert!(
        session.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.set_max_durability(10);
            item.set_durability(0);
        })
    );
    let _ = session.apply_initial_loaded_item_mods_like_cpp(&[item_guid]);
    let _ = session.send_stat_update();
    let broken = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("broken-item stat projection");
    assert_eq!(
        broken.stats[wow_constants::Stats::Strength as usize],
        10,
        "a broken item contributes no static stats during login"
    );

    // A repaired item then follows the same login path and produces the same
    // projection as equipping it after login.
    assert!(
        session.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.set_durability(10);
        })
    );
    let _ = session.apply_initial_loaded_item_mods_like_cpp(&[item_guid]);
    let _ = session.send_stat_update();
    let loaded = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("loaded stat projection");
    assert_eq!(
        loaded.stats[wow_constants::Stats::Strength as usize],
        equipped.stats[wow_constants::Stats::Strength as usize]
    );

    // A broken equipped item removes its contribution; the durability repair
    // path reapplies it and publishes the same complete projection.
    session.record_represented_item_mods_like_cpp(
        item_guid,
        wow_entities::EQUIPMENT_SLOT_CHEST,
        false,
    );
    assert!(
        session.update_inventory_item_object_like_cpp(item_guid, |item| {
            item.set_durability(0);
        })
    );
    let _ = session.send_stat_update();
    assert_eq!(
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("broken stat projection")
            .stats[wow_constants::Stats::Strength as usize],
        10
    );
    assert!(
        session
            .repair_inventory_item_durability_like_cpp(item_guid, false, 1.0, 1.0)
            .await
    );
    assert_eq!(
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("repaired stat projection")
            .stats[wow_constants::Stats::Strength as usize],
        20
    );
}

#[test]
fn send_new_item_plan_direct_routes_item_push_result_to_realm_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(1);
    session.install_realm_send_channel_for_test(realm_tx);
    let plan = send_new_item_plan(SendNewItemDelivery::Direct);
    let expected = crate::session_rules::item_push_result_from_send_new_item_plan(&plan).to_bytes();

    session.send_new_item_plan(&plan);

    assert_eq!(realm_rx.try_recv().unwrap(), expected);
    assert!(send_rx.try_recv().is_err());
}
#[test]
fn send_item_time_update_plan_sends_cpp_packet() {
    let (session, _, send_rx) = make_session();
    let update = PlayerItemTimeUpdate {
        item_guid: ObjectGuid::new(0, 0x0102),
        expiration: 300,
    };
    let expected = ItemTimeUpdate {
        item_guid: update.item_guid,
        duration_left: update.expiration,
    }
    .to_bytes();

    session.send_item_time_update_plan(&update);

    assert_eq!(send_rx.try_recv().unwrap(), expected);
}
#[test]
fn loaded_inventory_registers_item_and_non_equipped_enchantment_durations_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 122_126);
    let equipped_guid = ObjectGuid::create_item(1, 122_127);
    let backpack_guid = ObjectGuid::create_item(1, 122_128);
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

    let mut equipped = session.make_inventory_item_object(
        equipped_guid,
        700,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    equipped.set_expiration(300);
    equipped.set_enchantment(EnchantmentSlot::EnhancementTemporary, 900, 12_000, 0);
    session.insert_inventory_item_object(equipped);

    let mut backpack = session.make_inventory_item_object(
        backpack_guid,
        701,
        player_guid,
        1,
        0,
        ItemContext::None,
        INVENTORY_SLOT_ITEM_START,
    );
    backpack.set_expiration(600);
    backpack.set_enchantment(EnchantmentSlot::EnhancementTemporary, 901, 9_000, 0);
    session.insert_inventory_item_object(backpack);

    let (item_updates, enchantment_updates) = session
        .register_loaded_inventory_item_duration_refs_like_cpp(
            &[equipped_guid, backpack_guid],
            &[equipped_guid],
        );

    assert_eq!(
        item_updates,
        vec![
            PlayerItemTimeUpdate {
                item_guid: equipped_guid,
                expiration: 300,
            },
            PlayerItemTimeUpdate {
                item_guid: backpack_guid,
                expiration: 600,
            },
        ]
    );
    assert_eq!(
        enchantment_updates,
        vec![PlayerEnchantTimeUpdate {
            item_guid: backpack_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            duration_secs: 9,
        }]
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.item_durations().to_vec()),
        Some(vec![equipped_guid, backpack_guid])
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player.enchant_durations().to_vec()),
        Some(vec![PlayerEnchantDuration {
            item_guid: backpack_guid,
            slot: EnchantmentSlot::EnhancementTemporary,
            left_duration_ms: 9_000,
        }])
    );
    assert!(
        send_rx.try_recv().is_err(),
        "login duration packets are delayed until after the CREATE_OBJECT sequence"
    );
}
#[test]
fn send_item_enchant_time_update_plan_sends_cpp_packet() {
    let (session, _, send_rx) = make_session();
    let owner_guid = ObjectGuid::new(0, 0x0102);
    let update = PlayerEnchantTimeUpdate {
        item_guid: ObjectGuid::new(0, 0x0506),
        slot: EnchantmentSlot::EnhancementSocket,
        duration_secs: 45,
    };
    let expected = ItemEnchantTimeUpdate {
        owner_guid,
        item_guid: update.item_guid,
        duration_left: update.duration_secs,
        slot: update.slot as u32,
    }
    .to_bytes();

    session.send_item_enchant_time_update_plan(owner_guid, &update);

    assert_eq!(send_rx.try_recv().unwrap(), expected);
}

#[tokio::test]
async fn expertise_aura_modifiers_filter_by_weapon_fit_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_100);
    let weapon_guid = ObjectGuid::create_item(1, 61_100);
    let weapon_id = 61_100u32;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: weapon_id,
        class_id: ItemClass::Weapon as u8,
        subclass_id: ItemSubClassWeapon::Axe as u8,
        material: 0,
        inventory_type: InventoryType::WeaponMainhand as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            weapon_id,
            ItemStatEntry {
                stats: std::array::from_fn(|_| (wow_constants::ItemModType::None as i8, 0)),
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )));
    let weapon = session.make_inventory_item_object(
        weapon_guid,
        weapon_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(weapon);
    session.insert_inventory_item_like_cpp(
        wow_entities::EQUIPMENT_SLOT_MAINHAND,
        InventoryItem {
            guid: weapon_guid,
            entry_id: weapon_id,
            db_guid: weapon_guid.counter() as u64,
            inventory_type: Some(InventoryType::WeaponMainhand as u8),
        },
    );

    // Four `SPELL_AURA_MOD_EXPERTISE` producers: item-neutral, axe-fit,
    // sword-only (must not match the equipped axe) and armor-only (must not
    // match a weapon).
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, amount) in [(90_300, 30), (90_301, 20), (90_302, 40), (90_303, 10)] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_equipped_items_store(Arc::new(SpellEquippedItemsStore::from_entries([
        SpellEquippedItemsEntry {
            id: 1,
            spell_id: 90_301,
            equipped_item_class: ItemClass::Weapon as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 1_i32 << (ItemSubClassWeapon::Axe as u32),
        },
        SpellEquippedItemsEntry {
            id: 2,
            spell_id: 90_302,
            equipped_item_class: ItemClass::Weapon as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 1_i32 << (ItemSubClassWeapon::Sword as u32),
        },
        SpellEquippedItemsEntry {
            id: 3,
            spell_id: 90_303,
            equipped_item_class: ItemClass::Armor as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 0,
        },
    ])));
    session.set_state(crate::session::SessionState::LoggedIn);
    for spell_id in [90_300, 90_301, 90_302, 90_303] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply expertise aura");
    }

    let _ = session.send_stat_update();
    let stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("aura expertise projection");
    assert_eq!(
        stats.mainhand_expertise, 50.0,
        "item-neutral (30) plus axe-fit (20) auras apply to the equipped mainhand axe"
    );
    assert_eq!(
        stats.offhand_expertise, 30.0,
        "without an offhand weapon only the item-neutral aura applies"
    );
    assert_eq!(stats.ranged_expertise, 0.0);
    assert_eq!(stats.combat_rating_expertise, 0.0);

    // Removing the aura removes its contribution on the next projection.
    let slot = session
        .visible_aura_slot_for_spell_like_cpp(90_301)
        .expect("axe-fit aura slot");
    session.remove_aura(slot).expect("remove axe-fit aura");
    let _ = session.send_stat_update();
    let stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("post-removal expertise projection");
    assert_eq!(stats.mainhand_expertise, 30.0);
    assert_eq!(stats.offhand_expertise, 30.0);
}

#[tokio::test]
async fn armor_aura_producers_follow_update_armor_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_200);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);

    // C++ 3.4.3 aura types consumed by `Player::UpdateArmor`
    // (`StatSystem.cpp:251-276`).
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, misc_value, misc_value_b, amount) in [
        (90_400, 22, 1, 0, 300),  // MOD_RESISTANCE, normal mask
        (90_401, 22, 4, 0, 500),  // MOD_RESISTANCE, fire mask only
        (90_402, 101, 1, 0, 50),  // MOD_RESISTANCE_PCT, normal mask
        (90_403, 466, 0, 0, 100), // MOD_BONUS_ARMOR_PCT
        (90_404, 182, 1, 1, 100), // MOD_RESISTANCE_OF_STAT_PERCENT, agility
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_base_points: amount,
                    effect_misc_value_1: misc_value,
                    effect_misc_value_2: misc_value_b,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let armor = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("armor projection")
            .armor
    };

    let _ = session.send_stat_update();
    // No items: C++ `Player::UpdateArmor` armor is Agility * 2.
    assert_eq!(armor(&session), 20);

    // Flat `SPELL_AURA_MOD_RESISTANCE` with the normal mask adds directly.
    session
        .apply_aura(90_400, player_guid, 30_000, 1)
        .expect("apply armor aura");
    let _ = session.send_stat_update();
    assert_eq!(armor(&session), 320);

    // A fire-mask flat aura does not change the physical armor.
    session
        .apply_aura(90_401, player_guid, 30_000, 1)
        .expect("apply fire resistance aura");
    let _ = session.send_stat_update();
    assert_eq!(armor(&session), 320);

    let normal_slot = session
        .visible_aura_slot_for_spell_like_cpp(90_400)
        .expect("armor aura slot");
    session.remove_aura(normal_slot).expect("remove armor aura");
    let _ = session.send_stat_update();
    assert_eq!(armor(&session), 20, "removing the aura restores the armor");

    // `SPELL_AURA_MOD_RESISTANCE_PCT` scales the whole armor value.
    session
        .apply_aura(90_402, player_guid, 30_000, 1)
        .expect("apply armor percentage aura");
    let _ = session.send_stat_update();
    assert_eq!(armor(&session), 30);

    // `SPELL_AURA_MOD_BONUS_ARMOR_PCT` applies last.
    session
        .apply_aura(90_403, player_guid, 30_000, 1)
        .expect("apply bonus armor aura");
    let _ = session.send_stat_update();
    assert_eq!(armor(&session), 60);

    // `SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT` adds 100% of Agility before
    // the percentage multipliers.
    session
        .apply_aura(90_404, player_guid, 30_000, 1)
        .expect("apply resistance-of-stat aura");
    let _ = session.send_stat_update();
    assert_eq!(armor(&session), 90);
}

#[tokio::test]
async fn avoidance_aura_percentages_follow_update_percentages_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_300);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    assert!(
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().set_can_parry_like_cpp(true);
                player.unit_mut().set_can_block_like_cpp(true);
            })
            .is_some()
    );

    // C++ `SPELL_AURA_MOD_PARRY_PERCENT`/`MOD_DODGE_PERCENT`/`MOD_BLOCK_PERCENT`
    // feed `Player::UpdateParryPercentage`/`UpdateDodgePercentage`/
    // `UpdateBlockPercentage` (`StatSystem.cpp:483-499`, `659-679`, `700-717`).
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount) in [(90_500, 47, 3), (90_501, 49, 10), (90_502, 51, 7)] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let stats = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("avoidance projection")
    };

    let _ = session.send_stat_update();
    let before = stats(&session);
    assert_eq!(before.dodge_pct, 0.0);
    assert_eq!(before.parry_pct, 5.0);
    assert_eq!(before.block_pct, 5.0);

    for spell_id in [90_500, 90_501, 90_502] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply avoidance aura");
    }
    let _ = session.send_stat_update();
    let with_auras = stats(&session);
    assert_eq!(with_auras.dodge_pct, 10.0);
    assert_eq!(with_auras.parry_pct, 8.0);
    assert_eq!(with_auras.block_pct, 12.0);

    let dodge_slot = session
        .visible_aura_slot_for_spell_like_cpp(90_501)
        .expect("dodge aura slot");
    session.remove_aura(dodge_slot).expect("remove dodge aura");
    let _ = session.send_stat_update();
    assert_eq!(stats(&session).dodge_pct, 0.0);
}

#[tokio::test]
async fn crit_aura_percentages_follow_weapon_dependent_auras_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_400);
    let weapon_guid = ObjectGuid::create_item(1, 61_400);
    let weapon_id = 61_400u32;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: weapon_id,
        class_id: ItemClass::Weapon as u8,
        subclass_id: ItemSubClassWeapon::Sword as u8,
        material: 0,
        inventory_type: InventoryType::WeaponMainhand as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            weapon_id,
            ItemStatEntry {
                stats: std::array::from_fn(|_| (wow_constants::ItemModType::None as i8, 0)),
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )));

    // C++ `Player::UpdateWeaponDependentCritAuras` (`Player.cpp:8079-8107`):
    // `SPELL_AURA_MOD_WEAPON_CRIT_PERCENT` is filtered by the attack's weapon
    // requirement, while `SPELL_AURA_MOD_CRIT_PCT` is global.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount) in [
        (90_600, 52, 2),
        (90_601, 52, 5),
        (90_602, 290, 1),
        (90_603, 57, 4),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_equipped_items_store(Arc::new(SpellEquippedItemsStore::from_entries([
        SpellEquippedItemsEntry {
            id: 1,
            spell_id: 90_601,
            equipped_item_class: ItemClass::Weapon as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 1_i32 << (ItemSubClassWeapon::Sword as u32),
        },
    ])));
    session.set_state(crate::session::SessionState::LoggedIn);
    for spell_id in [90_600, 90_601, 90_602, 90_603] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply crit aura");
    }

    let stats = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("crit projection")
    };

    // Without a weapon the item-dependent aura is rejected, so only the
    // item-neutral `MOD_WEAPON_CRIT_PERCENT` (2) and `MOD_CRIT_PCT` (1) apply.
    let _ = session.send_stat_update();
    let unarmed = stats(&session);
    assert_eq!(unarmed.crit_pct, 8.0);
    assert_eq!(unarmed.offhand_crit_pct, 8.0);
    assert_eq!(unarmed.ranged_crit_pct, 8.0);
    assert_eq!(unarmed.spell_crit_pct, [10.0; 7]);

    // Equipping the sword enables the sword-only aura for the mainhand only.
    let weapon = session.make_inventory_item_object(
        weapon_guid,
        weapon_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::EQUIPMENT_SLOT_MAINHAND,
    );
    session.insert_inventory_item_object(weapon);
    session.insert_inventory_item_like_cpp(
        wow_entities::EQUIPMENT_SLOT_MAINHAND,
        InventoryItem {
            guid: weapon_guid,
            entry_id: weapon_id,
            db_guid: weapon_guid.counter() as u64,
            inventory_type: Some(InventoryType::WeaponMainhand as u8),
        },
    );
    let _ = session.send_stat_update();
    let armed = stats(&session);
    assert_eq!(armed.crit_pct, 13.0);
    assert_eq!(armed.offhand_crit_pct, 8.0);
    assert_eq!(armed.ranged_crit_pct, 8.0);
}

#[tokio::test]
async fn weapon_damage_pct_follows_update_damage_pct_done_mods_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_900);
    let mainhand_guid = ObjectGuid::create_item(1, 61_900);
    let offhand_guid = ObjectGuid::create_item(1, 61_901);
    let mainhand_id = 61_900u32;
    let offhand_id = 61_901u32;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_item_store(Arc::new(ItemStore::from_records([
        ItemRecord {
            id: mainhand_id,
            class_id: ItemClass::Weapon as u8,
            subclass_id: ItemSubClassWeapon::Sword as u8,
            material: 0,
            inventory_type: InventoryType::WeaponMainhand as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
        ItemRecord {
            id: offhand_id,
            class_id: ItemClass::Weapon as u8,
            subclass_id: ItemSubClassWeapon::Dagger as u8,
            material: 0,
            inventory_type: InventoryType::WeaponOffhand as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
            scaling_stat_distribution_id: 0,
            scaling_stat_value: 0,
        },
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [
            (
                mainhand_id,
                ItemStatEntry {
                    stats: std::array::from_fn(|_| (wow_constants::ItemModType::None as i8, 0)),
                    resistances: [0; 7],
                    armor: 0,
                },
            ),
            (
                offhand_id,
                ItemStatEntry {
                    stats: std::array::from_fn(|_| (wow_constants::ItemModType::None as i8, 0)),
                    resistances: [0; 7],
                    armor: 0,
                },
            ),
        ],
        [],
    )));
    for (guid, item_id, slot, inventory_type) in [
        (
            mainhand_guid,
            mainhand_id,
            wow_entities::EQUIPMENT_SLOT_MAINHAND,
            InventoryType::WeaponMainhand,
        ),
        (
            offhand_guid,
            offhand_id,
            wow_entities::EQUIPMENT_SLOT_OFFHAND,
            InventoryType::WeaponOffhand,
        ),
    ] {
        let item = session.make_inventory_item_object(
            guid,
            item_id,
            player_guid,
            1,
            0,
            ItemContext::None,
            slot,
        );
        session.insert_inventory_item_object(item);
        session.insert_inventory_item_like_cpp(
            slot,
            InventoryItem {
                guid,
                entry_id: item_id,
                db_guid: guid.counter() as u64,
                inventory_type: Some(inventory_type as u8),
            },
        );
    }

    // `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` effects: one item-neutral +50% and
    // one +100% restricted to swords.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, misc_value, amount) in [(90_930, 1, 50), (90_931, 1, 100)] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(79),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: 79,
                    effect_misc_value_1: misc_value,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_equipped_items_store(Arc::new(SpellEquippedItemsStore::from_entries([
        SpellEquippedItemsEntry {
            id: 1,
            spell_id: 90_931,
            equipped_item_class: ItemClass::Weapon as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 1_i32 << (ItemSubClassWeapon::Sword as u32),
        },
    ])));
    session.set_state(crate::session::SessionState::LoggedIn);

    let stats = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("weapon damage projection")
    };

    // C++ `UpdateDamagePctDoneMods`: mainhand/ranged 1.0, offhand 0.5.
    let _ = session.send_stat_update();
    let baseline = stats(&session);
    assert_eq!(baseline.weapon_damage_pct, [1.0, 0.5, 1.0]);
    assert!(
        (baseline.weapon_damage[1][0] * 2.0 - baseline.weapon_damage[0][0]).abs() < 0.01,
        "the offhand TOTAL_PCT halves the represented range"
    );

    session
        .apply_aura(90_930, player_guid, 30_000, 1)
        .expect("apply physical damage percentage aura");
    let _ = session.send_stat_update();
    let neutral = stats(&session);
    assert_eq!(neutral.weapon_damage_pct, [1.5, 0.75, 1.5]);

    // The sword-restricted aura applies to the mainhand only: the offhand
    // dagger fails `CheckAttackFitToAuraRequirement`, and the ranged attack has
    // no resolved weapon.
    session
        .apply_aura(90_931, player_guid, 30_000, 1)
        .expect("apply sword damage percentage aura");
    let _ = session.send_stat_update();
    let restricted = stats(&session);
    assert_eq!(restricted.weapon_damage_pct, [3.0, 0.75, 1.5]);
}

#[tokio::test]
async fn school_resistances_follow_update_resistances_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_500);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);

    // C++ `Unit::UpdateResistances` (`Unit.cpp:9148-9163`) with the aura
    // producers of `HandleAuraModResistance` and `HandleModResistancePercent`.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, misc_value, amount) in [
        (90_700, 22, 2, 20),   // MOD_RESISTANCE, holy mask
        (90_701, 22, 4, 30),   // MOD_RESISTANCE, fire mask
        (90_702, 101, 4, 100), // MOD_RESISTANCE_PCT, fire mask
        (90_703, 101, 2, 50),  // MOD_RESISTANCE_PCT, holy mask
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_base_points: amount,
                    effect_misc_value_1: misc_value,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let resistances = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("resistance projection")
            .resistances
    };

    let _ = session.send_stat_update();
    assert_eq!(resistances(&session), [20, 0, 0, 0, 0, 0, 0]);

    for spell_id in [90_700, 90_701, 90_702, 90_703] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply resistance aura");
    }
    let _ = session.send_stat_update();
    // Holy: 20 flat * 1.5; fire: 30 flat * 2.0; the rest stay zero.
    assert_eq!(resistances(&session), [20, 30, 60, 0, 0, 0, 0]);

    let fire_slot = session
        .visible_aura_slot_for_spell_like_cpp(90_701)
        .expect("fire resistance aura slot");
    session
        .remove_aura(fire_slot)
        .expect("remove fire resistance aura");
    let _ = session.send_stat_update();
    assert_eq!(resistances(&session), [20, 30, 0, 0, 0, 0, 0]);
}

#[tokio::test]
async fn attack_power_aura_producers_follow_update_attack_power_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_600);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);

    // C++ `HandleAuraModAttackPower`/`HandleAuraModAttackPowerPercent` and the
    // ranged variants (`SpellAuraEffects.cpp:4434-4492`).
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount) in [
        (90_800, 99, 50),
        (90_801, 166, 50),
        (90_802, 124, 30),
        (90_803, 167, 100),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let stats = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("attack power projection")
    };

    let _ = session.send_stat_update();
    let baseline = stats(&session);
    assert_eq!(baseline.attack_power, 220);
    assert_eq!(baseline.attack_power_mod_pos, 0);
    assert_eq!(baseline.attack_power_multiplier, 0.0);
    assert_eq!(baseline.ranged_attack_power, -10);
    assert_eq!(baseline.ranged_attack_power_mod_pos, 0);
    assert_eq!(baseline.ranged_attack_power_multiplier, 0.0);

    for spell_id in [90_800, 90_801, 90_802, 90_803] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply attack power aura");
    }
    let _ = session.send_stat_update();
    let with_auras = stats(&session);
    assert_eq!(with_auras.attack_power_mod_pos, 50);
    assert_eq!(with_auras.attack_power_multiplier, 0.5);
    assert_eq!(with_auras.ranged_attack_power_mod_pos, 30);
    assert_eq!(with_auras.ranged_attack_power_multiplier, 1.0);
    assert_eq!(
        session.canonical_player_total_attack_power_like_cpp(),
        Some(405.0),
        "C++ GetTotalAttackPowerValue clamps the base plus modifier then applies the multiplier"
    );
}

#[tokio::test]
async fn override_attack_power_by_spell_power_aura_replaces_both_attack_mods_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_700);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);

    // C++ `Player::ApplySpellPowerBonus` (`StatSystem.cpp:153-168`) feeds the
    // `ModHealingDonePos`/`ModDamageDonePos` fields the override reads.
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SpellPowerBonus {
            amount: 1_000,
            apply: true,
        }
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, amount) in [(90_820, 15), (90_821, 5)] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(
                    wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
                ),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura:
                        wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let stats = |session: &WorldSession| {
        session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("attack power projection")
    };

    let _ = session.send_stat_update();
    let baseline = stats(&session);
    assert_eq!(baseline.attack_power, 220);
    assert_eq!(baseline.ranged_attack_power, -10);

    // `ApplyModUpdateFieldValue` accumulates both active effects: 15 + 5.
    for spell_id in [90_820, 90_821] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply override attack power aura");
    }
    let _ = session.send_stat_update();
    let overridden = stats(&session);
    assert_eq!(
        overridden.attack_power, 200,
        "C++ replaces the strength/agility/level base with CalculatePct(1000 spell power, 20%)"
    );
    assert_eq!(overridden.ranged_attack_power, 200);
}

#[tokio::test]
async fn spell_damage_and_healing_bonus_auras_publish_update_spell_bonus_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_800);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    // C++ `Player::ApplySpellPowerBonus` (`StatSystem.cpp:153-168`) accumulates
    // the item spell power into `GetBaseSpellPowerBonus()`.
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SpellPowerBonus {
            amount: 100,
            apply: true,
        }
    ));
    // C++ `ApplySpellPenetrationBonus` subtracts the item penetration from
    // `ModTargetResistance`.
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SpellPenetrationBonus {
            amount: 15,
            apply: true,
        }
    ));

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, misc_value_1, misc_value_2, amount) in [
        // +30 holy damage, +20 fire damage and -50 fire damage.
        (90_900, 13, 1 << 1, 0, 30),
        (90_901, 13, 1 << 2, 0, 20),
        (90_902, 13, 1 << 2, 0, -50),
        // +40 flat healing for every school.
        (90_903, 135, 0, 0, 40),
        // +50% of intellect (40) as holy damage.
        (90_904, 174, 1 << 1, 3, 50),
        // +25% of spirit (30) as healing.
        (90_905, 175, 4, 0, 25),
        // +50% and +100% holy damage done, +25% fire damage done.
        (90_907, 79, 1 << 1, 0, 50),
        (90_908, 79, 1 << 1, 0, 100),
        (90_909, 79, 1 << 2, 0, 25),
        // +50% and +100% healing done.
        (90_910, 136, 0, 0, 50),
        (90_911, 136, 0, 0, 100),
        // Spell penetration aura (full magic mask) and armor-only penetration.
        (90_912, 123, 0x3E, 0, 20),
        (90_913, 123, 1, 0, 30),
        // Versatility bonus.
        (90_915, 471, 0, 0, 200),
    ] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_misc_value_1: misc_value_1,
                    effect_misc_value_2: misc_value_2,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    // `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` (404) has no misc values.
    spell_store.insert(
        90_906,
        wow_data::SpellInfo {
            spell_id: 90_906,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(
                wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
            ),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura:
                    wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    for spell_id in [
        90_900, 90_901, 90_902, 90_903, 90_904, 90_905, 90_907, 90_908, 90_909, 90_910, 90_911,
        90_912, 90_913, 90_915,
    ] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply spell bonus aura");
    }
    let _ = session.send_stat_update();
    let stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("spell bonus projection");
    // Holy: 100 base + 30 flat + 50% of intellect 40.
    assert_eq!(stats.mod_damage_done_pos[1], 150);
    // Fire: (100 + 20 - 50) - (-50) leaves the positive aura only.
    assert_eq!(stats.mod_damage_done_pos[2], 120);
    assert_eq!(stats.mod_damage_done_neg[2], -50);
    assert_eq!(stats.mod_damage_done_pos[0], 0);
    // Healing: 100 base + 40 flat + intellect 40 (mana class) + 25% of spirit 30.
    assert_eq!(stats.mod_healing_done_pos, 187);
    // `HandleModDamagePercentDone` multiplies every matching effect:
    // holy (1 + 0.5) * (1 + 1.0) = 3.0, fire 1.25, the rest keep 1.0.
    assert_eq!(stats.mod_damage_done_percent[0], 1.0);
    assert_eq!(stats.mod_damage_done_percent[1], 3.0);
    assert_eq!(stats.mod_damage_done_percent[2], 1.25);
    assert_eq!(stats.mod_damage_done_percent[3..], [1.0; 4]);
    // `UpdateHealingDonePercentMod`: (1 + 0.5) * (1 + 1.0) = 3.0.
    assert_eq!(stats.mod_healing_done_percent, 3.0);
    // Aura 123 magic mask 20 minus item penetration 15; armor mask 30.
    assert_eq!(stats.mod_target_resistance, 5);
    assert_eq!(stats.mod_target_physical_resistance, 30);
    // No override aura is active yet: C++ fields hold the 0.0 default.
    assert_eq!(stats.override_spell_power_by_ap_percent, 0.0);
    assert_eq!(stats.override_ap_by_spell_power_percent, 0.0);
    // `HandleModVersatilityByPct` sums aura 471 into `VersatilityBonus`.
    assert_eq!(stats.versatility_bonus, 200.0);

    // `HasAuraType` on 404 then replaces both attack mods with
    // `CalculatePct(min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX]), 50)`:
    // the minimum magic school is 100, so both become 50.
    session
        .apply_aura(90_906, player_guid, 30_000, 1)
        .expect("apply override attack power aura");
    let _ = session.send_stat_update();
    let overridden = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("override attack power projection");
    assert_eq!(overridden.attack_power, 50);
    assert_eq!(overridden.ranged_attack_power, 50);
    assert_eq!(
        overridden.mod_healing_done_pos, 187,
        "the attack-power override does not rewrite the spell fields"
    );
    assert_eq!(
        overridden.override_ap_by_spell_power_percent, 50.0,
        "the aura amount is published on ActivePlayerData"
    );
}

#[tokio::test]
async fn override_spell_power_by_ap_publishes_the_field_and_recomputes_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_850);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 1, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 0,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 1;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    assert!(session.apply_represented_item_bonus_action_state_like_cpp(
        ApplyEnchantmentEffectAction::SpellPowerBonus {
            amount: 100,
            apply: true,
        }
    ));

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        90_920,
        wow_data::SpellInfo {
            spell_id: 90_920,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 50,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT,
                effect_base_points: 50,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    let _ = session.send_stat_update();
    let baseline = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("baseline projection");
    assert_eq!(baseline.override_spell_power_by_ap_percent, 0.0);
    assert_eq!(baseline.mod_healing_done_pos, 100);
    assert_eq!(baseline.attack_power, 220);

    session
        .apply_aura(90_920, player_guid, 30_000, 1)
        .expect("apply override spell power aura");
    let _ = session.send_stat_update();
    let overridden = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("override spell power projection");
    assert_eq!(overridden.override_spell_power_by_ap_percent, 50.0);
    // `SpellBaseDamageBonusDone`/`SpellBaseHealingBonusDone` return
    // `int32(CalculatePct(melee AP 220, 50) + 0.5) = 110` and discard the item
    // spell power.
    assert_eq!(overridden.mod_healing_done_pos, 110);
    assert_eq!(overridden.mod_damage_done_pos[1..], [110; 6]);
    assert_eq!(overridden.attack_power, 220);
}

#[tokio::test]
async fn ranged_attack_power_auras_skip_wand_users_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_601);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([{
            let mut entry = wow_data::character_progression::ChrClassesEntry::default();
            entry.id = 5;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);

    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount) in [(90_810, 124, 30), (90_811, 167, 100)] {
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: amount,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(aura_type),
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: aura_type,
                    effect_base_points: amount,
                    ..Default::default()
                }],
            },
        );
    }
    session.set_spell_store(Arc::new(spell_store));
    session.set_state(crate::session::SessionState::LoggedIn);

    for spell_id in [90_810, 90_811] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply ranged attack power aura");
    }
    let _ = session.send_stat_update();
    let stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("ranged attack power projection");
    assert_eq!(
        stats.ranged_attack_power_mod_pos, 0,
        "CLASSMASK_WAND_USERS classes ignore the ranged flat aura"
    );
    assert_eq!(stats.ranged_attack_power_multiplier, 0.0);
}
