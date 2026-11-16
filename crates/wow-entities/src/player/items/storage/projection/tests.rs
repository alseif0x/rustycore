use super::InventoryPlayerSnapshotLikeCpp;
use crate::AURA_STATE_NONE;
use crate::player::{
    INVENTORY_SLOT_BAG_START, Player, PlayerEffectiveCombatStatsLikeCpp, PlayerValuesUpdate,
    VisibleItemValues,
};
use crate::unit::{MAX_POWERS_PER_CLASS, UNIT_DATA_LEVEL_BIT, UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT};
use crate::update_fields::{TYPEID_OBJECT, TYPEID_UNIT};
use wow_constants::PowerType;
use wow_core::ObjectGuid;

fn seeded_player() -> Player {
    let mut player = Player::new(Some(11), false);
    let owner_guid = ObjectGuid::create_player(11, 0xC8A);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(owner_guid);
    player.unit_mut().world_mut().object_mut().set_scale(1.25);
    player.unit_mut().set_level(47);
    player.set_power_index(PowerType::Mana, Some(3));
    player.unit_mut().set_max_power(PowerType::Mana, 140);
    player.unit_mut().set_power(PowerType::Mana, 87);
    player.set_money(45_000);
    player.set_bank_bag_slot_count(4);
    player.set_bank_bag_slot_flag_value_like_cpp(2, 0x40);
    player.set_inventory_slot_count(16);
    player.set_visible_item_slot(
        2,
        Some(VisibleItemValues {
            item_id: 19_019,
            item_appearance_mod_id: 7,
            item_visual: 8,
        }),
    );
    let item_guid = ObjectGuid::new(0, 0xCAFE);
    player.set_inv_slot(38, item_guid);
    player.set_buyback_price(1, 700);
    player.set_buyback_timestamp(1, 1_735_000_000);
    player.set_player_flag(0x200);
    player.set_quest_slot_like_cpp(4, 702);
    player.set_quest_slot_counter_like_cpp(4, 2, 9);
    player.set_rest_info_like_cpp(0, 120, 2);
    player.set_rest_info_like_cpp(1, 240, 3);
    player.set_player_local_flags_like_cpp(0x80);
    for spell_id in 0..34 {
        player.add_self_res_spell(spell_id);
    }
    player.add_toy_like_cpp(44_000);
    player
        .inventory
        .register_bag_storage(INVENTORY_SLOT_BAG_START, ObjectGuid::new(0, 0xBABA), 10)
        .unwrap();
    player
        .inventory
        .store_bag_item(INVENTORY_SLOT_BAG_START, 3, item_guid)
        .unwrap();
    player.active_data.transmog_update_mask = Some(vec![0x10, 0x20]);
    player.active_player_data_changes.set(603);
    player.rest_info_change_masks = [0b01, 0b10];
    let mut stats = PlayerEffectiveCombatStatsLikeCpp::default();
    stats.base_health = 1_234;
    stats.max_mana = 5_678;
    player.effective_combat_stats = stats;
    player.can_titan_grip = true;
    player.unit_mut().set_virtual_item(
        1,
        Some(VisibleItemValues {
            item_id: 20_100,
            item_appearance_mod_id: 4,
            item_visual: 5,
        }),
    );
    player
}

fn assert_same_projection(
    snapshot: &InventoryPlayerSnapshotLikeCpp,
    player: &Player,
) -> (PlayerValuesUpdate, PlayerValuesUpdate) {
    let without_active_snapshot = snapshot.values_update(false);
    let without_active_player = player.values_update(false);
    assert_eq!(without_active_snapshot, without_active_player);
    let with_active_snapshot = snapshot.values_update(true);
    let with_active_player = player.values_update(true);
    assert_eq!(with_active_snapshot, with_active_player);
    (with_active_snapshot, with_active_player)
}

#[test]
fn inventory_player_snapshot_captures_topology_and_freezes_publication() {
    let owner = seeded_player();
    let canonical = seeded_player();
    let snapshot = InventoryPlayerSnapshotLikeCpp::capture(&owner);

    assert_eq!(snapshot.inventory.as_ref(), owner.inventory.as_ref());
    assert_eq!(snapshot.owner_guid, owner.guid());
    assert!(snapshot.can_titan_grip);
    let (captured, expected) = assert_same_projection(&snapshot, &canonical);
    assert!(captured.player_data.is_some());
    assert!(captured.active_player_data.is_some());
    assert_eq!(captured.unit_data.as_ref().unwrap().values.level, 47);
    assert!(
        captured
            .unit_data
            .as_ref()
            .unwrap()
            .mask
            .is_set(UNIT_DATA_LEVEL_BIT)
    );
    assert_eq!(
        captured.active_player_data.as_ref().unwrap().local_flags,
        0x80
    );
    assert_eq!(
        captured
            .active_player_data
            .as_ref()
            .unwrap()
            .rest_info_change_masks,
        [1, 2]
    );
    assert_eq!(
        captured.derived_stats.as_ref().unwrap().mana_power_index,
        Some(3)
    );
    assert_eq!(captured, expected);

    let before_inactive = snapshot.values_update(false);
    owner_after_capture_mutation(owner);
    assert_eq!(snapshot.values_update(false), before_inactive);
    assert_eq!(snapshot.values_update(true), captured);
}

