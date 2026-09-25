//! Weapon-fit and weapon-driven offense scenarios.

use super::*;

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

    // `SPELL_AURA_MOD_DAMAGE_DONE` (13) physical flat bonus, and
    // `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` (79) effects: one item-neutral +50%
    // and one +100% restricted to swords.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, misc_value, amount) in [
        (90_930, 79, 1, 50),
        (90_931, 79, 1, 100),
        (90_932, 13, 1, 20),
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
    assert_eq!(baseline.weapon_damage_flat, [0.0; 3]);
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
    assert_eq!(neutral.weapon_damage_flat, [0.0; 3]);

    // The sword-restricted aura applies to the mainhand only: the offhand
    // dagger fails `CheckAttackFitToAuraRequirement`, and the ranged attack has
    // no resolved weapon.
    session
        .apply_aura(90_931, player_guid, 30_000, 1)
        .expect("apply sword damage percentage aura");
    let _ = session.send_stat_update();
    let restricted = stats(&session);
    assert_eq!(restricted.weapon_damage_pct, [3.0, 0.75, 1.5]);

    // C++ `Unit::UpdateDamageDoneMods`: the physical `MOD_DAMAGE_DONE` flat sum
    // is applied before the percentage, and the neutral aura reaches every
    // attack while the sword-restricted one reaches the mainhand only.
    let mainhand_before = restricted.weapon_damage[0][0];
    session
        .apply_aura(90_932, player_guid, 30_000, 1)
        .expect("apply physical flat damage aura");
    let _ = session.send_stat_update();
    let flat = stats(&session);
    assert_eq!(flat.weapon_damage_flat, [20.0, 20.0, 20.0]);
    assert!(
        (flat.weapon_damage[0][0] - (mainhand_before + 20.0 * 3.0)).abs() < 0.01,
        "the flat bonus is multiplied by the attack percentage"
    );
}

#[tokio::test]
async fn weapon_fit_resolves_the_ranged_weapon_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 62_000);
    let bow_guid = ObjectGuid::create_item(1, 62_000);
    let bow_id = 62_000u32;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 3, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 3, 80),
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
            entry.id = 3;
            entry
        }]),
    ));
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_loaded_player_identity_like_cpp(571, 1, 3, 80, 0);
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        id: bow_id,
        class_id: ItemClass::Weapon as u8,
        subclass_id: ItemSubClassWeapon::Bow as u8,
        material: 0,
        inventory_type: InventoryType::Ranged as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            bow_id,
            ItemStatEntry {
                stats: std::array::from_fn(|_| (wow_constants::ItemModType::None as i8, 0)),
                resistances: [0; 7],
                armor: 0,
            },
        )],
        [],
    )));

    // `SPELL_AURA_MOD_WEAPON_CRIT_PERCENT` (52) and
    // `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` (79), both restricted to bows.
    let mut spell_store = wow_data::SpellStore::new();
    for (spell_id, aura_type, amount) in [(90_940, 52, 5), (90_941, 79, 100)] {
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
                    effect_misc_value_1: if aura_type == 79 { 1 } else { 0 },
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
            spell_id: 90_940,
            equipped_item_class: ItemClass::Weapon as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 1_i32 << (ItemSubClassWeapon::Bow as u32),
        },
        SpellEquippedItemsEntry {
            id: 2,
            spell_id: 90_941,
            equipped_item_class: ItemClass::Weapon as i8,
            equipped_item_inv_types: 0,
            equipped_item_subclass: 1_i32 << (ItemSubClassWeapon::Bow as u32),
        },
    ])));
    session.set_state(crate::session::SessionState::LoggedIn);
    let bow = session.make_inventory_item_object(
        bow_guid,
        bow_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        wow_entities::EQUIPMENT_SLOT_RANGED,
    );
    session.insert_inventory_item_object(bow);
    session.insert_inventory_item_like_cpp(
        wow_entities::EQUIPMENT_SLOT_RANGED,
        InventoryItem {
            guid: bow_guid,
            entry_id: bow_id,
            db_guid: bow_guid.counter() as u64,
            inventory_type: Some(InventoryType::Ranged as u8),
        },
    );
    for spell_id in [90_940, 90_941] {
        session
            .apply_aura(spell_id, player_guid, 30_000, 1)
            .expect("apply bow-restricted aura");
    }
    let _ = session.send_stat_update();
    let stats = session
        .canonical_player_effective_combat_stats_like_cpp()
        .expect("ranged fit projection");
    // `GetWeaponForAttack(RANGED_ATTACK, true)` resolves the equipped bow, so
    // both restricted auras apply to the ranged attack only.
    assert_eq!(stats.ranged_crit_pct, 10.0);
    assert_eq!(stats.crit_pct, 5.0);
    assert_eq!(stats.offhand_crit_pct, 5.0);
    assert_eq!(stats.weapon_damage_pct, [1.0, 0.5, 2.0]);
}

