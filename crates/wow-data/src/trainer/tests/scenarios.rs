//! Trainer store regressions.
//!
//! Moved out of trainer.rs under #683; every test is unchanged.

use super::*;

#[test]
fn trainer_store_groups_spells_after_trainer_rows_like_cpp() {
    let outcome = from_rows_with_existing_references(
        [trainer_row(10), trainer_row(11)],
        [
            spell_row(10, 1000),
            spell_row(10, 1001),
            spell_row(11, 2000),
        ],
        [],
        [],
    );

    let trainer = outcome.store.get_trainer_like_cpp(10).unwrap();
    assert_eq!(
        trainer.trainer_type_like_cpp(),
        TRAINER_TYPE_TRADESKILL_LIKE_CPP
    );
    assert_eq!(
        trainer
            .spells_like_cpp()
            .iter()
            .map(|spell| spell.spell_id)
            .collect::<Vec<_>>(),
        vec![1000, 1001]
    );
    assert_eq!(trainer.get_spell_like_cpp(1001).unwrap().money_cost, 100);
    assert_eq!(outcome.report.trainer_rows, 2);
    assert_eq!(outcome.report.trainer_spell_rows, 3);
}

#[test]
fn trainer_store_reports_spells_without_existing_trainer_like_cpp() {
    let outcome =
        from_rows_with_existing_references([trainer_row(10)], [spell_row(99, 3000)], [], []);

    assert_eq!(outcome.store.spell_count_like_cpp(), 0);
    assert_eq!(
        outcome.report.skipped_spells_missing_trainer,
        vec![(99, 3000)]
    );
}

#[test]
fn trainer_locales_skip_enus_and_fallback_to_default_like_cpp() {
    let outcome = from_rows_with_existing_references(
        [trainer_row(10)],
        [],
        [
            TrainerLocaleRowLikeCpp {
                id: 10,
                locale: "enUS".to_string(),
                greeting: "Default locale ignored".to_string(),
            },
            TrainerLocaleRowLikeCpp {
                id: 10,
                locale: "esES".to_string(),
                greeting: "Hola".to_string(),
            },
            TrainerLocaleRowLikeCpp {
                id: 10,
                locale: "none".to_string(),
                greeting: "Invalid locale ignored".to_string(),
            },
        ],
        [],
    );

    let trainer = outcome.store.get_trainer_like_cpp(10).unwrap();
    assert_eq!(trainer.greeting_like_cpp(Locale::EnUS), "Hello 10");
    assert_eq!(trainer.greeting_like_cpp(Locale::EsES), "Hola");
    assert_eq!(trainer.greeting_like_cpp(Locale::FrFR), "Hello 10");
    assert_eq!(trainer.greeting_for_locale_name_like_cpp("esES"), "Hola");
    assert_eq!(
        trainer.greeting_for_locale_name_like_cpp("unsupported"),
        "Hello 10"
    );
    assert_eq!(
        trainer.greeting_for_locale_name_like_cpp("none"),
        "Hello 10"
    );
    assert_eq!(outcome.report.trainer_locale_entries, 1);
}

#[test]
fn trainer_locales_report_missing_trainer_like_cpp() {
    let outcome = from_rows_with_existing_references(
        [],
        [],
        [TrainerLocaleRowLikeCpp {
            id: 99,
            locale: "esES".to_string(),
            greeting: "Hola".to_string(),
        }],
        [],
    );

    assert_eq!(
        outcome.report.skipped_locales_missing_trainer,
        vec![(99, "esES".to_string())]
    );
}

#[test]
fn creature_trainer_map_matches_cpp_lookup_shape() {
    let outcome = from_rows_with_existing_references(
        [trainer_row(10), trainer_row(20)],
        [],
        [],
        [
            CreatureTrainerRowLikeCpp {
                creature_id: 100,
                trainer_id: 10,
                menu_id: 0,
                option_id: 0,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: 100,
                trainer_id: 20,
                menu_id: 7,
                option_id: 2,
            },
        ],
    );

    assert_eq!(outcome.store.get_creature_default_trainer_like_cpp(100), 10);
    assert_eq!(
        outcome
            .store
            .get_creature_trainer_for_gossip_option_like_cpp(100, 7, 2),
        20
    );
    assert_eq!(outcome.store.creature_trainer_count_like_cpp(), 2);
}

