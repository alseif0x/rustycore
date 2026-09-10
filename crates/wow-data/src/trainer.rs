// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadTrainers` / `LoadCreatureTrainers` represented model.

use std::collections::HashMap;

use wow_constants::shared::Locale;

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
#[path = "trainer/tests/mod.rs"]
mod tests;
