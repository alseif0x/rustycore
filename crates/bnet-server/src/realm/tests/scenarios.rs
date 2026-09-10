//! Realm list regressions.
//!
//! Moved out of mod.rs under #685; every test is unchanged.

use super::*;

#[test]
fn realm_handle_address_matches_cpp_packing_and_lookup() {
    let handle = RealmHandleLikeCpp::new_like_cpp(5, 6, 9);
    let realm_address = handle.get_address_like_cpp();
    assert_eq!(realm_address, 0x0506_0009);
    assert_eq!(
        RealmHandleLikeCpp::from_address_like_cpp(realm_address),
        handle
    );
    assert_eq!(handle.get_address_string_like_cpp(), "5-6-9");
    assert_eq!(handle.get_sub_region_address_like_cpp(), "5-6-0");

    let same_realm_different_region = RealmHandleLikeCpp::new_like_cpp(1, 2, 9);
    let other_realm = RealmHandleLikeCpp::new_like_cpp(5, 6, 10);
    assert_eq!(handle, same_realm_different_region);
    assert_eq!(
        handle.cmp(&same_realm_different_region),
        std::cmp::Ordering::Equal
    );
    assert!(handle < other_realm);

    let mut manager = RealmManager::new();
    manager.realms.insert(
        RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
        test_realm(9, 5, 6, 1, 1),
    );
    assert_eq!(
        manager
            .get_realm_by_realm_address_like_cpp(realm_address)
            .map(|realm| realm.id),
        Some(9)
    );
}

#[test]
fn realm_manager_storage_key_matches_cpp_realm_only_ordering() {
    let mut manager = RealmManager::new();
    manager.realms.insert(
        RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
        test_realm(9, 5, 6, 1, 1),
    );
    let mut replacement = test_realm(9, 1, 2, 3, 1);
    replacement.name = "Replacement".to_string();
    manager
        .realms
        .insert(RealmHandleLikeCpp::new_like_cpp(1, 2, 9), replacement);

    assert_eq!(manager.realms.len(), 1);
    assert_eq!(
        manager
            .get_realm_by_realm_address_like_cpp(realm_address_like_cpp(5, 6, 9))
            .map(|realm| realm.name.as_str()),
        Some("Replacement")
    );
    assert_eq!(
        manager
            .get_realm_by_realm_address_like_cpp(realm_address_like_cpp(1, 2, 9))
            .map(|realm| realm.name.as_str()),
        Some("Replacement")
    );
}

#[test]
fn realm_names_strip_ascii_whitespace_like_cpp() {
    assert_eq!(
        normalized_realm_name_like_cpp("Ice Crown\t Citadel\n"),
        "IceCrownCitadel"
    );

    let mut manager = RealmManager::new();
    let mut realm = test_realm(9, 5, 6, 1, 1);
    realm.name = "Ice Crown".to_string();
    realm.normalized_name = normalized_realm_name_like_cpp(&realm.name);
    manager
        .realms
        .insert(RealmHandleLikeCpp::new_like_cpp(5, 6, 9), realm);

    assert_eq!(
        manager.get_realm_names_like_cpp(realm_address_like_cpp(5, 6, 9)),
        Some(("Ice Crown".to_string(), "IceCrown".to_string()))
    );
    assert_eq!(
        manager.get_realm_names_like_cpp(realm_address_like_cpp(5, 6, 10)),
        None
    );
}

#[test]
fn minor_major_bugfix_version_uses_cpp_lower_bound_semantics() {
    let mut manager = RealmManager::new();
    manager.builds = vec![
        test_build_info(51800, 3, 4, 2),
        test_build_info(51943, 3, 4, 3),
    ];

    assert_eq!(
        manager.get_minor_major_bugfix_version_for_build_like_cpp(51800),
        30_402
    );
    assert_eq!(
        manager.get_minor_major_bugfix_version_for_build_like_cpp(51801),
        30_403
    );
    assert_eq!(
        manager.get_minor_major_bugfix_version_for_build_like_cpp(99999),
        0
    );
}

