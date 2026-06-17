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
    pub skipped_spells_missing_trainer: Vec<(u32, u32)>,
    pub skipped_locales_missing_trainer: Vec<(u32, String)>,
    pub skipped_creature_trainers_missing_trainer: Vec<(u32, u32, u32, u32)>,
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

    /// C++ `Trainer::AddGreetingLocale`.
    pub fn add_greeting_locale_like_cpp(&mut self, locale: Locale, greeting: String) {
        self.greeting_locales.insert(locale, greeting);
    }
}

impl TrainerStoreLikeCpp {
    pub fn from_rows_like_cpp(
        trainer_rows: impl IntoIterator<Item = TrainerRowLikeCpp>,
        trainer_spell_rows: impl IntoIterator<Item = TrainerSpellRowLikeCpp>,
        trainer_locale_rows: impl IntoIterator<Item = TrainerLocaleRowLikeCpp>,
        creature_trainer_rows: impl IntoIterator<Item = CreatureTrainerRowLikeCpp>,
    ) -> TrainerLoadOutcomeLikeCpp {
        let trainer_spell_rows: Vec<TrainerSpellRowLikeCpp> =
            trainer_spell_rows.into_iter().collect();
        let mut spells_by_trainer: HashMap<u32, Vec<TrainerSpellLikeCpp>> = HashMap::new();
        for row in &trainer_spell_rows {
            spells_by_trainer
                .entry(row.trainer_id)
                .or_default()
                .push(row.spell.clone());
        }

        let mut store = Self::default();
        let mut report = TrainerLoadReportLikeCpp {
            trainer_spell_rows: trainer_spell_rows.len(),
            ..TrainerLoadReportLikeCpp::default()
        };

        for row in trainer_rows {
            let spells = spells_by_trainer.remove(&row.id).unwrap_or_default();
            store.trainers.insert(
                row.id,
                TrainerLikeCpp {
                    id: row.id,
                    trainer_type: row.trainer_type,
                    spells,
                    greeting: row.greeting,
                    greeting_locales: HashMap::new(),
                },
            );
            report.trainer_rows += 1;
        }

        for (trainer_id, spells) in spells_by_trainer {
            for spell in spells {
                report
                    .skipped_spells_missing_trainer
                    .push((trainer_id, spell.spell_id));
            }
        }

        for row in trainer_locale_rows {
            report.trainer_locale_rows_seen += 1;
            let Some(locale) = locale_from_name_like_cpp(&row.locale) else {
                continue;
            };
            if locale == Locale::EnUS {
                continue;
            }

            if let Some(trainer) = store.trainers.get_mut(&row.id) {
                trainer.add_greeting_locale_like_cpp(locale, row.greeting);
                report.trainer_locale_entries += 1;
            } else {
                report
                    .skipped_locales_missing_trainer
                    .push((row.id, row.locale));
            }
        }

        for row in creature_trainer_rows {
            report.creature_trainer_rows_seen += 1;
            if !store.trainers.contains_key(&row.trainer_id) {
                report.skipped_creature_trainers_missing_trainer.push((
                    row.creature_id,
                    row.trainer_id,
                    row.menu_id,
                    row.option_id,
                ));
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
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<TrainerLoadOutcomeLikeCpp> {
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

    #[test]
    fn trainer_store_groups_spells_after_trainer_rows_like_cpp() {
        let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
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
        let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
            [trainer_row(10)],
            [spell_row(99, 3000)],
            [],
            [],
        );

        assert_eq!(outcome.store.spell_count_like_cpp(), 0);
        assert_eq!(
            outcome.report.skipped_spells_missing_trainer,
            vec![(99, 3000)]
        );
    }

    #[test]
    fn trainer_locales_skip_enus_and_fallback_to_default_like_cpp() {
        let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
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
            ],
            [],
        );

        let trainer = outcome.store.get_trainer_like_cpp(10).unwrap();
        assert_eq!(trainer.greeting_like_cpp(Locale::EnUS), "Hello 10");
        assert_eq!(trainer.greeting_like_cpp(Locale::EsES), "Hola");
        assert_eq!(trainer.greeting_like_cpp(Locale::FrFR), "Hello 10");
        assert_eq!(outcome.report.trainer_locale_entries, 1);
    }

    #[test]
    fn trainer_locales_report_missing_trainer_like_cpp() {
        let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
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
        let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
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
        let outcome = TrainerStoreLikeCpp::from_rows_like_cpp(
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
}
