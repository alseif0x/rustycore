// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-save and logout lifecycle scenarios.

use super::*;
fn character_save_session_with_port(
    outcome: PersistenceOutcomeLikeCpp,
    guid_counter: i64,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, port) = session_with_port(outcome);
    session.set_player_guid(Some(ObjectGuid::create_player(1, guid_counter)));
    session.set_state(SessionState::LoggedIn);
    session.set_player_map_position_like_cpp(
        0,
        Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            orientation: 0.5,
        },
    );
    session.lifecycle.tutorials_changed_like_cpp = true;
    session.lifecycle.tutorials_loaded_coherently_like_cpp = true;
    (session, port)
}

fn represented_buyback_item_like_cpp(db_guid: u64) -> InventoryItem {
    InventoryItem {
        guid: ObjectGuid::create_item(1, db_guid as i64),
        entry_id: 25,
        db_guid,
        inventory_type: None,
    }
}

#[tokio::test]
async fn logout_buyback_clear_reaches_port_and_publishes_only_after_apply_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7500_0101)));
    session.insert_buyback_item_like_cpp(94, represented_buyback_item_like_cpp(0x8100_0001));

    session.clear_buyback_on_logout().await;

    assert_eq!(
        port.buyback_clears(),
        vec![PlayerBuybackClearRequestLikeCpp {
            player_guid: 0x7500_0101,
            item_db_guids: vec![0x8100_0001],
        }]
    );
    assert!(session.buyback_items_like_cpp().is_empty());
}

#[tokio::test]
async fn logout_buyback_clear_preserves_runtime_for_failed_and_unknown_durability_like_cpp() {
    for outcome in [
        PersistenceOutcomeLikeCpp::Failed {
            reason: "rolled back before COMMIT".to_owned(),
        },
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "connection lost after COMMIT".to_owned(),
        },
    ] {
        let (mut session, port) = session_with_port(outcome);
        session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7500_0102)));
        session.insert_buyback_item_like_cpp(94, represented_buyback_item_like_cpp(0x8100_0002));

        session.clear_buyback_on_logout().await;

        assert_eq!(port.buyback_clears().len(), 1);
        assert!(session.buyback_items_like_cpp().contains_key(&94));
    }
}

#[tokio::test]
async fn logout_buyback_clear_without_port_does_not_fabricate_durable_success_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7500_0103)));
    session.insert_buyback_item_like_cpp(94, represented_buyback_item_like_cpp(0x8100_0003));

    session.clear_buyback_on_logout().await;

    assert!(session.buyback_items_like_cpp().contains_key(&94));
}

/// Each offline mark reaches the port as its own request, naming the logical
/// database it belongs to — the characters writes and the login write are not
/// collapsed into one call.

/// With no selected character there is nothing to mark offline, so the port is
/// not called at all.

/// A failed write is reported, not retried and not escalated.

/// An indeterminate outcome is a distinct class the caller must see; it is not
/// silently treated as success or as rollback.

/// A session with no port installed performs no durable write and does not
/// panic: unit sessions and tests never reach a database.
