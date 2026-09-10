//! Social handler regressions.
//!
//! Moved out of social.rs under #685; every test is unchanged.

use super::*;

#[tokio::test]
async fn social_contract_request_sends_false_response_like_cpp() {
    let (mut session, send_rx) = make_session();

    session.handle_social_contract_request().await;

    let bytes = send_rx.try_recv().expect("social contract response");
    assert_eq!(
        opcode(&bytes),
        ServerOpcodes::SocialContractRequestResponse
            .to_u16()
            .expect("opcode")
    );
    assert_eq!(bytes.last().copied(), Some(0));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn accept_social_contract_is_no_response_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_accept_social_contract(AcceptSocialContract)
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn account_notification_acknowledged_is_no_response_like_cpp() {
    let (mut session, send_rx) = make_session();

    session
        .handle_account_notification_acknowledged(AccountNotificationAcknowledged {
            notification_id: 42,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn empty_contact_list_sends_only_contact_list_like_cpp() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

    session.send_contact_list_like_cpp(7).await;

    let bytes = send_rx.try_recv().expect("empty contact list");
    assert_eq!(
        opcode(&bytes),
        ServerOpcodes::ContactList.to_u16().expect("opcode")
    );
    let mut body = wow_packet::WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(body.read_uint32().unwrap(), 7);
    assert_eq!(body.read_bits(8).unwrap(), 0);
    assert_eq!(body.remaining(), 0);
    assert!(
        send_rx.try_recv().is_err(),
        "C++ PlayerSocial::SendSocialList does not inject QueryPlayerNamesResponse"
    );
}

#[tokio::test]
async fn contact_list_uses_the_sqlx_free_port_and_projects_loaded_rows() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let port = recording_port(
        SocialContactListLoadOutcomeLikeCpp::Loaded(vec![SocialContactLoadRowLikeCpp {
            friend_guid: 77,
            type_flags: 1,
            note: "raid".into(),
            class_id: 8,
            level: 80,
            zone_id: 1519,
        }]),
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_social_persistence_port_like_cpp(port.clone());

    session.send_contact_list_like_cpp(1).await;

    let bytes = send_rx.try_recv().expect("contact list");
    assert_eq!(opcode(&bytes), ServerOpcodes::ContactList.to_u16().unwrap());
    assert_eq!(port.calls.lock().unwrap().as_slice(), ["load:42:1"]);
}

#[tokio::test]
async fn failed_remove_does_not_publish_the_friend_status_packet() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let port = recording_port(
        SocialContactListLoadOutcomeLikeCpp::Loaded(Vec::new()),
        PersistenceOutcomeLikeCpp::Failed {
            reason: "write failed".into(),
        },
    );
    session.set_social_persistence_port_like_cpp(port.clone());

    session
        .handle_del_ignore(DelIgnore {
            player_guid: ObjectGuid::create_player(1, 77),
            virtual_realm_address: 0,
        })
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        port.calls.lock().unwrap().as_slice(),
        ["remove:42:77:Ignored"]
    );
}

#[tokio::test]
async fn committed_remove_publishes_after_the_port_returns() {
    let (mut session, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let port = recording_port(
        SocialContactListLoadOutcomeLikeCpp::Loaded(Vec::new()),
        PersistenceOutcomeLikeCpp::Applied { rows: 2 },
    );
    session.set_social_persistence_port_like_cpp(port);

    session
        .handle_del_ignore(DelIgnore {
            player_guid: ObjectGuid::create_player(1, 77),
            virtual_realm_address: 0,
        })
        .await;

    let bytes = send_rx.try_recv().expect("remove status");
    assert_eq!(
        opcode(&bytes),
        ServerOpcodes::FriendStatus.to_u16().unwrap()
    );
}

#[test]
fn normalize_player_name_empty_rejects_like_cpp() {
    assert_eq!(normalize_player_name_like_cpp(""), None);
}

#[test]
fn normalize_player_name_capitalizes_first_and_lowers_rest_like_cpp() {
    assert_eq!(
        normalize_player_name_like_cpp("tHrAlL").as_deref(),
        Some("Thrall")
    );
    assert_eq!(
        normalize_player_name_like_cpp("jaina").as_deref(),
        Some("Jaina")
    );
}

#[test]
fn normalize_player_name_handles_unicode_case_like_cpp_wide_string_path() {
    assert_eq!(
        normalize_player_name_like_cpp("éLUNE").as_deref(),
        Some("Élune")
    );
}

#[test]
fn del_ignore_dispatch_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::DelIgnore)
        .expect("DelIgnore handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::ThreadUnsafe);
    assert_eq!(entry.handler_name, "handle_del_ignore");
}

#[test]
fn social_handler_source_cannot_reacquire_concrete_persistence() {
    let source = include_str!("../../social.rs")
        .split("#[cfg(test)]")
        .next()
        .expect("production source prefix");
    for forbidden in [
        "sqlx::",
        "CharacterDatabase",
        "CharStatements",
        ".pool()",
        "self.char_db()",
        "SELECT ",
        "INSERT INTO character_social",
        "UPDATE character_social",
        "DELETE FROM character_social",
    ] {
        assert!(
            !source.contains(forbidden),
            "social handler reacquired concrete persistence syntax: {forbidden}"
        );
    }
}