#[test]
fn build_info_hotfix_and_auth_seeds_match_cpp_load_rules() {
    assert_eq!(parse_hotfix_version_like_cpp("ab"), [b'a', b'b', 0, 0]);
    assert_eq!(parse_hotfix_version_like_cpp("abcd"), [0; 4]);
    assert_eq!(parse_hotfix_version_like_cpp("abcde"), [0; 4]);

    assert_eq!(
        parse_auth_seed_like_cpp("000102030405060708090A0B0C0D0E0F"),
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]
    );
    assert_eq!(parse_auth_seed_like_cpp("000102"), [0; 16]);
    assert_eq!(
        parse_auth_seed_like_cpp("000102030405060708090A0B0C0D0E0Z"),
        [0; 16]
    );
}

#[test]
fn realm_flags_match_cpp_bits() {
    assert_eq!(RealmFlagsLikeCpp::NONE.bits(), 0x00);
    assert_eq!(RealmFlagsLikeCpp::VERSION_MISMATCH.bits(), 0x01);
    assert_eq!(RealmFlagsLikeCpp::OFFLINE.bits(), 0x02);
    assert_eq!(RealmFlagsLikeCpp::SPECIFYBUILD.bits(), 0x04);
    assert_eq!(RealmFlagsLikeCpp::UNK1.bits(), 0x08);
    assert_eq!(RealmFlagsLikeCpp::UNK2.bits(), 0x10);
    assert_eq!(RealmFlagsLikeCpp::RECOMMENDED.bits(), 0x20);
    assert_eq!(RealmFlagsLikeCpp::NEW.bits(), 0x40);
    assert_eq!(RealmFlagsLikeCpp::FULL.bits(), 0x80);
}

#[test]
fn realm_type_normalization_matches_cpp() {
    assert_eq!(RealmTypeLikeCpp::NORMAL.as_u8(), 0);
    assert_eq!(RealmTypeLikeCpp::PVP.as_u8(), 1);
    assert_eq!(RealmTypeLikeCpp::NORMAL2.as_u8(), 4);
    assert_eq!(RealmTypeLikeCpp::RP.as_u8(), 6);
    assert_eq!(RealmTypeLikeCpp::RPPVP.as_u8(), 8);
    assert_eq!(RealmTypeLikeCpp::MAX_CLIENT_REALM_TYPE, 14);
    assert_eq!(RealmTypeLikeCpp::FFA_PVP.as_u8(), 16);

    assert_eq!(
        RealmTypeLikeCpp::from_db_like_cpp(RealmTypeLikeCpp::FFA_PVP.as_u8()).as_u8(),
        RealmTypeLikeCpp::PVP.as_u8()
    );
    assert_eq!(
        RealmTypeLikeCpp::from_db_like_cpp(RealmTypeLikeCpp::MAX_CLIENT_REALM_TYPE).as_u8(),
        RealmTypeLikeCpp::NORMAL.as_u8()
    );
    assert_eq!(RealmTypeLikeCpp::from_db_like_cpp(13).as_u8(), 13);
    assert_eq!(
        RealmTypeLikeCpp::from_db_like_cpp(13).get_config_id_like_cpp(),
        14
    );
}

#[test]
fn realm_address_resolution_selects_first_ipv4_like_cpp() {
    let endpoints = [
        SocketAddr::new(IpAddr::V6("2001:db8::1".parse().unwrap()), 8085),
        SocketAddr::new(IpAddr::V4("203.0.113.10".parse().unwrap()), 8085),
        SocketAddr::new(IpAddr::V4("203.0.113.11".parse().unwrap()), 8085),
    ];

    assert_eq!(
        first_ipv4_address_like_cpp(endpoints),
        Some("203.0.113.10".parse().unwrap())
    );
    assert_eq!(
        first_ipv4_address_like_cpp([SocketAddr::new(
            IpAddr::V6("2001:db8::1".parse().unwrap()),
            8085
        )]),
        None
    );
}