fn owner_after_capture_mutation(mut owner: Player) {
    owner.set_money(99);
    owner.set_visible_item_slot(2, None);
    owner.unit_mut().set_level(60);
    owner.clear_data_changes();
}

#[test]
fn inventory_player_snapshot_public_mutations_and_clear_match_player() {
    let mut player = seeded_player();
    let captured_owner = seeded_player();
    let mut snapshot = InventoryPlayerSnapshotLikeCpp::capture(&captured_owner);
    assert_same_projection(&snapshot, &player);

    snapshot.clear_data_changes();
    player.clear_data_changes();
    assert_same_projection(&snapshot, &player);
    assert_eq!(snapshot.quest_log_change_masks, [0; 25]);
    assert_eq!(snapshot.rest_info_change_masks, [0; 2]);
    assert_eq!(
        snapshot.active_player_data.self_res_spells_update_mask,
        Some(vec![0, 0])
    );
    assert_eq!(snapshot.active_player_data.toys_update_mask, Some(vec![1]));
    assert_eq!(
        snapshot.active_player_data.transmog_update_mask,
        Some(vec![0x10, 0x20])
    );

    let unchanged_money = player.active_data.coinage;
    snapshot.set_money(unchanged_money);
    player.set_money(unchanged_money);
    let unchanged_visible = player.data.visible_items[2];
    snapshot.set_visible_item_slot(2, Some(unchanged_visible));
    player.set_visible_item_slot(2, Some(unchanged_visible));
    let unchanged_inv_slot = player.active_data.inv_slots[38];
    snapshot.set_inv_slot(38, unchanged_inv_slot);
    player.set_inv_slot(38, unchanged_inv_slot);
    let unchanged_buyback_price = player.active_data.buyback_price[1];
    snapshot.set_buyback_price(1, unchanged_buyback_price);
    player.set_buyback_price(1, unchanged_buyback_price);
    let unchanged_buyback_timestamp = player.active_data.buyback_timestamp[1];
    snapshot.set_buyback_timestamp(1, unchanged_buyback_timestamp);
    player.set_buyback_timestamp(1, unchanged_buyback_timestamp);
    snapshot.set_bank_bag_slot_count(player.data.num_bank_slots);
    player.set_bank_bag_slot_count(player.data.num_bank_slots);
    snapshot.set_inventory_slot_count(player.active_data.num_backpack_slots);
    player.set_inventory_slot_count(player.active_data.num_backpack_slots);
    assert!(snapshot.set_bank_bag_slot_flag_value_like_cpp(2, 0x40));
    assert!(player.set_bank_bag_slot_flag_value_like_cpp(2, 0x40));
    assert!(!snapshot.set_bank_bag_slot_flag_value_like_cpp(7, 1));
    assert!(!player.set_bank_bag_slot_flag_value_like_cpp(7, 1));
    assert_same_projection(&snapshot, &player);

    snapshot.mark_money_changed();
    player.mark_money_changed();
    snapshot.mark_bank_bag_slot_count_changed_like_cpp();
    player.mark_bank_bag_slot_count_changed_like_cpp();
    snapshot.mark_bank_bag_slot_flag_changed_like_cpp(2);
    player.mark_bank_bag_slot_flag_changed_like_cpp(2);
    snapshot.mark_visible_item_slot_changed(2);
    player.mark_visible_item_slot_changed(2);
    snapshot.mark_inv_slot_changed(38);
    player.mark_inv_slot_changed(38);
    snapshot.mark_buyback_price_changed(1);
    player.mark_buyback_price_changed(1);
    snapshot.mark_buyback_timestamp_changed(1);
    player.mark_buyback_timestamp_changed(1);
    snapshot.set_money(90_000);
    player.set_money(90_000);
    snapshot.set_bank_bag_slot_count(5);
    player.set_bank_bag_slot_count(5);
    snapshot.set_inventory_slot_count(18);
    player.set_inventory_slot_count(18);
    snapshot.set_bank_bag_slot_flag_value_like_cpp(3, 0x80);
    player.set_bank_bag_slot_flag_value_like_cpp(3, 0x80);
    snapshot.set_visible_item_slot(
        3,
        Some(VisibleItemValues {
            item_id: 20_200,
            item_appearance_mod_id: 1,
            item_visual: 2,
        }),
    );
    player.set_visible_item_slot(
        3,
        Some(VisibleItemValues {
            item_id: 20_200,
            item_appearance_mod_id: 1,
            item_visual: 2,
        }),
    );
    let replacement_guid = ObjectGuid::new(0, 0x1234);
    snapshot.set_inv_slot(39, replacement_guid);
    player.set_inv_slot(39, replacement_guid);
    snapshot.set_buyback_price(2, 1_500);
    player.set_buyback_price(2, 1_500);
    snapshot.set_buyback_timestamp(2, 1_735_000_010);
    player.set_buyback_timestamp(2, 1_735_000_010);
    snapshot.replace_all_player_flags(0x400);
    player.replace_all_player_flags(0x400);
    snapshot.set_player_flag(0x20);
    player.set_player_flag(0x20);
    snapshot.remove_player_flag(0x400);
    player.remove_player_flag(0x400);
    snapshot.set_visible_item_slot(u8::MAX, None);
    player.set_visible_item_slot(u8::MAX, None);
    snapshot.set_inv_slot(usize::MAX, replacement_guid);
    player.set_inv_slot(usize::MAX, replacement_guid);
    snapshot.mark_visible_item_slot_changed(u8::MAX);
    player.mark_visible_item_slot_changed(u8::MAX);
    snapshot.mark_inv_slot_changed(usize::MAX);
    player.mark_inv_slot_changed(usize::MAX);
    snapshot.set_buyback_price(usize::MAX, 1);
    player.set_buyback_price(usize::MAX, 1);
    snapshot.set_buyback_timestamp(usize::MAX, 1);
    player.set_buyback_timestamp(usize::MAX, 1);
    snapshot.set_virtual_item(
        2,
        Some(VisibleItemValues {
            item_id: 20_300,
            item_appearance_mod_id: 5,
            item_visual: 6,
        }),
    );
    player.unit_mut().set_virtual_item(
        2,
        Some(VisibleItemValues {
            item_id: 20_300,
            item_appearance_mod_id: 5,
            item_visual: 6,
        }),
    );
    snapshot.mark_virtual_item_changed(2);
    player.unit_mut().mark_virtual_item_changed(2);
    snapshot.set_virtual_item(usize::MAX, None);
    player.unit_mut().set_virtual_item(usize::MAX, None);
    snapshot.mark_virtual_item_changed(usize::MAX);
    player.unit_mut().mark_virtual_item_changed(usize::MAX);

    assert_same_projection(&snapshot, &player);

    let empty_owner = Player::new(None, false);
    let mut empty_snapshot = InventoryPlayerSnapshotLikeCpp::capture(&empty_owner);
    let mut empty_canonical = Player::new(None, false);
    empty_snapshot.clear_data_changes();
    empty_canonical.clear_data_changes();
    assert_same_projection(&empty_snapshot, &empty_canonical);
    assert_eq!(
        empty_snapshot
            .active_player_data
            .self_res_spells_update_mask,
        Some(vec![])
    );
}

