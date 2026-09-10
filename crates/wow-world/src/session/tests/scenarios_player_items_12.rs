//! Session scenarios exercising the represented player items responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

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