#[test]
fn write_sub_regions_like_cpp_emits_string_values_in_order() {
    let mut manager = RealmManager::new();
    manager.sub_regions = vec!["5-6-0".to_string(), "7-8-0".to_string()];

    let values = manager.write_sub_regions_like_cpp();

    assert_eq!(values.len(), 2);
    assert_eq!(values[0].string_value.as_deref(), Some("5-6-0"));
    assert_eq!(values[1].string_value.as_deref(), Some("7-8-0"));
    assert!(values[0].blob_value.is_none());
    assert!(values[0].uint_value.is_none());
}

#[test]
fn realm_list_json_filters_subregion_and_uses_cpp_fields() {
    let mut manager = RealmManager::new();
    manager.realms.insert(
        RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
        test_realm(9, 5, 6, 3, 1),
    );
    manager.realms.insert(
        RealmHandleLikeCpp::new_like_cpp(7, 8, 10),
        test_realm(10, 7, 8, 4, 6),
    );
    manager.builds.push(test_build_info(51943, 3, 4, 3));

    let mut counts = HashMap::new();
    counts.insert(realm_address_like_cpp(5, 6, 9), 2);

    let (realms, char_counts) = manager.get_realm_list_json(51943, "5-6-0", &counts);
    let realms = inflate_payload(&realms);
    let json = parse_enveloped_json(&realms, "JSONRealmListUpdates:");
    let updates = json["updates"].as_array().unwrap();
    assert_eq!(updates.len(), 1);

    let update = &updates[0]["update"];
    assert_eq!(update["wowRealmAddress"], 0x0506_0009);
    assert_eq!(update["cfgTimezonesId"], 1);
    assert_eq!(update["cfgCategoriesId"], 3);
    assert_eq!(update["cfgConfigsId"], 2);
    assert_eq!(update["cfgRealmsId"], 9);
    assert_eq!(update["version"]["versionMajor"], 3);
    assert_eq!(update["version"]["versionMinor"], 4);
    assert_eq!(update["version"]["versionRevision"], 3);
    assert_eq!(update["version"]["versionBuild"], 51943);

    let char_counts = inflate_payload(&char_counts);
    let json = parse_enveloped_json(&char_counts, "JSONRealmCharacterCountList:");
    assert_eq!(json["counts"][0]["wowRealmAddress"], 0x0506_0009);
    assert_eq!(json["counts"][0]["count"], 2);
}

#[test]
fn realm_list_json_empty_payload_matches_cpp_envelopes() {
    let manager = RealmManager::new();

    let (realms, char_counts) = manager.get_realm_list_json(51943, "5-6-0", &HashMap::new());

    assert_eq!(
        inflate_payload(&realms),
        "JSONRealmListUpdates:{\"updates\":[]}\0"
    );
    assert_eq!(
        inflate_payload(&char_counts),
        "JSONRealmCharacterCountList:{\"counts\":[]}\0"
    );
}

#[test]
fn realm_list_json_uses_cpp_fallback_version_and_type_normalization() {
    let mut manager = RealmManager::new();
    manager.realms.insert(
        RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
        test_realm(9, 5, 6, 3, RealmTypeLikeCpp::FFA_PVP.as_u8()),
    );

    let (realms, _) = manager.get_realm_list_json(12340, "5-6-0", &HashMap::new());
    let realms = inflate_payload(&realms);
    let json = parse_enveloped_json(&realms, "JSONRealmListUpdates:");
    let update = &json["updates"][0]["update"];

    assert_eq!(update["flags"], RealmFlagsLikeCpp::VERSION_MISMATCH.bits());
    assert_eq!(update["cfgConfigsId"], 2);
    assert_eq!(update["version"]["versionMajor"], DEFAULT_VERSION_MAJOR);
    assert_eq!(update["version"]["versionMinor"], DEFAULT_VERSION_MINOR);
    assert_eq!(
        update["version"]["versionRevision"],
        DEFAULT_VERSION_REVISION
    );
}