#[test]
fn inventory_player_snapshot_freezes_filtered_cached_mana_index() {
    for (configured_index, expected_index) in [
        (None, None),
        (Some(3), Some(3)),
        (Some(MAX_POWERS_PER_CLASS), None),
    ] {
        let mut owner = seeded_player();
        owner.set_power_index(PowerType::Mana, configured_index);
        let mut canonical = seeded_player();
        canonical.set_power_index(PowerType::Mana, configured_index);
        let snapshot = InventoryPlayerSnapshotLikeCpp::capture(&owner);

        assert_same_projection(&snapshot, &canonical);
        assert_eq!(owner.get_power_index(PowerType::Mana), expected_index);
        assert_eq!(
            snapshot
                .values_update(true)
                .derived_stats
                .as_ref()
                .unwrap()
                .mana_power_index,
            expected_index
        );
    }

    let mut owner = seeded_player();
    let canonical = seeded_player();
    let snapshot = InventoryPlayerSnapshotLikeCpp::capture(&owner);
    assert_same_projection(&snapshot, &canonical);
    let frozen_derived_stats = snapshot.values_update(true).derived_stats;

    owner.set_power_index(PowerType::Mana, None);

    assert_eq!(
        snapshot.values_update(true).derived_stats,
        frozen_derived_stats
    );
    assert_ne!(
        owner.values_update(true).derived_stats,
        frozen_derived_stats
    );
}

#[cfg(debug_assertions)]
#[test]
fn inventory_player_snapshot_defers_invalid_empty_aura_state_until_unit_projection() {
    let mut player = Player::new(None, false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::new(0, 0xC8A));
    player.unit_mut().world_mut().object_mut().set_scale(1.5);
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .aura_state_auras
        .insert(AURA_STATE_NONE, Vec::new());
    player.unit_mut().clear_unit_data_changes();

    let mut snapshot = InventoryPlayerSnapshotLikeCpp::capture(&player);
    let object_only = snapshot.values_update(false);
    assert_eq!(object_only.changed_object_type_mask, 1 << TYPEID_OBJECT);
    assert!(object_only.object_data.is_some());
    assert!(object_only.unit_data.is_none());

    snapshot.set_virtual_item(
        0,
        Some(VisibleItemValues {
            item_id: 19_019,
            item_appearance_mod_id: 2,
            item_visual: 3,
        }),
    );
    assert!(
        snapshot
            .unit
            .unit_data_changes_mask()
            .is_set(UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT)
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            snapshot.values_update(false)
        }))
        .is_err()
    );
    assert_eq!(object_only.changed_object_type_mask & (1 << TYPEID_UNIT), 0);
}
