use super::*;
use wow_constants::ItemFieldFlags;
use wow_entities::{INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START, PlayerInventoryItem};

fn player(counter: i64, money: u64) -> Player {
    let mut player = Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, counter));
    player.set_inventory_slot_count(6);
    player.set_money(money);
    player
}

fn item(counter: i64, entry: u32, count: u32, wrapped: bool) -> Item {
    let mut item = Item::default();
    item.object_mut()
        .create(ObjectGuid::create_item(1, counter));
    item.object_mut().set_entry(entry);
    item.set_count(count);
    if wrapped {
        item.set_item_flag(ItemFieldFlags::WRAPPED);
    }
    item
}

fn admitted_pair() -> (Player, Player, TradeCommitSnapshotLikeCpp) {
    let mut own = player(1, 1_000);
    let mut partner = player(2, 2_000);
    own.open_trade_like_cpp(partner.guid());
    partner.open_trade_like_cpp(own.guid());
    let install = |player: &mut Player, specs: &[(i64, u32, u32, bool)]| {
        let mut guids = Vec::new();
        for (index, &(counter, entry, count, wrapped)) in specs.iter().enumerate() {
            let item = item(counter, entry, count, wrapped);
            let guid = item.object().guid();
            assert!(player.trade_store_item_like_cpp(
                item,
                counter as u64,
                None,
                INVENTORY_SLOT_BAG_0,
                INVENTORY_SLOT_ITEM_START + index as u8,
            ));
            assert_eq!(
                player.set_trade_slot_item_like_cpp(index as u8, Some(guid), false, 1),
                Some(true)
            );
            guids.push(guid);
        }
        guids
    };
    let own_items = install(
        &mut own,
        &[(101, 25, 6, false), (102, 26, 5, true), (103, 27, 1, false)],
    );
    let partner_items = install(
        &mut partner,
        &[
            (201, 25, 8, true),
            (202, 26, 4, false),
            (203, 27, 2, false),
            (204, 28, 3, true),
        ],
    );
    own.set_trade_money_value_like_cpp(10, 1);
    partner.set_trade_money_value_like_cpp(20, 1);
    own.set_trade_in_accept_process_like_cpp(true);
    partner.set_trade_in_accept_process_like_cpp(true);
    let snapshot = TradeCommitSnapshotLikeCpp {
        own_guid: own.guid(),
        partner_guid: partner.guid(),
        same_bnet: false,
        own_items,
        partner_items,
        own_offer: 10,
        partner_offer: 20,
    };
    (own, partner, snapshot)
}

#[test]
fn capture_rejects_changed_offer_on_either_side() {
    for change_own in [true, false] {
        let (mut own, mut partner, snapshot) = admitted_pair();
        let changed = if change_own { &mut own } else { &mut partner };
        changed.set_trade_money_value_like_cpp(99, 2);
        assert!(capture_trade_planning_like_cpp(&own, &partner, &snapshot).is_none());
    }
}

#[test]
fn capture_rejects_changed_item_set_or_disappeared_item() {
    for change_own in [true, false] {
        let (mut own, mut partner, snapshot) = admitted_pair();
        let changed = if change_own { &mut own } else { &mut partner };
        changed.set_trade_slot_item_like_cpp(0, None, false, 2);
        assert!(capture_trade_planning_like_cpp(&own, &partner, &snapshot).is_none());

        let (mut own, mut partner, snapshot) = admitted_pair();
        let (changed, guid) = if change_own {
            (&mut own, snapshot.own_items[0])
        } else {
            (&mut partner, snapshot.partner_items[0])
        };
        changed
            .inventory_runtime_mut_like_cpp()
            .remove_item_object_like_cpp(guid);
        assert!(capture_trade_planning_like_cpp(&own, &partner, &snapshot).is_none());
    }
}

#[test]
fn capture_rejects_accept_process_cleared_on_either_side() {
    for change_own in [true, false] {
        let (mut own, mut partner, snapshot) = admitted_pair();
        let changed = if change_own { &mut own } else { &mut partner };
        changed.set_trade_in_accept_process_like_cpp(false);
        assert!(capture_trade_planning_like_cpp(&own, &partner, &snapshot).is_none());
    }
}

#[test]
fn capture_rejects_missing_trade_on_either_side() {
    for change_own in [true, false] {
        let (mut own, mut partner, snapshot) = admitted_pair();
        let changed = if change_own { &mut own } else { &mut partner };
        changed.clear_trade_like_cpp();
        assert!(capture_trade_planning_like_cpp(&own, &partner, &snapshot).is_none());
    }
}

