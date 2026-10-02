use super::*;
use crate::realm::forever::test_fixture::*;

#[test]
fn explicit_routing_validates_and_never_falls_back_to_an_unrelated_ruleset() {
    let (catalog, _) = fixture();
    assert_eq!(catalog.realm_for_content(136), Some(ADDRESS));
    assert_eq!(catalog.realm_for_content(137), None);
    for config in [
        "{}".to_string(),
        BINDINGS.replace("33619969", "0"),
        BINDINGS.replace("33619969", "33619968"),
        BINDINGS.replace("136", "-1"),
        BINDINGS.replace("136", "2147483648"),
        BINDINGS.replace("\"superDistrictID\":1", "\"superDistrictID\":2"),
        BINDINGS.replace("\"contentSetID\"", "\"contentSetId\""),
        format!(
            "[{},{}]",
            &BINDINGS[1..BINDINGS.len() - 1],
            &BINDINGS[1..BINDINGS.len() - 1]
        ),
        " ".repeat(16385),
    ] {
        assert!(
            ForeverCatalog::parse(DISTRICTS)
                .unwrap()
                .with_realm_bindings(&config)
                .is_err()
        );
    }
    assert!(
        ForeverCatalog::default()
            .with_realm_bindings(BINDINGS)
            .is_err()
    );
    let oversized: Vec<_> = (0..65)
        .map(|n| {
            json!({
                "realmAddress": ADDRESS + n, "contentSetID": 136 + n, "superDistrictID": 1,
            })
        })
        .collect();
    assert!(
        ForeverCatalog::parse(DISTRICTS)
            .unwrap()
            .with_realm_bindings(&serde_json::to_string(&oversized).unwrap())
            .is_err()
    );
}

#[test]
fn entry_has_exact_70170_json_keys_types_and_compression_envelope() {
    let (catalog, realms) = fixture();
    let data = catalog
        .realm_entry(&realms, ADDRESS, 70170, 0, 100)
        .unwrap();
    let text = inflate(&data);
    let value: serde_json::Value = serde_json::from_str(
        text.strip_prefix("JamJSONRealmEntry:")
            .unwrap()
            .trim_end_matches('\0'),
    )
    .unwrap();
    assert_eq!(
        value,
        json!({
            "wowRealmAddress": ADDRESS, "cfgTimezonesID": 1, "populationState": 1,
            "cfgCategoriesID": 1, "version": { "versionMajor": 1, "versionMinor": 60,
                "versionRevision": 1, "versionBuild": 70170 },
            "cfgRealmsID": 1, "flags": 0, "name": "Forever test", "cfgConfigsID": 2,
            "cfgLanguagesID": 1, "cfgContentSetID": 136, "superDistrictID": 1,
            "useBleepChance": 0.0,
        })
    );
}

#[test]
fn unavailable_or_mismatched_realms_are_not_recommended() {
    let (catalog, mut realms) = fixture();
    assert!(
        catalog
            .realm_entry(&realms, ADDRESS, 54261, 0, 100)
            .is_none()
    );
    for flag in [Flags::OFFLINE, Flags::OFFLINE | Flags::RECOMMENDED] {
        realms.realms.values_mut().next().unwrap().flag = flag;
        assert!(
            catalog
                .realm_entry(&realms, ADDRESS, 70170, 0, 100)
                .is_none()
        );
    }
    let realm = realms.realms.values_mut().next().unwrap();
    realm.flag = Flags::NONE;
    realm.allowed_security_level = 1;
    assert!(
        catalog
            .realm_entry(&realms, ADDRESS, 70170, 0, 100)
            .is_none()
    );
    assert!(
        catalog
            .realm_entry(&realms, ADDRESS, 70170, 1, 100)
            .is_some()
    );
    realms.realms.values_mut().next().unwrap().region = 1;
    assert!(
        catalog
            .realm_entry(&realms, ADDRESS, 70170, 1, 100)
            .is_none()
    );
    realms.realms.values_mut().next().unwrap().region = 2;
    realms.realms.values_mut().next().unwrap().build = 54261;
    assert!(
        catalog
            .realm_entry(&realms, ADDRESS, 70170, 1, 100)
            .is_none()
    );
    realms.realms.values_mut().next().unwrap().build = 70170;
    realms.builds.clear();
    assert!(
        catalog
            .realm_entry(&realms, ADDRESS, 70170, 1, 100)
            .is_none()
    );
}

#[test]
fn district_hold_and_legacy_population_conversion_are_explicit() {
    let (_, realms) = fixture();
    for districts in [
        DISTRICTS.replace("false", "true"),
        DISTRICTS.replace("Time\":0", "Time\":101"),
    ] {
        let catalog = ForeverCatalog::parse(&districts)
            .unwrap()
            .with_realm_bindings(BINDINGS)
            .unwrap();
        assert!(
            catalog
                .realm_entry(&realms, ADDRESS, 70170, 0, 100)
                .is_none()
        );
    }
    let held = ForeverCatalog::parse(&DISTRICTS.replace("Time\":0", "Time\":100"))
        .unwrap()
        .with_realm_bindings(BINDINGS)
        .unwrap();
    assert!(held.realm_entry(&realms, ADDRESS, 70170, 0, 100).is_some());
    for (flags, value, expected) in [
        (Flags::NONE, 0.33, 1),
        (Flags::NONE, 0.34, 2),
        (Flags::NONE, 0.67, 3),
        (Flags::NONE, 0.96, 6),
        (Flags::NEW, 1.0, 4),
        (Flags::RECOMMENDED | Flags::NEW, 1.0, 5),
        (Flags::FULL, 0.0, 6),
    ] {
        assert_eq!(population(flags, value), expected);
    }
}
