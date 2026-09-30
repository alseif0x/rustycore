use super::*;

#[test]
fn represented_item_mods_apply_scaling_stat_loop_spell_bonus_and_armor_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 902);
    session.set_player_guid(Some(player_guid));
    session.set_player_level_like_cpp(80);
    session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
        scaling_stat_distribution_id: 78,
        scaling_stat_value: 0x0010_8008,
        ..represented_test_item_record_like_cpp(
            102,
            InventoryType::Chest,
            ItemClass::Armor,
            1,
        )
    }])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_parts(
        [(
            102,
            ItemStatEntry {
                stats: [
                    (ItemModType::Intellect as i8, 999),
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
                resistances: [17, 0, 0, 0, 0, 0, 0],
                armor: 17,
            },
        )],
        [],
    )));
    let mut stat_id = [-1; 10];
    stat_id[0] = ItemModType::Strength as i32;
    let mut bonus = [0; 10];
    bonus[0] = 5_000;
    session.set_scaling_stat_distribution_store(Arc::new(
        ScalingStatDistributionStore::from_entries([ScalingStatDistributionEntry {
            id: 78,
            player_level_to_item_level_curve_id: 0,
            min_level: 10,
            max_level: 20,
            bonus,
            stat_id,
        }]),
    ));
    session.set_scaling_stat_values_store(Arc::new(ScalingStatValuesStore::from_entries([
        ScalingStatValuesEntry {
            id: 20,
            char_level: 20,
            weapon_dps_1h: 0,
            weapon_dps_2h: 0,
            spellcaster_dps_1h: 0,
            spellcaster_dps_2h: 0,
            ranged_dps: 0,
            wand_dps: 0,
            spell_power: 33,
            shoulder_budget: 0,
            trinket_budget: 0,
            weapon_budget_1h: 0,
            primary_budget: 200,
            ranged_budget: 0,
            tertiary_budget: 0,
            cloth_shoulder_armor: 0,
            leather_shoulder_armor: 0,
            mail_shoulder_armor: 0,
            plate_shoulder_armor: 0,
            cloth_cloak_armor: 0,
            cloth_chest_armor: 77,
            leather_chest_armor: 0,
            mail_chest_armor: 0,
            plate_chest_armor: 0,
        },
    ])));
    session
        .player_item_test_fixture_like_cpp
        .inventory_items
        .insert(
            EQUIPMENT_SLOT_CHEST,
            InventoryItem {
                guid: item_guid,
                entry_id: 102,
                db_guid: item_guid.counter() as u64,
                inventory_type: Some(InventoryType::Chest as u8),
            },
        );
    let item = session.make_inventory_item_object(
        item_guid,
        102,
        player_guid,
        1,
        0,
        ItemContext::None,
        EQUIPMENT_SLOT_CHEST,
    );
    session.insert_inventory_item_object(item);

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_CHEST, true);

    assert_eq!(
        session.represented_item_bonus_actions_like_cpp(),
        &[
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                action: ApplyEnchantmentEffectAction::UnitModifier {
                    unit_mod: wow_entities::ApplyEnchantmentUnitMod::StatStrength,
                    modifier: wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
                    amount: 100,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                action: ApplyEnchantmentEffectAction::UpdateStatBuffMod(
                    wow_constants::Stats::Strength,
                ),
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                action: ApplyEnchantmentEffectAction::SpellPowerBonus {
                    amount: 33,
                    apply: true,
                },
            },
            RepresentedItemBonusActionLikeCpp {
                item_guid,
                slot: EQUIPMENT_SLOT_CHEST,
                action: ApplyEnchantmentEffectAction::UnitModifier {
                    unit_mod: wow_entities::ApplyEnchantmentUnitMod::Resistance(
                        wow_constants::spell::SpellSchools::Normal as u32,
                    ),
                    modifier: wow_entities::ApplyEnchantmentUnitModifier::BaseValue,
                    amount: 77,
                    apply: true,
                },
            },
        ],
        "C++ uses ScalingStatDistribution stat slots instead of ItemSparse stats, then applies getSpellBonus and getArmorMod"
    );
    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base
            [wow_constants::Stats::Strength as usize],
        100
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .spell_power_bonus,
        33
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .resistances_base[wow_constants::spell::SpellSchools::Normal as usize],
        77
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .stat_buff_updates,
        &[wow_constants::Stats::Strength]
    );
    let stat_changes =
        represented_player_stat_changes_like_cpp(&session.represented_item_bonus_state_like_cpp());
    assert_eq!(
        stat_changes.stats[wow_constants::Stats::Strength as usize],
        100
    );
    assert_eq!(
        stat_changes.stat_pos_buff[wow_constants::Stats::Strength as usize],
        100
    );
    assert_eq!(
        stat_changes.mod_healing_done_pos, 33,
        "C++ ApplySpellPowerBonus updates ModHealingDonePos and magic ModDamageDonePos update fields"
    );
    assert_eq!(
        &stat_changes.mod_damage_done_pos[1..],
        &[33; 6],
        "C++ publishes the item spell power to every magic school"
    );
    assert_eq!(stat_changes.mod_damage_done_pos[0], 0);
    assert_eq!(stat_changes.mod_damage_done_percent, [1.0; 7]);
    assert_eq!(stat_changes.mod_healing_done_pct, 1.0);
    assert_eq!(stat_changes.mod_target_resistance, 0);
    assert_eq!(stat_changes.mod_target_physical_resistance, 0);
    assert_eq!(stat_changes.versatility_bonus, 0.0);
    assert_eq!(stat_changes.override_spell_power_by_ap_percent, 0.0);
    assert_eq!(stat_changes.override_ap_by_spell_power_percent, 0.0);
    assert_eq!(
        stat_changes.mod_damage_done_neg, [0; 7],
        "no negative damage aura is active in this fixture"
    );
    assert_eq!(
        stat_changes.armor, 77,
        "C++ armor/resistance item mods surface as UnitData::Resistances[0]"
    );

    session.record_represented_item_mods_like_cpp(item_guid, EQUIPMENT_SLOT_CHEST, false);

    assert_eq!(
        session.represented_item_bonus_state_like_cpp().stats_base
            [wow_constants::Stats::Strength as usize],
        0,
        "C++ _ApplyItemBonuses(..., false) removes the same represented stat delta"
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .spell_power_bonus,
        0
    );
    assert_eq!(
        session
            .represented_item_bonus_state_like_cpp()
            .resistances_base[wow_constants::spell::SpellSchools::Normal as usize],
        0
    );
    let removed_changes =
        represented_player_stat_changes_like_cpp(&session.represented_item_bonus_state_like_cpp());
    assert_eq!(
        removed_changes.stats[wow_constants::Stats::Strength as usize],
        0
    );
    assert_eq!(removed_changes.mod_healing_done_pos, 0);
    assert_eq!(removed_changes.mod_damage_done_pos, [0; 7]);
    assert_eq!(removed_changes.mod_damage_done_neg, [0; 7]);
    assert_eq!(removed_changes.mod_damage_done_percent, [1.0; 7]);
    assert_eq!(removed_changes.mod_healing_done_pct, 1.0);
    assert_eq!(removed_changes.mod_target_resistance, 0);
    assert_eq!(removed_changes.mod_target_physical_resistance, 0);
    assert_eq!(removed_changes.versatility_bonus, 0.0);
    assert_eq!(removed_changes.override_spell_power_by_ap_percent, 0.0);
    assert_eq!(removed_changes.override_ap_by_spell_power_percent, 0.0);
    assert_eq!(removed_changes.armor, 0);
}
#[test]
fn represented_item_set_add_remove_tracks_threshold_spell_events_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let chest_guid = ObjectGuid::create_item(1, 906);
    let hands_guid = ObjectGuid::create_item(1, 907);
    session.set_player_guid(Some(player_guid));
    session.set_item_set_store(Arc::new(ItemSetStore::from_entries([ItemSetEntry {
        id: 700,
        name: "Test Set".to_string(),
        set_flags: 0,
        required_skill: 0,
        required_skill_rank: 0,
        item_id: std::array::from_fn(|i| match i {
            0 => 100,
            1 => 101,
            _ => 0,
        }),
    }])));
    session.set_item_set_spell_store(Arc::new(ItemSetSpellStore::from_entries([
        ItemSetSpellEntry {
            id: 1,
            chr_spec_id: 0,
            spell_id: 9001,
            threshold: 2,
            item_set_id: 700,
        },
        ItemSetSpellEntry {
            id: 2,
            chr_spec_id: 0,
            spell_id: 9002,
            threshold: 3,
            item_set_id: 700,
        },
    ])));
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_CHEST,
        chest_guid,
        100,
        InventoryType::Chest,
    );
    equip_represented_test_item_like_cpp(
        &mut session,
        EQUIPMENT_SLOT_HANDS,
        hands_guid,
        101,
        InventoryType::Hands,
    );

    assert!(!session.record_represented_items_set_item_like_cpp(chest_guid, true));
    assert!(session.record_represented_items_set_item_like_cpp(hands_guid, true));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp(),
        &[RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 700,
            spell_entry_id: 1,
            spell_id: 9001,
            threshold: 2,
            apply: true,
        }]
    );
    assert_eq!(
        session
            .represented_item_set_effect_like_cpp(700)
            .expect("set effect")
            .equipped_items
            .len(),
        2
    );

    assert!(session.record_represented_items_set_item_like_cpp(chest_guid, false));
    assert_eq!(
        session.represented_item_set_spell_events_like_cpp()[1],
        RepresentedItemSetSpellEventLikeCpp {
            item_set_id: 700,
            spell_entry_id: 1,
            spell_id: 9001,
            threshold: 2,
            apply: false,
        }
    );
}
