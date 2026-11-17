use super::*;

use std::sync::{Arc, Mutex};

use wow_constants::{InventoryType, ItemExtendedCostFlags, ItemFieldFlags};
use wow_core::{ObjectGuid, Position};
use wow_data::{ItemExtendedCostEntry, ItemExtendedCostStore};
use wow_entities::{
    INVENTORY_SLOT_ITEM_START, Item, ItemValuesSnapshot, Player, PlayerInventoryItem,
};
use wow_map::{MIN_GRID_DELAY_MS, MapKey, MapManager, PlayerHandle};
use wow_packet::packets::item::{
    ItemPurchaseContents, ItemPurchaseRefundCurrency, ItemPurchaseRefundItem, SetItemPurchaseData,
};
use wow_packet::{ServerPacket, WorldPacket};

const MAP: MapKey = MapKey::new(571, 0);
const ITEM_ENTRY: u32 = 900;
const EXTENDED_COST_ID: u32 = 91_901;
const CANONICAL_MONEY: u64 = 45_678;
const FIXTURE_MONEY: u64 = 987_654_321;

fn player_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_player(1, counter)
}

fn item_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_item(1, counter)
}

fn inventory_record(guid: ObjectGuid) -> PlayerInventoryItem {
    PlayerInventoryItem {
        guid,
        entry_id: ITEM_ENTRY,
        db_guid: guid.counter() as u64,
        inventory_type: Some(InventoryType::NonEquip as u8),
    }
}

fn purchase_item(
    owner_guid: ObjectGuid,
    guid: ObjectGuid,
    refundable: bool,
    refund_recipient: ObjectGuid,
    create_played_time: u32,
    last_played_time_update: i64,
    money: u64,
) -> Item {
    let mut item = Item::new(900);
    item.object_mut().create(guid);
    item.object_mut().set_entry(ITEM_ENTRY);
    item.set_owner_guid(owner_guid);
    item.set_contained_in(owner_guid);
    item.set_slot(INVENTORY_SLOT_ITEM_START);
    item.set_count(1);
    item.set_create_played_time(create_played_time);
    item.set_last_played_time_update(last_played_time_update);
    item.set_refund_recipient(refund_recipient);
    item.set_paid_money(money);
    item.set_paid_extended_cost(EXTENDED_COST_ID);
    if refundable {
        item.set_item_flag(ItemFieldFlags::REFUNDABLE);
    }
    item
}

fn player_with_item(owner_guid: ObjectGuid, guid: ObjectGuid, item: Item) -> Player {
    let mut player = empty_player(owner_guid);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, guid)
        .expect("purchase item should occupy a valid backpack slot");
    let runtime = player.inventory_runtime_mut_like_cpp();
    runtime.store_item_in_slot_like_cpp(INVENTORY_SLOT_ITEM_START, inventory_record(guid));
    runtime.store_item_object_like_cpp(item);
    player
}

fn empty_player(owner_guid: ObjectGuid) -> Player {
    let mut player = Player::new(Some(80), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(owner_guid);
    player.set_inventory_slot_count(16);
    player
}

fn install_owner(
    session: &mut WorldSession,
    player: Player,
    owner_guid: ObjectGuid,
    active: bool,
) -> (Arc<Mutex<MapManager>>, PlayerHandle) {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 10);
    if active {
        manager.create_world_map(MAP.map_id, MAP.instance_id);
    }
    let handle = manager
        .install_detached_player_like_cpp(Box::new(player))
        .expect("canonical Player should install");
    if active {
        manager
            .attach_player_like_cpp(handle, MAP, Position::ZERO)
            .expect("canonical Player should attach");
    }
    let manager = Arc::new(Mutex::new(manager));
    session.set_player_guid(Some(owner_guid));
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.core.current_map_id = MAP.map_id as u16;
    session.core.player_handle_like_cpp = Some(handle);
    (manager, handle)
}

fn install_extended_cost(session: &mut WorldSession, row_id: u32) {
    session.set_item_extended_cost_store(Arc::new(ItemExtendedCostStore::from_entries([
        ItemExtendedCostEntry {
            id: row_id,
            required_arena_rating: 0,
            arena_bracket: -1,
            flags: ItemExtendedCostFlags::REQUIRE_SEASON_EARNED_3,
            min_faction_id: 0,
            min_reputation: 0,
            required_achievement: 0,
            item_id: [7_001, 7_002, 7_003, 7_004, 7_005],
            item_count: [1, 2, 3, 4, 5],
            currency_id: [101, 102, 103, 104, 105],
            currency_count: [11, 12, 13, 14, 15],
        },
    ])));
}

