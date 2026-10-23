//! `CMSG_ACCEPT_TRADE` through the atomic exchange (#38 slice 3): C++
//! `HandleAcceptTradeOpcode` and `moveItems` on two canonical sessions, with
//! the one-transaction persistence observed through a recording port.

use std::sync::{Arc, Mutex};

use super::contents::{accept_both, drain, insert_trade_item, open_pair, status, trade};
use super::initiate::TradePairLikeCpp;
use super::*;
use crate::session::WorldSession;
use wow_packet::packets::misc::{TRADE_STATUS_ACCEPTED_LIKE_CPP, TRADE_STATUS_COMPLETE_LIKE_CPP};
use wow_persistence::{
    PersistenceFutureLikeCpp, PlayerTradeItemPersistenceLikeCpp,
    PlayerTradePersistenceOutcomeLikeCpp, PlayerTradePersistencePortLikeCpp,
    PlayerTradePersistenceRequestLikeCpp,
};

#[derive(Default)]
struct RecordingTradePortLikeCpp {
    requests: Mutex<Vec<PlayerTradePersistenceRequestLikeCpp>>,
    roll_back: bool,
    /// Answer `CommitOutcomeUnknown` with no observed money (indeterminate).
    unknown: bool,
    /// Hold the COMMIT until the test opens this gate.
    gate: tokio::sync::Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
}

impl PlayerTradePersistencePortLikeCpp for RecordingTradePortLikeCpp {
    fn persist_player_trade_like_cpp(
        &self,
        request: PlayerTradePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerTradePersistenceOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = if self.unknown {
            PlayerTradePersistenceOutcomeLikeCpp::CommitOutcomeUnknown {
                reason: "test connection lost during COMMIT".into(),
                observed_money: Vec::new(),
            }
        } else if self.roll_back {
            PlayerTradePersistenceOutcomeLikeCpp::DefinitelyRolledBack {
                reason: "test rollback".into(),
            }
        } else {
            PlayerTradePersistenceOutcomeLikeCpp::Committed
        };
        Box::pin(async move {
            if let Some(gate) = self.gate.lock().await.take() {
                let _ = gate.await;
            }
            outcome
        })
    }
}

/// A four-slot bag (C++ `ITEM_CLASS_CONTAINER`, `INVTYPE_BAG`).
const BAG_ENTRY: u32 = 702;

/// Item templates for the traded entries (C++ `CanStoreItem` needs one).
fn install_trade_templates(pair: &mut TradePairLikeCpp) {
    use wow_data::{ItemRecord, ItemSparseTemplateEntry, ItemStatsStore, ItemStore};
    let record = |id: u32| ItemRecord {
        id,
        class_id: wow_constants::ItemClass::Miscellaneous as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: wow_constants::InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    };
    let sparse = ItemSparseTemplateEntry {
        flags: [0; 4],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 20,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 0.0,
        price_random_value: 0.0,
        max_durability: 0,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0, 0],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: 0,
        container_slots: 0,
        inventory_type: wow_constants::InventoryType::NonEquip as i8,
    };
    for session in [&mut pair.source, &mut pair.partner] {
        session.set_item_store(Arc::new(ItemStore::from_records([
            record(700),
            record(701),
            ItemRecord {
                class_id: wow_constants::ItemClass::Container as u8,
                inventory_type: wow_constants::InventoryType::Bag as i8,
                ..record(BAG_ENTRY)
            },
        ])));
        session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
            (700, sparse),
            (701, sparse),
            (
                BAG_ENTRY,
                ItemSparseTemplateEntry {
                    stackable: 1,
                    container_slots: 4,
                    inventory_type: wow_constants::InventoryType::Bag as i8,
                    ..sparse
                },
            ),
        ])));
    }
}

fn install_port(pair: &mut TradePairLikeCpp, roll_back: bool) -> Arc<RecordingTradePortLikeCpp> {
    install_trade_templates(pair);
    install_port_with(
        pair,
        RecordingTradePortLikeCpp {
            roll_back,
            ..RecordingTradePortLikeCpp::default()
        },
    )
}

