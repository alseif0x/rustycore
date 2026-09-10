//! Game-utilities RPC regressions.
//!
//! Moved out of game_utilities.rs under #685; every test is unchanged.

use super::*;

#[test]
fn bnet_session_key_data_is_raw_client_then_server_secret_like_cpp() {
    let client_secret: Vec<u8> = (0..32).collect();
    let server_secret: Vec<u8> = (32..64).collect();

    let key_data = bnet_session_key_data_like_cpp(&client_secret, &server_secret).unwrap();

    assert_eq!(key_data.len(), 64);
    assert_eq!(&key_data[..32], client_secret.as_slice());
    assert_eq!(&key_data[32..], server_secret.as_slice());
}

#[test]
fn bnet_session_key_data_rejects_non_32_byte_secrets_like_cpp_array_contract() {
    assert!(bnet_session_key_data_like_cpp(&[0; 31], &[1; 32]).is_none());
    assert!(bnet_session_key_data_like_cpp(&[0; 32], &[1; 31]).is_none());
}

#[test]
fn realm_list_ticket_identity_selects_requested_game_account_like_cpp() {
    let attrs = vec![Attribute {
        name: "Param_Identity".to_string(),
        value: Variant {
            blob_value: Some(
                b"JSONRealmListTicketIdentity:{\"gameAccountID\":42,\"gameAccountRegion\":1}\0"
                    .to_vec(),
            ),
            ..Default::default()
        },
    }];

    assert_eq!(
        parse_realm_list_ticket_game_account_id_like_cpp(&attrs),
        Some(42)
    );
}

#[test]
fn realm_list_ticket_client_secret_accepts_exact_32_byte_array_like_cpp() {
    let attrs = vec![client_info_attr(
        "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31]}}\0",
    )];

    assert_eq!(
        parse_realm_list_ticket_client_secret_like_cpp(&attrs),
        Some((0..32).collect())
    );
}

#[test]
fn realm_list_ticket_client_secret_rejects_malformed_secret_like_cpp() {
    let too_short = vec![client_info_attr(
        "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30]}}\0",
    )];
    let too_long = vec![client_info_attr(
        "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]}}\0",
    )];
    let out_of_range = vec![client_info_attr(
        "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,256]}}\0",
    )];
    let negative = vec![client_info_attr(
        "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,-1]}}\0",
    )];
    let non_integer = vec![client_info_attr(
        "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,\"31\"]}}\0",
    )];
    let no_protocol_prefix = vec![client_info_attr(
        "{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31]}}\0",
    )];

    assert!(parse_realm_list_ticket_client_secret_like_cpp(&too_short).is_none());
    assert!(parse_realm_list_ticket_client_secret_like_cpp(&too_long).is_none());
    assert!(parse_realm_list_ticket_client_secret_like_cpp(&out_of_range).is_none());
    assert!(parse_realm_list_ticket_client_secret_like_cpp(&negative).is_none());
    assert!(parse_realm_list_ticket_client_secret_like_cpp(&non_integer).is_none());
    assert!(parse_realm_list_ticket_client_secret_like_cpp(&no_protocol_prefix).is_none());
}

#[test]
fn selected_game_account_like_cpp_uses_identity_selection_not_hashmap_order() {
    let mut game_accounts = HashMap::new();
    game_accounts.insert(1, test_game_account(1, "2#1"));
    game_accounts.insert(42, test_game_account(42, "2#42"));
    let account = AccountInfo {
        id: 2,
        login: "user@example.test".to_string(),
        is_locked_to_ip: false,
        lock_country: String::new(),
        last_ip: String::new(),
        failed_logins: 0,
        is_banned: false,
        is_permanently_banned: false,
        game_accounts,
    };

    let selected = selected_game_account_like_cpp(&account, Some(42)).unwrap();
    assert_eq!(selected.name, "2#42");
    assert!(selected_game_account_like_cpp(&account, Some(7)).is_err());
    assert!(selected_game_account_like_cpp(&account, None).is_err());
}

#[test]
fn missing_account_info_returns_caller_status_like_cpp() {
    let err =
        account_info_or_status_like_cpp(None, status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS)
            .expect_err("missing account info must become the caller's BNet status");
    let status_err = err
        .downcast_ref::<RpcStatusError>()
        .expect("expected RpcStatusError");
    assert_eq!(
        status_err.status(),
        status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS
    );

    let err = account_info_or_status_like_cpp(None, status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT)
        .expect_err("missing game account context must become BAD_WOW_ACCOUNT");
    let status_err = err
        .downcast_ref::<RpcStatusError>()
        .expect("expected RpcStatusError");
    assert_eq!(
        status_err.status(),
        status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT
    );
}

