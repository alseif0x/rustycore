use super::*;
use crate::entity_update_bridge::{
    player_values_update_to_packet, player_values_update_to_update_object,
};
use crate::session::SessionCore;
use std::sync::{Arc, Mutex};
use wow_constants::PowerType;
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    Player, PlayerEffectiveCombatStatsLikeCpp, PlayerValuesUpdate, VisibleItemValues,
};
use wow_map::{MapKey, MapManager, PlayerHandle};
use wow_packet::ServerPacket;

const MAP: MapKey = MapKey::new(571, 0);
type Manager = Arc<Mutex<MapManager>>;

fn seeded_player() -> Player {
    let mut player = Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, 90_001));
    player.unit_mut().world_mut().object_mut().set_scale(1.25);
    player.unit_mut().set_level(47);
    player.set_power_index(PowerType::Mana, Some(3));
    player.unit_mut().set_max_power(PowerType::Mana, 140);
    player.unit_mut().set_power(PowerType::Mana, 87);
    player.set_money(45_000);
    player.set_bank_bag_slot_count(4);
    assert!(player.set_bank_bag_slot_flag_value_like_cpp(2, 0x40));
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
        let _ = player.add_self_res_spell(spell_id);
    }
    player.add_toy_like_cpp(44_000);
    player
        .register_bag_storage(
            wow_entities::INVENTORY_SLOT_BAG_START,
            ObjectGuid::new(0, 0xBABA),
            10,
        )
        .unwrap();
    player
        .store_bag_item(wow_entities::INVENTORY_SLOT_BAG_START, 3, item_guid)
        .unwrap();
    player.unit_mut().set_virtual_item(
        1,
        Some(VisibleItemValues {
            item_id: 20_100,
            item_appearance_mod_id: 4,
            item_visual: 5,
        }),
    );
    let mut stats = PlayerEffectiveCombatStatsLikeCpp::default();
    stats.base_health = 1_234;
    stats.max_mana = 5_678;
    player.replace_effective_combat_stats_for_values(stats);
    player.set_can_titan_grip(true, 0);
    player
}

fn session_core(
    guid: ObjectGuid,
    handle: Option<PlayerHandle>,
    manager: Option<Manager>,
) -> SessionCore {
    let (_input, packet_rx) = flume::bounded(1);
    let (send_tx, _send_rx) = flume::unbounded();
    let (command_tx, command_rx) = flume::bounded(1);
    let connection = wow_session::SessionConnection::new(send_tx, packet_rx);
    let mut core = SessionCore::new_for_world_session_like_cpp(
        1,
        "InventoryProjectionTest".into(),
        0,
        2,
        2,
        54_261,
        "enUS".into(),
        command_tx,
        command_rx,
        connection,
        Vec::new(),
    );
    core.player_guid = Some(guid);
    core.player_handle_like_cpp = handle;
    core.canonical_map_manager = manager;
    core.current_map_id = MAP.map_id as u16;
    core
}

fn owned_fixture(player: Player, active: bool) -> (SessionCore, Manager, PlayerHandle) {
    let guid = player.guid();
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    if active {
        manager.create_world_map(MAP.map_id, MAP.instance_id);
    }
    let handle = manager
        .install_detached_player_like_cpp(Box::new(player))
        .expect("fixture Player should install under map ownership");
    if active {
        manager
            .attach_player_like_cpp(handle, MAP, Position::ZERO)
            .expect("fixture Player should attach to the map");
    }
    let manager = Arc::new(Mutex::new(manager));
    let core = session_core(guid, Some(handle), Some(Arc::clone(&manager)));
    (core, manager, handle)
}

fn update_bytes(update: &PlayerValuesUpdate, guid: ObjectGuid) -> Option<Vec<u8>> {
    player_values_update_to_update_object(guid, MAP.map_id as u16, update)
        .map(|update_object| update_object.to_bytes())
}

fn assert_matches_player(
    projection: &InventoryPlayerProjectionLikeCpp,
    player: &Player,
    guid: ObjectGuid,
) {
    for include_active in [false, true] {
        let projected = projection.values_update_like_cpp(include_active);
        let canonical = player.values_update(include_active);
        assert_eq!(projected, canonical, "include_active={include_active}");
        assert_eq!(
            update_bytes(&projected, guid),
            update_bytes(&canonical, guid),
            "encoded update include_active={include_active}"
        );
    }
}