fn runtime_bag_record(player: &mut Player, slot: u8, counter: i64) {
    player
        .inventory_runtime_mut_like_cpp()
        .store_item_in_slot_like_cpp(
            slot,
            PlayerInventoryItem {
                guid: ObjectGuid::create_item(1, counter),
                entry_id: 900,
                db_guid: counter as u64,
                inventory_type: None,
            },
        );
}

#[test]
fn captured_original_facts_survive_later_canonical_mutations() {
    let (mut own, mut partner, snapshot) = admitted_pair();
    own.register_bag_storage(INVENTORY_SLOT_BAG_START, ObjectGuid::create_item(1, 401), 4)
        .unwrap();
    runtime_bag_record(&mut own, INVENTORY_SLOT_BAG_START, 402);
    let captured = capture_trade_planning_like_cpp(&own, &partner, &snapshot).unwrap();
    own.set_money(5);
    partner.set_money(6);
    own.trade_set_stack_count_like_cpp(snapshot.own_items[0], 99);
    partner
        .inventory_runtime_mut_like_cpp()
        .remove_item_object_like_cpp(snapshot.partner_items[0]);
    own.register_bag_storage(INVENTORY_SLOT_BAG_START, ObjectGuid::create_item(1, 499), 4)
        .unwrap();
    runtime_bag_record(&mut own, INVENTORY_SLOT_BAG_START, 498);
    own.unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(1, 9));

    assert_eq!(captured.own_sql.guid, snapshot.own_guid);
    assert_eq!(
        (captured.own_sql.money, captured.partner_sql.money),
        (1_000, 2_000)
    );
    assert_eq!(
        captured
            .own_sql
            .outgoing
            .iter()
            .map(|item| item.object().guid())
            .collect::<Vec<_>>(),
        snapshot.own_items
    );
    assert_eq!(
        captured
            .partner_sql
            .outgoing
            .iter()
            .map(|item| item.object().guid())
            .collect::<Vec<_>>(),
        snapshot.partner_items
    );
    assert_eq!(captured.own_sql.outgoing[0].count(), 6);
    assert_eq!(captured.partner_sql.outgoing[0].count(), 8);
    assert!(captured.partner_sql.outgoing[0].is_wrapped());
    assert_eq!(
        bag_db_guid(&captured.own_sql, INVENTORY_SLOT_BAG_START),
        402
    );

    // Consuming a captured inventory after the source changed still sees its
    // original outgoing item and frees that original slot.
    let template = |entry| Some(wow_entities::ItemStorageTemplate::regular_item(entry, 20));
    let limit = |_| None;
    let catalogs = TradeStoreCatalogsLikeCpp {
        template: &template,
        limit_category: &limit,
    };
    let plans = plan_trade_receive_snapshot_like_cpp(
        captured.own_storage,
        &snapshot.own_items,
        &[item(800, 99, 1, false)],
        &catalogs,
    )
    .unwrap();
    assert_eq!(
        plans[0].actions,
        vec![TradeStoreActionLikeCpp::Store {
            bag: INVENTORY_SLOT_BAG_0,
            slot: INVENTORY_SLOT_ITEM_START,
            count: 1,
        }]
    );
}

