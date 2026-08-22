// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Session publishes offline state through the typed lifecycle port.
//!
//! These drive the real production methods against a recording port, so they
//! pin which marks are requested, against which logical database, and that
//! every outcome class is handled without panicking.

use super::*;

use std::sync::Mutex;
use wow_persistence::{
    AccountCollectionSaveLikeCpp, AccountMaskBlockLikeCpp, LogicalDatabaseLikeCpp,
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerLifecyclePortLikeCpp,
    PlayerOfflineMarkLikeCpp,
};

struct RecordingPortLikeCpp {
    seen: Mutex<Vec<PlayerOfflineMarkLikeCpp>>,
    collections: Mutex<Vec<AccountCollectionSaveLikeCpp>>,
    outcome: PersistenceOutcomeLikeCpp,
}

impl RecordingPortLikeCpp {
    fn new(outcome: PersistenceOutcomeLikeCpp) -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            collections: Mutex::new(Vec::new()),
            outcome,
        })
    }
    fn marks(&self) -> Vec<PlayerOfflineMarkLikeCpp> {
        self.seen.lock().unwrap().clone()
    }
    fn collection_saves(&self) -> Vec<AccountCollectionSaveLikeCpp> {
        self.collections.lock().unwrap().clone()
    }
}

impl PlayerLifecyclePortLikeCpp for RecordingPortLikeCpp {
    fn mark_offline_like_cpp<'a>(
        &'a self,
        mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.seen.lock().unwrap().push(mark);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn save_account_collection_like_cpp<'a>(
        &'a self,
        save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.collections.lock().unwrap().push(save);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }
}

fn session_with_port(
    outcome: PersistenceOutcomeLikeCpp,
) -> (WorldSession, Arc<RecordingPortLikeCpp>) {
    let (mut session, _, _) = make_session();
    let port = RecordingPortLikeCpp::new(outcome);
    session.set_player_lifecycle_port_like_cpp(port.clone());
    (session, port)
}

/// Each offline mark reaches the port as its own request, naming the logical
/// database it belongs to — the characters writes and the login write are not
/// collapsed into one call.
#[tokio::test]
async fn logout_publishes_each_offline_mark_through_the_port_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let guid = ObjectGuid::create_player(1, 0x7200_0001);
    session.set_player_guid(Some(guid));

    session.mark_character_offline().await;
    session.mark_character_account_offline_like_cpp().await;
    session
        .mark_login_account_offline_on_disconnect_like_cpp()
        .await;

    let marks = port.marks();
    assert_eq!(
        marks,
        vec![
            PlayerOfflineMarkLikeCpp::Character {
                guid_low: guid.counter() as u32
            },
            PlayerOfflineMarkLikeCpp::CharacterAccount {
                account_id: session.account_id
            },
            PlayerOfflineMarkLikeCpp::LoginAccount {
                account_id: session.account_id
            },
        ]
    );
    assert_eq!(
        marks
            .iter()
            .map(|m| m.logical_database())
            .collect::<Vec<_>>(),
        vec![
            LogicalDatabaseLikeCpp::Characters,
            LogicalDatabaseLikeCpp::Characters,
            LogicalDatabaseLikeCpp::Login,
        ]
    );
}

/// With no selected character there is nothing to mark offline, so the port is
/// not called at all.
#[tokio::test]
async fn no_character_means_no_character_offline_request_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    assert!(session.player_guid().is_none());

    session.mark_character_offline().await;

    assert!(port.marks().is_empty());
}

/// A failed write is reported, not retried and not escalated.
#[tokio::test]
async fn a_failed_offline_mark_is_handled_without_panicking_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Failed {
        reason: "connection refused".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0002)));

    session.mark_character_offline().await;
    session.mark_character_account_offline_like_cpp().await;

    assert_eq!(port.marks().len(), 2);
}

/// An indeterminate outcome is a distinct class the caller must see; it is not
/// silently treated as success or as rollback.
#[tokio::test]
async fn an_unknown_offline_mark_outcome_is_handled_distinctly_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Unknown {
        reason: "connection lost after the write was sent".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0003)));

    session.mark_character_offline().await;

    assert_eq!(port.marks().len(), 1);
    assert!(
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "x".to_owned()
        }
        .is_indeterminate()
    );
}

/// A session with no port installed performs no durable write and does not
/// panic: unit sessions and tests never reach a database.
#[tokio::test]
async fn a_session_without_a_port_performs_no_durable_write_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0004)));

    session.mark_character_offline().await;
    session.mark_character_account_offline_like_cpp().await;
    session
        .mark_login_account_offline_on_disconnect_like_cpp()
        .await;
}

/// Each account collection reaches the port as its own request against the
/// Login database. Three requests, not one: C++ logout commits them
/// separately, and #187 freezes that until a deliberate behaviour change.
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