fn install_port_with(
    pair: &mut TradePairLikeCpp,
    port: RecordingTradePortLikeCpp,
) -> Arc<RecordingTradePortLikeCpp> {
    install_trade_templates(pair);
    let port = Arc::new(port);
    pair.source
        .lifecycle
        .set_player_trade_persistence_port_like_cpp(port.clone());
    pair.partner
        .lifecycle
        .set_player_trade_persistence_port_like_cpp(port.clone());
    port
}

/// The second accepter coordinates. Its accept pass hands the exchange to
/// the partner and returns without waiting (a world step waits for every
/// pass, so an in-pass wait on another session would stall the step). The
/// partner executes when it drains its mailbox; the coordinator takes its
/// outcome at the head of a later pass.
async fn accept_as_coordinator(pair: &mut TradePairLikeCpp) {
    let state_index = trade(&pair.partner).server_state_index;
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        pair.source
            .handle_accept_trade(accept_trade_packet(state_index)),
    )
    .await
    .expect("the accept pass must not wait for the partner");
    for _ in 0..200 {
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            pair.partner.process_represented_session_commands_like_cpp(),
        )
        .await
        .expect("the executor waits only for its worker");
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            pair.source
                .continue_pending_trade_commit_for_test_like_cpp(),
        )
        .await
        .expect("the coordinator never waits at the head of its pass");
        if !wow_world_social::trade_commit_pending_like_cpp(&pair.source.social) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(!wow_world_social::trade_commit_pending_like_cpp(
        &pair.source.social
    ));
}

fn item_count_at(pair_session: &WorldSession, guid: ObjectGuid) -> Option<u32> {
    pair_session
        .core
        .with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&guid)
                .map(|item| item.count())
        })
        .flatten()
}

fn money(session: &WorldSession) -> u64 {
    session
        .core
        .with_owned_player_like_cpp(|player| player.money())
        .unwrap()
}

#[tokio::test]
async fn first_accept_only_informs_the_trader_like_cpp() {
    let mut pair = open_pair().await;
    let port = install_port(&mut pair, false);
    let index = trade(&pair.partner).server_state_index;
    pair.source
        .handle_accept_trade(accept_trade_packet(index))
        .await;
    let (source, partner) = drain(&mut pair).await;
    assert!(trade(&pair.source).accepted);
    assert!(source.is_empty());
    assert_eq!(status(&partner[0]), TRADE_STATUS_ACCEPTED_LIKE_CPP);
    assert!(port.requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn second_accept_swaps_items_and_gold_in_one_commit_like_cpp() {
    let mut pair = open_pair().await;
    let port = install_port(&mut pair, false);
    let given = ObjectGuid::create_item(1, 1234);
    let received = ObjectGuid::create_item(1, 5678);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, given, 700);
    insert_trade_item(&mut pair.partner, pair.partner_guid, 35, received, 701);
    for session in [&mut pair.source, &mut pair.partner] {
        session
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(16))
            .unwrap();
    }
    assert!(pair.source.set_player_gold_like_cpp(100));
    assert!(pair.partner.set_player_gold_like_cpp(10));

    pair.source
        .handle_set_trade_item(set_trade_item_packet(0, 255, 35))
        .await;
    pair.source
        .handle_set_trade_gold(set_trade_gold_packet(60))
        .await;
    pair.partner
        .handle_set_trade_item(set_trade_item_packet(0, 255, 35))
        .await;
    let _ = drain(&mut pair).await;
    // The partner accepted first.
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_trade_accepted_like_cpp(true))
        .unwrap();

    accept_as_coordinator(&mut pair).await;
    let (source, partner) = drain(&mut pair).await;

    // Both `m_trade` are gone, the items changed owner and the money moved.
    assert!(
        wow_world_social::player_trade_state_snapshot_like_cpp(
            &pair.source.core,
            &pair.source.social
        )
        .flatten()
        .is_none()
    );
    assert_eq!(item_count_at(&pair.source, given), None);
    assert_eq!(item_count_at(&pair.source, received), Some(1));
    assert_eq!(item_count_at(&pair.partner, given), Some(1));
    assert_eq!(item_count_at(&pair.partner, received), None);
    assert_eq!(money(&pair.source), 40);
    assert_eq!(money(&pair.partner), 70);

    // One transaction for both participants.
    let requests = port.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let moves = requests[0]
        .items
        .iter()
        .filter(|item| matches!(item, PlayerTradeItemPersistenceLikeCpp::Move { .. }))
        .count();
    assert_eq!(moves, 2);
    assert_eq!(requests[0].money.len(), 2);

    // TRADE_STATUS_COMPLETE reaches both clients last among the trade packets.
    let last_status = |packets: &[Vec<u8>]| {
        packets
            .iter()
            .rev()
            .find(|bytes| {
                u16::from_le_bytes([bytes[0], bytes[1]]) == ServerOpcodes::TradeStatus as u16
            })
            .map(|bytes| status(bytes))
    };
    assert_eq!(last_status(&source), Some(TRADE_STATUS_COMPLETE_LIKE_CPP));
    assert_eq!(last_status(&partner), Some(TRADE_STATUS_COMPLETE_LIKE_CPP));
    // The partner was told ACCEPTED before the exchange ("inform partner").
    assert!(partner.iter().any(|bytes| {
        u16::from_le_bytes([bytes[0], bytes[1]]) == ServerOpcodes::TradeStatus as u16
            && status(bytes) == TRADE_STATUS_ACCEPTED_LIKE_CPP
    }));
}