#[test]
fn creature_trainer_skips_missing_trainer_like_cpp() {
    let outcome = from_rows_with_existing_references(
        [],
        [],
        [],
        [CreatureTrainerRowLikeCpp {
            creature_id: 100,
            trainer_id: 99,
            menu_id: 7,
            option_id: 2,
        }],
    );

    assert_eq!(outcome.store.get_creature_default_trainer_like_cpp(100), 0);
    assert_eq!(
        outcome.report.skipped_creature_trainers_missing_trainer,
        vec![(100, 99, 7, 2)]
    );
}

#[test]
fn trainer_spell_validation_short_circuits_in_cpp_order() {
    let mut missing_main_spell = spell_row(10, 1000);
    missing_main_spell.spell.req_skill_line = 2000;
    missing_main_spell.spell.req_ability = [3000, 0, 4000];

    let main_spell_calls = RefCell::new(Vec::new());
    let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
        [trainer_row(10)],
        [missing_main_spell],
        [],
        [],
        |spell_id| {
            main_spell_calls.borrow_mut().push(spell_id);
            false
        },
        |_| panic!("missing main SpellId must short-circuit ReqSkillLine"),
        |_| true,
        |_, _| true,
    );
    assert_eq!(*main_spell_calls.borrow(), vec![1000]);
    assert_eq!(
        outcome.report.skipped_spells_missing_spell,
        vec![(10, 1000)]
    );
    assert!(outcome.report.skipped_spells_missing_skill_line.is_empty());
    assert!(
        outcome
            .report
            .skipped_spells_missing_required_spell
            .is_empty()
    );

    let mut missing_skill_line = spell_row(10, 1000);
    missing_skill_line.spell.req_skill_line = 2000;
    missing_skill_line.spell.req_ability = [3000, 0, 4000];

    let spell_calls = RefCell::new(Vec::new());
    let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
        [trainer_row(10)],
        [missing_skill_line],
        [],
        [],
        |spell_id| {
            spell_calls.borrow_mut().push(spell_id);
            true
        },
        |_| false,
        |_| true,
        |_, _| true,
    );
    assert_eq!(*spell_calls.borrow(), vec![1000]);
    assert!(outcome.report.skipped_spells_missing_spell.is_empty());
    assert_eq!(
        outcome.report.skipped_spells_missing_skill_line,
        vec![(10, 1000, 2000)]
    );
    assert!(
        outcome
            .report
            .skipped_spells_missing_required_spell
            .is_empty()
    );
}

#[test]
fn trainer_spell_accepts_existing_skill_and_required_spells_like_cpp() {
    let mut row = spell_row(10, 1000);
    row.spell.req_skill_line = 2000;
    row.spell.req_ability = [3000, 0, 4000];

    let spell_calls = RefCell::new(Vec::new());
    let skill_calls = RefCell::new(Vec::new());
    let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
        [trainer_row(10)],
        [row],
        [],
        [],
        |spell_id| {
            spell_calls.borrow_mut().push(spell_id);
            matches!(spell_id, 1000 | 3000 | 4000)
        },
        |skill_line_id| {
            skill_calls.borrow_mut().push(skill_line_id);
            skill_line_id == 2000
        },
        |_| true,
        |_, _| true,
    );

    assert_eq!(*spell_calls.borrow(), vec![1000, 3000, 4000]);
    assert_eq!(*skill_calls.borrow(), vec![2000]);
    assert!(
        outcome
            .store
            .get_trainer_like_cpp(10)
            .unwrap()
            .get_spell_like_cpp(1000)
            .is_some()
    );
    assert!(outcome.report.skipped_spells_missing_spell.is_empty());
    assert!(outcome.report.skipped_spells_missing_skill_line.is_empty());
    assert!(
        outcome
            .report
            .skipped_spells_missing_required_spell
            .is_empty()
    );
}

