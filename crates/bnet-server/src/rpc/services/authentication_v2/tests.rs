use super::*;
use crate::state::GameAccountInfo;
use std::collections::HashMap;

#[test]
fn completion_uses_v2_record_handles_and_only_the_documented_fields() {
    let account = AccountInfo {
        id: 17,
        login: "LOCAL@TEST".into(),
        is_locked_to_ip: false,
        lock_country: "00".into(),
        last_ip: "127.0.0.1".into(),
        failed_logins: 0,
        is_banned: false,
        is_permanently_banned: false,
        game_accounts: HashMap::from([(
            23,
            GameAccountInfo {
                id: 23,
                name: "1#1".into(),
                display_name: "WoW1".into(),
                ban_date: 0,
                unban_date: 0,
                is_permanently_banned: false,
                is_banned: false,
                security_level: 0,
                char_counts: HashMap::new(),
                last_played_chars: HashMap::new(),
            },
        )]),
    };
    let result = logon_complete(&account, "ES", &[0xAB; 64]);
    assert_eq!(result.error_code, Some(0));
    let encoded = result.encode_to_vec();
    assert_eq!(
        LogonCompleteNotification::decode(encoded.as_slice()).unwrap(),
        result
    );
    let record = result.record.unwrap();
    assert_eq!(record.account_id, Some(17));
    assert_eq!(
        record.game_account,
        vec![GameAccountHandle {
            id: Some(23),
            title_id: Some(0x0057_6F57),
            region: Some(2),
        }]
    );
    assert_eq!(record.geoip_country.as_deref(), Some("ES"));
    assert_eq!(record.session_key.as_deref(), Some([0xAB; 64].as_slice()));
    assert!(record.battle_tag.is_none());
    assert!(record.login_ticket.is_none());
    assert!(record.employee_only_mode.is_none());
    assert!(
        logon_complete(&account, "", &[0; 64])
            .record
            .unwrap()
            .geoip_country
            .is_none()
    );
}

#[test]
fn malformed_protobuf_is_rpc_malformed_request_not_internal_error() {
    let error = decode::<LogonRequest>(&[0x52, 0xFF]).unwrap_err();
    assert_eq!(
        error.downcast_ref::<RpcStatusError>().unwrap().status(),
        status::ERROR_RPC_MALFORMED_REQUEST
    );
    assert!(decode::<LogonRequest>(&[]).is_ok());
}
