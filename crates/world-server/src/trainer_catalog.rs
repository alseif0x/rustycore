//! Composition boundary for C++ trainer startup catalogs.

use anyhow::{Result, bail};
use wow_persistence::{
    CreatureTrainerPersistenceRowLikeCpp, TrainerCatalogLoadOutcomeLikeCpp,
    TrainerCatalogPersistencePortLikeCpp, TrainerLocalePersistenceRowLikeCpp,
    TrainerPersistenceRowLikeCpp, TrainerSpellPersistenceRowLikeCpp,
};

fn trainer_spell_row_like_cpp(
    row: TrainerSpellPersistenceRowLikeCpp,
) -> wow_data::TrainerSpellRowLikeCpp {
    wow_data::TrainerSpellRowLikeCpp {
        trainer_id: row.trainer_id,
        spell: wow_data::TrainerSpellLikeCpp {
            spell_id: row.spell_id,
            money_cost: row.money_cost,
            req_skill_line: row.req_skill_line,
            req_skill_rank: row.req_skill_rank,
            req_ability: row.req_ability,
            req_level: row.req_level,
        },
    }
}

fn trainer_row_like_cpp(row: TrainerPersistenceRowLikeCpp) -> wow_data::TrainerRowLikeCpp {
    wow_data::TrainerRowLikeCpp {
        id: row.id,
        trainer_type: row.trainer_type,
        greeting: row.greeting,
    }
}

fn trainer_locale_row_like_cpp(
    row: TrainerLocalePersistenceRowLikeCpp,
) -> wow_data::TrainerLocaleRowLikeCpp {
    wow_data::TrainerLocaleRowLikeCpp {
        id: row.id,
        locale: row.locale,
        greeting: row.greeting,
    }
}

fn creature_trainer_row_like_cpp(
    row: CreatureTrainerPersistenceRowLikeCpp,
) -> wow_data::CreatureTrainerRowLikeCpp {
    wow_data::CreatureTrainerRowLikeCpp {
        creature_id: row.creature_id,
        trainer_id: row.trainer_id,
        menu_id: row.menu_id,
        option_id: row.option_id,
    }
}

pub(super) async fn load_trainer_catalog_like_cpp<
    SpellExists,
    SkillLineExists,
    CreatureTemplateExists,
    GossipOptionExists,
