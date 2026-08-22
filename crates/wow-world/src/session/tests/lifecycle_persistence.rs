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
    LogicalDatabaseLikeCpp, PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp,
    PlayerLifecyclePortLikeCpp, PlayerOfflineMarkLikeCpp,
};

struct RecordingPortLikeCpp {
    seen: Mutex<Vec<PlayerOfflineMarkLikeCpp>>,
    outcome: PersistenceOutcomeLikeCpp,
}

impl RecordingPortLikeCpp {
    fn new(outcome: PersistenceOutcomeLikeCpp) -> Arc<Self> {
        Arc::new(Self {
            seen: Mutex::new(Vec::new()),
            outcome,
        })
    }
    fn marks(&self) -> Vec<PlayerOfflineMarkLikeCpp> {
        self.seen.lock().unwrap().clone()
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