#[test]
fn trainer_spell_rejects_whole_row_and_reports_every_missing_required_spell_like_cpp() {
    let mut row = spell_row(10, 1000);
    row.spell.req_skill_line = 2000;
    row.spell.req_ability = [3000, 0, 4000];
    let mut one_missing_requirement = spell_row(10, 1001);
    one_missing_requirement.spell.req_skill_line = 2000;
    one_missing_requirement.spell.req_ability = [0, 3001, 0];

    let spell_calls = RefCell::new(Vec::new());
    let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
        [trainer_row(10)],
        [row, one_missing_requirement],
        [],
        [],
        |spell_id| {
            spell_calls.borrow_mut().push(spell_id);
            matches!(spell_id, 1000 | 1001)
        },
        |skill_line_id| skill_line_id == 2000,
        |_| true,
        |_, _| true,
    );

    assert_eq!(*spell_calls.borrow(), vec![1000, 3000, 4000, 1001, 3001]);
    assert_eq!(
        outcome.report.skipped_spells_missing_required_spell,
        vec![
            (10, 1000, 1, 3000),
            (10, 1000, 3, 4000),
            (10, 1001, 2, 3001)
        ]
    );
    assert_eq!(outcome.store.spell_count_like_cpp(), 0);
    assert!(outcome.report.skipped_spells_missing_trainer.is_empty());
}

#[test]
fn trainer_spell_zero_fields_skip_optional_lookups_like_cpp() {
    let mut row = spell_row(10, 1000);
    row.spell.money_cost = 0;
    row.spell.req_skill_line = 0;
    row.spell.req_skill_rank = 999;
    row.spell.req_ability = [0, 0, 0];
    row.spell.req_level = 0;

    let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
        [trainer_row(10)],
        [row, spell_row(10, 0)],
        [],
        [],
        |spell_id| spell_id == 1000,
        |_| panic!("ReqSkillLine zero must not be looked up"),
        |_| true,
        |_, _| true,
    );

    let spell = outcome
        .store
        .get_trainer_like_cpp(10)
        .unwrap()
        .get_spell_like_cpp(1000)
        .unwrap();
    assert_eq!(spell.money_cost, 0);
    assert_eq!(spell.req_skill_rank, 999);
    assert_eq!(spell.req_level, 0);
    assert_eq!(outcome.report.skipped_spells_missing_spell, vec![(10, 0)]);
}

#[test]
fn only_validated_orphan_trainer_spells_are_reported_like_cpp() {
    let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
        [],
        [spell_row(90, 1000), spell_row(91, 1001)],
        [],
        [],
        |spell_id| spell_id == 1000,
        |_| true,
        |_| true,
        |_, _| true,
    );

    assert_eq!(
        outcome.report.skipped_spells_missing_spell,
        vec![(91, 1001)]
    );
    assert_eq!(
        outcome.report.skipped_spells_missing_trainer,
        vec![(90, 1000)]
    );
}

#[test]
fn duplicate_trainer_rows_keep_first_definition_like_cpp() {
    let first = TrainerRowLikeCpp {
        id: 10,
        trainer_type: TRAINER_TYPE_TRADESKILL_LIKE_CPP,
        greeting: "First".to_string(),
    };
    let second = TrainerRowLikeCpp {
        id: 10,
        trainer_type: TRAINER_TYPE_PET_LIKE_CPP,
        greeting: "Second".to_string(),
    };
    let outcome =
        from_rows_with_existing_references([first, second], [spell_row(10, 1000)], [], []);

    let trainer = outcome.store.get_trainer_like_cpp(10).unwrap();
    assert_eq!(
        trainer.trainer_type_like_cpp(),
        TRAINER_TYPE_TRADESKILL_LIKE_CPP
    );
    assert_eq!(trainer.greeting_like_cpp(Locale::EnUS), "First");
    assert!(trainer.get_spell_like_cpp(1000).is_some());
    assert_eq!(outcome.store.len(), 1);
}