#[test]
fn inventory_player_projection_captures_and_mutates_frozen_publication_values() {
    let canonical = seeded_player();
    let guid = canonical.guid();
    let (core, manager, handle) = owned_fixture(seeded_player(), false);
    let mut projection = core
        .owned_inventory_access_like_cpp()
        .inventory_player_projection_snapshot_like_cpp()
        .expect("current detached owner should be captured");

    assert_matches_player(&projection, &canonical, guid);
    projection.clear_data_changes_like_cpp();
    let mut canonical = canonical;
    canonical.clear_data_changes();
    assert_matches_player(&projection, &canonical, guid);

    let existing_money = canonical.money();
    projection.set_money_like_cpp(existing_money);
    canonical.set_money(existing_money);
    projection.mark_money_changed_like_cpp();
    canonical.mark_money_changed();
    projection.set_inventory_slot_count_like_cpp(16);
    canonical.set_inventory_slot_count(16);
    projection.set_bank_bag_slot_count_like_cpp(4);
    canonical.set_bank_bag_slot_count(4);
    projection.mark_bank_bag_slot_count_changed_like_cpp();
    canonical.mark_bank_bag_slot_count_changed_like_cpp();
    projection.set_inventory_slot_count_like_cpp(18);
    canonical.set_inventory_slot_count(18);
    projection.set_bank_bag_slot_count_like_cpp(5);
    canonical.set_bank_bag_slot_count(5);
    assert!(projection.set_bank_bag_slot_flag_value_like_cpp(2, 0x40));
    assert!(canonical.set_bank_bag_slot_flag_value_like_cpp(2, 0x40));
    assert!(!projection.set_bank_bag_slot_flag_value_like_cpp(9, 1));
    assert!(!canonical.set_bank_bag_slot_flag_value_like_cpp(9, 1));
    projection.mark_bank_bag_slot_flag_changed_like_cpp(2);
    canonical.mark_bank_bag_slot_flag_changed_like_cpp(2);
    assert!(projection.set_bank_bag_slot_flag_value_like_cpp(3, 0x80));
    assert!(canonical.set_bank_bag_slot_flag_value_like_cpp(3, 0x80));
    projection.set_visible_item_slot_like_cpp(
        2,
        Some(VisibleItemValues {
            item_id: 19_019,
            item_appearance_mod_id: 7,
            item_visual: 8,
        }),
    );
    canonical.set_visible_item_slot(
        2,
        Some(VisibleItemValues {
            item_id: 19_019,
            item_appearance_mod_id: 7,
            item_visual: 8,
        }),
    );
    projection.mark_visible_item_slot_changed_like_cpp(2);
    canonical.mark_visible_item_slot_changed(2);
    projection.set_player_visible_item_values_like_cpp(3, (-7, 5, 6));
    crate::canonical_player_access::set_player_visible_item_values_like_cpp(
        &mut canonical,
        3,
        (-7, 5, 6),
    );
    assert_eq!(
        projection
            .values_update_like_cpp(true)
            .player_data
            .unwrap()
            .values
            .visible_items[3]
            .item_id,
        -7
    );
    assert_eq!(
        canonical
            .values_update(true)
            .player_data
            .unwrap()
            .values
            .visible_items[3]
            .item_id,
        -7
    );
    projection.set_player_visible_item_values_like_cpp(3, (0, 0, 0));
    crate::canonical_player_access::set_player_visible_item_values_like_cpp(
        &mut canonical,
        3,
        (0, 0, 0),
    );
    projection.set_player_visible_item_values_like_cpp(4, (20_201, 2, 3));
    crate::canonical_player_access::set_player_visible_item_values_like_cpp(
        &mut canonical,
        4,
        (20_201, 2, 3),
    );
    let item_guid = ObjectGuid::new(0, 0xD00D);
    projection.set_inv_slot_like_cpp(39, item_guid);
    canonical.set_inv_slot(39, item_guid);
    projection.mark_inv_slot_changed_like_cpp(38);
    canonical.mark_inv_slot_changed(38);
    projection.set_buyback_price_like_cpp(2, 1_500);
    canonical.set_buyback_price(2, 1_500);
    projection.mark_buyback_price_changed_like_cpp(1);
    canonical.mark_buyback_price_changed(1);
    projection.set_buyback_timestamp_like_cpp(2, 1_735_000_010);
    canonical.set_buyback_timestamp(2, 1_735_000_010);
    projection.mark_buyback_timestamp_changed_like_cpp(1);
    canonical.mark_buyback_timestamp_changed(1);
    projection.set_money_like_cpp(90_000);
    canonical.set_money(90_000);
    projection.replace_all_player_flags_like_cpp(0x400);
    canonical.replace_all_player_flags(0x400);
    projection.set_player_flag_like_cpp(0x20);
    canonical.set_player_flag(0x20);
    projection.remove_player_flag_like_cpp(0x400);
    canonical.remove_player_flag(0x400);
    let visible = VisibleItemValues {
        item_id: 20_300,
        item_appearance_mod_id: 5,
        item_visual: 6,
    };
    projection.set_virtual_item_like_cpp(2, Some(visible));
    canonical.unit_mut().set_virtual_item(2, Some(visible));
    projection.mark_virtual_item_changed_like_cpp(2);
    canonical.unit_mut().mark_virtual_item_changed(2);
    projection.set_can_titan_grip_like_cpp(false);
    canonical.set_can_titan_grip(false, 0);

    assert_matches_player(&projection, &canonical, guid);
    let frozen: Vec<_> = [false, true]
        .into_iter()
        .map(|include_active| {
            let update = projection.values_update_like_cpp(include_active);
            (
                update,
                update_bytes(&projection.values_update_like_cpp(include_active), guid),
            )
        })
        .collect();
    manager
        .lock()
        .unwrap()
        .with_player_mut_like_cpp(handle, |owner| {
            owner.set_money(1);
            owner.set_visible_item_slot(2, None);
            owner.unit_mut().set_level(60);
            owner.clear_data_changes();
        })
        .expect("captured owner should remain installed");
    for (index, include_active) in [false, true].into_iter().enumerate() {
        let update = projection.values_update_like_cpp(include_active);
        assert_eq!(update, frozen[index].0);
        assert_eq!(update_bytes(&update, guid), frozen[index].1);
    }
}

