use crate::{Player, PlayerEffectiveCombatStatsLikeCpp};
use wow_core::ObjectGuid;

fn player_in_world() -> Player {
    let mut player = Player::new(Some(7), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, 9_100));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.clear_data_changes();
    player
}

fn assert_mask_is_exact(actual: &crate::UpdateMask, bits: &[usize]) {
    let mut expected = crate::UpdateMask::new(actual.bits());
    for bit in bits {
        expected.set(*bit);
    }
    assert_eq!(actual.blocks(), expected.blocks());
}

fn derived_stats_with_changes() -> PlayerEffectiveCombatStatsLikeCpp {
    PlayerEffectiveCombatStatsLikeCpp {
        stats: [1, 2, 3, 4, 5],
        stat_pos_buff: [6, 7, 8, 9, 10],
        stat_neg_buff: [11, 12, 13, 14, 15],
        base_health: 101,
        max_health: 102,
        base_mana: 103,
        max_mana: 104,
        resistances: [16, 17, 18, 19, 20, 21, 22],
        attack_power: 23,
        attack_power_mod_pos: 24,
        attack_power_mod_neg: 25,
        attack_power_multiplier: 1.25,
        ranged_attack_power: 26,
        ranged_attack_power_mod_pos: 27,
        ranged_attack_power_mod_neg: 28,
        ranged_attack_power_multiplier: 1.5,
        min_damage: 2.0,
        max_damage: 3.0,
        weapon_damage: [[2.0, 3.0], [4.0, 5.0], [6.0, 7.0]],
        min_ranged_damage: 6.0,
        max_ranged_damage: 7.0,
        mana_regen: 8.0,
        mana_regen_combat: 9.0,
        mana_regen_mp5: 10.0,
        mainhand_expertise: 11.0,
        offhand_expertise: 12.0,
        ranged_expertise: 13.0,
        combat_rating_expertise: 14.0,
        block_pct: 15.0,
        dodge_pct: 16.0,
        dodge_from_attr: 17.0,
        parry_pct: 18.0,
        parry_from_attr: 19.0,
        crit_pct: 20.0,
        ranged_crit_pct: 21.0,
        offhand_crit_pct: 22.0,
        spell_crit_pct: [0.1; 7],
        shield_block: 23,
        // The default zero does not become a leaf update until it changes.
        shield_block_crit_pct: 0.0,
        mod_damage_done_pos: [24; 7],
        mod_damage_done_neg: [-25; 7],
        mod_damage_done_percent: [1.25; 7],
        combat_ratings: [26; 32],
        mod_healing_done_pos: 27,
        mod_healing_done_percent: 1.5,
        versatility_bonus: 28.0,
        override_spell_power_by_ap_percent: 1.75,
        override_ap_by_spell_power_percent: 2.0,
        mod_target_resistance: 29,
        mod_target_physical_resistance: 30,
        // These snapshot-only values have no update-field leaf in this seam.
        armor_penetration_pct: 31.0,
        melee_hit_chance_pct: 32.0,
        health_regen: 33,
        spell_penetration: 34,
        ..Default::default()
    }
}

#[test]
fn derived_stat_legacy_replacement_does_not_dirty_values() {
    let mut player = Player::new(None, false);
    player.clear_data_changes();

    let mut stats = PlayerEffectiveCombatStatsLikeCpp::default();
    stats.stats[0] = 17;
    player.replace_effective_combat_stats_like_cpp(stats);

    assert_eq!(player.effective_combat_stats_like_cpp(), &stats);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
    assert!(!player.unit().world().object().is_object_updated());
}

