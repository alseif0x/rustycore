use super::*;
use wow_data::forever_initialization::{CLASS_HASH, ClassRecord, InitializationRecords};
use wow_persistence::forever::creation::templates::{TemplateClassRow, TemplateRow};

fn initialization(removed: bool) -> InitializationCatalog {
    let records = InitializationRecords {
        classes: [1, 2]
            .into_iter()
            .map(|id| ClassRecord {
                id,
                flags: 0,
                starting_level: 1,
                cinematic: 0,
                default_spec: 0,
                strength_bonus: 0,
                primary_stat_priority: 0,
                display_power: 0,
                ranged_attack_per_agility: 0,
                attack_per_agility: 0,
                attack_per_strength: 0,
                spell_class_set: 0,
            })
            .collect(),
        ..Default::default()
    };
    let removals = if removed {
        wow_data::Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(CLASS_HASH, 1, 2)])
    } else {
        Default::default()
    };
    records
        .finish(Default::default(), Default::default(), &removals)
        .unwrap()
}
fn template(id: u32, level: u8) -> TemplateRow {
    TemplateRow {
        id,
        level,
        name: "Synthetic ñ".into(),
        description: "Fixture 字".into(),
    }
}
fn class(template_id: u32, class: u8, faction_group: u8) -> TemplateClassRow {
    TemplateClassRow {
        template_id,
        class,
        faction_group,
    }
}

#[test]
fn valid_faction_masks_keep_extra_bits_and_both_teams_in_source_row_order() {
    let mut rows = TemplateRows {
        classes: (0..16).map(|mask| class(10, 1, mask)).collect(),
        templates: vec![template(10, 255)],
    };
    rows.classes.push(class(10, 1, 255));
    rows.classes.push(class(10, 3, 3)); // absent effective class
    let templates = CharacterTemplates::load(rows, &initialization(false)).unwrap();
    let template = templates.get(10).unwrap();
    assert_eq!(
        template
            .classes()
            .iter()
            .map(|row| row.faction_group())
            .collect::<Vec<_>>(),
        [3, 5, 7, 11, 13, 15, 255]
    );
    assert!(template.classes().iter().all(|row| row.class() == 1));
    assert_eq!(template.level(), 255);
    assert_eq!(
        (template.name(), template.description()),
        ("Synthetic ñ", "Fixture 字")
    );
}

#[test]
fn no_valid_class_means_skipped_template_not_a_zero_class_or_fabricated_entry() {
    let make = || TemplateRows {
        classes: vec![class(10, 1, 3), class(20, 2, 1), class(30, 3, 3)],
        templates: vec![
            template(10, 10),
            template(20, 20),
            template(30, 30),
            template(40, 40),
        ],
    };
    let templates = CharacterTemplates::load(make(), &initialization(false)).unwrap();
    assert_eq!(templates.count(), 1);
    assert!(
        templates.get(10).is_some() && templates.get(20).is_none() && templates.get(40).is_none()
    );
    let templates = CharacterTemplates::load(make(), &initialization(true)).unwrap();
    assert_eq!(templates.count(), 0); // removed effective class is no longer present
    assert_eq!(
        CharacterTemplates::load(Default::default(), &initialization(false))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn source_zero_level_ids_and_class_rows_are_retained_without_new_cap_or_dedup_rules() {
    let templates = CharacterTemplates::load(
        TemplateRows {
            classes: vec![class(0, 1, 3), class(0, 1, 3), class(u32::MAX, 2, 5)],
            templates: vec![template(0, 0), template(u32::MAX, 255)],
        },
        &initialization(false),
    )
    .unwrap();
    assert_eq!(templates.get(0).unwrap().level(), 0);
    assert_eq!(templates.get(0).unwrap().classes().len(), 2);
    assert_eq!(templates.get(u32::MAX).unwrap().level(), 255);
    assert_eq!(
        templates.iter().map(|row| row.id()).collect::<HashSet<_>>(),
        [0, u32::MAX].into_iter().collect()
    );
}

#[test]
fn duplicate_primary_ids_fail_whole_batch_even_if_the_duplicate_has_no_classes() {
    assert!(matches!(
        CharacterTemplates::load(
            TemplateRows {
                classes: vec![class(10, 1, 3)],
                templates: vec![template(10, 10), template(10, 20)],
            },
            &initialization(false)
        ),
        Err(SourceError::DuplicateIdentity)
    ));
}