#[test]
fn inventory_player_projection_preserves_raw_float_bits_in_update_bytes() {
    let nan_payload = f32::from_bits(0x7fc0_1234);
    let negative_zero = f32::from_bits(0x8000_0000);
    let mut owner = Player::new(Some(1), false);
    let guid = ObjectGuid::create_player(1, 90_002);
    owner.unit_mut().world_mut().object_mut().create(guid);
    owner
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_scale(nan_payload);
    let mut initial_stats = PlayerEffectiveCombatStatsLikeCpp {
        versatility_bonus: 1.0,
        ..PlayerEffectiveCombatStatsLikeCpp::default()
    };
    owner.replace_effective_combat_stats_for_values(initial_stats);
    let mut stats = PlayerEffectiveCombatStatsLikeCpp::default();
    stats.attack_power_multiplier = nan_payload;
    stats.versatility_bonus = negative_zero;
    owner.replace_effective_combat_stats_for_values(stats);
    let mut canonical = Player::new(Some(1), false);
    canonical.unit_mut().world_mut().object_mut().create(guid);
    canonical
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_scale(nan_payload);
    initial_stats.versatility_bonus = 1.0;
    canonical.replace_effective_combat_stats_for_values(initial_stats);
    canonical.replace_effective_combat_stats_for_values(stats);
    let (core, _manager, _handle) = owned_fixture(owner, false);
    let projection = core
        .owned_inventory_access_like_cpp()
        .inventory_player_projection_snapshot_like_cpp()
        .expect("current raw-float owner should be captured");

    let projected_update = projection.values_update_like_cpp(true);
    let canonical_update = canonical.values_update(true);
    assert_raw_float_fields(&projected_update, nan_payload, negative_zero);
    assert_raw_float_fields(&canonical_update, nan_payload, negative_zero);
    let projected = update_bytes(&projected_update, guid);
    let expected = update_bytes(&canonical_update, guid);
    assert_eq!(projected, expected);
    assert!(projected.is_some());
}