#[tokio::test]
async fn weapon_enchant_damage_adds_flat_and_shaman_totem_scaling_like_cpp() {
    let weapon_id = 62_100u32;
    let enchantment_id = 9_001u32;

    let enchantment = wow_data::SpellItemEnchantmentEntry {
        id: enchantment_id,
        effect_arg: [0; 3],
        // 25 flat damage and a shaman totem scaled by the 2.0 s weapon delay.
        effect_scaling_points: [25.0, 10.0, 0.0],
        effect_points_min: [0; 3],
        item_visual: 0,
        flags: wow_constants::SpellItemEnchantmentFlags::empty(),
        required_skill_id: 0,
        required_skill_rank: 0,
        item_level: 0,
        charges: 0,
        effect: [2, 6, 0],
        condition_id: 0,
        min_level: 0,
        max_level: 0,
    };

    for (class, expected_flat) in [(7u8, 45.0f32), (1u8, 25.0f32)] {
        let (mut session, _, _) = make_session();
        let player_guid = ObjectGuid::create_player(1, 62_100 + i64::from(class));
        let weapon_guid = ObjectGuid::create_item(1, 62_100 + i64::from(class));
        session.set_player_guid(Some(player_guid));
        session.set_loaded_player_identity_like_cpp(571, 1, class, 80, 0);
        session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
            (1, class, 80),
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
                entry.id = u32::from(class);
                entry
            }]),
        ));
        crate::canonical_player_access::install_canonical_player_owner_for_test(
            &mut session,
            571,
            0,
        );
        session.set_loaded_player_identity_like_cpp(571, 1, class, 80, 0);
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
        session.set_item_stats_store(Arc::new(ItemStatsStore::from_weapon_templates([(
            weapon_id,
            wow_data::ItemWeaponTemplateEntry {
                dmg_variance: 1.0,
                item_delay: 2_000,
                min_damage: [10, 0, 0, 0, 0],
                max_damage: [20, 0, 0, 0, 0],
                damage_damage_type: 0,
            },
        )])));
        session.set_spell_item_enchantment_store(Arc::new(
            wow_data::SpellItemEnchantmentStore::from_entries([enchantment]),
        ));
        session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
        let mut item = session.make_inventory_item_object(
            weapon_guid,
            weapon_id,
            player_guid,
            1,
            0,
            ItemContext::None,
            wow_entities::EQUIPMENT_SLOT_MAINHAND,
        );
        item.set_enchantment(
            wow_constants::EnchantmentSlot::EnhancementPermanent,
            enchantment_id as i32,
            0,
            0,
        );
        session.insert_inventory_item_object(item);
        session.insert_inventory_item_like_cpp(
            wow_entities::EQUIPMENT_SLOT_MAINHAND,
            InventoryItem {
                guid: weapon_guid,
                entry_id: weapon_id,
                db_guid: weapon_guid.counter() as u64,
                inventory_type: Some(InventoryType::WeaponMainhand as u8),
            },
        );
        session.set_state(crate::session::SessionState::LoggedIn);
        let _ = session.send_stat_update();
        let stats = session
            .canonical_player_effective_combat_stats_like_cpp()
            .expect("enchant damage projection");
        assert_eq!(
            stats.weapon_damage_flat,
            [expected_flat, 0.0, 0.0],
            "class {class}: ITEM_ENCHANTMENT_TYPE_DAMAGE always applies and \
             ITEM_ENCHANTMENT_TYPE_TOTEM applies to shamans only"
        );
    }
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