#[test]
fn derived_stat_opt_in_marks_only_backed_children_and_captures_snapshot() {
    use wow_constants::PowerType;

    let mut player = player_in_world();
    player.unit_mut().set_power_index(PowerType::Mana, Some(2));
    let stats = derived_stats_with_changes();

    player.replace_effective_combat_stats_for_values(stats);

    let unit_mask = player.unit().unit_data_changes_mask();
    let mut unit_bits = vec![32, 52, 53, 54, 55, 64, 76];
    unit_bits.extend(81..=88);
    unit_bits.extend([91, 92, 116, 119, 129, 174, 190]);
    unit_bits.extend(175..=189);
    unit_bits.extend(191..=197);
    assert_mask_is_exact(unit_mask, &unit_bits);
    assert!(!unit_mask.is_set(5), "Health remains Unit-owned");
    assert!(!unit_mask.is_set(6), "MaxHealth remains Unit-owned");
    assert!(!unit_mask.is_set(75), "BaseMana remains Unit-owned");
    assert!(
        !unit_mask.is_set(159),
        "ModPowerRegen has no represented producer"
    );

    let active_mask = player.active_player_data_changes_mask();
    let mut active_bits = vec![0, 36, 37, 38, 49, 56, 59, 61, 65, 66, 67, 68, 269, 574];
    active_bits.extend(39..=48);
    active_bits.extend(270..=297);
    active_bits.extend(575..=606);
    active_bits.sort_unstable();
    active_bits.dedup();
    assert_mask_is_exact(active_mask, &active_bits);
    assert!(
        !active_mask.is_set(50),
        "unchanged zero shield crit stays clean"
    );
    assert!(
        !active_mask.is_set(69),
        "LocalFlags is not part of derived stats"
    );
    assert!(player.unit().world().object().is_object_updated());

    let captured = player.values_update(true);
    let derived = captured.derived_stats.unwrap();
    assert_eq!(derived.effective_combat_stats, stats);
    assert_eq!(derived.mana_power_index, Some(2));

    player.clear_data_changes();
    let mut later = stats;
    later.stats[0] = 99;
    player.replace_effective_combat_stats_like_cpp(later);
    assert_eq!(derived.effective_combat_stats.stats[0], 1);
    assert!(!player.unit().world().object().is_object_updated());
}

#[test]
fn derived_stat_changes_respect_optional_mana_index_and_no_change() {
    use wow_constants::PowerType;

    let mut player = player_in_world();
    player.unit_mut().set_power_index(PowerType::Mana, None);

    let mut stats = PlayerEffectiveCombatStatsLikeCpp::default();
    stats.mana_regen = 5.0;
    player.replace_effective_combat_stats_for_values(stats);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
    assert!(player.values_update(true).derived_stats.is_none());
    assert!(!player.unit().world().object().is_object_updated());

    player.unit_mut().set_power_index(PowerType::Mana, Some(3));
    let mut mp5_only = stats;
    mp5_only.mana_regen_mp5 = 4.0;
    player.replace_effective_combat_stats_for_values(mp5_only);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
    assert!(!player.unit().world().object().is_object_updated());

    let mut changed = stats;
    changed.mana_regen = 6.0;
    player.replace_effective_combat_stats_for_values(changed);
    assert!(player.unit().unit_data_changes_mask().is_set(120));
    assert!(player.unit().unit_data_changes_mask().is_set(116));
    assert_eq!(
        player
            .values_update(false)
            .derived_stats
            .unwrap()
            .mana_power_index,
        Some(3)
    );

    player.clear_data_changes();
    player.replace_effective_combat_stats_for_values(changed);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
    assert!(!player.active_player_data_changes_mask().is_any_set());
    assert!(!player.unit().world().object().is_object_updated());

    let mut active_only = changed;
    active_only.mainhand_expertise = 1.0;
    player.replace_effective_combat_stats_for_values(active_only);
    assert!(!player.unit().unit_data_changes_mask().is_any_set());
    assert!(player.active_player_data_changes_mask().is_set(36));
    assert!(player.unit().world().object().is_object_updated());
    let self_update = player.values_update(true);
    assert!(self_update.unit_data.is_none());
    assert!(self_update.active_player_data.is_some());
    assert!(self_update.derived_stats.is_some());
    let partial_update = player.values_update(false);
    assert!(partial_update.active_player_data.is_none());
    assert!(partial_update.derived_stats.is_none());
}

#[test]
fn effective_combat_stats_are_player_owned_and_replace_as_one_snapshot() {
    let mut player = Player::new(None, false);
    assert_eq!(
        player.effective_combat_stats_like_cpp(),
        &PlayerEffectiveCombatStatsLikeCpp::default()
    );

    let mut stats = PlayerEffectiveCombatStatsLikeCpp::default();
    stats.stats = [10, 20, 30, 40, 50];
    stats.attack_power = 123;
    stats.resistances[2] = 17;
    player.replace_effective_combat_stats_like_cpp(stats);
    assert_eq!(player.effective_combat_stats_like_cpp(), &stats);

    player.clear_effective_combat_stats_like_cpp();
    assert_eq!(
        player.effective_combat_stats_like_cpp(),
        &PlayerEffectiveCombatStatsLikeCpp::default()
    );
}

