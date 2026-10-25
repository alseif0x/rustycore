//! Derived Player stat publication through the canonical VALUES bridge.

use super::*;
use wow_constants::PowerType;
use wow_entities::{MapObjectRecord, PlayerEffectiveCombatStatsLikeCpp};
use wow_map::{Difficulty, Map};

#[test]
fn derived_stat_bridge_merges_snapshot_without_overwriting_unit_vitals() {
    let mut map = Map::new(571, 7, Difficulty::default(), 60_000);
    let guid = ObjectGuid::create_player(1, 7_701);
    let mut player = Player::new(Some(7), false);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_power_index(PowerType::Mana, Some(2));
    player.unit_mut().set_max_health(77);
    player.unit_mut().set_health(33);
    player.unit_mut().set_create_mana_like_cpp(99);
    player.unit_mut().set_max_power(PowerType::Mana, 80);
    player.unit_mut().set_power(PowerType::Mana, 40);
    player.clear_data_changes();

    let stats = PlayerEffectiveCombatStatsLikeCpp {
        min_damage: 1.25,
        max_damage: 2.75,
        weapon_damage: [[1.25, 2.75], [3.5, 4.5], [5.5, 6.5]],
        base_health: 8,
        max_health: 8_888,
        base_mana: 7_777,
        max_mana: 6_666,
        attack_power: 11,
        attack_power_mod_pos: 12,
        attack_power_mod_neg: -13,
        attack_power_multiplier: 0.25,
        ranged_attack_power: 14,
        ranged_attack_power_mod_pos: 15,
        ranged_attack_power_mod_neg: -16,
        ranged_attack_power_multiplier: 0.5,
        min_ranged_damage: 5.5,
        max_ranged_damage: 6.5,
        stats: [21, 22, 23, 24, 25],
        stat_pos_buff: [31, 32, 33, 34, 35],
        stat_neg_buff: [-41, -42, -43, -44, -45],
        resistances: [51, 52, 53, 54, 55, 56, 57],
        mana_regen: 8.5,
        mana_regen_combat: 3.5,
        mana_regen_mp5: 2.5,
        mainhand_expertise: 1.1,
        offhand_expertise: 1.2,
        ranged_expertise: 1.3,
        combat_rating_expertise: 1.4,
        block_pct: 1.5,
        dodge_pct: 1.6,
        dodge_from_attr: 1.7,
        parry_pct: 1.8,
        parry_from_attr: 1.9,
        crit_pct: 2.0,
        ranged_crit_pct: 2.1,
        offhand_crit_pct: 2.2,
        spell_crit_pct: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7],
        shield_block: 61,
        shield_block_crit_pct: 0.0,
        versatility_bonus: 2.3,
        mod_healing_done_pos: 62,
        mod_healing_done_percent: 2.4,
        override_spell_power_by_ap_percent: 2.5,
        override_ap_by_spell_power_percent: 2.6,
        mod_target_resistance: 63,
        mod_target_physical_resistance: 64,
        mod_damage_done_pos: [71, 72, 73, 74, 75, 76, 77],
        mod_damage_done_neg: [-81, -82, -83, -84, -85, -86, -87],
        mod_damage_done_percent: [1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7],
        combat_ratings: [91; 32],
        ..Default::default()
    };
    player.replace_effective_combat_stats_for_values(stats);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let first = map.send_object_updates_like_cpp();
    assert_eq!(first.queued_before, 1);
    assert_eq!(first.processed, 1);
    assert_eq!(first.cleared_update_masks, 1);
    assert_eq!(first.player_values_updates.len(), 1);
    let captured = first.player_values_updates.into_iter().next().unwrap();
    assert_eq!(captured.guid, guid);

    let packet = player_values_update_to_packet(&captured.values_update).unwrap();
    let encoded_before = player_values_update_to_update_object(guid, 571, &captured.values_update)
        .unwrap()
        .to_bytes();
    let unit = packet.unit_data.as_ref().unwrap();
    assert_eq!(unit.min_damage, 1.25);
    assert_eq!(unit.max_damage, 2.75);
    assert_eq!(unit.min_off_hand_damage, 3.5);
    assert_eq!(unit.max_off_hand_damage, 4.5);
    assert_eq!(unit.base_health, 8);
    assert_eq!(unit.attack_power, 11);
    assert_eq!(unit.attack_power_mod_pos, 12);
    assert_eq!(unit.attack_power_mod_neg, -13);
    assert_eq!(unit.attack_power_multiplier, 0.25);
    assert_eq!(unit.ranged_attack_power, 14);
    assert_eq!(unit.ranged_attack_power_mod_pos, 15);
    assert_eq!(unit.ranged_attack_power_mod_neg, -16);
    assert_eq!(unit.ranged_attack_power_multiplier, 0.5);
    assert_eq!(unit.min_ranged_damage, 5.5);
    assert_eq!(unit.max_ranged_damage, 6.5);
    assert_eq!(unit.stats, [21, 22, 23, 24, 25]);
    assert_eq!(unit.stat_pos_buff, [31, 32, 33, 34, 35]);
    assert_eq!(unit.stat_neg_buff, [-41, -42, -43, -44, -45]);
    assert_eq!(unit.resistances, [51, 52, 53, 54, 55, 56, 57]);
    assert_eq!(unit.power_regen_flat_modifier[2], 8.5);
    assert_eq!(unit.power_regen_interrupted_flat_modifier[2], 3.5);
    assert_eq!(unit.mod_power_regen[2], 0.0);
    assert_eq!(unit.health, 33);
    assert_eq!(unit.max_health, 77);
    assert_eq!(unit.base_mana, 99);
    assert_eq!(unit.power[2], 40);
    assert_eq!(unit.max_power[2], 80);
    assert!(!mask_has(&unit.unit_data_mask, 5));
    assert!(!mask_has(&unit.unit_data_mask, 6));
    assert!(!mask_has(&unit.unit_data_mask, 75));
    assert!(mask_has(&unit.unit_data_mask, 76));
    assert!(mask_has(&unit.unit_data_mask, 119));
    assert!(mask_has(&unit.unit_data_mask, 129));
    assert!(!mask_has(&unit.unit_data_mask, 159));

    let active = packet.active_player_data.as_ref().unwrap();
    assert_eq!(active.mainhand_expertise, 1.1);
    assert_eq!(active.offhand_expertise, 1.2);
    assert_eq!(active.ranged_expertise, 1.3);
    assert_eq!(active.combat_rating_expertise, 1.4);
    assert_eq!(active.block_percentage, 1.5);
    assert_eq!(active.dodge_percentage, 1.6);
    assert_eq!(active.dodge_percentage_from_attribute, 1.7);
    assert_eq!(active.parry_percentage, 1.8);
    assert_eq!(active.parry_percentage_from_attribute, 1.9);
    assert_eq!(active.crit_percentage, 2.0);
    assert_eq!(active.ranged_crit_percentage, 2.1);
    assert_eq!(active.offhand_crit_percentage, 2.2);
    assert_eq!(active.shield_block, 61);
    assert_eq!(active.versatility_bonus, 2.3);
    assert_eq!(active.mod_healing_done_pos, 62);
    assert_eq!(active.mod_healing_done_percent, 2.4);
    assert_eq!(active.override_spell_power_by_ap_percent, 2.5);
    assert_eq!(active.override_ap_by_spell_power_percent, 2.6);
    assert_eq!(active.mod_target_resistance, 63);
    assert_eq!(active.mod_target_physical_resistance, 64);
    assert_eq!(active.spell_crit_percentage, stats.spell_crit_pct);
    assert_eq!(active.mod_damage_done_pos, stats.mod_damage_done_pos);
    assert_eq!(active.mod_damage_done_neg, stats.mod_damage_done_neg);
    assert_eq!(
        active.mod_damage_done_percent,
        stats.mod_damage_done_percent
    );
    assert_eq!(active.combat_ratings, [91; 32]);
    assert!(!mask_has(&active.active_player_data_mask, 50));
    assert!(!mask_has(&active.active_player_data_mask, 69));

    let second = map.send_object_updates_like_cpp();
    assert_eq!(second.queued_before, 0);
    assert!(second.player_values_updates.is_empty());

    let live = map.get_typed_player_mut(guid).unwrap();
    live.unit_mut().set_power_index(PowerType::Mana, Some(3));
    live.unit_mut().set_health(12);
    let mut later = *live.effective_combat_stats_like_cpp();
    later.stats = [91, 92, 93, 94, 95];
    later.mana_regen = 18.5;
    live.replace_effective_combat_stats_like_cpp(later);
    assert_eq!(
        player_values_update_to_update_object(guid, 571, &captured.values_update)
            .unwrap()
            .to_bytes(),
        encoded_before
    );
}
