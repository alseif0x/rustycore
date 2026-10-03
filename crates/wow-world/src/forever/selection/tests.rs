use super::*;
use wow_data::forever_appearance::{AppearanceRecords, Choice, Model, OptionRecord, RaceModel};
use wow_data::forever_initialization::{InitializationRecords, SpecializationRecord};
use wow_persistence::forever::selection::CustomizationRow;

pub(super) fn policy() -> SelectionPolicy {
    let appearance = AppearanceRecords {
        models: vec![Model {
            id: 1,
            display: 100,
        }],
        race_models: vec![RaceModel {
            id: 1,
            race: 1,
            model: 1,
            sex: 0,
        }],
        options: vec![OptionRecord {
            id: 10,
            model: 1,
            requirement: 0,
        }],
        choices: vec![Choice {
            id: 20,
            option: 10,
            requirement: 0,
        }],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &Default::default())
    .unwrap();
    let spec = |id, order_index| SpecializationRecord {
        id,
        class: 1,
        order_index,
        pet_talent_type: -1,
        role: 2,
        flags: 0,
        primary_stat_priority: -1,
        mastery_spells: [0; 2],
    };
    let initialization = InitializationRecords {
        specializations: vec![spec(101, 0), spec(70000, 4)],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &Default::default())
    .unwrap();
    SelectionPolicy {
        appearance: Arc::new(appearance),
        initialization: Arc::new(initialization),
        declined_names: false,
        super_district: 2,
        class_disable_mask: 0,
    }
}

fn row(guid: u64) -> CharacterRow {
    CharacterRow {
        guid,
        name: "Synthetic".into(),
        surname: "Fixture".into(),
        race: 1,
        class: 1,
        gender: 0,
        level: 7,
        personal_tabard: [-1; 5],
        ..Default::default()
    }
}

fn rows(characters: Vec<CharacterRow>) -> SelectionRows {
    SelectionRows {
        characters,
        customizations: vec![],
    }
}

#[test]
fn populated_projection_uses_target_guids_saved_spec_and_all_equipment_fields() {
    let mut character = row(17);
    character.guild = 99;
    character.slot = 8;
    character.zone = u16::MAX;
    character.map = 9;
    character.position = [1.25, -2.5, 3.75];
    character.create_time = -99;
    character.logout_time = 123;
    character.last_login_build = 70170;
    character.at_login = 0x20;
    character.personal_tabard = [1, 2, 3, 4, 5];
    for (slot, item) in character.equipment.iter_mut().enumerate() {
        *item = wow_persistence::forever::selection::VisualItemRow {
            item_id: slot as u32 + 1,
            visible_item_id: slot as u32 + 2,
            subclass: 3,
            inventory_type: 4,
            display_id: 5,
            display_enchant_id: 6,
            secondary_appearance_id: -7,
            sheathe_category: 8,
        };
    }
    let input = SelectionRows {
        characters: vec![character],
        customizations: vec![CustomizationRow {
            guid: 17,
            option: 10,
            choice: 20,
        }],
    };
    let projection = project(input, 0x02013ABC, &policy()).unwrap();
    let basic = &projection.characters[0].basic;
    assert_eq!(
        basic.guid,
        ObjectGuid::new(((2_u64 << 58) | (0x3ABC_u64 << 42)) as i64, 17)
    );
    assert_eq!(
        basic.guild_guid,
        ObjectGuid::new(((28_u64 << 58) | (0x3ABC_u64 << 42)) as i64, 99)
    );
    assert_eq!(basic.guild_club_member_id, 17 | (0xABC_u64 << 48));
    assert_eq!(basic.virtual_realm_address, 0x02013ABC);
    assert_eq!(basic.spec_id, 101); // default specialization is index 4, not 0
    assert_eq!(basic.list_position, 8);
    assert_eq!((basic.zone_id, basic.map_id), (65535, 9));
    assert_eq!(basic.preload_position, Position::new(1.25, -2.5, 3.75, 0.0));
    assert_eq!((basic.create_time, basic.last_active_time), (-99, 123));
    assert_eq!(basic.last_login_version, 70170);
    assert!(basic.first_login);
    assert_eq!(basic.name(), "Synthetic");
    assert_eq!(basic.surname(), "Fixture");
    assert_eq!(basic.super_district_id, 2);
    assert_eq!(basic.customizations.len(), 1);
    assert_eq!(basic.personal_tabard.background_color, 5);
    for (slot, item) in basic.visual_items.iter().enumerate() {
        assert_eq!(
            (item.item_id, item.transmogrified_item_id),
            (slot as u32 + 1, slot as u32 + 2)
        );
        assert_eq!(
            (
                item.subclass,
                item.inv_type,
                item.display_id,
                item.display_enchant_id,
                item.secondary_item_modified_appearance_id,
                item.sheathe_category
            ),
            (3, 4, 5, 6, -7, 8)
        );
    }
    assert_eq!(projection.max_level, 7);
    assert!(projection.legitimate.contains(&basic.guid));
    assert!(projection.recustomize.is_empty());
    assert_eq!(
        projection.characters[0]
            .restrictions_and_mails
            .no_rpe_reason,
        4
    );
}