fn install_poisoned_fixture(session: &mut WorldSession, owner_guid: ObjectGuid, guid: ObjectGuid) {
    session.insert_inventory_item_like_cpp(INVENTORY_SLOT_ITEM_START, inventory_record(guid));
    session.insert_inventory_item_object(purchase_item(
        owner_guid,
        guid,
        true,
        owner_guid,
        10,
        900,
        FIXTURE_MONEY,
    ));
}

fn player_item_facts(player: &Player) -> Vec<(ObjectGuid, ItemValuesSnapshot)> {
    let mut facts = player
        .inventory_runtime_like_cpp()
        .item_objects()
        .iter()
        .map(|(guid, item)| (*guid, ItemValuesSnapshot::capture(item)))
        .collect::<Vec<_>>();
    facts.sort_by_key(|(guid, _)| guid.counter());
    facts
}

fn canonical_item_facts(session: &WorldSession) -> Option<Vec<(ObjectGuid, ItemValuesSnapshot)>> {
    session.core.with_owned_player_like_cpp(player_item_facts)
}

fn item_facts_for_handle(
    manager: &Arc<Mutex<MapManager>>,
    handle: PlayerHandle,
) -> Option<Vec<(ObjectGuid, ItemValuesSnapshot)>> {
    manager
        .lock()
        .expect("canonical map manager should not be poisoned")
        .with_player_like_cpp(handle, player_item_facts)
}

fn fixture_item_facts(session: &WorldSession) -> Vec<(ObjectGuid, ItemValuesSnapshot)> {
    let mut facts = session
        .inventory_item_objects_like_cpp()
        .iter()
        .map(|(guid, item)| (*guid, ItemValuesSnapshot::capture(item)))
        .collect::<Vec<_>>();
    facts.sort_by_key(|(guid, _)| guid.counter());
    facts
}

async fn query_purchase_data(session: &mut WorldSession, guid: ObjectGuid) {
    let mut request = WorldPacket::new_empty();
    request.write_packed_guid(&guid);
    request.reset_read();

    let (quest_state, inventory, lifecycle, visibility, instances, hub) =
        crate::session::split_player_handler_states_mut(session);
    let mut handler = wow_world_application::PlayerHandlerCxLikeCpp::new(
        hub,
        quest_state,
        inventory,
        lifecycle,
        visibility,
        instances,
    );
    handler.handle_get_item_purchase_data(request).await;
}

fn expected_purchase_data(guid: ObjectGuid, money: u64) -> Vec<u8> {
    SetItemPurchaseData {
        item_guid: guid,
        contents: ItemPurchaseContents {
            money,
            items: [
                ItemPurchaseRefundItem {
                    item_id: 7_001,
                    item_count: 1,
                },
                ItemPurchaseRefundItem {
                    item_id: 7_002,
                    item_count: 2,
                },
                ItemPurchaseRefundItem {
                    item_id: 7_003,
                    item_count: 3,
                },
                ItemPurchaseRefundItem {
                    item_id: 7_004,
                    item_count: 4,
                },
                ItemPurchaseRefundItem {
                    item_id: 7_005,
                    item_count: 5,
                },
            ],
            currencies: [
                ItemPurchaseRefundCurrency {
                    currency_id: 101,
                    currency_count: 11,
                },
                ItemPurchaseRefundCurrency {
                    currency_id: 102,
                    currency_count: 12,
                },
                ItemPurchaseRefundCurrency::default(),
                ItemPurchaseRefundCurrency {
                    currency_id: 104,
                    currency_count: 14,
                },
                ItemPurchaseRefundCurrency {
                    currency_id: 105,
                    currency_count: 15,
                },
            ],
        },
        flags: 0,
        purchase_time: 890,
    }
    .to_bytes()
}

fn assert_one_packet(send_rx: &flume::Receiver<Vec<u8>>, expected: Vec<u8>) {
    assert_eq!(
        send_rx.try_recv().expect("purchase data response"),
        expected
    );
    assert!(
        send_rx.try_recv().is_err(),
        "response should be sent exactly once"
    );
}

