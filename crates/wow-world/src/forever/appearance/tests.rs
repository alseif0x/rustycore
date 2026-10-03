use super::*;
use wow_data::forever_appearance::*;
use wow_packet::{WorldPacket, forever::character_create::CharacterCreatePayload};

fn req() -> Requirement {
    Requirement {
        id: 7,
        flags: 1,
        class_mask: 0,
        race_mask: [0; 2],
        achievement: 0,
        quest: 0,
        item_appearance: 0,
    }
}
fn create(race: u8, class: u8, sex: u8, pairs: &[(u32, u32)]) -> CharacterCreatePayload {
    let mut packet = WorldPacket::new_empty();
    packet.write_bits(0, 6);
    for _ in 0..4 {
        packet.write_bit(false);
    }
    packet.write_bits(0, 2);
    packet.write_bits(0, 6);
    packet.flush_bits();
    packet.write_uint8(race);
    packet.write_uint8(class);
    packet.write_uint8(sex);
    packet.write_uint32(pairs.len() as u32);
    packet.write_int32(-1);
    packet.write_int32(0);
    for &(option, choice) in pairs {
        packet.write_uint32(option);
        packet.write_uint32(choice);
    }
    CharacterCreatePayload::decode(&packet.into_data()).unwrap()
}
fn catalog(requirement: Requirement, option_req: bool) -> AppearanceCatalog {
    AppearanceRecords {
        models: vec![Model {
            id: 1,
            display: 100,
        }],
        race_models: vec![RaceModel {
            id: 1,
            race: 95,
            model: 1,
            sex: 0,
        }],
        options: vec![
            OptionRecord {
                id: 1,
                model: 1,
                requirement: if option_req { 7 } else { 0 },
            },
            OptionRecord {
                id: 2,
                model: 1,
                requirement: 0,
            },
            OptionRecord {
                id: 3,
                model: 1,
                requirement: 0,
            },
        ],
        choices: vec![
            Choice {
                id: 10,
                option: 1,
                requirement: if option_req { 0 } else { 7 },
            },
            Choice {
                id: 20,
                option: 2,
                requirement: 0,
            },
            Choice {
                id: 21,
                option: 2,
                requirement: 0,
            },
            Choice {
                id: 30,
                option: 3,
                requirement: 0,
            },
        ],
        requirements: vec![requirement],
        required_choices: vec![
            RequiredChoice {
                id: 1,
                choice: 20,
                requirement: 7,
            },
            RequiredChoice {
                id: 2,
                choice: 21,
                requirement: 7,
            },
            RequiredChoice {
                id: 3,
                choice: 30,
                requirement: 7,
            },
        ],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &Default::default())
    .unwrap()
}
fn valid(catalog: &AppearanceCatalog, pairs: &[(u32, u32)]) -> bool {
    validate_creation_appearance(catalog, &create(95, 1, 0, pairs), &BTreeSet::new())
}

#[test]
fn dependency_groups_are_all_required_with_any_choice_per_option() {
    let catalog = catalog(req(), false);
    assert!(!valid(&catalog, &[(1, 10)]));
    assert!(!valid(&catalog, &[(1, 10), (2, 20)]));
    assert!(valid(&catalog, &[(1, 10), (2, 20), (3, 30)]));
    assert!(valid(&catalog, &[(1, 10), (2, 21), (3, 30)]));
}
#[test]
fn option_requirements_skip_dependencies_and_absent_requirements_are_not_invented() {
    assert!(valid(&catalog(req(), true), &[(1, 10)]));
    let mut requirement = req();
    requirement.id = 99;
    assert!(valid(&catalog(requirement, false), &[(1, 10)]));
}
#[test]
fn flag_class_and_new_race_mask_word_are_source_backed() {
    let mut requirement = req();
    requirement.class_mask = 2;
    assert!(!valid(&catalog(requirement, true), &[(1, 10)]));
    requirement.class_mask = 1;
    requirement.race_mask = [0, 1];
    assert!(valid(&catalog(requirement, true), &[(1, 10)]));
    requirement.race_mask = [1, 0];
    assert!(!valid(&catalog(requirement, true), &[(1, 10)]));
    requirement.race_mask = [u32::MAX; 2];
    assert!(valid(&catalog(requirement, true), &[(1, 10)]));
    requirement.flags = 0;
    requirement.class_mask = 2;
    requirement.achievement = 1;
    assert!(valid(&catalog(requirement, false), &[(1, 10)]));
}
#[test]
fn achievements_and_creation_quests_reject_and_collection_ownership_is_real() {
    for field in 0..2 {
        let mut requirement = req();
        if field == 0 {
            requirement.achievement = 1;
        } else {
            requirement.quest = 1;
        }
        assert!(!valid(&catalog(requirement, true), &[(1, 10)]));
    }
    let mut requirement = req();
    requirement.item_appearance = 900;
    let catalog = catalog(requirement, true);
    assert!(!valid(&catalog, &[(1, 10)]));
    assert!(validate_creation_appearance(
        &catalog,
        &create(95, 1, 0, &[(1, 10)]),
        &BTreeSet::from([900])
    ));
}
#[test]
fn duplicate_zero_foreign_option_wrong_choice_gender_and_invalid_class_reject() {
    let catalog = catalog(req(), true);
    for pairs in [
        vec![(1, 10), (1, 10)],
        vec![(0, 10)],
        vec![(99, 10)],
        vec![(1, 20)],
    ] {
        assert!(!valid(&catalog, &pairs));
    }
    for (race, class, sex) in [(1, 1, 0), (95, 1, 1), (95, 0, 0), (95, 33, 0)] {
        assert!(!validate_creation_appearance(
            &catalog,
            &create(race, class, sex, &[(1, 10)]),
            &BTreeSet::new()
        ));
    }
    // Source does not require every available option to be submitted.
    assert!(valid(&catalog, &[]));
}
#[test]
fn race_bits_match_target_non_contiguous_and_forever_ids() {
    assert_eq!(race_bit(95), Some(32));
    assert_eq!(race_bit(96), Some(33));
    assert_eq!(race_bit(34), Some(11));
    assert_eq!(race_bit(70), Some(15));
    assert_eq!(race_bit(0), None);
    assert_eq!(race_bit(94), None);
}