fn assert_raw_float_fields(update: &PlayerValuesUpdate, nan_payload: f32, negative_zero: f32) {
    let derived = update
        .derived_stats
        .as_ref()
        .unwrap()
        .effective_combat_stats;
    assert_eq!(
        derived.attack_power_multiplier.to_bits(),
        nan_payload.to_bits()
    );
    assert_eq!(derived.versatility_bonus.to_bits(), negative_zero.to_bits());
    assert_eq!(
        update.object_data.as_ref().unwrap().values.scale.to_bits(),
        nan_payload.to_bits()
    );

    let packet = player_values_update_to_packet(update).unwrap();
    assert_eq!(
        packet
            .unit_data
            .as_ref()
            .unwrap()
            .attack_power_multiplier
            .to_bits(),
        nan_payload.to_bits()
    );
    assert_eq!(
        packet
            .active_player_data
            .as_ref()
            .unwrap()
            .versatility_bonus
            .to_bits(),
        negative_zero.to_bits()
    );
    assert_eq!(
        packet.object_data.as_ref().unwrap().scale.to_bits(),
        nan_payload.to_bits()
    );
}

#[test]
fn inventory_player_projection_uses_only_the_current_generation_checked_owner() {
    for active in [false, true] {
        let player = seeded_player();
        let guid = player.guid();
        let (core, manager, handle) = owned_fixture(player, active);
        let projection = core
            .owned_inventory_access_like_cpp()
            .inventory_player_projection_snapshot_like_cpp()
            .expect("current active or detached owner should resolve");
        assert!(
            projection
                .values_update_like_cpp(true)
                .player_data
                .is_some()
        );
        assert!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |owner| owner.guid())
                .is_some()
        );
        assert_eq!(guid, handle.guid());
    }

    let player = seeded_player();
    let guid = player.guid();
    let (core, manager, handle) = owned_fixture(player, false);
    assert!(
        session_core(guid, None, Some(Arc::clone(&manager)))
            .owned_inventory_access_like_cpp()
            .inventory_player_projection_snapshot_like_cpp()
            .is_none()
    );
    assert!(
        session_core(guid, Some(handle), None)
            .owned_inventory_access_like_cpp()
            .inventory_player_projection_snapshot_like_cpp()
            .is_none()
    );

    manager
        .lock()
        .unwrap()
        .retire_player_like_cpp(handle)
        .expect("old owner should retire");
    let mut replacement = Player::new(Some(1), false);
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.set_money(999);
    let replacement_handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(Box::new(replacement))
        .expect("same-guid replacement should install with a new generation");
    assert_ne!(replacement_handle, handle);
    assert!(
        core.owned_inventory_access_like_cpp()
            .inventory_player_projection_snapshot_like_cpp()
            .is_none()
    );
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |owner| owner.money()),
        Some(999)
    );
}

#[cfg(debug_assertions)]
#[test]
fn inventory_player_projection_defers_invalid_aura_projection_until_after_owned_read() {
    let mut player = Player::new(None, false);
    let guid = ObjectGuid::create_player(1, 90_003);
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().world_mut().object_mut().set_scale(1.5);
    player
        .unit_mut()
        .subsystems_mut()
        .auras
        .aura_state_auras
        .insert(wow_entities::AURA_STATE_NONE, Vec::new());
    player.unit_mut().clear_unit_data_changes();
    let (core, manager, handle) = owned_fixture(player, false);
    let mut projection = core
        .owned_inventory_access_like_cpp()
        .inventory_player_projection_snapshot_like_cpp()
        .expect("invalid raw aura row is captured as data");

    assert!(
        manager.try_lock().is_ok(),
        "owned read must release manager lock"
    );
    let object_only = projection.values_update_like_cpp(false);
    assert_eq!(
        object_only.changed_object_type_mask,
        1 << wow_entities::TYPEID_OBJECT
    );
    assert!(object_only.object_data.is_some());
    assert!(object_only.unit_data.is_none());
    projection.mark_virtual_item_changed_like_cpp(0);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            projection.values_update_like_cpp(false)
        }))
        .is_err()
    );
    assert!(
        !manager.is_poisoned(),
        "deferred panic must occur outside manager lock"
    );
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |owner| owner.guid()),
        Some(guid)
    );
}