#[tokio::test]
async fn rolled_back_commit_changes_nothing_and_unaccepts_like_cpp() {
    let mut pair = open_pair().await;
    let port = install_port(&mut pair, true);
    let given = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, given, 700);
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(16))
        .unwrap();
    pair.source
        .handle_set_trade_item(set_trade_item_packet(0, 255, 35))
        .await;
    let _ = drain(&mut pair).await;
    accept_both(&mut pair);
    pair.source
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_trade_accepted_like_cpp(false))
        .unwrap();

    accept_as_coordinator(&mut pair).await;
    assert_eq!(port.requests.lock().unwrap().len(), 1);
    assert_eq!(item_count_at(&pair.source, given), Some(1));
    assert_eq!(item_count_at(&pair.partner, given), None);
    assert!(
        trade(&pair.source).items[0].is_some(),
        "the trade stays open"
    );
}

#[tokio::test]
async fn missing_space_reports_failed_to_both_like_cpp() {
    let mut pair = open_pair().await;
    let port = install_port(&mut pair, false);
    let given = ObjectGuid::create_item(1, 1234);
    insert_trade_item(&mut pair.source, pair.source_guid, 35, given, 700);
    // The partner has no backpack slot at all.
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(0))
        .unwrap();
    pair.source
        .handle_set_trade_item(set_trade_item_packet(0, 255, 35))
        .await;
    let _ = drain(&mut pair).await;
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_trade_accepted_like_cpp(true))
        .unwrap();

    accept_as_coordinator(&mut pair).await;
    let (source, partner) = drain(&mut pair).await;
    assert!(port.requests.lock().unwrap().is_empty());
    let failed = |packets: &[Vec<u8>]| {
        packets.iter().any(|bytes| {
            u16::from_le_bytes([bytes[0], bytes[1]]) == ServerOpcodes::TradeStatus as u16
                && status(bytes) == wow_packet::packets::misc::TRADE_STATUS_FAILED_LIKE_CPP
        })
    };
    assert!(failed(&source));
    assert!(failed(&partner));
    assert!(!trade(&pair.source).accepted && !trade(&pair.partner).accepted);
    assert!(!trade(&pair.source).in_accept_process);
}

fn tracker_indeterminate(session: &WorldSession) -> bool {
    session
        .durable_loot_money_persistence_tracker_like_cpp()
        .is_indeterminate_like_cpp()
}

