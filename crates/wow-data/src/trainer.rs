// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadTrainers` / `LoadCreatureTrainers` represented model.

use std::collections::HashMap;

use anyhow::Result;
use wow_constants::shared::Locale;
use wow_database::{WorldDatabase, WorldStatements};

pub const TRAINER_TYPE_NONE_LIKE_CPP: u8 = 0;
pub const TRAINER_TYPE_TALENT_LIKE_CPP: u8 = 1;
pub const TRAINER_TYPE_TRADESKILL_LIKE_CPP: u8 = 2;
pub const TRAINER_TYPE_PET_LIKE_CPP: u8 = 3;

pub const TRAINER_SPELL_STATE_KNOWN_LIKE_CPP: u8 = 0;
pub const TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP: u8 = 1;
pub const TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerSpellLikeCpp {
    pub spell_id: u32,
    pub money_cost: u32,
    pub req_skill_line: u32,
    pub req_skill_rank: u32,
    pub req_ability: [u32; 3],
    pub req_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerSpellRowLikeCpp {
    pub trainer_id: u32,
    pub spell: TrainerSpellLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerRowLikeCpp {
    pub id: u32,
    pub trainer_type: u8,
    pub greeting: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerLocaleRowLikeCpp {
    pub id: u32,
    pub locale: String,
    pub greeting: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureTrainerRowLikeCpp {
    pub creature_id: u32,
    pub trainer_id: u32,
    pub menu_id: u32,
    pub option_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrainerLikeCpp {
    id: u32,
    trainer_type: u8,
    spells: Vec<TrainerSpellLikeCpp>,
    greeting: String,
    greeting_locales: HashMap<Locale, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrainerLoadReportLikeCpp {
    pub trainer_spell_rows: usize,
    pub trainer_rows: usize,
    pub trainer_locale_rows_seen: usize,
    pub trainer_locale_entries: usize,
    pub creature_trainer_rows_seen: usize,
    pub creature_trainer_entries: usize,
    pub skipped_spells_missing_spell: Vec<(u32, u32)>,
    pub skipped_spells_missing_skill_line: Vec<(u32, u32, u32)>,
    pub skipped_spells_missing_required_spell: Vec<(u32, u32, u8, u32)>,
    pub skipped_spells_missing_trainer: Vec<(u32, u32)>,
    pub skipped_locales_missing_trainer: Vec<(u32, String)>,
    pub skipped_creature_trainers_missing_creature_template: Vec<(u32, u32, u32, u32)>,
    pub skipped_creature_trainers_missing_trainer: Vec<(u32, u32, u32, u32)>,
    pub skipped_creature_trainers_missing_gossip_option: Vec<(u32, u32, u32, u32)>,
    /// C++ writes loader diagnostics as each row is validated. Keep the
    /// category buckets above for counts/tests, but use this stream when
    /// publishing diagnostics so independent categories are not regrouped.
    pub diagnostics_in_load_order_like_cpp: Vec<TrainerLoadDiagnosticLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerLoadDiagnosticLikeCpp {
    TrainerSpellMissingSpell {
        trainer_id: u32,
        spell_id: u32,
    },
    TrainerSpellMissingSkillLine {
        trainer_id: u32,
        spell_id: u32,
        skill_line_id: u32,
    },
    TrainerSpellMissingRequiredSpell {
        trainer_id: u32,
        spell_id: u32,
        required_index: u8,
        required_spell_id: u32,
    },
    TrainerSpellMissingTrainer {
        trainer_id: u32,
        spell_id: u32,
    },
    TrainerLocaleMissingTrainer {
        trainer_id: u32,
        locale: String,
    },
    CreatureTrainerMissingCreatureTemplate {
        creature_id: u32,
    },
    CreatureTrainerMissingTrainer {
        creature_id: u32,
        trainer_id: u32,
        menu_id: u32,
        option_id: u32,
    },
    CreatureTrainerMissingGossipOption {
        creature_id: u32,
        trainer_id: u32,
        menu_id: u32,
        option_id: u32,
    },
}

#[derive(Debug, Clone, Default)]
pub struct TrainerStoreLikeCpp {
    trainers: HashMap<u32, TrainerLikeCpp>,
    creature_default_trainers: HashMap<(u32, u32, u32), u32>,
}

pub struct TrainerLoadOutcomeLikeCpp {
    pub store: TrainerStoreLikeCpp,
    pub report: TrainerLoadReportLikeCpp,
}

impl TrainerLikeCpp {
    pub fn id_like_cpp(&self) -> u32 {
        self.id
    }

    pub fn trainer_type_like_cpp(&self) -> u8 {
        self.trainer_type
    }

    pub fn spells_like_cpp(&self) -> &[TrainerSpellLikeCpp] {
        &self.spells
    }

    /// C++ `Trainer::GetSpell`.
    pub fn get_spell_like_cpp(&self, spell_id: u32) -> Option<&TrainerSpellLikeCpp> {
        self.spells.iter().find(|spell| spell.spell_id == spell_id)
    }

    /// C++ `Trainer::GetGreeting`.
    pub fn greeting_like_cpp(&self, locale: Locale) -> &str {
        self.greeting_locales
            .get(&locale)
            .filter(|greeting| !greeting.is_empty())
            .unwrap_or(&self.greeting)
    }

    /// C++ `Trainer::GetGreeting(WorldSession::GetSessionDbLocaleIndex())`.
    pub fn greeting_for_locale_name_like_cpp(&self, locale_name: &str) -> &str {
        let locale = locale_from_name_like_cpp(locale_name)
            .filter(|locale| *locale != Locale::None)
            .unwrap_or(Locale::EnUS);
        self.greeting_like_cpp(locale)
    }

    /// C++ `Trainer::AddGreetingLocale`.
    pub fn add_greeting_locale_like_cpp(&mut self, locale: Locale, greeting: String) {
        self.greeting_locales.insert(locale, greeting);
    }
}

impl TrainerStoreLikeCpp {
    /// Builds the immutable trainer catalog after applying the external
    /// `SpellMgr`/DB2/ObjectMgr/gossip existence checks owned by the caller.
    ///
    /// The callbacks are required so no production call site can accidentally
    /// publish unvalidated rows.
    #[allow(clippy::too_many_arguments)]
    pub fn from_rows_like_cpp<
        SpellExists,
        SkillLineExists,
        CreatureTemplateExists,
        GossipOptionExists,
    >(
        trainer_rows: impl IntoIterator<Item = TrainerRowLikeCpp>,
        trainer_spell_rows: impl IntoIterator<Item = TrainerSpellRowLikeCpp>,
        trainer_locale_rows: impl IntoIterator<Item = TrainerLocaleRowLikeCpp>,
        creature_trainer_rows: impl IntoIterator<Item = CreatureTrainerRowLikeCpp>,
        mut spell_exists: SpellExists,
        mut skill_line_exists: SkillLineExists,
        mut creature_template_exists: CreatureTemplateExists,
        mut gossip_option_exists: GossipOptionExists,
    ) -> TrainerLoadOutcomeLikeCpp
    where
        SpellExists: FnMut(u32) -> bool,
        SkillLineExists: FnMut(u32) -> bool,
        CreatureTemplateExists: FnMut(u32) -> bool,
        GossipOptionExists: FnMut(u32, u32) -> bool,
    {
        let trainer_spell_rows: Vec<TrainerSpellRowLikeCpp> =
            trainer_spell_rows.into_iter().collect();
        let mut report = TrainerLoadReportLikeCpp {
            trainer_spell_rows: trainer_spell_rows.len(),
            ..TrainerLoadReportLikeCpp::default()
        };
        let mut spells_by_trainer: HashMap<u32, Vec<TrainerSpellLikeCpp>> = HashMap::new();
        for row in &trainer_spell_rows {
            if !spell_exists(row.spell.spell_id) {
                report
                    .skipped_spells_missing_spell
                    .push((row.trainer_id, row.spell.spell_id));
                report.diagnostics_in_load_order_like_cpp.push(
                    TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingSpell {
                        trainer_id: row.trainer_id,
                        spell_id: row.spell.spell_id,
                    },
                );
                continue;
            }

            if row.spell.req_skill_line != 0 && !skill_line_exists(row.spell.req_skill_line) {
                report.skipped_spells_missing_skill_line.push((
                    row.trainer_id,
                    row.spell.spell_id,
                    row.spell.req_skill_line,
                ));
                report.diagnostics_in_load_order_like_cpp.push(
                    TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingSkillLine {
                        trainer_id: row.trainer_id,
                        spell_id: row.spell.spell_id,
                        skill_line_id: row.spell.req_skill_line,
                    },
                );
                continue;
            }

            let mut all_required_spells_valid = true;
            for (index, required_spell) in row.spell.req_ability.iter().copied().enumerate() {
                if required_spell != 0 && !spell_exists(required_spell) {
                    let required_index =
                        u8::try_from(index + 1).expect("trainer required-spell index fits u8");
                    report.skipped_spells_missing_required_spell.push((
                        row.trainer_id,
                        row.spell.spell_id,
                        required_index,
                        required_spell,
                    ));
                    report.diagnostics_in_load_order_like_cpp.push(
                        TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingRequiredSpell {
                            trainer_id: row.trainer_id,
                            spell_id: row.spell.spell_id,
                            required_index,
                            required_spell_id: required_spell,
                        },
                    );
                    all_required_spells_valid = false;
                }
            }
            if !all_required_spells_valid {
                continue;
            }

            spells_by_trainer
                .entry(row.trainer_id)
                .or_default()
                .push(row.spell.clone());
        }

        let mut store = Self::default();

        for row in trainer_rows {
            let spells = spells_by_trainer.remove(&row.id).unwrap_or_default();
            store
                .trainers
                .entry(row.id)
                .or_insert_with(|| TrainerLikeCpp {
                    id: row.id,
                    trainer_type: row.trainer_type,
                    spells,
                    greeting: row.greeting,
                    greeting_locales: HashMap::new(),
                });
            report.trainer_rows += 1;
        }

        for (trainer_id, spells) in spells_by_trainer {
            for spell in spells {
                report
                    .skipped_spells_missing_trainer
                    .push((trainer_id, spell.spell_id));
                report.diagnostics_in_load_order_like_cpp.push(
                    TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingTrainer {
                        trainer_id,
                        spell_id: spell.spell_id,
                    },
                );
            }
        }

        for row in trainer_locale_rows {
            report.trainer_locale_rows_seen += 1;
            let Some(locale) = locale_from_name_like_cpp(&row.locale) else {
                continue;
            };
            if matches!(locale, Locale::EnUS | Locale::None) {
                continue;
            }

            if let Some(trainer) = store.trainers.get_mut(&row.id) {
                trainer.add_greeting_locale_like_cpp(locale, row.greeting);
                report.trainer_locale_entries += 1;
            } else {
                report
                    .skipped_locales_missing_trainer
                    .push((row.id, row.locale.clone()));
                report.diagnostics_in_load_order_like_cpp.push(
                    TrainerLoadDiagnosticLikeCpp::TrainerLocaleMissingTrainer {
                        trainer_id: row.id,
                        locale: row.locale,
                    },
                );
            }
        }

        for row in creature_trainer_rows {
            report.creature_trainer_rows_seen += 1;
            if !creature_template_exists(row.creature_id) {
                report
                    .skipped_creature_trainers_missing_creature_template
                    .push((row.creature_id, row.trainer_id, row.menu_id, row.option_id));
                report.diagnostics_in_load_order_like_cpp.push(
                    TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingCreatureTemplate {
                        creature_id: row.creature_id,
                    },
                );
                continue;
            }

            if !store.trainers.contains_key(&row.trainer_id) {
                report.skipped_creature_trainers_missing_trainer.push((
                    row.creature_id,
                    row.trainer_id,
                    row.menu_id,
                    row.option_id,
                ));
                report.diagnostics_in_load_order_like_cpp.push(
                    TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingTrainer {
                        creature_id: row.creature_id,
                        trainer_id: row.trainer_id,
                        menu_id: row.menu_id,
                        option_id: row.option_id,
                    },
                );
                continue;
            }

            if (row.menu_id != 0 || row.option_id != 0)
                && !gossip_option_exists(row.menu_id, row.option_id)
            {
                report
                    .skipped_creature_trainers_missing_gossip_option
                    .push((row.creature_id, row.trainer_id, row.menu_id, row.option_id));
                report.diagnostics_in_load_order_like_cpp.push(
                    TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingGossipOption {
                        creature_id: row.creature_id,
                        trainer_id: row.trainer_id,
                        menu_id: row.menu_id,
                        option_id: row.option_id,
                    },
                );
                continue;
            }

            store.creature_default_trainers.insert(
                (row.creature_id, row.menu_id, row.option_id),
                row.trainer_id,
            );
            report.creature_trainer_entries = store.creature_default_trainers.len();
        }

        TrainerLoadOutcomeLikeCpp { store, report }
    }

    /// C++ `ObjectMgr::LoadTrainers` + `LoadCreatureTrainers`.
    pub async fn load_like_cpp<
        SpellExists,
        SkillLineExists,
        CreatureTemplateExists,
        GossipOptionExists,
    >(
        db: &WorldDatabase,
        spell_exists: SpellExists,
        skill_line_exists: SkillLineExists,
        creature_template_exists: CreatureTemplateExists,
        gossip_option_exists: GossipOptionExists,
    ) -> Result<TrainerLoadOutcomeLikeCpp>
    where
        SpellExists: FnMut(u32) -> bool,
        SkillLineExists: FnMut(u32) -> bool,
        CreatureTemplateExists: FnMut(u32) -> bool,
        GossipOptionExists: FnMut(u32, u32) -> bool,
    {
        let stmt = db.prepare(WorldStatements::SEL_TRAINER_SPELLS_ALL);
        let mut result = db.query(&stmt).await?;
        let mut spell_rows = Vec::new();
        if !result.is_empty() {
            loop {
                spell_rows.push(TrainerSpellRowLikeCpp {
                    trainer_id: result.read(0),
                    spell: TrainerSpellLikeCpp {
                        spell_id: result.read(1),
                        money_cost: result.read(2),
                        req_skill_line: result.read(3),
                        req_skill_rank: result.read(4),
                        req_ability: [result.read(5), result.read(6), result.read(7)],
                        req_level: result.read(8),
                    },
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        let stmt = db.prepare(WorldStatements::SEL_TRAINERS_ALL);
        let mut result = db.query(&stmt).await?;
        let mut trainer_rows = Vec::new();
        if !result.is_empty() {
            loop {
                trainer_rows.push(TrainerRowLikeCpp {
                    id: result.read(0),
                    trainer_type: result.read(1),
                    greeting: result.read_string(2),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        let stmt = db.prepare(WorldStatements::SEL_TRAINER_LOCALES);
        let mut result = db.query(&stmt).await?;
        let mut locale_rows = Vec::new();
        if !result.is_empty() {
            loop {
                locale_rows.push(TrainerLocaleRowLikeCpp {
                    id: result.read(0),
                    locale: result.read_string(1),
                    greeting: result.read_string(2),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        let stmt = db.prepare(WorldStatements::SEL_CREATURE_TRAINERS_ALL);
        let mut result = db.query(&stmt).await?;
        let mut creature_trainer_rows = Vec::new();
        if !result.is_empty() {
            loop {
                creature_trainer_rows.push(CreatureTrainerRowLikeCpp {
                    creature_id: result.read(0),
                    trainer_id: result.read(1),
                    menu_id: result.read(2),
                    option_id: result.read(3),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            trainer_rows,
            spell_rows,
            locale_rows,
            creature_trainer_rows,
            spell_exists,
            skill_line_exists,
            creature_template_exists,
            gossip_option_exists,
        ))
    }

    /// C++ `ObjectMgr::GetTrainer`.
    pub fn get_trainer_like_cpp(&self, trainer_id: u32) -> Option<&TrainerLikeCpp> {
        self.trainers.get(&trainer_id)
    }

    /// C++ `ObjectMgr::GetCreatureDefaultTrainer`.
    pub fn get_creature_default_trainer_like_cpp(&self, creature_id: u32) -> u32 {
        self.get_creature_trainer_for_gossip_option_like_cpp(creature_id, 0, 0)
    }

    /// C++ `ObjectMgr::GetCreatureTrainerForGossipOption`.
    pub fn get_creature_trainer_for_gossip_option_like_cpp(
        &self,
        creature_id: u32,
        gossip_menu_id: u32,
        gossip_option_id: u32,
    ) -> u32 {
        self.creature_default_trainers
            .get(&(creature_id, gossip_menu_id, gossip_option_id))
            .copied()
            .unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.trainers.len()
    }

    pub fn spell_count_like_cpp(&self) -> usize {
        self.trainers
            .values()
            .map(|trainer| trainer.spells.len())
            .sum()
    }

    pub fn creature_trainer_count_like_cpp(&self) -> usize {
        self.creature_default_trainers.len()
    }
}

fn locale_from_name_like_cpp(name: &str) -> Option<Locale> {
    match name {
        "enUS" => Some(Locale::EnUS),
        "koKR" => Some(Locale::KoKR),
        "frFR" => Some(Locale::FrFR),
        "deDE" => Some(Locale::DeDE),
        "zhCN" => Some(Locale::ZhCN),
        "zhTW" => Some(Locale::ZhTW),
        "esES" => Some(Locale::EsES),
        "esMX" => Some(Locale::EsMX),
        "ruRU" => Some(Locale::RuRU),
        "none" => Some(Locale::None),
        "ptBR" => Some(Locale::PtBR),
        "itIT" => Some(Locale::ItIT),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    fn spell_row(trainer_id: u32, spell_id: u32) -> TrainerSpellRowLikeCpp {
        TrainerSpellRowLikeCpp {
            trainer_id,
            spell: TrainerSpellLikeCpp {
                spell_id,
                money_cost: 100,
                req_skill_line: 0,
                req_skill_rank: 0,
                req_ability: [0, 0, 0],
                req_level: 1,
            },
        }
    }

    fn trainer_row(id: u32) -> TrainerRowLikeCpp {
        TrainerRowLikeCpp {
            id,
            trainer_type: TRAINER_TYPE_TRADESKILL_LIKE_CPP,
            greeting: format!("Hello {id}"),
        }
    }

    fn from_rows_with_existing_references(
        trainer_rows: impl IntoIterator<Item = TrainerRowLikeCpp>,
        trainer_spell_rows: impl IntoIterator<Item = TrainerSpellRowLikeCpp>,
        trainer_locale_rows: impl IntoIterator<Item = TrainerLocaleRowLikeCpp>,
        creature_trainer_rows: impl IntoIterator<Item = CreatureTrainerRowLikeCpp>,
    ) -> TrainerLoadOutcomeLikeCpp {
        TrainerStoreLikeCpp::from_rows_like_cpp(
            trainer_rows,
            trainer_spell_rows,
            trainer_locale_rows,
            creature_trainer_rows,
            |_| true,
            |_| true,
            |_| true,
            |_, _| true,
        )
    }

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
}
