// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordered quest turn-in removal and its pre-commit publication.

use super::QuestRewardCx;
use super::super::QuestRewardDurablePlanLikeCpp;
use wow_constants::quest::QUEST_OBJECTIVE_ITEM_LIKE_CPP;
use wow_packet::packets::misc::SetCurrency;
use wow_world_inventory::ExtendedCostItemTurninChange;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::StatsFixtureRefs;

const QUEST_OBJECTIVE_CURRENCY_LIKE_CPP: u8 = 4;
const QUEST_FLAGS_REMOVE_SURPLUS_ITEMS_LIKE_CPP: u32 = 0x0200_0000;
const QUEST_FLAGS_EX_NO_ITEM_REMOVAL_LIKE_CPP: u32 = 0x0000_0001;
const CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP: i32 = 3;

impl QuestRewardCx<'_> {
    fn apply_item_turnin_changes_like_cpp(
        &mut self,
        player_guid: wow_core::ObjectGuid,
        map_id: u16,
        changes: &[ExtendedCostItemTurninChange],
        #[cfg(any(test, feature = "test-fixtures"))]
        stats_fixtures: StatsFixtureRefs<'_>,
    ) {
        let access = self.player.inventory_like_cpp();
        let publication = self.player.packet_publication_access_like_cpp();
        let send_stat_update = self.inventory.apply_item_turnin_changes_with_access_like_cpp(
            &access,
            &publication,
            self.catalogs.items.store.as_ref(),
            self.catalogs.items.stats_store.as_ref(),
            player_guid,
            map_id,
            changes,
        );
        if send_stat_update {
            #[cfg(any(test, feature = "test-fixtures"))]
            let player = self.player.stats_access_like_cpp(
                self.catalogs, self.config, &*self.player_level, stats_fixtures,
            );
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let player = self.player.stats_access_like_cpp(self.catalogs, self.config);
            crate::CharacterStatsApplicationCxLikeCpp::new(
                player, self.inventory, publication,
            ).send_stat_update_like_cpp();
        }
    }

    pub async fn remove_quest_required_items_and_currencies_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        quest: &wow_data::quest::QuestTemplate,
        #[cfg(any(test, feature = "test-fixtures"))]
        stats_fixtures: StatsFixtureRefs<'_>,
    ) -> bool {
        let Some(player_guid) = self.player.player_guid_like_cpp() else {
            return false;
        };
        let map_id = self.player.player_map_id_like_cpp();
        let mut item_changes = Vec::new();
        let Some(currency_snapshot) = self.inventory
            .player_currencies_with_quest_reward_access_like_cpp(&self.player)
        else {
            return false;
        };
        let mut currency_losses = Vec::new();

        for objective in &quest.objectives {
            match objective.obj_type {
                QUEST_OBJECTIVE_ITEM_LIKE_CPP => {
                    let Ok(item_entry) = u32::try_from(objective.object_id) else {
                        return false;
                    };
                    let count = if (quest.flags & QUEST_FLAGS_REMOVE_SURPLUS_ITEMS_LIKE_CPP) != 0 {
                        u32::MAX
                    } else {
                        u32::try_from(objective.amount).unwrap_or(u32::MAX)
                    };
                    let Some(mut changes) = self
                        .plan_quest_reward_item_removal_like_cpp(item_entry, count)
                    else {
                        return false;
                    };
                    item_changes.append(&mut changes);
                }
                QUEST_OBJECTIVE_CURRENCY_LIKE_CPP => {
                    let (Ok(currency_id), Ok(amount)) = (
                        u32::try_from(objective.object_id),
                        u32::try_from(objective.amount),
                    ) else {
                        return false;
                    };
                    let access = self.player.currency_like_cpp();
                    let Some(before) = self.inventory
                        .player_currency_quantity_with_access_like_cpp(&access, currency_id)
                    else {
                        self.inventory.set_player_currencies_with_access_like_cpp(
                            &access, currency_snapshot,
                        );
                        return false;
                    };
                    if !self.inventory.remove_currency_with_access_like_cpp(
                        &access, currency_id, amount,
                    ) {
                        self.inventory.set_player_currencies_with_access_like_cpp(
                            &access, currency_snapshot,
                        );
                        return false;
                    }
                    let Some(after) = self.inventory
                        .player_currency_quantity_with_access_like_cpp(&access, currency_id)
                    else {
                        self.inventory.set_player_currencies_with_access_like_cpp(
                            &access, currency_snapshot,
                        );
                        return false;
                    };
                    let removed = before.saturating_sub(after);
                    if removed > 0 {
                        currency_losses.push((currency_id, after, removed));
                    }
                }
                _ => {}
            }
        }

        if (quest.flags_ex & QUEST_FLAGS_EX_NO_ITEM_REMOVAL_LIKE_CPP) == 0 {
            for (item_entry, count) in quest.item_drop.iter().zip(quest.item_drop_quantity.iter()) {
                if *item_entry == 0 {
                    continue;
                }
                let count = if *count == 0 { u32::MAX } else { *count };
                let Some(mut changes) = self
                    .plan_quest_reward_item_removal_like_cpp(*item_entry, count)
                else {
                    self.inventory.set_player_currencies_with_quest_reward_access_like_cpp(
                        &self.player, currency_snapshot,
                    );
                    return false;
                };
                item_changes.append(&mut changes);
            }
        }

        {
            let items = item_changes
                .iter()
                .map(|change| match *change {
                    ExtendedCostItemTurninChange::Update {
                        db_guid, new_count, ..
                    } => wow_persistence::QuestTurnInItemPersistenceLikeCpp::Update {
                        item_guid: db_guid,
                        new_count,
                    },
                    ExtendedCostItemTurninChange::Delete { db_guid, .. } => {
                        wow_persistence::QuestTurnInItemPersistenceLikeCpp::Delete {
                            item_guid: db_guid,
                        }
                    }
                })
                .collect();
            // The removals join the operation's single character transaction.
            // Their currency half is empty here because `_SaveCurrency` writes
            // the complete state once when the operation closes.
            plan.push_inventory_mutation(
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestTurnIn(
                    wow_persistence::QuestTurnInPersistenceLikeCpp {
                        owner_guid: player_guid.counter() as u64,
                        items,
                        currency_save: wow_persistence::PlayerCurrencySaveRequestLikeCpp {
                            player_guid: player_guid.counter() as u64,
                            rows: Vec::new(),
                        },
                    },
                ),
            );
        }

        self.apply_item_turnin_changes_like_cpp(
            player_guid, map_id, &item_changes,
            #[cfg(any(test, feature = "test-fixtures"))]
            stats_fixtures,
        );
        for (currency_id, quantity, removed) in currency_losses {
            let (Some(quantity), Some(removed)) =
                (i32::try_from(quantity).ok(), i32::try_from(removed).ok())
            else {
                continue;
            };
            self.send_packet_like_cpp(&SetCurrency {
                type_id: currency_id as i32,
                quantity,
                flags: 0,
                weekly_quantity: None,
                tracked_quantity: None,
                max_quantity: None,
                total_earned: None,
                suppress_chat_log: false,
                quantity_change: Some(-removed),
                quantity_gain_source: None,
                quantity_lost_source: Some(CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP),
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            });
        }

        true
    }
}
