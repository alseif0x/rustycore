// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordered quest-objective event draining and its selected application owner.

use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_world_core::session::QuestObjectiveAccessLikeCpp;
use wow_world_loot::LootState;

mod context;
pub use self::context::QuestObjectiveProgressCx;
#[cfg(any(test, feature = "test-fixtures"))]
pub use self::context::QuestObjectiveRegistryFixtureRefsLikeCpp;

use super::{
    QuestRewardCx, RepresentedQuestObjectiveProgressEventLikeCpp, SessionQuestState,
    objective_progress::{
        current_quest_gameplay_snapshot_like_cpp, invalidate_player_quest_status_authority_like_cpp,
        mark_quest_incomplete_if_complete_like_cpp, save_changed_quest_statuses_like_cpp,
        update_objective_count_like_cpp,
    },
};

impl<'cx, 'session> QuestObjectiveProgressCx<'cx, 'session> {
    pub fn new(
        reward: &'cx mut QuestRewardCx<'session>,
        loot: &'cx LootState,
        item_guid_generator: &'cx ObjectGuidGenerator,
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation_fixture:
            &'cx mut super::QuestRewardReputationFixtureRefsLikeCpp<'session>,
        #[cfg(any(test, feature = "test-fixtures"))]
        player_game_master_fixture: &'cx bool,
        #[cfg(any(test, feature = "test-fixtures"))]
        registry_fixtures: QuestObjectiveRegistryFixtureRefsLikeCpp<'cx>,
    ) -> Self {
        Self {
            reward,
            loot,
            item_guid_generator,
            #[cfg(any(test, feature = "test-fixtures"))]
            reputation_fixture,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_game_master_fixture,
            #[cfg(any(test, feature = "test-fixtures"))]
            registry_fixtures,
        }
    }