#[test]
fn realm_utility_status_constants_match_cpp() {
    assert_eq!(status::ERROR_DENIED, 3);
    assert_eq!(status::ERROR_RPC_MALFORMED_REQUEST, 0x0000_0BC5);
    assert_eq!(status::ERROR_RPC_NOT_IMPLEMENTED, 0x0000_0BC7);
    assert_eq!(status::ERROR_UTIL_SERVER_UNKNOWN_REALM, 0x8000_0069);
    assert_eq!(status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS, 0x8000_006E);
    assert_eq!(
        status::ERROR_UTIL_SERVER_FAILED_TO_SERIALIZE_RESPONSE,
        0x8000_0073
    );
    assert_eq!(status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT, 0x8000_00D3);
    assert_eq!(
        status::ERROR_USER_SERVER_NOT_PERMITTED_ON_REALM,
        0x8000_00E1
    );
    assert_eq!(status::ERROR_WOW_SERVICES_INVALID_JOIN_TICKET, 0x8000_012E);
    assert_eq!(
        status::ERROR_WOW_SERVICES_DENIED_REALM_LIST_TICKET,
        0x8000_0132
    );
}

#[test]
fn get_all_values_for_attribute_auth_and_prefix_match_cpp() {
    assert!(should_write_sub_regions_like_cpp(true, "Command_RealmListRequest_v1").unwrap());
    assert!(should_write_sub_regions_like_cpp(true, "Command_RealmListRequest_v1_wotlk1").unwrap());
    assert!(!should_write_sub_regions_like_cpp(true, "Other_Command_RealmListRequest_v1").unwrap());
    assert!(!should_write_sub_regions_like_cpp(true, "Command_Other_v1").unwrap());

    let err = should_write_sub_regions_like_cpp(false, "Command_RealmListRequest_v1")
        .expect_err("unauthenticated requests must be denied like C++");
    let status = err
        .downcast_ref::<RpcStatusError>()
        .expect("expected RpcStatusError");
    assert_eq!(status.status(), status::ERROR_DENIED);
}

#[test]
fn process_client_request_dispatch_statuses_match_cpp() {
    let err = process_client_request_command_like_cpp(false, Some("Command_RealmListRequest_v1"))
        .expect_err("unauthenticated ProcessClientRequest must be denied before dispatch");
    assert_eq!(
        err.downcast_ref::<RpcStatusError>().unwrap().status(),
        status::ERROR_DENIED
    );

    let err = process_client_request_command_like_cpp(true, None)
        .expect_err("missing command must be malformed like C++");
    assert_eq!(
        err.downcast_ref::<RpcStatusError>().unwrap().status(),
        status::ERROR_RPC_MALFORMED_REQUEST
    );

    let err = process_client_request_command_like_cpp(true, Some("Command_Unknown_v1"))
        .expect_err("unknown command must be not implemented like C++");
    assert_eq!(
        err.downcast_ref::<RpcStatusError>().unwrap().status(),
        status::ERROR_RPC_NOT_IMPLEMENTED
    );

    let suffixed = remove_suffix("Command_RealmListRequest_v1_wotlk1");
    assert_eq!(
        process_client_request_command_like_cpp(true, Some(suffixed)).unwrap(),
        "Command_RealmListRequest_v1"
    );
}

#[test]
fn process_client_request_attribute_selection_matches_cpp_last_wins() {
    let attrs = vec![
        Attribute {
            name: "Command_RealmListRequest_v1_wotlk1".to_string(),
            value: Variant {
                string_value: Some("first".to_string()),
                ..Default::default()
            },
        },
        Attribute {
            name: "Param_RealmAddress".to_string(),
            value: Variant {
                uint_value: Some(1),
                ..Default::default()
            },
        },
        Attribute {
            name: "Command_RealmJoinRequest_v1_wotlk1".to_string(),
            value: Variant {
                string_value: Some("second".to_string()),
                ..Default::default()
            },
        },
        Attribute {
            name: "Command_RealmListRequest_v1_other".to_string(),
            value: Variant {
                string_value: Some("last-list".to_string()),
                ..Default::default()
            },
        },
        Attribute {
            name: "Param_RealmAddress".to_string(),
            value: Variant {
                uint_value: Some(2),
                ..Default::default()
            },
        },
    ];

    let command = find_command_attr_like_cpp(&attrs).expect("expected command attr");
    assert_eq!(command.name, "Command_RealmListRequest_v1_other");

    let realm_list = find_command_param_like_cpp(&attrs, "Command_RealmListRequest_v1").unwrap();
    assert_eq!(realm_list.value.string_value.as_deref(), Some("last-list"));

    let realm_address = find_param_like_cpp(&attrs, "Param_RealmAddress").unwrap();
    assert_eq!(realm_address.value.uint_value, Some(2));
}

#[test]
fn command_param_matching_uses_cpp_remove_suffix_not_prefix_contains() {
    let attrs = vec![Attribute {
        name: "Command_RealmListRequest_v1_extra_suffix".to_string(),
        value: Variant {
            string_value: Some("wrong".to_string()),
            ..Default::default()
        },
    }];

    assert!(
        find_command_param_like_cpp(&attrs, "Command_RealmListRequest_v1").is_none(),
        "C++ removeSuffix strips only the final suffix, so this key becomes Command_RealmListRequest_v1_extra"
    );
}