#[tokio::test]
async fn item_purchase_data_uses_canonical_facts_and_preserves_owner_fallbacks() {
    for (index, active) in [(0i64, false), (1, true)] {
        let (mut session, _, send_rx) = make_session();
        let owner_guid = player_guid(91_910 + index);
        let guid = item_guid(91_920 + index);
        session.lifecycle.set_total_played_time_like_cpp(1_000);
        session.lifecycle.set_login_time_like_cpp(None);
        install_extended_cost(&mut session, EXTENDED_COST_ID);
        install_poisoned_fixture(&mut session, owner_guid, guid);

        let canonical_item =
            purchase_item(owner_guid, guid, true, owner_guid, 10, 900, CANONICAL_MONEY);
        let player = player_with_item(owner_guid, guid, canonical_item);
        let _owner = install_owner(&mut session, player, owner_guid, active);
        let before = canonical_item_facts(&session).expect("selected canonical owner");
        assert_eq!(before.len(), 1);
        let fixture_before = fixture_item_facts(&session);

        query_purchase_data(&mut session, guid).await;

        assert_eq!(canonical_item_facts(&session), Some(before));
        assert_eq!(fixture_item_facts(&session), fixture_before);
        assert_one_packet(&send_rx, expected_purchase_data(guid, CANONICAL_MONEY));
    }

    let (mut stale_session, _, stale_send_rx) = make_session();
    let stale_owner_guid = player_guid(91_930);
    let stale_item_guid = item_guid(91_931);
    stale_session
        .lifecycle
        .set_total_played_time_like_cpp(1_000);
    stale_session.lifecycle.set_login_time_like_cpp(None);
    install_extended_cost(&mut stale_session, EXTENDED_COST_ID);
    install_poisoned_fixture(&mut stale_session, stale_owner_guid, stale_item_guid);
    let stale_item = purchase_item(
        stale_owner_guid,
        stale_item_guid,
        true,
        stale_owner_guid,
        10,
        900,
        CANONICAL_MONEY,
    );
    let (manager, stale_handle) = install_owner(
        &mut stale_session,
        player_with_item(stale_owner_guid, stale_item_guid, stale_item),
        stale_owner_guid,
        false,
    );
    assert_eq!(
        item_facts_for_handle(&manager, stale_handle)
            .expect("old canonical owner before retirement")
            .len(),
        1
    );
    manager
        .lock()
        .expect("canonical map manager should not be poisoned")
        .retire_player_like_cpp(stale_handle)
        .expect("old canonical owner should retire");
    let replacement_item = purchase_item(
        stale_owner_guid,
        stale_item_guid,
        true,
        stale_owner_guid,
        10,
        900,
        CANONICAL_MONEY + 1,
    );
    let replacement_handle = manager
        .lock()
        .expect("canonical map manager should not be poisoned")
        .install_detached_player_like_cpp(Box::new(player_with_item(
            stale_owner_guid,
            stale_item_guid,
            replacement_item,
        )))
        .expect("same-GUID replacement owner should install");
    let replacement_before = item_facts_for_handle(&manager, replacement_handle)
        .expect("replacement owner facts before stale query");
    let stale_fixture_before = fixture_item_facts(&stale_session);

    query_purchase_data(&mut stale_session, stale_item_guid).await;

    assert!(
        stale_send_rx.try_recv().is_err(),
        "stale generation must not reply"
    );
    assert_eq!(
        item_facts_for_handle(&manager, replacement_handle),
        Some(replacement_before)
    );
    assert_eq!(fixture_item_facts(&stale_session), stale_fixture_before);

    let (mut empty_session, _, empty_send_rx) = make_session();
    let empty_owner_guid = player_guid(91_940);
    let empty_item_guid = item_guid(91_941);
    empty_session
        .lifecycle
        .set_total_played_time_like_cpp(1_000);
    empty_session.lifecycle.set_login_time_like_cpp(None);
    install_extended_cost(&mut empty_session, EXTENDED_COST_ID);
    install_poisoned_fixture(&mut empty_session, empty_owner_guid, empty_item_guid);
    let (empty_manager, empty_handle) = install_owner(
        &mut empty_session,
        empty_player(empty_owner_guid),
        empty_owner_guid,
        false,
    );
    let empty_before = canonical_item_facts(&empty_session).expect("current empty owner");
    assert!(empty_before.is_empty());
    let empty_fixture_before = fixture_item_facts(&empty_session);

    query_purchase_data(&mut empty_session, empty_item_guid).await;

    assert!(
        empty_send_rx.try_recv().is_err(),
        "current empty owner must not reply"
    );
    assert_eq!(canonical_item_facts(&empty_session), Some(empty_before));
    assert_eq!(fixture_item_facts(&empty_session), empty_fixture_before);
    assert!(item_facts_for_handle(&empty_manager, empty_handle).is_some());

    let (mut fixture_session, _, fixture_send_rx) = make_session();
    let fixture_owner_guid = player_guid(91_950);
    let fixture_guid = item_guid(91_951);
    fixture_session
        .lifecycle
        .set_total_played_time_like_cpp(1_000);
    fixture_session.lifecycle.set_login_time_like_cpp(None);
    fixture_session.set_player_guid(Some(fixture_owner_guid));
    install_extended_cost(&mut fixture_session, EXTENDED_COST_ID);
    install_poisoned_fixture(&mut fixture_session, fixture_owner_guid, fixture_guid);
    assert!(
        fixture_session
            .core
            .owned_inventory_access_like_cpp()
            .owner_handle_absent_like_cpp()
    );
    let fixture_before = fixture_item_facts(&fixture_session);

    query_purchase_data(&mut fixture_session, fixture_guid).await;

    assert_eq!(fixture_item_facts(&fixture_session), fixture_before);
    assert_one_packet(
        &fixture_send_rx,
        expected_purchase_data(fixture_guid, FIXTURE_MONEY),
    );
}

