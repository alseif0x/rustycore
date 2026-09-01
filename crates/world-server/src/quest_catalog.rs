//! Composition boundary for the C++ ObjectMgr quest catalog.

use anyhow::{Result, bail};
use wow_persistence::{QuestCatalogLoadOutcomeLikeCpp, QuestCatalogPersistencePortLikeCpp};

fn loaded_rows_like_cpp<T>(outcome: QuestCatalogLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        QuestCatalogLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        QuestCatalogLoadOutcomeLikeCpp::Failed { reason } => bail!(reason),
    }
}

pub(super) async fn load_quests_like_cpp(
    port: &dyn QuestCatalogPersistencePortLikeCpp,
) -> Result<wow_data::quest::QuestStore> {
    let template_rows = loaded_rows_like_cpp(port.load_quest_template_rows_like_cpp().await)?;
    let mut store = wow_data::quest::QuestStore::from_template_rows_like_cpp(
        template_rows
            .into_iter()
            .map(|r| wow_data::quest::QuestTemplate {
                id: r.id,
                quest_type: r.quest_type,
                quest_level: r.quest_level,
                quest_max_scaling_level: r.quest_max_scaling_level,
                quest_package_id: r.quest_package_id,
                min_level: r.min_level,
                quest_sort_id: r.quest_sort_id,
                quest_info_id: r.quest_info_id,
                suggested_group_num: r.suggested_group_num,
                reward_next_quest: r.reward_next_quest,
                reward_xp_difficulty: r.reward_xp_difficulty,
                reward_xp_multiplier: r.reward_xp_multiplier,
                reward_money_difficulty: r.reward_money_difficulty,
                reward_money_multiplier: r.reward_money_multiplier,
                reward_bonus_money: r.reward_bonus_money,
                reward_display_spell: r.reward_display_spell,
                reward_spell: r.reward_spell,
                reward_honor: r.reward_honor,
                reward_title_id: r.reward_title_id,
                reward_skill_line_id: r.reward_skill_line_id,
                reward_skill_points: r.reward_skill_points,
                reward_mail_template_id: r.reward_mail_template_id,
                reward_mail_delay_secs: r.reward_mail_delay_secs,
                reward_mail_sender_entry: r.reward_mail_sender_entry,
                reward_faction_ids: r.reward_faction_ids,
                reward_faction_values: r.reward_faction_values,
                reward_faction_overrides: r.reward_faction_overrides,
                reward_faction_cap_in: r.reward_faction_cap_in,
                reward_faction_flags: r.reward_faction_flags,
                source_item_id: r.source_item_id,
                source_item_count: r.source_item_count,
                source_spell_id: r.source_spell_id,
                limit_time_secs: r.limit_time_secs,
                expansion: r.expansion,
                flags: r.flags,
                flags_ex: r.flags_ex,
                flags_ex2: r.flags_ex2,
                special_flags: r.special_flags,
                event_id_for_quest: 0,
                reward_items: r.reward_items,
                reward_amounts: r.reward_amounts,
                reward_currencies: r.reward_currencies,
                reward_currency_amounts: r.reward_currency_amounts,
                item_drop: r.item_drop,
                item_drop_quantity: r.item_drop_quantity,
                log_title: r.log_title,
                log_description: r.log_description,
                quest_description: r.quest_description,
                area_description: r.area_description,
                quest_completion_log: r.quest_completion_log,
                objectives: Vec::new(),
                allowable_races: r.allowable_races,
                allowable_classes: r.allowable_classes,
                max_level: r.max_level,
                prev_quest_id: r.prev_quest_id,
                next_quest_id: r.next_quest_id,
                exclusive_group: r.exclusive_group,
                breadcrumb_for_quest_id: r.breadcrumb_for_quest_id,
                dependent_previous_quests: Vec::new(),
                dependent_breadcrumb_quests: Vec::new(),
                required_min_rep_faction: r.required_min_rep_faction,
                required_min_rep_value: r.required_min_rep_value,
                required_max_rep_faction: r.required_max_rep_faction,
                required_max_rep_value: r.required_max_rep_value,
                required_skill_id: r.required_skill_id,
                required_skill_points: r.required_skill_points,
                reward_choice_items: r.reward_choice_items,
                reward_choice_item_types: r.reward_choice_item_types,
            })
            .collect(),
    );

    store.apply_special_flag_rows_like_cpp(loaded_rows_like_cpp(
        port.load_quest_special_flag_rows_like_cpp().await,
    )?);
    store.apply_seasonal_relation_rows_like_cpp(loaded_rows_like_cpp(
        port.load_seasonal_quest_relation_rows_like_cpp().await,
    )?);
    store.apply_objective_rows_like_cpp(
        loaded_rows_like_cpp(port.load_quest_objective_rows_like_cpp().await)?
            .into_iter()
            .map(|r| wow_data::quest::QuestObjective {
                id: r.id,
                quest_id: r.quest_id,
                obj_type: r.obj_type,
                order: r.order,
                storage_index: r.storage_index,
                object_id: r.object_id,
                amount: r.amount,
                flags: r.flags,
                flags2: r.flags2,
                progress_bar_weight: r.progress_bar_weight,
                description: r.description,
            })
            .collect(),
    );
    store.apply_creature_starter_rows_like_cpp(loaded_rows_like_cpp(
        port.load_creature_quest_starter_rows_like_cpp().await,
    )?);
    store.apply_creature_ender_rows_like_cpp(loaded_rows_like_cpp(
        port.load_creature_quest_ender_rows_like_cpp().await,
    )?);
    store.apply_gameobject_starter_rows_like_cpp(loaded_rows_like_cpp(
        port.load_gameobject_quest_starter_rows_like_cpp().await,
    )?);
    store.apply_gameobject_ender_rows_like_cpp(loaded_rows_like_cpp(
        port.load_gameobject_quest_ender_rows_like_cpp().await,
    )?);
    store.log_relation_counts_like_cpp();
    Ok(store)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use wow_persistence::{
        PersistenceFutureLikeCpp, QuestObjectivePersistenceRowLikeCpp,
        QuestTemplatePersistenceRowLikeCpp,
    };

    struct RecordingPort {
        calls: Mutex<Vec<&'static str>>,
        fail_at: Option<&'static str>,
    }

    impl RecordingPort {
        fn outcome<T>(&self, stage: &'static str) -> QuestCatalogLoadOutcomeLikeCpp<T> {
            self.calls.lock().unwrap().push(stage);
            if self.fail_at == Some(stage) {
                QuestCatalogLoadOutcomeLikeCpp::Failed {
                    reason: format!("{stage} read failed"),
                }
            } else {
                QuestCatalogLoadOutcomeLikeCpp::Loaded(Vec::new())
            }
        }
    }

    impl QuestCatalogPersistencePortLikeCpp for RecordingPort {
        fn load_quest_template_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            QuestCatalogLoadOutcomeLikeCpp<QuestTemplatePersistenceRowLikeCpp>,
        > {
            Box::pin(async move { self.outcome("templates") })
        }

        fn load_quest_special_flag_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
            Box::pin(async move { self.outcome("special_flags") })
        }

        fn load_seasonal_quest_relation_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
            Box::pin(async move { self.outcome("seasonal_relations") })
        }

        fn load_quest_objective_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<
            '_,
            QuestCatalogLoadOutcomeLikeCpp<QuestObjectivePersistenceRowLikeCpp>,
        > {
            Box::pin(async move { self.outcome("objectives") })
        }

        fn load_creature_quest_starter_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
            Box::pin(async move { self.outcome("creature_starters") })
        }

        fn load_creature_quest_ender_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
            Box::pin(async move { self.outcome("creature_enders") })
        }

        fn load_gameobject_quest_starter_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
            Box::pin(async move { self.outcome("gameobject_starters") })
        }

        fn load_gameobject_quest_ender_rows_like_cpp(
            &self,
        ) -> PersistenceFutureLikeCpp<'_, QuestCatalogLoadOutcomeLikeCpp<(u32, u32)>> {
            Box::pin(async move { self.outcome("gameobject_enders") })
        }
    }

    #[tokio::test]
    async fn staged_port_preserves_the_eight_step_startup_order() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: None,
        };
        load_quests_like_cpp(&port).await.unwrap();
        assert_eq!(
            *port.calls.lock().unwrap(),
            [
                "templates",
                "special_flags",
                "seasonal_relations",
                "objectives",
                "creature_starters",
                "creature_enders",
                "gameobject_starters",
                "gameobject_enders",
            ]
        );
    }

    #[tokio::test]
    async fn a_failed_stage_prevents_every_later_read() {
        let port = RecordingPort {
            calls: Mutex::new(Vec::new()),
            fail_at: Some("objectives"),
        };
        assert!(load_quests_like_cpp(&port).await.is_err());
        assert_eq!(
            *port.calls.lock().unwrap(),
            [
                "templates",
                "special_flags",
                "seasonal_relations",
                "objectives"
            ]
        );
    }
}