#[test]
fn locale_string_to_id_matches_cpp_get_locale_by_name() {
    let locales = [
        ("enUS", 0),
        ("koKR", 1),
        ("frFR", 2),
        ("deDE", 3),
        ("zhCN", 4),
        ("zhTW", 5),
        ("esES", 6),
        ("esMX", 7),
        ("ruRU", 8),
        ("none", 9),
        ("ptBR", 10),
        ("itIT", 11),
        ("bad", 12),
    ];

    for (locale, id) in locales {
        assert_eq!(locale_string_to_id_like_cpp(locale), id, "{locale}");
    }
}

#[test]
fn bnet_last_login_info_update_binds_locale_as_u8_like_cpp() {
    let update = BnetLastLoginInfoUpdateLikeCpp {
        client_ip: "203.0.113.44".to_string(),
        locale: locale_string_to_id_like_cpp("esES"),
        os: "Win".to_string(),
        account_id: 77,
    };

    let mut stmt = PreparedStatement::with_capacity_like_cpp(
        "UPDATE battlenet_accounts SET last_ip = ?, last_login = NOW(), locale = ?, failed_logins = 0, os = ? WHERE id = ?",
        4,
    );
    apply_bnet_last_login_info_update_like_cpp(&mut stmt, &update);

    assert_eq!(stmt.params().len(), 4);
    assert_eq!(
        stmt.params()[0],
        SqlParam::String("203.0.113.44".to_string())
    );
    assert_eq!(stmt.params()[1], SqlParam::U8(6));
    assert_eq!(stmt.params()[2], SqlParam::String("Win".to_string()));
    assert_eq!(stmt.params()[3], SqlParam::U32(77));
}

#[test]
fn join_realm_login_info_update_binds_cpp_statement_params_in_order() {
    let update = JoinRealmLoginInfoUpdateLikeCpp {
        key_data: (0..64).collect(),
        client_ip: "203.0.113.44".to_string(),
        locale: 6,
        os: "Win".to_string(),
        timezone_offset: -60,
        account_name: "2#1".to_string(),
    };

    let mut stmt = PreparedStatement::with_capacity_like_cpp(
        "UPDATE account SET session_key_bnet = ?, last_ip = ?, locale = ?, os = ?, timezone_offset = ? WHERE username = ?",
        6,
    );
    apply_join_realm_login_info_update_like_cpp(&mut stmt, &update);

    assert_eq!(stmt.params().len(), 6);
    assert_eq!(stmt.params()[0], SqlParam::Bytes((0..64).collect()));
    assert_eq!(
        stmt.params()[1],
        SqlParam::String("203.0.113.44".to_string())
    );
    assert_eq!(stmt.params()[2], SqlParam::U8(6));
    assert_eq!(stmt.params()[3], SqlParam::String("Win".to_string()));
    assert_eq!(stmt.params()[4], SqlParam::I16(-60));
    assert_eq!(stmt.params()[5], SqlParam::String("2#1".to_string()));
}

#[test]
fn join_realm_response_attributes_match_cpp_order_and_blob_values() {
    let server_addresses = vec![1, 2, 3, 4];
    let server_secret: Vec<u8> = (32..64).collect();

    let attrs = join_realm_response_attributes_like_cpp("2#1", &server_addresses, &server_secret);

    assert_eq!(attrs.len(), 3);
    assert_eq!(attrs[0].name, "Param_RealmJoinTicket");
    assert_eq!(
        attrs[0].value.blob_value.as_deref(),
        Some(b"2#1".as_slice())
    );
    assert_eq!(attrs[1].name, "Param_ServerAddresses");
    assert_eq!(
        attrs[1].value.blob_value.as_deref(),
        Some(server_addresses.as_slice())
    );
    assert_eq!(attrs[2].name, "Param_JoinSecret");
    assert_eq!(
        attrs[2].value.blob_value.as_deref(),
        Some(server_secret.as_slice())
    );
}

#[test]
fn last_char_played_response_attributes_match_cpp_order_and_value_kinds() {
    let realm_entry = vec![1, 2, 3, 4];
    let last_played = LastPlayedCharInfo {
        realm_address: 0x0102_0003,
        character_name: "Tester".to_string(),
        character_guid: 0x0102_0304_0506_0708,
        last_played_time: 0xFFFF_FFFE,
    };

    let attrs = last_char_played_response_attributes_like_cpp(&realm_entry, &last_played);

    assert_eq!(attrs.len(), 4);
    assert_eq!(attrs[0].name, "Param_RealmEntry");
    assert_eq!(
        attrs[0].value.blob_value.as_deref(),
        Some(&[1, 2, 3, 4][..])
    );
    assert_eq!(attrs[1].name, "Param_CharacterName");
    assert_eq!(attrs[1].value.string_value.as_deref(), Some("Tester"));
    assert_eq!(attrs[2].name, "Param_CharacterGUID");
    assert_eq!(
        attrs[2].value.blob_value.as_deref(),
        Some(&0x0102_0304_0506_0708u64.to_le_bytes()[..])
    );
    assert!(attrs[2].value.uint_value.is_none());
    assert_eq!(attrs[3].name, "Param_LastPlayedTime");
    assert_eq!(attrs[3].value.int_value, Some(-2));
    assert!(attrs[3].value.uint_value.is_none());
}