>(
    persistence: &dyn TrainerCatalogPersistencePortLikeCpp,
    spell_exists: SpellExists,
    skill_line_exists: SkillLineExists,
    creature_template_exists: CreatureTemplateExists,
    gossip_option_exists: GossipOptionExists,
) -> Result<wow_data::TrainerLoadOutcomeLikeCpp>
where
    SpellExists: FnMut(u32) -> bool,
    SkillLineExists: FnMut(u32) -> bool,
    CreatureTemplateExists: FnMut(u32) -> bool,
    GossipOptionExists: FnMut(u32, u32) -> bool,
{
    let rows = match persistence.load_trainer_catalog_rows_like_cpp().await {
        TrainerCatalogLoadOutcomeLikeCpp::Loaded(rows) => rows,
        TrainerCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    };
    Ok(wow_data::TrainerStoreLikeCpp::from_rows_like_cpp(
        rows.trainers.into_iter().map(trainer_row_like_cpp),
        rows.trainer_spells
            .into_iter()
            .map(trainer_spell_row_like_cpp),
        rows.trainer_locales
            .into_iter()
            .map(trainer_locale_row_like_cpp),
        rows.creature_trainers
            .into_iter()
            .map(creature_trainer_row_like_cpp),
        spell_exists,
        skill_line_exists,
        creature_template_exists,
        gossip_option_exists,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_persistence::{PersistenceFutureLikeCpp, TrainerCatalogPersistenceRowsLikeCpp};

    struct FixedTrainerCatalogPersistenceLikeCpp(TrainerCatalogLoadOutcomeLikeCpp);

    impl TrainerCatalogPersistencePortLikeCpp for FixedTrainerCatalogPersistenceLikeCpp {
        fn load_trainer_catalog_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, TrainerCatalogLoadOutcomeLikeCpp> {
            Box::pin(async { self.0.clone() })
        }
    }

    #[test]
    fn typed_trainer_rows_preserve_every_field() {
        assert_eq!(
            trainer_spell_row_like_cpp(TrainerSpellPersistenceRowLikeCpp {
                trainer_id: 1,
                spell_id: 2,
                money_cost: 3,
                req_skill_line: 4,
                req_skill_rank: 5,
                req_ability: [6, 7, 8],
                req_level: 9,
            }),
            wow_data::TrainerSpellRowLikeCpp {
                trainer_id: 1,
                spell: wow_data::TrainerSpellLikeCpp {
                    spell_id: 2,
                    money_cost: 3,
                    req_skill_line: 4,
                    req_skill_rank: 5,
                    req_ability: [6, 7, 8],
                    req_level: 9,
                },
            }
        );
        assert_eq!(
            trainer_row_like_cpp(TrainerPersistenceRowLikeCpp {
                id: 10,
                trainer_type: 2,
                greeting: "hello".into(),
            }),
            wow_data::TrainerRowLikeCpp {
                id: 10,
                trainer_type: 2,
                greeting: "hello".into(),
            }
        );
        assert_eq!(
            trainer_locale_row_like_cpp(TrainerLocalePersistenceRowLikeCpp {
                id: 11,
                locale: "frFR".into(),
                greeting: "bonjour".into(),
            }),
            wow_data::TrainerLocaleRowLikeCpp {
                id: 11,
                locale: "frFR".into(),
                greeting: "bonjour".into(),
            }
        );
        assert_eq!(
            creature_trainer_row_like_cpp(CreatureTrainerPersistenceRowLikeCpp {
                creature_id: 12,
                trainer_id: 13,
                menu_id: 14,
                option_id: 15,
            }),
            wow_data::CreatureTrainerRowLikeCpp {
                creature_id: 12,
                trainer_id: 13,
                menu_id: 14,
                option_id: 15,
            }
        );
    }

    #[tokio::test]
    async fn failed_read_stops_before_domain_publication() {
        let persistence =
            FixedTrainerCatalogPersistenceLikeCpp(TrainerCatalogLoadOutcomeLikeCpp::Failed {
                reason: "world read failed".into(),
            });
        let result = load_trainer_catalog_like_cpp(
            &persistence,
            |_| panic!("validation must not run after a failed persistence read"),
            |_| panic!("validation must not run after a failed persistence read"),
            |_| panic!("validation must not run after a failed persistence read"),
            |_, _| panic!("validation must not run after a failed persistence read"),
        )
        .await;
        let error = match result {
            Ok(_) => panic!("failed persistence read must not publish a trainer store"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), "world read failed");
    }

    #[tokio::test]
    async fn loaded_rows_reach_domain_validation_without_field_defaults() {
        let persistence = FixedTrainerCatalogPersistenceLikeCpp(
            TrainerCatalogLoadOutcomeLikeCpp::Loaded(TrainerCatalogPersistenceRowsLikeCpp {
                trainer_spells: vec![TrainerSpellPersistenceRowLikeCpp {
                    trainer_id: 1,
                    spell_id: 2,
                    money_cost: 3,
                    req_skill_line: 4,
                    req_skill_rank: 5,
                    req_ability: [6, 7, 8],
                    req_level: 9,
                }],
                trainers: vec![TrainerPersistenceRowLikeCpp {
                    id: 1,
                    trainer_type: 0,
                    greeting: "hello".into(),
                }],
                ..Default::default()
            }),
        );
        let outcome = load_trainer_catalog_like_cpp(
            &persistence,
            |spell_id| spell_id == 2 || matches!(spell_id, 6..=8),
            |skill_line| skill_line == 4,
            |_| true,
            |_, _| true,
        )
        .await
        .unwrap();

        let trainer = outcome.store.get_trainer_like_cpp(1).unwrap();
        let spell = trainer.get_spell_like_cpp(2).unwrap();
        assert_eq!(spell.money_cost, 3);
        assert_eq!(spell.req_skill_line, 4);
        assert_eq!(spell.req_skill_rank, 5);
        assert_eq!(spell.req_ability, [6, 7, 8]);
        assert_eq!(spell.req_level, 9);
    }
}
