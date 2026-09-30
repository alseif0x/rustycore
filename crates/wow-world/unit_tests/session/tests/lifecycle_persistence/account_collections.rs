// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Account collection persistence scenarios.

use super::*;
#[tokio::test]
async fn account_collections_are_saved_as_separate_login_requests_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7300_0001)));

    session.save_account_mounts_like_cpp().await;
    session.save_account_toys_like_cpp().await;
    session.save_account_heirlooms_like_cpp().await;

    for save in port.collection_saves() {
        assert_eq!(
            save.logical_database(),
            LogicalDatabaseLikeCpp::Login,
            "account collections belong to the login database"
        );
        assert!(!save.is_empty(), "an empty collection must not be sent");
    }
}

/// An empty collection opens no transaction.
#[tokio::test]
async fn empty_account_collections_are_not_sent_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 0 });

    session.save_account_mounts_like_cpp().await;
    session.save_account_toys_like_cpp().await;
    session.save_account_heirlooms_like_cpp().await;

    assert!(port.collection_saves().is_empty());
}

/// With no port installed nothing is written and nothing panics.
#[tokio::test]
async fn account_collections_without_a_port_write_nothing_like_cpp() {
    let (mut session, _, _) = make_session();

    session.save_account_mounts_like_cpp().await;
    session.save_account_toys_like_cpp().await;
    session.save_account_heirlooms_like_cpp().await;
}

/// Appearances keep their favourite inserts ahead of their deletes. They share
/// one transaction, and a delete that overtook its insert would drop a
/// favourite the client still shows.
#[test]
fn appearance_saves_keep_inserts_before_deletes_like_cpp() {
    let save = AccountCollectionSaveLikeCpp::ItemAppearances {
        bnet_account_id: 7,
        appearance_blocks: vec![AccountMaskBlockLikeCpp {
            block_index: 0,
            mask: 0b1011,
        }],
        favorite_inserts: vec![101, 102],
        favorite_deletes: vec![103],
    };

    assert!(!save.is_empty());
    assert_eq!(save.logical_database(), LogicalDatabaseLikeCpp::Login);
    match save {
        AccountCollectionSaveLikeCpp::ItemAppearances {
            favorite_inserts,
            favorite_deletes,
            ..
        } => {
            assert_eq!(favorite_inserts, vec![101, 102]);
            assert_eq!(favorite_deletes, vec![103]);
        }
        other => panic!("unexpected variant: {other:?}"),
    }
}

/// An appearance plan with no blocks and no favourite changes writes nothing.
#[test]
fn an_empty_appearance_plan_opens_no_transaction_like_cpp() {
    assert!(
        AccountCollectionSaveLikeCpp::ItemAppearances {
            bnet_account_id: 7,
            appearance_blocks: Vec::new(),
            favorite_inserts: Vec::new(),
            favorite_deletes: Vec::new(),
        }
        .is_empty()
    );
    assert!(
        AccountCollectionSaveLikeCpp::TransmogIllusions {
            bnet_account_id: 7,
            illusion_blocks: Vec::new(),
        }
        .is_empty()
    );
}

/// Appearances and illusions reach the port without a database handle.
#[tokio::test]
async fn appearances_and_illusions_are_saved_through_the_port_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7400_0001)));

    session.save_account_item_appearances_like_cpp().await;
    session.save_account_transmog_illusions_like_cpp().await;

    for save in port.collection_saves() {
        assert_eq!(save.logical_database(), LogicalDatabaseLikeCpp::Login);
    }
}