#[test]
fn target_flag_priority_resurrection_and_billing_are_not_legacy_masks() {
    let mut character = row(17);
    character.player_flags = 0x20 | 0x10 | 0x02000000 | 0x10000 | 0x08000000 | 0x00800000 | 0x800;
    character.at_login = 1 | 4 | 8 | 0x20 | 0x40 | 0x80 | 0x100;
    character.active_ban_guid = 17;
    character.declined_genitive = Some("Fixture".into());
    let mut policy = policy();
    policy.declined_names = true;
    let projection = project(rows(vec![character]), 0x02010001, &policy).unwrap();
    let basic = &projection.characters[0].basic;
    assert_eq!(basic.flags, 2 | 0x100 | 0x4000 | 0x01000000 | 0x02000000);
    assert_eq!(basic.flags2, 1 | 0x40000 | 0x20000000 | 0x40000000);
    assert_eq!(basic.flags3, 2 | 0x08000000);
    assert!(projection.legitimate.is_empty());
    assert!(projection.recustomize.is_empty());
    for (at_login, expected) in [(0x40 | 0x80, 0x10000), (0x80, 0x100000)] {
        let mut character = row(17);
        character.at_login = at_login;
        assert_eq!(
            project(rows(vec![character]), 0x02010001, &policy)
                .unwrap()
                .characters[0]
                .basic
                .flags2,
            expected
        );
    }
}

#[test]
fn invalid_appearance_clears_choices_and_requests_only_missing_service_flag() {
    let mut character = row(17);
    character.player_flags = 0x02000000;
    let mut input = rows(vec![character.clone()]);
    input.customizations.push(CustomizationRow {
        guid: 17,
        option: 10,
        choice: 99,
    });
    let projection = project(input.clone(), 0x02010001, &policy()).unwrap();
    assert_eq!(projection.recustomize, vec![17]);
    assert_eq!(projection.characters[0].basic.flags2, 1); // source assignment
    assert!(projection.characters[0].basic.customizations.is_empty());
    input.characters[0].at_login = 0x40;
    let projection = project(input, 0x02010001, &policy()).unwrap();
    assert!(projection.recustomize.is_empty());
    assert_eq!(projection.characters[0].basic.flags2, 0x10000 | 0x40000);
    assert!(projection.characters[0].basic.customizations.is_empty());
}

#[test]
fn unsupported_live_pet_is_not_manufactured_but_ghost_pet_is_hidden() {
    let mut character = row(17);
    character.class = 3;
    character.pet_entry = 123;
    assert!(matches!(
        project(rows(vec![character.clone()]), 0x02010001, &policy()),
        Err(SessionError::Persistence(LoadError::UnsupportedState))
    ));
    character.player_flags = 0x10;
    let projection = project(rows(vec![character.clone()]), 0x02010001, &policy()).unwrap();
    assert_eq!(projection.characters[0].basic.flags & 0x2000, 0x2000);
    assert_eq!(projection.characters[0].basic.pet_creature_family_id, 0);
    character.at_login = 0x100;
    assert!(matches!(
        project(rows(vec![character]), 0x02010001, &policy()),
        Err(SessionError::Persistence(LoadError::UnsupportedState))
    ));
}

#[test]
fn local_source_limit_is_truncation_but_invalid_identity_is_fail_closed() {
    let projection = project(rows((1..=201).map(row).collect()), 0x02010001, &policy()).unwrap();
    assert_eq!(projection.characters.len(), 200);
    assert_eq!(projection.legitimate.len(), 200);
    for characters in [vec![row(0)], vec![row(1 << 40)], vec![row(17), row(17)]] {
        assert!(matches!(
            project(rows(characters), 0x02010001, &policy()),
            Err(SessionError::Persistence(LoadError::InvalidRow))
        ));
    }
}

#[test]
fn saved_spec_uses_source_uint16_wire_conversion_and_missing_spec_zero() {
    let mut character = row(17);
    character.active_talent_group = 4;
    assert_eq!(
        project(rows(vec![character.clone()]), 0x02010001, &policy())
            .unwrap()
            .characters[0]
            .basic
            .spec_id,
        70000_u32 as u16
    );
    character.active_talent_group = 3;
    assert_eq!(
        project(rows(vec![character]), 0x02010001, &policy())
            .unwrap()
            .characters[0]
            .basic
            .spec_id,
        0
    );
}

#[test]
fn content_set_ruleset_mapping_is_target_classic_not_retail_icon() {
    for (content, expected) in [(136, 1), (137, 2), (138, 3), (140, 4), (0, 5), (999, 5)] {
        assert_eq!(super_district_for_content_set(content, 5), expected);
    }
}

#[test]
fn holder_choices_are_joined_by_guid_not_position_or_shared_across_characters() {
    let mut input = rows(vec![row(17), row(18)]);
    input.customizations = vec![
        CustomizationRow {
            guid: 17,
            option: 10,
            choice: 20,
        },
        CustomizationRow {
            guid: 18,
            option: 10,
            choice: 99,
        },
        CustomizationRow {
            guid: 99,
            option: 10,
            choice: 99,
        },
    ];
    let projection = project(input, 0x02010001, &policy()).unwrap();
    assert_eq!(projection.characters[0].basic.customizations.len(), 1);
    assert!(projection.characters[1].basic.customizations.is_empty());
    assert_eq!(projection.recustomize, vec![18]);
    assert_eq!(projection.legitimate.len(), 2);
}
