// Existing Character application scenarios, moved with original assertion operands.

use super::fixtures::*;
use super::fixtures::session::make_session;

#[tokio::test]
async fn realm_character_count_refresh_reaches_the_lifecycle_port_without_database_handles() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    session.set_realm_id(12);

    session.character_update_realm_characters_for_test().await;

    assert_eq!(
        port.realm_character_count_refreshes(),
        vec![PlayerRealmCharacterCountRefreshRequestLikeCpp {
            account_id: 1,
            realm_id: 12,
        }]
    );
}

#[tokio::test]
async fn realm_character_count_refresh_without_port_does_not_fabricate_a_request() {
    let (session, _, _) = make_session();
    session.character_update_realm_characters_for_test().await;
}

#[tokio::test]
async fn logout_publishes_each_offline_mark_through_the_port_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    let guid = ObjectGuid::create_player(1, 0x7200_0001);
    session.set_player_guid(Some(guid));

    session.character_mark_character_offline_for_test().await;
    session.character_mark_character_account_offline_for_test().await;
    session
        .character_mark_login_account_offline_on_disconnect_for_test()
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

#[tokio::test]
async fn no_character_means_no_character_offline_request_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Applied { rows: 1 });
    assert!(session.player_guid().is_none());

    session.character_mark_character_offline_for_test().await;

    assert!(port.marks().is_empty());
}

#[tokio::test]
async fn a_failed_offline_mark_is_handled_without_panicking_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Failed {
        reason: "connection refused".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0002)));

    session.character_mark_character_offline_for_test().await;
    session.character_mark_character_account_offline_for_test().await;

    assert_eq!(port.marks().len(), 2);
}

#[tokio::test]
async fn an_unknown_offline_mark_outcome_is_handled_distinctly_like_cpp() {
    let (mut session, port) = session_with_port(PersistenceOutcomeLikeCpp::Unknown {
        reason: "connection lost after the write was sent".to_owned(),
    });
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0003)));

    session.character_mark_character_offline_for_test().await;

    assert_eq!(port.marks().len(), 1);
    assert!(
        PersistenceOutcomeLikeCpp::Unknown {
            reason: "x".to_owned()
        }
        .is_indeterminate()
    );
}

#[tokio::test]
async fn a_session_without_a_port_performs_no_durable_write_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 0x7200_0004)));

    session.character_mark_character_offline_for_test().await;
    session.character_mark_character_account_offline_for_test().await;
    session
        .character_mark_login_account_offline_on_disconnect_for_test()
        .await;
}
