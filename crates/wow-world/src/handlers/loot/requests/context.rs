// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player/group context and quest gates used by loot requests.

use super::*;

impl WorldSession {
    /// C++ `Loot::FillLoot` calls `FillNotNormalLootFor` for every connected
    /// group member at reward distance from the opening player before the
    /// chest's shared `Loot` becomes visible.
    pub(in crate::handlers::loot) fn represented_group_looters_at_reward_distance_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(group_guid) = self.resolved_group_guid_like_cpp() else {
            return vec![player_guid];
        };
        let Some(group_registry) = self.group_registry() else {
            return vec![player_guid];
        };
        let Some(group) = group_registry.get(&group_guid) else {
            return vec![player_guid];
        };
        let Some(source_position) = self.player_position_like_cpp() else {
            return vec![player_guid];
        };
        let map_id = self.player_map_id_like_cpp();
        let Some(instance_id) = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
        else {
            return vec![player_guid];
        };
        let registry = self.player_registry();
        let mut looters = Vec::new();

        for member_guid in &group.members {
            if *member_guid == player_guid {
                looters.push(*member_guid);
                continue;
            }
            let Some(member) = registry.and_then(|registry| registry.loot_presence(*member_guid))
            else {
                continue;
            };
            if member.is_in_world
                && member.map_id == map_id
                && member.instance_id == instance_id
                && (self.current_map_is_dungeon_like_cpp()
                    || source_position.is_within_dist(&member.position, 74.0))
            {
                looters.push(*member_guid);
            }
        }