#[test]
fn creature_trainer_validation_short_circuits_and_preserves_zero_matrix_like_cpp() {
    let gossip_calls = RefCell::new(Vec::new());
    let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
        [trainer_row(10)],
        [],
        [],
        [
            CreatureTrainerRowLikeCpp {
                creature_id: 100,
                trainer_id: 99,
                menu_id: 7,
                option_id: 2,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: 101,
                trainer_id: 99,
                menu_id: 7,
                option_id: 2,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: 102,
                trainer_id: 10,
                menu_id: 7,
                option_id: 9,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: 103,
                trainer_id: 10,
                menu_id: 0,
                option_id: 0,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: 104,
                trainer_id: 10,
                menu_id: 7,
                option_id: 2,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: 105,
                trainer_id: 10,
                menu_id: 8,
                option_id: 0,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: 106,
                trainer_id: 10,
                menu_id: 0,
                option_id: 3,
            },
        ],
        |_| true,
        |_| true,
        |creature_id| creature_id != 100,
        |menu_id, option_id| {
            gossip_calls.borrow_mut().push((menu_id, option_id));
            matches!((menu_id, option_id), (7, 2) | (8, 0) | (0, 3))
        },
    );

    assert_eq!(
        outcome
            .report
            .skipped_creature_trainers_missing_creature_template,
        vec![(100, 99, 7, 2)]
    );
    assert_eq!(
        outcome.report.skipped_creature_trainers_missing_trainer,
        vec![(101, 99, 7, 2)]
    );
    assert_eq!(
        outcome
            .report
            .skipped_creature_trainers_missing_gossip_option,
        vec![(102, 10, 7, 9)]
    );
    assert_eq!(*gossip_calls.borrow(), vec![(7, 9), (7, 2), (8, 0), (0, 3)]);
    assert_eq!(outcome.store.get_creature_default_trainer_like_cpp(103), 10);
    assert_eq!(
        outcome
            .store
            .get_creature_trainer_for_gossip_option_like_cpp(104, 7, 2),
        10
    );
    assert_eq!(
        outcome
            .store
            .get_creature_trainer_for_gossip_option_like_cpp(105, 8, 0),
        10
    );
    assert_eq!(
        outcome
            .store
            .get_creature_trainer_for_gossip_option_like_cpp(106, 0, 3),
        10
    );
    assert_eq!(outcome.store.creature_trainer_count_like_cpp(), 4);
}

#[test]
fn trainer_load_diagnostics_preserve_cpp_phase_and_row_order() {
    let mut missing_skill = spell_row(10, 1000);
    missing_skill.spell.req_skill_line = 2000;

    let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
        [trainer_row(10)],
        [missing_skill, spell_row(10, 1001), spell_row(99, 1002)],
        [TrainerLocaleRowLikeCpp {
            id: 99,
            locale: "frFR".to_string(),
            greeting: "bonjour".to_string(),
        }],
        [
            CreatureTrainerRowLikeCpp {
                creature_id: 100,
                trainer_id: 10,
                menu_id: 0,
                option_id: 0,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: 101,
                trainer_id: 99,
                menu_id: 0,
                option_id: 0,
            },
            CreatureTrainerRowLikeCpp {
                creature_id: 102,
                trainer_id: 10,
                menu_id: 7,
                option_id: 9,
            },
        ],
        |spell_id| matches!(spell_id, 1000 | 1002),
        |_| false,
        |creature_id| creature_id != 100,
        |_, _| false,
    );

    assert_eq!(
        outcome.report.diagnostics_in_load_order_like_cpp,
        vec![
            TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingSkillLine {
                trainer_id: 10,
                spell_id: 1000,
                skill_line_id: 2000,
            },
            TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingSpell {
                trainer_id: 10,
                spell_id: 1001,
            },
            TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingTrainer {
                trainer_id: 99,
                spell_id: 1002,
            },
            TrainerLoadDiagnosticLikeCpp::TrainerLocaleMissingTrainer {
                trainer_id: 99,
                locale: "frFR".to_string(),
            },
            TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingCreatureTemplate {
                creature_id: 100,
            },
            TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingTrainer {
                creature_id: 101,
                trainer_id: 99,
                menu_id: 0,
                option_id: 0,
            },
            TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingGossipOption {
                creature_id: 102,
                trainer_id: 10,
                menu_id: 7,
                option_id: 9,
            },
        ]
    );
}