    fn current_quest_status_like_cpp(&self, quest_id: u32) -> Option<u8> {
        let owner = self.reward.player.quest_objective_access_like_cpp();
        current_quest_gameplay_snapshot_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        )
        .and_then(|state| {
            state
                .statuses_like_cpp()
                .get(&quest_id)
                .map(|status| status.status)
        })
    }

    fn sync_player_registry_state_like_cpp(&self, owner: &QuestObjectiveAccessLikeCpp<'_>) {
        super::completion::sync_quest_completion_registry_like_cpp(
            owner,
            self.loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.registry_fixtures.spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.reward.quest_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            (
                self.registry_fixtures.mount_vehicle,
                self.registry_fixtures.vehicle_seat_flags,
                self.registry_fixtures.vehicle_seat_id,
                self.registry_fixtures.pet_guid,
            ),
            #[cfg(any(test, feature = "test-fixtures"))]
            self.registry_fixtures.position,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.registry_fixtures.health,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.registry_fixtures.max_health,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.registry_fixtures.alive,
            #[cfg(any(test, feature = "test-fixtures"))]
            &*self.reward.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.registry_fixtures.transport,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.reward.world_test_consumer,
        );
    }

    pub async fn drain_represented_quest_objective_progress_like_cpp(&mut self) {
        if !self
            .reward
            .quest_state
            .begin_represented_quest_objective_progress_drain_like_cpp()
        {
            return;
        }

        while let Some(event) = self
            .reward
            .quest_state
            .pop_represented_quest_objective_progress_like_cpp()
        {
            match event {
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                } => {
                    self.update_money_quest_objective_progress_like_cpp(old_money, new_money)
                        .await;
                }
                RepresentedQuestObjectiveProgressEventLikeCpp::CurrencyChanged {
                    currency_id,
                    change,
                } => {
                    let object_id = i32::try_from(currency_id).unwrap_or(i32::MAX);
                    self.update_currency_quest_objective_progress_like_cpp(currency_id, change)
                        .await;
                    self.update_storing_value_quest_objective_progress_like_cpp(
                        wow_constants::quest::QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP,
                        object_id,
                        change,
                        ObjectGuid::new(0, 0),
                    )
                    .await;
                    self.update_storing_value_quest_objective_progress_like_cpp(
                        wow_constants::quest::QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP,
                        object_id,
                        change,
                        ObjectGuid::new(0, 0),
                    )
                    .await;
                }
                RepresentedQuestObjectiveProgressEventLikeCpp::ReputationChanged {
                    faction_id,
                    change,
                } => {
                    let object_id = i32::try_from(faction_id).unwrap_or(i32::MAX);
                    self.update_reputation_quest_objective_progress_like_cpp(
                        wow_constants::quest::QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP,
                        faction_id,
                        change,
                    )
                    .await;
                    self.update_reputation_quest_objective_progress_like_cpp(
                        wow_constants::quest::QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP,
                        faction_id,
                        change,
                    )
                    .await;
                    self.update_storing_value_quest_objective_progress_like_cpp(
                        wow_constants::quest::QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP,
                        object_id,
                        change,
                        ObjectGuid::new(0, 0),
                    )
                    .await;
                }
            }
        }

        self.reward
            .quest_state
            .finish_represented_quest_objective_progress_drain_like_cpp();
    }

    async fn update_storing_value_quest_objective_progress_like_cpp(
        &mut self,
        objective_type: u8,
        object_id: i32,
        add_count: i32,
        credit_guid: ObjectGuid,
    ) {
        let owner = self.reward.player.quest_objective_access_like_cpp();
        invalidate_player_quest_status_authority_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        );

        use wow_packet::packets::quest::{
            QuestUpdateAddCredit, QuestUpdateAddPvpCredit, QuestUpdateComplete,
        };

        let Some(store) = self.reward.catalogs.quests.store.clone() else {
            return;
        };

        let victim_team = if objective_type
            == wow_constants::quest::QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP
            && !credit_guid.is_empty()
        {
            owner
                .quest_credit_race_like_cpp(credit_guid)
                .map(wow_world_core::session::player_team_for_race_cpp)
        } else {
            None
        };
        let player_team = wow_world_core::session::player_team_for_race_cpp(
            self.reward.player.player_race_like_cpp(),
        );
        let Some(quests) = current_quest_gameplay_snapshot_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        ) else {
            return;
        };
        let matching: Vec<(u32, usize, i32, u32)> = quests
            .statuses_like_cpp()
            .values()
            .filter(|qs| qs.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            .flat_map(|qs| {
                let quest = store.get(qs.quest_id)?;
                Some(quest.objectives.iter().filter_map(move |obj| {
                    if obj.obj_type != objective_type || obj.object_id != object_id {
                        return None;
                    }
                    if objective_type
                        == wow_constants::quest::QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP
                        && (obj.flags
                            & wow_constants::quest::QUEST_OBJECTIVE_FLAG_KILL_PLAYERS_SAME_FACTION_LIKE_CPP)
                            != 0
                        && victim_team.is_some_and(|team| team != player_team)
                    {
                        return None;
                    }
                    let Ok(idx) = usize::try_from(obj.storage_index) else {
                        return None;
                    };
                    Some((qs.quest_id, idx, obj.amount, obj.id))
                }))
            })
            .flatten()
            .collect();

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, obj_idx, required, objective_id) in matching {
            let Some(current) = update_objective_count_like_cpp(
                &owner,
                self.reward.quest_state,
                quest_id,
                obj_idx,
                required,
                add_count,
                self.reward.world_test_consumer,
            ) else {
                continue;
            };
            quests_to_save.push(quest_id);

            tracing::debug!(
                account = owner.account_id_like_cpp(),
                quest_id,
                obj_idx,
                current,
                required,
                objective_type,
                object_id,
                "Quest objective progress"
            );

            if add_count > 0 {
                if objective_type == wow_constants::quest::QUEST_OBJECTIVE_PLAYERKILLS_LIKE_CPP {
                    let _ = self.reward.send_packet_like_cpp(&QuestUpdateAddPvpCredit {
                        quest_id,
                        count: current as u16,
                    });
                } else {
                    let _ = self.reward.send_packet_like_cpp(&QuestUpdateAddCredit {
                        victim_guid: credit_guid,
                        quest_id,
                        object_id,
                        count: current as u16,
                        required: required as u16,
                        objective_type,
                    });
                }
            }

            if current >= required {
                if let (Some(quest), Some(quests)) = (
                    store.get(quest_id),
                    current_quest_gameplay_snapshot_like_cpp(
                        &owner,
                        self.reward.quest_state,
                        self.reward.world_test_consumer,
                    ),
                ) {
                    let quest_already_rewarded =
                        quests.rewarded_quest_ids_like_cpp().contains(&quest_id);
                    if quests
                        .statuses_like_cpp()
                        .get(&quest_id)
                        .is_some_and(|status| {
                            wow_entities::represented_can_complete_quest_after_objective_like_cpp(
                                status,
                                &quest.objective_rules_like_cpp(),
                                objective_id,
                                quest_already_rewarded,
                            )
                        })
                    {
                        quests_to_complete.push(quest_id);
                    }
                }
            }
        }

        drop(owner);
        for quest_id in quests_to_complete {
            let Some(quest) = store.get(quest_id).cloned() else {
                continue;
            };
            let completed = self.complete_represented_quest_after_add_like_cpp(&quest).await;
            if completed {
                let owner = self.reward.player.quest_objective_access_like_cpp();
                if current_quest_gameplay_snapshot_like_cpp(
                    &owner,
                    self.reward.quest_state,
                    self.reward.world_test_consumer,
                )
                .and_then(|state| {
                    state
                        .statuses_like_cpp()
                        .get(&quest_id)
                        .map(|status| status.status)
                }) == Some(wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
                {
                    let _ = self
                        .reward
                        .send_packet_like_cpp(&QuestUpdateComplete { quest_id });
                }
                tracing::info!(
                    account = owner.account_id_like_cpp(),
                    quest_id,
                    "Quest objectives complete"
                );
            }
        }

        let owner = self.reward.player.quest_objective_access_like_cpp();
        save_changed_quest_statuses_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.catalogs,
            self.reward.lifecycle,
            &mut quests_to_save,
            self.reward.world_test_consumer,
        )
        .await;
        self.sync_player_registry_state_like_cpp(&owner);
    }

    async fn update_storing_flag_quest_objective_progress_like_cpp(
        &mut self,
        objective_type: u8,
        object_id: i32,
        add_count: i32,
    ) {
        let owner = self.reward.player.quest_objective_access_like_cpp();
        invalidate_player_quest_status_authority_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        );
        use wow_packet::packets::quest::{QuestUpdateAddCreditSimple, QuestUpdateComplete};

        let Some(store) = self.reward.catalogs.quests.store.clone() else {
            return;
        };
        let Some(quests) = current_quest_gameplay_snapshot_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        ) else {
            return;
        };
        let matching: Vec<(u32, usize, u32)> = quests
            .statuses_like_cpp()
            .values()
            .filter(|qs| qs.status == wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP)
            .flat_map(|qs| {
                let quest = store.get(qs.quest_id)?;
                Some(quest.objectives.iter().filter_map(move |obj| {
                    if obj.obj_type != objective_type
                        || obj.object_id != object_id
                        || !obj.is_storing_flag_like_cpp()
                    {
                        return None;
                    }
                    let Ok(idx) = usize::try_from(obj.storage_index) else {
                        return None;
                    };
                    Some((qs.quest_id, idx, obj.id))
                }))
            })
            .flatten()
            .collect();

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for (quest_id, obj_idx, objective_id) in matching {
            let Some((objective_was_complete, objective_is_now_complete)) =
                super::objective_progress::update_storing_flag_like_cpp(
                    &owner,
                    self.reward.quest_state,
                    quest_id,
                    obj_idx,
                    add_count,
                    self.reward.world_test_consumer,
                )
            else {
                continue;
            };
            if objective_was_complete != objective_is_now_complete {
                quests_to_save.push(quest_id);
            }
            if add_count > 0 {
                let _ = self.reward.send_packet_like_cpp(&QuestUpdateAddCreditSimple {
                    quest_id,
                    object_id,
                    objective_type,
                });
            }
            if !objective_was_complete && objective_is_now_complete {
                if let (Some(quest), Some(quests)) = (
                    store.get(quest_id),
                    current_quest_gameplay_snapshot_like_cpp(
                        &owner,
                        self.reward.quest_state,
                        self.reward.world_test_consumer,
                    ),
                ) {
                    let rewarded = quests.rewarded_quest_ids_like_cpp().contains(&quest_id);
                    if quests
                        .statuses_like_cpp()
                        .get(&quest_id)
                        .is_some_and(|status| {
                            wow_entities::represented_can_complete_quest_after_objective_like_cpp(
                                status,
                                &quest.objective_rules_like_cpp(),
                                objective_id,
                                rewarded,
                            )
                        })
                    {
                        quests_to_complete.push((quest_id, objective_id));
                    }
                }
            }
        }

        drop(owner);
        for (quest_id, objective_id) in quests_to_complete {
            let Some(quest) = store.get(quest_id).cloned() else {
                continue;
            };
            if self
                .complete_represented_quest_after_objective_like_cpp(&quest, objective_id)
                .await
            {
                let owner = self.reward.player.quest_objective_access_like_cpp();
                if self.current_quest_status_like_cpp(quest_id)
                    == Some(wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
                {
                    let _ = self
                        .reward
                        .send_packet_like_cpp(&QuestUpdateComplete { quest_id });
                }
                tracing::info!(
                    account = owner.account_id_like_cpp(),
                    quest_id,
                    "Quest objectives complete"
                );
            }
        }
        let owner = self.reward.player.quest_objective_access_like_cpp();
        save_changed_quest_statuses_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.catalogs,
            self.reward.lifecycle,
            &mut quests_to_save,
            self.reward.world_test_consumer,
        )
        .await;
        self.sync_player_registry_state_like_cpp(&owner);
    }

    async fn update_money_quest_objective_progress_like_cpp(
        &mut self,
        old_money: u64,
        new_money: u64,
    ) {
        let owner = self.reward.player.quest_objective_access_like_cpp();
        invalidate_player_quest_status_authority_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        );
        use wow_packet::packets::quest::QuestUpdateComplete;

        let Some(store) = self.reward.catalogs.quests.store.clone() else {
            return;
        };
        let old_money = old_money.min(i64::MAX as u64) as i64;
        let new_money_i64 = new_money.min(i64::MAX as u64) as i64;
        let Some(quests) = current_quest_gameplay_snapshot_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        ) else {
            return;
        };
        let matching = wow_entities::plan_threshold_quest_objective_changes_like_cpp(
            quests.statuses_like_cpp(),
            |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
            wow_constants::quest::QUEST_OBJECTIVE_MONEY_LIKE_CPP,
            0,
            old_money,
            new_money_i64,
            false,
        );

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for change in matching {
            let quest_id = change.quest_id;
            let objective_id = change.objective_id;
            let required = change.required;
            let objective_was_complete = change.objective_was_complete;
            let objective_is_now_complete = change.objective_is_now_complete;
            tracing::debug!(
                account = owner.account_id_like_cpp(),
                quest_id,
                old_money,
                new_money = new_money_i64,
                required,
                objective_was_complete,
                objective_is_now_complete,
                "Quest money objective progress"
            );

            if objective_is_now_complete {
                if let (Some(quest), Some(quests)) = (
                    store.get(quest_id),
                    current_quest_gameplay_snapshot_like_cpp(
                        &owner,
                        self.reward.quest_state,
                        self.reward.world_test_consumer,
                    ),
                ) {
                    let rewarded = quests.rewarded_quest_ids_like_cpp().contains(&quest_id);
                    if quests
                        .statuses_like_cpp()
                        .get(&quest_id)
                        .is_some_and(|status| {
                            wow_entities::represented_can_complete_quest_after_objective_like_cpp(
                                status,
                                &quest.objective_rules_like_cpp(),
                                objective_id,
                                rewarded,
                            )
                        })
                    {
                        quests_to_complete.push((quest_id, objective_id));
                    }
                }
            } else if objective_was_complete
                && mark_quest_incomplete_if_complete_like_cpp(
                    &owner,
                    self.reward.quest_state,
                    quest_id,
                    wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
                    wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                    self.reward.world_test_consumer,
                )
            {
                quests_to_save.push(quest_id);
            }
        }

        drop(owner);
        for (quest_id, objective_id) in quests_to_complete {
            let Some(quest) = store.get(quest_id).cloned() else {
                continue;
            };
            if self
                .complete_represented_quest_after_objective_like_cpp(&quest, objective_id)
                .await
            {
                quests_to_save.push(quest_id);
                let owner = self.reward.player.quest_objective_access_like_cpp();
                if self.current_quest_status_like_cpp(quest_id)
                    == Some(wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
                {
                    let _ = self
                        .reward
                        .send_packet_like_cpp(&QuestUpdateComplete { quest_id });
                }
                tracing::info!(
                    account = owner.account_id_like_cpp(),
                    quest_id,
                    "Quest objectives complete"
                );
            }
        }
        let owner = self.reward.player.quest_objective_access_like_cpp();
        save_changed_quest_statuses_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.catalogs,
            self.reward.lifecycle,
            &mut quests_to_save,
            self.reward.world_test_consumer,
        )
        .await;
        self.sync_player_registry_state_like_cpp(&owner);
    }

    async fn update_currency_quest_objective_progress_like_cpp(
        &mut self,
        currency_id: u32,
        change: i32,
    ) {
        let owner = self.reward.player.quest_objective_access_like_cpp();
        invalidate_player_quest_status_authority_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        );
        use wow_packet::packets::quest::QuestUpdateComplete;

        let Some(store) = self.reward.catalogs.quests.store.clone() else {
            return;
        };
        let Some(currencies) = self
            .reward
            .player
            .currency_like_cpp()
            .player_currencies_like_cpp()
        else {
            return;
        };
        let current_quantity = i64::from(
            currencies
                .get(&currency_id)
                .map(|currency| currency.quantity)
                .unwrap_or(0),
        );
        let object_id = i32::try_from(currency_id).unwrap_or(i32::MAX);
        let Some(quests) = current_quest_gameplay_snapshot_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        ) else {
            return;
        };
        let matching = wow_entities::plan_threshold_quest_objective_changes_like_cpp(
            quests.statuses_like_cpp(),
            |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
            wow_constants::quest::QUEST_OBJECTIVE_CURRENCY_LIKE_CPP,
            object_id,
            current_quantity,
            current_quantity.saturating_add(i64::from(change)),
            false,
        );

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for change in matching {
            let quest_id = change.quest_id;
            let objective_id = change.objective_id;
            let required = change.required;
            let objective_was_complete = change.objective_was_complete;
            let objective_is_now_complete = change.objective_is_now_complete;
            tracing::debug!(
                account = owner.account_id_like_cpp(),
                quest_id,
                currency_id,
                current_quantity,
                change = ?change,
                required,
                objective_was_complete,
                objective_is_now_complete,
                "Quest currency objective progress"
            );
            if objective_is_now_complete {
                if let (Some(quest), Some(quests)) = (
                    store.get(quest_id),
                    current_quest_gameplay_snapshot_like_cpp(
                        &owner,
                        self.reward.quest_state,
                        self.reward.world_test_consumer,
                    ),
                ) {
                    let rewarded = quests.rewarded_quest_ids_like_cpp().contains(&quest_id);
                    if quests
                        .statuses_like_cpp()
                        .get(&quest_id)
                        .is_some_and(|status| {
                            wow_entities::represented_can_complete_quest_after_objective_like_cpp(
                                status,
                                &quest.objective_rules_like_cpp(),
                                objective_id,
                                rewarded,
                            )
                        })
                    {
                        quests_to_complete.push((quest_id, objective_id));
                    }
                }
            } else if objective_was_complete
                && mark_quest_incomplete_if_complete_like_cpp(
                    &owner,
                    self.reward.quest_state,
                    quest_id,
                    wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
                    wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                    self.reward.world_test_consumer,
                )
            {
                quests_to_save.push(quest_id);
            }
        }

        drop(owner);
        for (quest_id, objective_id) in quests_to_complete {
            let Some(quest) = store.get(quest_id).cloned() else {
                continue;
            };
            if self
                .complete_represented_quest_after_objective_like_cpp(&quest, objective_id)
                .await
            {
                quests_to_save.push(quest_id);
                let owner = self.reward.player.quest_objective_access_like_cpp();
                if self.current_quest_status_like_cpp(quest_id)
                    == Some(wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
                {
                    let _ = self
                        .reward
                        .send_packet_like_cpp(&QuestUpdateComplete { quest_id });
                }
                tracing::info!(
                    account = owner.account_id_like_cpp(),
                    quest_id,
                    "Quest objectives complete"
                );
            }
        }
        let owner = self.reward.player.quest_objective_access_like_cpp();
        save_changed_quest_statuses_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.catalogs,
            self.reward.lifecycle,
            &mut quests_to_save,
            self.reward.world_test_consumer,
        )
        .await;
        self.sync_player_registry_state_like_cpp(&owner);
    }

    async fn update_reputation_quest_objective_progress_like_cpp(
        &mut self,
        objective_type: u8,
        faction_id: u32,
        change: i32,
    ) {
        let owner = self.reward.player.quest_objective_access_like_cpp();
        invalidate_player_quest_status_authority_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        );
        use wow_packet::packets::quest::QuestUpdateComplete;

        let Some(store) = self.reward.catalogs.quests.store.clone() else {
            return;
        };
        let Some(faction_store) = self.reward.catalogs.factions.store.as_ref() else {
            return;
        };
        let Some(faction_entry) = faction_store.get(faction_id) else {
            return;
        };

        // Read identity before consulting ReputationMgr, whose canonical read
        // re-enters the selected Player owner.
        let player_race = self.reward.player.player_race_like_cpp();
        let player_class = self.reward.player.player_class_like_cpp();
        let Some(old_reputation) = self.reward.player.reputation_for_faction_like_cpp(
            faction_entry,
            player_race,
            player_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.reputation_fixture.state_like_cpp(),
        ) else {
            return;
        };
        let new_reputation = old_reputation.saturating_add(change);
        let object_id = i32::try_from(faction_id).unwrap_or(i32::MAX);
        let Some(quests) = current_quest_gameplay_snapshot_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.world_test_consumer,
        ) else {
            return;
        };
        let matching = wow_entities::plan_threshold_quest_objective_changes_like_cpp(
            quests.statuses_like_cpp(),
            |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
            objective_type,
            object_id,
            i64::from(old_reputation),
            i64::from(new_reputation),
            objective_type == wow_constants::quest::QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP,
        );

        let mut quests_to_complete = Vec::new();
        let mut quests_to_save = Vec::new();
        for change in matching {
            let quest_id = change.quest_id;
            let objective_id = change.objective_id;
            let required = change.required;
            let objective_was_complete = change.objective_was_complete;
            let objective_is_now_complete = change.objective_is_now_complete;
            tracing::debug!(
                account = owner.account_id_like_cpp(),
                quest_id,
                objective_type,
                faction_id,
                old_reputation,
                new_reputation,
                required,
                objective_was_complete,
                objective_is_now_complete,
                "Quest reputation objective progress"
            );
            if objective_is_now_complete {
                if let (Some(quest), Some(quests)) = (
                    store.get(quest_id),
                    current_quest_gameplay_snapshot_like_cpp(
                        &owner,
                        self.reward.quest_state,
                        self.reward.world_test_consumer,
                    ),
                ) {
                    let rewarded = quests.rewarded_quest_ids_like_cpp().contains(&quest_id);
                    if quests
                        .statuses_like_cpp()
                        .get(&quest_id)
                        .is_some_and(|status| {
                            wow_entities::represented_can_complete_quest_after_objective_like_cpp(
                                status,
                                &quest.objective_rules_like_cpp(),
                                objective_id,
                                rewarded,
                            )
                        })
                    {
                        quests_to_complete.push((quest_id, objective_id));
                    }
                }
            } else if objective_was_complete
                && mark_quest_incomplete_if_complete_like_cpp(
                    &owner,
                    self.reward.quest_state,
                    quest_id,
                    wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
                    wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                    self.reward.world_test_consumer,
                )
            {
                quests_to_save.push(quest_id);
            }
        }

        drop(owner);
        for (quest_id, objective_id) in quests_to_complete {
            let Some(quest) = store.get(quest_id).cloned() else {
                continue;
            };
            if self
                .complete_represented_quest_after_objective_like_cpp(&quest, objective_id)
                .await
            {
                quests_to_save.push(quest_id);
                let owner = self.reward.player.quest_objective_access_like_cpp();
                if self.current_quest_status_like_cpp(quest_id)
                    == Some(wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
                {
                    let _ = self
                        .reward
                        .send_packet_like_cpp(&QuestUpdateComplete { quest_id });
                }
                tracing::info!(
                    account = owner.account_id_like_cpp(),
                    quest_id,
                    "Quest objectives complete"
                );
            }
        }
        let owner = self.reward.player.quest_objective_access_like_cpp();
        save_changed_quest_statuses_like_cpp(
            &owner,
            self.reward.quest_state,
            self.reward.catalogs,
            self.reward.lifecycle,
            &mut quests_to_save,
            self.reward.world_test_consumer,
        )
        .await;
        self.sync_player_registry_state_like_cpp(&owner);
    }

}