#[test]
fn total_attack_power_matches_cpp_non_negative_modifier_and_multiplier_order() {
    let mut player = Player::new(None, false);
    let mut stats = PlayerEffectiveCombatStatsLikeCpp {
        attack_power: 120,
        attack_power_mod_pos: 15,
        attack_power_mod_neg: -5,
        attack_power_multiplier: 0.25,
        ranged_attack_power: 80,
        ranged_attack_power_mod_pos: 10,
        ranged_attack_power_mod_neg: -100,
        ranged_attack_power_multiplier: 0.5,
        ..Default::default()
    };
    player.replace_effective_combat_stats_like_cpp(stats);

    assert_eq!(player.total_attack_power_like_cpp(), 162.5);
    // C++ clamps a negative pre-multiplier total to zero.
    stats.ranged_attack_power_mod_neg = -200;
    player.replace_effective_combat_stats_like_cpp(stats);
    assert_eq!(player.total_ranged_attack_power_like_cpp(), 0.0);
}

#[test]
fn effective_weapon_damage_prefers_player_snapshot_and_falls_back_to_unit() {
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .set_weapon_damage(wow_constants::WeaponAttackType::BaseAttack, 7.0, 9.0);
    assert_eq!(
        player.weapon_damage_like_cpp(wow_constants::WeaponAttackType::BaseAttack),
        [7.0, 9.0]
    );

    let mut stats = *player.effective_combat_stats_like_cpp();
    stats.weapon_damage[wow_constants::WeaponAttackType::BaseAttack as usize] = [111.0, 137.0];
    player.replace_effective_combat_stats_like_cpp(stats);
    assert_eq!(
        player.weapon_damage_like_cpp(wow_constants::WeaponAttackType::BaseAttack),
        [111.0, 137.0]
    );
}

#[test]
fn stat_aura_footprint_marks_offhand_for_a_usable_weapon_or_changed_modifier() {
    use crate::{EQUIPMENT_SLOT_OFFHAND, Item, PlayerInventoryItem};
    use wow_constants::InventoryType;

    let mut player = player_in_world();
    player.mark_stat_aura_update_footprint_like_cpp(1, [false; 3]);
    assert!(!player.unit().unit_data_changes_mask().is_set(54));
    assert!(!player.unit().unit_data_changes_mask().is_set(55));

    player.clear_data_changes();
    let item_guid = ObjectGuid::create_item(1, 9_101);
    let mut offhand = Item::default();
    offhand.object_mut().create(item_guid);
    offhand.set_max_durability(100);
    offhand.set_durability(100);
    player
        .inventory_runtime_mut_like_cpp()
        .inventory_items_mut()
        .insert(
            EQUIPMENT_SLOT_OFFHAND,
            PlayerInventoryItem {
                guid: item_guid,
                entry_id: 9_101,
                db_guid: 9_101,
                inventory_type: Some(InventoryType::WeaponOffhand as u8),
            },
        );
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .insert(item_guid, offhand);

    player.mark_stat_aura_update_footprint_like_cpp(1, [false; 3]);
    assert!(player.unit().unit_data_changes_mask().is_set(54));
    assert!(player.unit().unit_data_changes_mask().is_set(55));

    player.clear_data_changes();
    player
        .inventory_runtime_mut_like_cpp()
        .item_objects_mut()
        .get_mut(&item_guid)
        .expect("canonical offhand item")
        .set_durability(0);
    player.mark_stat_aura_update_footprint_like_cpp(1, [false; 3]);
    assert!(!player.unit().unit_data_changes_mask().is_set(54));
    assert!(!player.unit().unit_data_changes_mask().is_set(55));

    player.clear_data_changes();
    player.mark_stat_aura_update_footprint_like_cpp(1, [false, true, false]);
    assert!(player.unit().unit_data_changes_mask().is_set(54));
    assert!(player.unit().unit_data_changes_mask().is_set(55));
    assert!(player.unit().unit_data_changes_mask().is_set(32));
}
