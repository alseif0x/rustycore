use super::*;

fn baseline() -> AppearanceRecords {
    AppearanceRecords {
        models: vec![
            Model {
                id: 10,
                display: 100,
            },
            Model {
                id: 11,
                display: 110,
            },
        ],
        races: vec![
            Race {
                id: 1,
                visual_parent: 0,
            },
            Race {
                id: 95,
                visual_parent: 1,
            },
            Race {
                id: 96,
                visual_parent: 1,
            },
        ],
        race_models: vec![
            RaceModel {
                id: 1,
                race: 1,
                model: 10,
                sex: 0,
            },
            RaceModel {
                id: 2,
                race: 1,
                model: 11,
                sex: 0,
            },
        ],
        options: vec![
            OptionRecord {
                id: 1,
                model: 10,
                requirement: 0,
            },
            OptionRecord {
                id: 2,
                model: 11,
                requirement: 0,
            },
        ],
        choices: vec![Choice {
            id: 20,
            option: 1,
            requirement: 0,
        }],
        required_choices: vec![
            RequiredChoice {
                id: 1,
                choice: 20,
                requirement: 7,
            },
            RequiredChoice {
                id: 2,
                choice: 999,
                requirement: 7,
            },
        ],
        ..Default::default()
    }
}
fn finish(records: AppearanceRecords) -> AppearanceCatalog {
    records
        .finish(Default::default(), Default::default(), &Default::default())
        .unwrap()
}

#[test]
fn indexes_preserve_source_last_model_append_options_and_single_visual_child() {
    let catalog = finish(baseline());
    assert_eq!(catalog.model(1, 0).unwrap().id, 11);
    assert!(catalog.model(96, 0).is_none()); // options inheritance does not copy model
    assert_eq!(
        catalog
            .options(1, 0)
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(catalog.options(96, 0).unwrap().len(), 2);
    assert!(catalog.options(95, 0).is_none()); // last derived race wins, not both
    assert!(catalog.options(1, 1).is_none());
    assert_eq!(catalog.required_choices(7).unwrap()[&1], [20]);
}

#[test]
fn official_then_custom_and_final_removals_are_applied_before_indexing() {
    let official = AppearanceRecords {
        models: vec![Model {
            id: 10,
            display: 200,
        }],
        ..Default::default()
    };
    let custom = AppearanceRecords {
        models: vec![Model {
            id: 10,
            display: 300,
        }],
        ..Default::default()
    };
    let removed = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
        (0x49349C6E, 10, 2),
        (0x49349C6E, 10, 1),
        (0x49349C6E, 11, 2),
        (0x681D0F3D, 20, 2),
    ]);
    let catalog = baseline().finish(official, custom, &removed).unwrap();
    assert_eq!(catalog.model(1, 0).unwrap().display, 300);
    assert_eq!(catalog.options(1, 0).unwrap().len(), 1);
    assert!(catalog.choices(1).is_none());
    assert!(catalog.required_choices(7).is_none());
}

#[test]
fn duplicate_ids_fail_in_every_batch_but_overlay_replacement_is_valid() {
    for batch in 0..3 {
        let duplicate = AppearanceRecords {
            choices: vec![
                Choice {
                    id: 1,
                    option: 1,
                    requirement: 0
                };
                2
            ],
            ..Default::default()
        };
        let result = match batch {
            0 => duplicate.finish(Default::default(), Default::default(), &Default::default()),
            1 => baseline().finish(duplicate, Default::default(), &Default::default()),
            _ => baseline().finish(Default::default(), duplicate, &Default::default()),
        };
        assert!(result.is_err());
    }
}

#[test]
fn absent_model_does_not_create_options_and_absent_options_remain_none() {
    let mut records = baseline();
    records.models.clear();
    assert!(finish(records).options(1, 0).is_none());
    let mut records = baseline();
    records.options.clear();
    let catalog = finish(records);
    assert!(catalog.model(1, 0).is_some());
    assert!(catalog.options(1, 0).is_none());
}