#[test]
fn realm_list_json_offline_realm_has_zero_population_like_cpp() {
    let mut manager = RealmManager::new();
    let mut realm = test_realm(9, 5, 6, 3, 1);
    realm.flag = RealmFlagsLikeCpp::OFFLINE;
    manager
        .realms
        .insert(RealmHandleLikeCpp::new_like_cpp(5, 6, 9), realm);

    let (realms, _) = manager.get_realm_list_json(51943, "5-6-0", &HashMap::new());
    let realms = inflate_payload(&realms);
    let json = parse_enveloped_json(&realms, "JSONRealmListUpdates:");
    let update = &json["updates"][0]["update"];

    assert_eq!(update["populationState"], 0);
    assert_eq!(update["flags"], RealmFlagsLikeCpp::OFFLINE.bits());
}

#[test]
fn realm_entry_json_matches_cpp_envelope_and_empty_gates() {
    let mut manager = RealmManager::new();
    manager.realms.insert(
        RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
        test_realm(9, 5, 6, 3, 1),
    );
    manager.builds.push(test_build_info(51943, 3, 4, 3));

    let packed = realm_address_like_cpp(5, 6, 9);
    let entry = manager.get_realm_entry_json_like_cpp(packed, 51943);
    let entry = inflate_payload(&entry);
    let json = parse_enveloped_json(&entry, "JamJSONRealmEntry:");
    assert_eq!(json["wowRealmAddress"], 0x0506_0009);
    assert_eq!(json["cfgTimezonesId"], 1);
    assert_eq!(json["cfgCategoriesId"], 3);
    assert_eq!(json["populationState"], 2);
    assert_eq!(json["version"]["versionBuild"], 51943);

    assert!(
        manager
            .get_realm_entry_json_like_cpp(packed, 12340)
            .is_empty()
    );

    manager
        .realms
        .get_mut(&RealmHandleLikeCpp::new_like_cpp(5, 6, 9))
        .unwrap()
        .flag = RealmFlagsLikeCpp::OFFLINE;
    assert!(
        manager
            .get_realm_entry_json_like_cpp(packed, 51943)
            .is_empty()
    );
}

#[test]
fn server_addresses_json_selects_local_or_external_like_cpp() {
    let manager = RealmManager::new();
    let realm = test_realm(9, 5, 6, 3, 1);
    let fixture_networks = [wow_core::Ipv4NetworkLikeCpp::new(
        "10.0.0.1".parse().unwrap(),
        24,
    )];

    assert_eq!(
        select_realm_ip_str_with_local_networks(
            Some(std::net::IpAddr::V4("127.0.0.1".parse().unwrap())),
            &realm.external_address,
            &realm.local_address,
            &[],
        ),
        realm.local_address
    );
    assert_eq!(
        select_realm_ip_str_with_local_networks(
            Some(std::net::IpAddr::V4("10.0.0.42".parse().unwrap())),
            &realm.external_address,
            &realm.local_address,
            &[],
        ),
        realm.local_address
    );
    assert_eq!(
        select_realm_ip_str_with_local_networks(
            Some(std::net::IpAddr::V4("198.51.100.42".parse().unwrap())),
            &realm.external_address,
            &realm.local_address,
            &[],
        ),
        realm.external_address
    );

    let addresses = manager.get_realm_server_addresses_json_with_local_networks_like_cpp(
        &realm,
        Some("127.0.0.1".parse().unwrap()),
        &fixture_networks,
    );
    let addresses = inflate_payload(&addresses);
    let json = parse_enveloped_json(&addresses, "JSONRealmListServerIPAddresses:");
    assert_eq!(json["families"][0]["family"], 1);
    assert_eq!(json["families"][0]["addresses"][0]["ip"], "10.0.0.10");
    assert_eq!(json["families"][0]["addresses"][0]["port"], 8085);

    let addresses = manager.get_realm_server_addresses_json_with_local_networks_like_cpp(
        &realm,
        Some("198.51.100.42".parse().unwrap()),
        &fixture_networks,
    );
    let addresses = inflate_payload(&addresses);
    let json = parse_enveloped_json(&addresses, "JSONRealmListServerIPAddresses:");
    assert_eq!(json["families"][0]["addresses"][0]["ip"], "203.0.113.10");
}

