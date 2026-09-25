//! Defensive combat-stat aura projection scenarios.

use super::*;

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