/// One item from the source, the partner already accepted; the source's
/// accept makes it the coordinator.
async fn offer_one_item(pair: &mut TradePairLikeCpp, given: ObjectGuid) {
    insert_trade_item(&mut pair.source, pair.source_guid, 35, given, 700);
    pair.source
        .handle_set_trade_item(set_trade_item_packet(0, 255, 35))
        .await;
    let _ = drain(pair).await;
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_trade_accepted_like_cpp(true))
        .unwrap();
}

/// Review of #1480: a production receiver has its bag only as an inventory
/// runtime record (the native bag arrays stay empty). The planner stores the
/// traded item in that bag, and the persisted Move must name the bag item
/// and the slot inside it (`CHARACTER_INVENTORY.bag`/`slot`), not bag 0.
#[tokio::test]
async fn traded_item_into_a_bag_persists_the_bag_and_in_bag_slot_like_cpp() {
    let mut pair = open_pair().await;
    let port = install_port(&mut pair, false);
    let bag = ObjectGuid::create_item(1, 4321);
    crate::handlers::test_support::world::insert_trade_test_item(
        &mut pair.partner,
        pair.partner_guid,
        30,
        bag,
        BAG_ENTRY,
    );
    // No backpack slot: the only free position is inside the bag.
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_inventory_slot_count(0);
            assert!(player.inventory().bags.iter().all(Option::is_none));
        })
        .unwrap();
    let given = ObjectGuid::create_item(1, 1234);
    offer_one_item(&mut pair, given).await;

    accept_as_coordinator(&mut pair).await;

    let requests = port.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let destination = requests[0].items.iter().find_map(|item| match item {
        PlayerTradeItemPersistenceLikeCpp::Move {
            item_guid,
            bag_guid,
            slot,
            ..
        } if *item_guid == 1234 => Some((*bag_guid, *slot)),
        _ => None,
    });
    assert_eq!(destination, Some((4321, 0)));
    let position = pair
        .partner
        .core
        .with_owned_player_like_cpp(|player| player.trade_item_position_like_cpp(given))
        .flatten();
    assert_eq!(position, Some((30, 0)));
}

/// An unreconcilable COMMIT closes both Players' saves before the fences are
/// released, so a later CharacterSave cannot overwrite the committed rows
/// with the pre-trade runtime.
#[tokio::test]
async fn unknown_commit_marks_both_trackers_before_release_like_cpp() {
    let mut pair = open_pair().await;
    let port = install_port_with(
        &mut pair,
        RecordingTradePortLikeCpp {
            unknown: true,
            ..RecordingTradePortLikeCpp::default()
        },
    );
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(16))
        .unwrap();
    let given = ObjectGuid::create_item(1, 1234);
    offer_one_item(&mut pair, given).await;

    accept_as_coordinator(&mut pair).await;

    assert_eq!(port.requests.lock().unwrap().len(), 1);
    assert!(tracker_indeterminate(&pair.source));
    assert!(tracker_indeterminate(&pair.partner));
    // The runtime was not applied.
    assert_eq!(item_count_at(&pair.source, given), Some(1));
    assert_eq!(item_count_at(&pair.partner, given), None);
}

/// A handover nobody claimed by the deadline is revoked: the fence comes back,
/// the accept process ends, and a late claim finds nothing to run.
#[tokio::test]
async fn unclaimed_handover_is_revoked_at_the_deadline_like_cpp() {
    let mut pair = open_pair().await;
    let port = install_port(&mut pair, false);
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(16))
        .unwrap();
    let given = ObjectGuid::create_item(1, 1234);
    offer_one_item(&mut pair, given).await;
    let state_index = trade(&pair.partner).server_state_index;
    pair.source
        .handle_accept_trade(accept_trade_packet(state_index))
        .await;
    assert!(wow_world_social::trade_commit_pending_like_cpp(
        &pair.source.social
    ));

    // Before the deadline the coordinator keeps waiting, without blocking.
    pair.source
        .continue_pending_trade_commit_for_test_like_cpp()
        .await;
    assert!(wow_world_social::trade_commit_pending_like_cpp(
        &pair.source.social
    ));

    wow_world_social::expire_pending_trade_commit_for_test_like_cpp(&mut pair.source.social);
    pair.source
        .continue_pending_trade_commit_for_test_like_cpp()
        .await;
    assert!(!wow_world_social::trade_commit_pending_like_cpp(
        &pair.source.social
    ));
    assert!(!trade(&pair.source).in_accept_process);

    // The partner drains the request late: nothing is claimed or persisted.
    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;
    assert!(port.requests.lock().unwrap().is_empty());
    assert_eq!(item_count_at(&pair.source, given), Some(1));
    assert!(!tracker_indeterminate(&pair.source));
}