        if looters.is_empty() {
            looters.push(player_guid);
        }
        looters.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        looters.dedup();
        looters
    }

    pub(in crate::handlers::loot) fn represented_dungeon_trash_looter_like_cpp(
        &self,
        connected_tappers: &[ObjectGuid],
    ) -> ObjectGuid {
        let selected = if let (Some(group_guid), Some(registry)) =
            (self.resolved_group_guid_like_cpp(), self.group_registry())
        {
            registry
                .get(&group_guid)
                .map(|group| group.looter_guid_like_cpp())
                .filter(|looter| connected_tappers.contains(looter))
        } else {
            None
        };
        selected.unwrap_or(connected_tappers[0])
    }

    pub(in crate::handlers::loot) fn advance_represented_dungeon_trash_looter_like_cpp(
        &self,
        connected_tappers: &[ObjectGuid],
    ) {
        let (Some(group_guid), Some(registry)) =
            (self.resolved_group_guid_like_cpp(), self.group_registry())
        else {
            return;
        };
        let _ = registry
            .advance_looter_transition_like_cpp(group_guid, connected_tappers.iter().copied());
    }

    pub(in crate::handlers::loot) fn item_loot_quest_status_allows_for_player_like_cpp(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        if player_context.is_current {
            return self.item_loot_quest_status_allows_like_cpp(
                item_id,
                needs_quest,
                addon_metadata,
            );
        }

        if addon_metadata.ignores_quest_status() {
            return true;
        }

        let start_quest_id = self.item_template_start_quest_id(item_id).unwrap_or(0);
        let has_non_none_start_quest_status = u32::try_from(start_quest_id)
            .ok()
            .is_some_and(|quest_id| quest_id != 0 && player_context.quest_status(quest_id) != 0);
        let has_quest_for_item =
            self.represented_has_quest_for_item_like_cpp(item_id, addon_metadata, player_context);

        (!needs_quest && !has_non_none_start_quest_status) || has_quest_for_item
    }

    pub(in crate::handlers::loot) fn represented_loot_player_context_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<RepresentedLootPlayerContext> {
        if Some(player_guid) == self.player_guid() {
            let quests = self.player_quest_gameplay_snapshot_like_cpp()?;
            return Some(RepresentedLootPlayerContext {
                race: self.player_race_like_cpp(),
                class: self.player_class_like_cpp(),
                gender: self.player_gender_like_cpp(),
                level: self.player_level_like_cpp(),
                known_spells: self.known_spells_like_cpp().to_vec(),
                active_quest_statuses: quests
                    .statuses_like_cpp()
                    .iter()
                    .map(|(quest_id, status)| (*quest_id, status.status))
                    .collect(),
                active_quest_objective_counts: quests
                    .statuses_like_cpp()
                    .iter()
                    .map(|(quest_id, status)| (*quest_id, status.objective_counts.clone()))
                    .collect(),
                rewarded_quests: quests
                    .rewarded_quest_ids_like_cpp()
                    .iter()
                    .copied()
                    .collect(),
                inventory_item_counts: self.represented_inventory_item_counts_like_cpp()?,
                is_current: true,
            });
        }

        let player = self.player_registry()?.loot_player_context(player_guid)?;
        Some(RepresentedLootPlayerContext {
            race: player.race,
            class: player.class,
            gender: player.sex,
            level: player.level,
            known_spells: player.known_spells.clone(),
            active_quest_statuses: player.active_quest_statuses.clone(),
            active_quest_objective_counts: player.active_quest_objective_counts.clone(),
            rewarded_quests: player.rewarded_quests.clone(),
            inventory_item_counts: player.inventory_item_counts.clone(),
            is_current: false,
        })
    }

    pub(in crate::handlers::loot) fn item_template_flags2_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<u32> {
        self.item_stats_store()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.flags[1])
    }

    pub(in crate::handlers::loot) fn item_loot_quest_status_allows_like_cpp(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    ) -> bool {
        let start_quest_id = self.item_template_start_quest_id(item_id).unwrap_or(0);
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let has_non_none_start_quest_status =
            u32::try_from(start_quest_id).ok().is_some_and(|quest_id| {
                quest_id != 0
                    && (quests.statuses_like_cpp().contains_key(&quest_id)
                        || quests.rewarded_quest_ids_like_cpp().contains(&quest_id))
            });
        let has_quest_for_item = self.has_incomplete_quest_objective_for_item_like_cpp(item_id)
            || (addon_metadata.quest_log_item_id != 0
                && self.has_incomplete_quest_objective_for_object_id_like_cpp(
                    addon_metadata.quest_log_item_id,
                ))
            || self.has_incomplete_quest_item_drop_for_item_like_cpp(item_id);

        addon_metadata.ignores_quest_status()
            || ((!needs_quest && !has_non_none_start_quest_status) || has_quest_for_item)
    }

    pub(in crate::handlers::loot) fn has_incomplete_quest_objective_for_item_like_cpp(
        &self,
        item_id: u32,
    ) -> bool {
        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };
        self.has_incomplete_quest_objective_for_object_id_like_cpp(item_object_id)
    }

    fn has_incomplete_quest_objective_for_object_id_like_cpp(&self, item_object_id: i32) -> bool {
        let Some(quest_store) = &self.quests.store else {
            return false;
        };

        self.player_quest_gameplay_snapshot_like_cpp()
            .is_some_and(|state| {
                state.statuses_like_cpp().values().any(|status| {
                    if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                        return false;
                    }

                    let Some(quest) = quest_store.get(status.quest_id) else {
                        return false;
                    };

                    quest
                        .objectives
                        .iter()
                        .enumerate()
                        .any(|(fallback_index, objective)| {
                            if objective.obj_type != 1 || objective.object_id != item_object_id {
                                return false;
                            }

                            let storage_index = usize::try_from(objective.storage_index)
                                .ok()
                                .unwrap_or(fallback_index);
                            let current = status
                                .objective_counts
                                .get(storage_index)
                                .copied()
                                .unwrap_or(0);
                            current < objective.amount.max(1)
                        })
                })
            })
    }

    fn represented_has_quest_for_item_like_cpp(
        &self,
        item_id: u32,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        if player_context.is_current {
            return self.has_incomplete_quest_objective_for_item_like_cpp(item_id)
                || (addon_metadata.quest_log_item_id != 0
                    && self.has_incomplete_quest_objective_for_object_id_like_cpp(
                        addon_metadata.quest_log_item_id,
                    ))
                || self.has_incomplete_quest_item_drop_for_item_like_cpp(item_id);
        }

        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };
        self.remote_has_incomplete_quest_objective_for_object_id_like_cpp(
            item_object_id,
            player_context,
        ) || (addon_metadata.quest_log_item_id != 0
            && self.remote_has_incomplete_quest_objective_for_object_id_like_cpp(
                addon_metadata.quest_log_item_id,
                player_context,
            ))
            || self.remote_has_incomplete_quest_item_drop_for_item_like_cpp(item_id, player_context)
    }

    fn remote_has_incomplete_quest_objective_for_object_id_like_cpp(
        &self,
        item_object_id: i32,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        let Some(quest_store) = &self.quests.store else {
            return false;
        };

        player_context
            .active_quest_objective_counts
            .iter()
            .any(|(quest_id, objective_counts)| {
                if player_context.quest_status(*quest_id) != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                    return false;
                }

                let Some(quest) = quest_store.get(*quest_id) else {
                    return false;
                };

                quest
                    .objectives
                    .iter()
                    .enumerate()
                    .any(|(fallback_index, objective)| {
                        if objective.obj_type != 1 || objective.object_id != item_object_id {
                            return false;
                        }

                        let storage_index = usize::try_from(objective.storage_index)
                            .ok()
                            .unwrap_or(fallback_index);
                        let current = objective_counts.get(storage_index).copied().unwrap_or(0);
                        current < objective.amount.max(1)
                    })
            })
    }

    pub(in crate::handlers::loot) fn direct_inventory_item_count_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<u32> {
        Some(
            self.represented_inventory_item_counts_like_cpp()?
                .get(&item_id)
                .copied()
                .unwrap_or(0),
        )
    }

    pub(in crate::handlers::loot) fn player_quest_objective_progress_like_cpp(
        &self,
        objective_id: u32,
    ) -> Option<i32> {
        let quest_store = self.quests.store.as_ref()?;

        for status in self
            .player_quest_gameplay_snapshot_like_cpp()?
            .statuses_like_cpp()
            .values()
        {
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            let Some((_, objective)) = quest
                .objectives
                .iter()
                .enumerate()
                .find(|(_, objective)| objective.id == objective_id)
            else {
                continue;
            };
            let objective_index = objective.storage_index.max(0) as usize;
            return Some(
                status
                    .objective_counts
                    .get(objective_index)
                    .copied()
                    .unwrap_or(0),
            );
        }

        None
    }

    pub(in crate::handlers::loot) fn remote_player_quest_objective_progress_like_cpp(
        &self,
        objective_id: u32,
        player_context: &RepresentedLootPlayerContext,
    ) -> Option<i32> {
        let quest_store = self.quests.store.as_ref()?;

        for (quest_id, objective_counts) in &player_context.active_quest_objective_counts {
            let Some(quest) = quest_store.get(*quest_id) else {
                continue;
            };
            let Some((_, objective)) = quest
                .objectives
                .iter()
                .enumerate()
                .find(|(_, objective)| objective.id == objective_id)
            else {
                continue;
            };
            let objective_index = objective.storage_index.max(0) as usize;
            return Some(objective_counts.get(objective_index).copied().unwrap_or(0));
        }

        None
    }

    pub(in crate::handlers::loot) async fn load_item_template_addon_loot_metadata_for_item_ids_like_cpp<
        I,
    >(
        &self,
        item_ids: I,
    ) -> HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp>
    where
        I: IntoIterator<Item = u32>,
    {
        let mut item_ids: Vec<u32> = item_ids.into_iter().collect();
        item_ids.sort_unstable();
        item_ids.dedup();

        let mut metadata = HashMap::with_capacity(item_ids.len());
        for item_id in item_ids {
            metadata.insert(
                item_id,
                self.load_creature_item_template_addon_loot_metadata_like_cpp(item_id)
                    .await,
            );
        }
        metadata
    }
}