#[test]
fn server_addresses_json_content_matches_cpp_envelope() {
    let manager = RealmManager::new();
    let realm = test_realm(9, 5, 6, 3, 1);
    let fixture_networks = [wow_core::Ipv4NetworkLikeCpp::new(
        "10.0.0.1".parse().unwrap(),
        24,
    )];

    let addresses = manager.get_realm_server_addresses_json_with_local_networks_like_cpp(
        &realm,
        Some("127.0.0.1".parse().unwrap()),
        &fixture_networks,
    );

    assert_eq!(
        inflate_payload(&addresses),
        "JSONRealmListServerIPAddresses:{\"families\":[{\"family\":1,\"addresses\":[{\"ip\":\"10.0.0.10\",\"port\":8085}]}]}\0"
    );
}

#[test]
fn prepare_join_realm_like_cpp_rejects_unknown_offline_and_build_mismatch() {
    let mut manager = RealmManager::new();
    let packed = realm_address_like_cpp(5, 6, 9);

    assert!(matches!(
        manager.prepare_join_realm_like_cpp(packed, 51943, None),
        Err(JoinRealmPrepareErrorLikeCpp::UnknownRealm)
    ));

    manager.realms.insert(
        RealmHandleLikeCpp::new_like_cpp(5, 6, 9),
        test_realm(9, 5, 6, 3, 1),
    );

    assert!(matches!(
        manager.prepare_join_realm_like_cpp(packed, 12340, None),
        Err(JoinRealmPrepareErrorLikeCpp::UserServerNotPermittedOnRealm)
    ));

    manager
        .realms
        .get_mut(&RealmHandleLikeCpp::new_like_cpp(5, 6, 9))
        .unwrap()
        .flag = RealmFlagsLikeCpp::OFFLINE;

    assert!(matches!(
        manager.prepare_join_realm_like_cpp(packed, 51943, None),
        Err(JoinRealmPrepareErrorLikeCpp::UserServerNotPermittedOnRealm)
    ));
}

#[test]
fn prepare_join_realm_like_cpp_returns_server_addresses_and_name() {
    let mut manager = RealmManager::new();
    let packed = realm_address_like_cpp(5, 6, 9);
    let mut realm = test_realm(9, 5, 6, 3, 1);
    realm.name = "Ice Crown".to_string();
    realm.external_address = "203.0.113.10".to_string();
    realm.local_address = "10.0.0.10".to_string();
    realm.port = 8086;
    manager
        .realms
        .insert(RealmHandleLikeCpp::new_like_cpp(5, 6, 9), realm);

    let prepared = manager
        .prepare_join_realm_like_cpp(packed, 51943, Some("198.51.100.1".parse().unwrap()))
        .unwrap();

    assert_eq!(prepared.realm_name, "Ice Crown");

    let addresses = inflate_payload(&prepared.server_addresses);
    let json = parse_enveloped_json(&addresses, "JSONRealmListServerIPAddresses:");
    assert_eq!(json["families"][0]["family"], 1);
    assert_eq!(json["families"][0]["addresses"][0]["ip"], "203.0.113.10");
    assert_eq!(json["families"][0]["addresses"][0]["port"], 8086);
}