#[tokio::test]
async fn item_purchase_data_rejects_invalid_refund_sources_without_mutation() {
    #[derive(Debug, Clone, Copy)]
    enum Rejection {
        NonRefundable,
        WrongRecipient,
        Expired,
        MissingCost,
        UnknownGuid,
    }

    for (index, rejection) in [
        Rejection::NonRefundable,
        Rejection::WrongRecipient,
        Rejection::Expired,
        Rejection::MissingCost,
        Rejection::UnknownGuid,
    ]
    .into_iter()
    .enumerate()
    {
        let (mut session, _, send_rx) = make_session();
        let owner_guid = player_guid(92_000 + index as i64);
        let guid = item_guid(92_100 + index as i64);
        let (refundable, recipient, create_played_time, last_played_time_update, cost_row_id) =
            match rejection {
                Rejection::NonRefundable => (false, owner_guid, 10, 900, EXTENDED_COST_ID),
                Rejection::WrongRecipient => (
                    true,
                    player_guid(92_500 + index as i64),
                    10,
                    900,
                    EXTENDED_COST_ID,
                ),
                Rejection::Expired => (true, owner_guid, 7_201, 1_000, EXTENDED_COST_ID),
                Rejection::MissingCost => (true, owner_guid, 10, 900, EXTENDED_COST_ID + 1),
                Rejection::UnknownGuid => (true, owner_guid, 10, 900, EXTENDED_COST_ID),
            };
        session.lifecycle.set_total_played_time_like_cpp(1_000);
        session.lifecycle.set_login_time_like_cpp(None);
        install_extended_cost(&mut session, cost_row_id);
        install_poisoned_fixture(&mut session, owner_guid, guid);
        let canonical_item = purchase_item(
            owner_guid,
            guid,
            refundable,
            recipient,
            create_played_time,
            last_played_time_update,
            CANONICAL_MONEY,
        );
        let _owner = install_owner(
            &mut session,
            player_with_item(owner_guid, guid, canonical_item),
            owner_guid,
            false,
        );
        let before = canonical_item_facts(&session).expect("canonical item facts before query");
        let fixture_before = fixture_item_facts(&session);
        let request_guid = if matches!(rejection, Rejection::UnknownGuid) {
            item_guid(92_900 + index as i64)
        } else {
            guid
        };

        query_purchase_data(&mut session, request_guid).await;

        assert_eq!(canonical_item_facts(&session), Some(before));
        assert_eq!(fixture_item_facts(&session), fixture_before);
        assert!(
            send_rx.try_recv().is_err(),
            "{rejection:?} source must not send purchase data"
        );
    }
}
