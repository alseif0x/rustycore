use super::*;
use crate::realm::RealmFlagsLikeCpp;
use crate::realm::forever::test_fixture::*;
use std::collections::HashMap;

fn game() -> GameAccountInfo {
    GameAccountInfo {
        id: 1,
        name: "1#1".into(),
        display_name: "WoW1".into(),
        ban_date: 0,
        unban_date: 0,
        is_permanently_banned: false,
        is_banned: false,
        security_level: 0,
        char_counts: HashMap::new(),
        last_played_chars: HashMap::new(),
    }
}

fn request(filter: i64) -> ClientRequest {
    ClientRequest {
        attribute: vec![
            Attribute {
                name: "Command_LastCharPlayedRequest_v1_classic".into(),
                value: Variant {
                    string_value: Some("70-1-70".into()),
                    ..Default::default()
                },
            },
            Attribute {
                name: "Param_ContentSetIDFilter".into(),
                value: Variant {
                    int_value: Some(filter),
                    ..Default::default()
                },
            },
        ],
        ..Default::default()
    }
}

#[test]
fn new_account_can_discover_only_its_explicit_ruleset_without_fake_character_data() {
    let (catalog, realms) = fixture();
    let account = game();
    for filter in [-1, 137, 0] {
        assert!(
            last_character(&catalog, &realms, &account, &request(filter), 100)
                .unwrap()
                .attribute
                .is_empty()
        );
    }
    let response = last_character(&catalog, &realms, &account, &request(136), 100).unwrap();
    assert_eq!(
        response
            .attribute
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        [
            "Param_RealmEntry",
            "Param_LastPlayedTime",
            "Param_UtilityInfo"
        ]
    );
    assert_eq!(response.attribute[1].value.int_value, Some(100));
    assert_eq!(
        inflate(response.attribute[2].value.blob_value.as_ref().unwrap()),
        "JSONUtilityInfo:{\"realmPermissions\":512}\0"
    );
    assert!(account.last_played_chars.is_empty());
    assert!(account.char_counts.is_empty());
}

#[test]
fn existing_character_keeps_real_guid_name_time_and_cannot_cross_rulesets() {
    let (catalog, realms) = fixture();
    let mut account = game();
    account.last_played_chars.insert(
        "2-1-0".into(),
        LastPlayedCharInfo {
            realm_address: ADDRESS,
            character_name: "Testchar".into(),
            character_guid: 42,
            last_played_time: 2200000000,
        },
    );
    let response = last_character(&catalog, &realms, &account, &request(136), 100).unwrap();
    assert_eq!(
        response.attribute[1].value.string_value.as_deref(),
        Some("Testchar")
    );
    assert_eq!(
        response.attribute[2].value.blob_value.as_deref(),
        Some(42u64.to_le_bytes().as_slice())
    );
    assert_eq!(response.attribute[3].value.int_value, Some(2200000000));
    assert_eq!(response.attribute[4].name, "Param_UtilityInfo");
    account
        .last_played_chars
        .values_mut()
        .next()
        .unwrap()
        .realm_address = 0x02010002;
    let response = last_character(&catalog, &realms, &account, &request(136), 100).unwrap();
    assert_eq!(response.attribute.len(), 3);
    assert_eq!(response.attribute[1].value.int_value, Some(100));
}

#[test]
fn missing_offline_or_forbidden_binding_never_publishes_an_entry() {
    let (catalog, mut realms) = fixture();
    let account = game();
    realms.realms.values_mut().next().unwrap().flag = RealmFlagsLikeCpp::OFFLINE;
    assert!(
        last_character(&catalog, &realms, &account, &request(136), 100)
            .unwrap()
            .attribute
            .is_empty()
    );
    realms.realms.values_mut().next().unwrap().flag = RealmFlagsLikeCpp::NONE;
    realms
        .realms
        .values_mut()
        .next()
        .unwrap()
        .allowed_security_level = 3;
    assert!(
        last_character(&catalog, &realms, &account, &request(136), 100)
            .unwrap()
            .attribute
            .is_empty()
    );
    assert!(
        last_character(
            &ForeverCatalog::default(),
            &realms,
            &account,
            &request(136),
            100
        )
        .unwrap()
        .attribute
        .is_empty()
    );
}

#[test]
fn malformed_filter_overflow_and_wrong_variant_are_rejected() {
    for filter in [-2, i64::MAX, i32::MAX as i64 + 1] {
        assert!(content_filter(&request(filter).attribute).is_err());
    }
    let mut wrong = request(136);
    wrong.attribute[1].value = Variant {
        bool_value: Some(true),
        ..Default::default()
    };
    assert!(content_filter(&wrong.attribute).is_err());
    wrong.attribute[1].value = Variant {
        string_value: Some("136".into()),
        ..Default::default()
    };
    assert!(content_filter(&wrong.attribute).is_err());
    assert_eq!(content_filter(&[]).unwrap(), None);
}

#[test]
fn modern_projection_preserves_first_content_filter_on_duplicate_parameters() {
    let make = |value| types::Attribute {
        name: Some("Param_ContentSetIDFilter".into()),
        value: Some(types::Variant {
            r#type: Some(Type::IntValue(value)),
        }),
    };
    let projected = project_attributes(vec![make(136), make(137)]);
    assert_eq!(content_filter(&projected.attribute).unwrap(), Some(136));
}