/// Claim wins over revoke: once the executor holds the handover the
/// coordinator keeps waiting past the deadline and then takes the outcome.
#[tokio::test]
async fn claimed_handover_is_not_revoked_like_cpp() {
    let mut pair = open_pair().await;
    let (gate_tx, gate_rx) = tokio::sync::oneshot::channel();
    let port = install_port_with(
        &mut pair,
        RecordingTradePortLikeCpp {
            gate: tokio::sync::Mutex::new(Some(gate_rx)),
            ..RecordingTradePortLikeCpp::default()
        },
    );
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(16))
        .unwrap();
    let given = ObjectGuid::create_item(1, 1234);
    offer_one_item(&mut pair, given).await;
    let state_index = trade(&pair.partner).server_state_index;
    pair.source
        .handle_accept_trade(accept_trade_packet(state_index))
        .await;

    let TradePairLikeCpp {
        source, partner, ..
    } = &mut pair;
    let executor = partner.process_represented_session_commands_like_cpp();
    let coordinator = async {
        for _ in 0..400 {
            if !port.requests.lock().unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert_eq!(port.requests.lock().unwrap().len(), 1, "executor claimed");
        wow_world_social::expire_pending_trade_commit_for_test_like_cpp(&mut source.social);
        source
            .continue_pending_trade_commit_for_test_like_cpp()
            .await;
        assert!(
            wow_world_social::trade_commit_pending_like_cpp(&source.social),
            "a claimed handover is never revoked"
        );
        let _ = gate_tx.send(());
    };
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(executor, coordinator)
    })
    .await
    .expect("neither side waits on the other");

    for _ in 0..200 {
        pair.source
            .continue_pending_trade_commit_for_test_like_cpp()
            .await;
        if !wow_world_social::trade_commit_pending_like_cpp(&pair.source.social) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert!(!wow_world_social::trade_commit_pending_like_cpp(
        &pair.source.social
    ));
    assert_eq!(item_count_at(&pair.source, given), None);
    assert_eq!(item_count_at(&pair.partner, given), Some(1));
    assert!(!tracker_indeterminate(&pair.source));
}

/// The coordinator's session goes away mid-handover (disconnect, or its pass
/// future cancelled): the executor still runs the exchange on both canonical
/// Players, and the coordinator's fence stays held until the worker ends, so
/// its logout save sees the applied runtime.
#[tokio::test]
async fn coordinator_gone_mid_handover_still_commits_once_like_cpp() {
    let mut pair = open_pair().await;
    let port = install_port(&mut pair, false);
    pair.partner
        .core
        .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(16))
        .unwrap();
    let given = ObjectGuid::create_item(1, 1234);
    offer_one_item(&mut pair, given).await;
    let state_index = trade(&pair.partner).server_state_index;
    pair.source
        .handle_accept_trade(accept_trade_packet(state_index))
        .await;
    wow_world_social::drop_pending_trade_commit_for_test_like_cpp(&mut pair.source.social);

    pair.partner
        .process_represented_session_commands_like_cpp()
        .await;

    assert_eq!(port.requests.lock().unwrap().len(), 1);
    assert_eq!(item_count_at(&pair.source, given), None);
    assert_eq!(item_count_at(&pair.partner, given), Some(1));
    assert!(!tracker_indeterminate(&pair.partner));
}