#[test]
fn sql_projection_preserves_two_sided_action_money_and_original_bag_order() {
    let (mut own, mut partner, snapshot) = admitted_pair();
    own.register_bag_storage(30, ObjectGuid::create_item(1, 401), 4)
        .unwrap();
    runtime_bag_record(&mut own, 30, 402);
    runtime_bag_record(&mut own, INVENTORY_SLOT_BAG_0, 499);
    partner
        .register_bag_storage(32, ObjectGuid::create_item(1, 410), 4)
        .unwrap();
    let captured = capture_trade_planning_like_cpp(&own, &partner, &snapshot).unwrap();
    // Explicit admitted action fixtures isolate SQL projection from storage selection.
    let own_receives = vec![
        TradeStorePlanLikeCpp {
            item_guid: snapshot.partner_items[0],
            actions: vec![
                TradeStoreActionLikeCpp::Merge {
                    target_guid: ObjectGuid::create_item(1, 301),
                    added: 3,
                    new_count: 20,
                },
                TradeStoreActionLikeCpp::Store {
                    bag: 30,
                    slot: 2,
                    count: 5,
                },
            ],
        },
        TradeStorePlanLikeCpp {
            item_guid: snapshot.partner_items[1],
            actions: vec![TradeStoreActionLikeCpp::Merge {
                target_guid: ObjectGuid::create_item(1, 302),
                added: 4,
                new_count: 10,
            }],
        },
        TradeStorePlanLikeCpp {
            item_guid: snapshot.partner_items[2],
            actions: vec![TradeStoreActionLikeCpp::Store {
                bag: INVENTORY_SLOT_BAG_0,
                slot: 35,
                count: 2,
            }],
        },
        TradeStorePlanLikeCpp {
            item_guid: snapshot.partner_items[3],
            actions: vec![TradeStoreActionLikeCpp::Store {
                bag: 31,
                slot: 0,
                count: 3,
            }],
        },
    ];
    let partner_receives = vec![
        TradeStorePlanLikeCpp {
            item_guid: snapshot.own_items[0],
            actions: vec![TradeStoreActionLikeCpp::Store {
                bag: 32,
                slot: 1,
                count: 6,
            }],
        },
        TradeStorePlanLikeCpp {
            item_guid: snapshot.own_items[1],
            actions: vec![
                TradeStoreActionLikeCpp::Merge {
                    target_guid: ObjectGuid::create_item(1, 303),
                    added: 2,
                    new_count: 18,
                },
                TradeStoreActionLikeCpp::Store {
                    bag: INVENTORY_SLOT_BAG_0,
                    slot: 36,
                    count: 3,
                },
            ],
        },
        TradeStorePlanLikeCpp {
            item_guid: snapshot.own_items[2],
            actions: vec![TradeStoreActionLikeCpp::Merge {
                target_guid: ObjectGuid::create_item(1, 304),
                added: 1,
                new_count: 20,
            }],
        },
    ];
    // Later source changes must not affect the original SQL facts.
    own.set_money(0);
    own.trade_set_stack_count_like_cpp(snapshot.own_items[1], 3);
    runtime_bag_record(&mut own, 30, 999);
    let request = trade_persistence_request_like_cpp(
        &snapshot,
        &captured.own_sql,
        &captured.partner_sql,
        &own_receives,
        &partner_receives,
    );
    use PlayerTradeItemPersistenceLikeCpp::{FullyMerged, MergeInto, Move};
    assert_eq!(
        request,
        PlayerTradePersistenceRequestLikeCpp {
            money: vec![
                PlayerTradeMoneyPersistenceLikeCpp {
                    player_guid: 1,
                    money_before: 1_000,
                    money_after: 1_010
                },
                PlayerTradeMoneyPersistenceLikeCpp {
                    player_guid: 2,
                    money_before: 2_000,
                    money_after: 1_990
                },
            ],
            items: vec![
                MergeInto {
                    item_guid: 301,
                    new_count: 20
                },
                Move {
                    item_guid: 201,
                    from_owner_guid: 2,
                    to_owner_guid: 1,
                    bag_guid: 402,
                    slot: 2,
                    new_count: Some(5),
                    wrapped: true
                },
                MergeInto {
                    item_guid: 302,
                    new_count: 10
                },
                FullyMerged {
                    item_guid: 202,
                    from_owner_guid: 2
                },
                Move {
                    item_guid: 203,
                    from_owner_guid: 2,
                    to_owner_guid: 1,
                    bag_guid: 0,
                    slot: 35,
                    new_count: None,
                    wrapped: false
                },
                Move {
                    item_guid: 204,
                    from_owner_guid: 2,
                    to_owner_guid: 1,
                    bag_guid: 0,
                    slot: 0,
                    new_count: None,
                    wrapped: true
                },
                Move {
                    item_guid: 101,
                    from_owner_guid: 1,
                    to_owner_guid: 2,
                    bag_guid: 410,
                    slot: 1,
                    new_count: None,
                    wrapped: false
                },
                MergeInto {
                    item_guid: 303,
                    new_count: 18
                },
                Move {
                    item_guid: 102,
                    from_owner_guid: 1,
                    to_owner_guid: 2,
                    bag_guid: 0,
                    slot: 36,
                    new_count: Some(3),
                    wrapped: true
                },
                MergeInto {
                    item_guid: 304,
                    new_count: 20
                },
                FullyMerged {
                    item_guid: 103,
                    from_owner_guid: 1
                },
            ],
        }
    );
}
