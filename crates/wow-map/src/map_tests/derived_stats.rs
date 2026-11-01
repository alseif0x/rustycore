use super::*;
use wow_constants::PowerType;
use wow_entities::PlayerEffectiveCombatStatsLikeCpp;

#[test]
fn derived_stat_map_snapshot_is_captured_once_and_keeps_frozen_values() {
    let mut map = test_map();
    let guid = ObjectGuid::create_player(1, 4_450_191);
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
    player.replace_effective_combat_stats_for_values(PlayerEffectiveCombatStatsLikeCpp {
        base_health: 8,
        max_health: 777,
        base_mana: 666,
        max_mana: 555,
        stats: [21, 22, 23, 24, 25],
        mana_regen: 8.5,
        mana_regen_combat: 3.5,
        mana_regen_mp5: 2.5,
        mainhand_expertise: 1.25,
        ..Default::default()
    });
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let first = map.send_object_updates_like_cpp();
    assert_eq!(first.queued_before, 1);
    assert_eq!(first.processed, 1);
    assert_eq!(first.cleared_update_masks, 1);
    assert_eq!(first.player_values_updates.len(), 1);
    let captured = first.player_values_updates.into_iter().next().unwrap();
    assert_eq!(captured.guid, guid);
    let derived = captured.values_update.derived_stats.as_ref().unwrap();
    assert_eq!(derived.mana_power_index, Some(2));
    assert_eq!(derived.effective_combat_stats.stats, [21, 22, 23, 24, 25]);
    let captured_unit = captured.values_update.unit_data.as_ref().unwrap();
    assert!(captured_unit.mask.is_set(76));
    assert!(captured_unit.mask.is_set(119));
    assert!(captured_unit.mask.is_set(129));
    assert!(!captured_unit.mask.is_set(5));
    assert!(!captured_unit.mask.is_set(6));
    assert!(!captured_unit.mask.is_set(75));
    assert_eq!(captured_unit.values.health, 33);
    assert_eq!(captured_unit.values.max_health, 77);
    assert_eq!(captured_unit.values.base_mana, 99);
    assert_eq!(captured_unit.values.power[2], 40);
    assert_eq!(captured_unit.values.max_power[2], 80);

    let record = map.entity_world.get(&guid).unwrap();
    assert!(!record.object().object().is_object_updated());
    assert!(
        !record
            .player()
            .unwrap()
            .unit()
            .unit_data_changes_mask()
            .is_any_set()
    );
    let second = map.send_object_updates_like_cpp();
    assert_eq!(second.queued_before, 0);
    assert!(second.player_values_updates.is_empty());

    let record = map.entity_world.get_mut(&guid).unwrap();
    let live = record.player_mut().unwrap();
    live.unit_mut().set_power_index(PowerType::Mana, Some(3));
    live.unit_mut().set_health(12);
    let mut later = *live.effective_combat_stats_like_cpp();
    later.stats = [91, 92, 93, 94, 95];
    later.mana_regen = 18.5;
    live.replace_effective_combat_stats_like_cpp(later);

    assert_eq!(
        captured
            .values_update
            .unit_data
            .as_ref()
            .unwrap()
            .values
            .health,
        33
    );
    let captured_derived = captured.values_update.derived_stats.as_ref().unwrap();
    assert_eq!(
        captured_derived.effective_combat_stats.stats,
        [21, 22, 23, 24, 25]
    );
    assert_eq!(captured_derived.mana_power_index, Some(2));
}
