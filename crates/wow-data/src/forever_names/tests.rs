use super::*;

fn names() -> NameRecords {
    NameRecords {
        profanity: vec![
            Profanity {
                id: 20,
                name: "explicit-locale".into(),
                language: 2,
            },
            Profanity {
                id: 10,
                name: "all-but-none".into(),
                language: -1,
            },
            Profanity {
                id: 30,
                name: "none-locale".into(),
                language: NONE_LOCALE as i8,
            },
        ],
        reserved: vec![Reserved {
            id: 4,
            name: "global-reserved".into(),
        }],
        locale_reserved: vec![LocaleReserved {
            id: 7,
            name: "local-reserved".into(),
            locale_mask: 0b0000_0101,
        }],
        categories: vec![Category {
            id: 100,
            creation_charset: 8,
        }],
    }
}

fn empty_removals() -> Db2HotfixRemovalStoreLikeCpp {
    Db2HotfixRemovalStoreLikeCpp::default()
}

#[test]
fn language_catalogs_are_sorted_and_none_global_excludes_locale_nine() {
    let catalog = names()
        .finish(Default::default(), Default::default(), &empty_removals())
        .unwrap();
    assert_eq!(
        catalog
            .profanity(0)
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["all-but-none"]
    );
    assert_eq!(
        catalog
            .profanity(2)
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["all-but-none", "explicit-locale"]
    );
    assert_eq!(
        catalog
            .profanity(NONE_LOCALE)
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["none-locale"]
    );
    assert!(catalog.profanity(TOTAL_LOCALES).is_none());
}

#[test]
fn local_reserved_is_not_global_reserved_and_masks_only_eight_bits() {
    let catalog = names()
        .finish(Default::default(), Default::default(), &empty_removals())
        .unwrap();
    assert_eq!(
        catalog
            .reserved()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["global-reserved"]
    );
    assert_eq!(
        catalog
            .locale_reserved(0)
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["local-reserved"]
    );
    assert!(catalog.locale_reserved(1).unwrap().is_empty());
    assert_eq!(
        catalog
            .locale_reserved(2)
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["local-reserved"]
    );
    assert!(catalog.locale_reserved(8).unwrap().is_empty());
    assert!(catalog.locale_reserved(11).unwrap().is_empty());
    assert!(catalog.locale_reserved(TOTAL_LOCALES).is_none());
}

#[test]
fn category_falls_back_to_english_when_missing() {
    let catalog = names()
        .finish(Default::default(), Default::default(), &empty_removals())
        .unwrap();
    assert_eq!(catalog.creation_charset(100), 8);
    assert_eq!(catalog.creation_charset(999), ENGLISH_CHARSET);
}

#[test]
fn official_then_custom_replace_ascending_baseline_and_final_remove_wins() {
    let official = NameRecords {
        profanity: vec![Profanity {
            id: 10,
            name: "official".into(),
            language: 1,
        }],
        ..Default::default()
    };
    let custom = NameRecords {
        profanity: vec![Profanity {
            id: 10,
            name: "custom".into(),
            language: 0,
        }],
        ..Default::default()
    };
    let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (0xDA82_D96C, 20, 2),
        (0xDA82_D96C, 20, 1),
        (0xDA82_D96C, 30, 2),
        (0x25C1_CB13, 4, 2),
    ]);
    let catalog = names().finish(official, custom, &removals).unwrap();
    assert_eq!(
        catalog
            .profanity(0)
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["custom"]
    );
    assert_eq!(
        catalog
            .profanity(2)
            .unwrap()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["explicit-locale"]
    );
    assert!(catalog.profanity(NONE_LOCALE).unwrap().is_empty());
    assert!(catalog.reserved().is_empty());
}

#[test]
fn duplicate_ids_are_rejected_within_each_batch() {
    let duplicate = || NameRecords {
        reserved: vec![
            Reserved {
                id: 1,
                name: "a".into(),
            },
            Reserved {
                id: 1,
                name: "b".into(),
            },
        ],
        ..Default::default()
    };
    assert!(
        names()
            .finish(duplicate(), Default::default(), &empty_removals())
            .is_err()
    );
    assert!(
        names()
            .finish(Default::default(), duplicate(), &empty_removals())
            .is_err()
    );
    assert!(
        names()
            .finish(Default::default(), Default::default(), &empty_removals())
            .is_ok()
    );
}

#[test]
fn invalid_profanity_language_is_rejected() {
    for language in [-2, TOTAL_LOCALES as i8] {
        let invalid = NameRecords {
            profanity: vec![Profanity {
                id: 1,
                name: "invalid".into(),
                language,
            }],
            ..Default::default()
        };
        assert!(
            invalid
                .finish(Default::default(), Default::default(), &empty_removals())
                .is_err()
        );
    }
}

#[test]
fn invalid_language_is_checked_only_after_effective_overlays_and_removals() {
    let invalid_baseline = || NameRecords {
        profanity: vec![Profanity {
            id: 50,
            name: "invalid-baseline".into(),
            language: -2,
        }],
        ..Default::default()
    };
    let valid_overlay = || NameRecords {
        profanity: vec![Profanity {
            id: 50,
            name: "valid-overlay".into(),
            language: 1,
        }],
        ..Default::default()
    };
    assert!(
        invalid_baseline()
            .finish(valid_overlay(), Default::default(), &empty_removals())
            .is_ok()
    );

    let removed = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(0xDA82_D96C, 50, 2)]);
    assert!(
        invalid_baseline()
            .finish(Default::default(), Default::default(), &removed)
            .is_ok()
    );

    assert!(
        invalid_baseline()
            .finish(Default::default(), Default::default(), &empty_removals())
            .is_err()
    );

    let duplicate_invalid = NameRecords {
        profanity: vec![
            Profanity {
                id: 51,
                name: "invalid-one".into(),
                language: -2,
            },
            Profanity {
                id: 51,
                name: "invalid-two".into(),
                language: -2,
            },
        ],
        ..Default::default()
    };
    assert!(
        duplicate_invalid
            .finish(Default::default(), Default::default(), &empty_removals())
            .is_err()
    );
}
