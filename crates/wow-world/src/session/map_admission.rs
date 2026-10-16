// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map admission: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::TRANSFER_ABORT_DIFFICULTY_LIKE_CPP;
use super::trinity_sprintf_like_cpp;
use super::{TRANSFER_ABORT_ERROR_LIKE_CPP, Team, WorldSession};
use super::{is_player_meeting_condition_like_cpp, player_team_for_race_cpp};

pub use wow_world_core::session::map_admission::{
    MMapRuntimeConfigLikeCpp, PlayerGridLoadOutcomeLikeCpp, PlayerGridLoadResolverLikeCpp,
    WaypointPathResolverLikeCpp,
};

impl WorldSession {
    pub(in crate::session) fn access_requirement_abort_like_cpp(
        &self,
        map_id: u32,
        requested_difficulty: u8,
    ) -> Option<(u32, u8, i32)> {
        let downscaled_entries = self
            .create_map_db2_entries_like_cpp(map_id, requested_difficulty as wow_map::Difficulty)?;
        let map_difficulty_id = self
            .catalogs
            .maps
            .difficulty_store
            .as_ref()
            .and_then(|store| store.get(map_id, downscaled_entries.difficulty_id))
            .map(|entry| entry.id)
            .unwrap_or(0);
        let map_difficulty_has_message = self
            .catalogs
            .maps
            .difficulty_store
            .as_ref()
            .and_then(|store| store.get(map_id, downscaled_entries.difficulty_id))
            .map(|entry| !entry.message.is_empty())
            .unwrap_or(false);

        let failed_map_difficulty_x_condition =
            if self.core.realm_policy.instance_ignore_level_like_cpp || map_difficulty_id == 0 {
                0
            } else {
                self.catalogs
                    .maps
                    .difficulty_x_condition_store
                    .as_ref()
                    .zip(self.catalogs.player_condition_store.as_ref())
                    .and_then(|(difficulty_conditions, player_conditions)| {
                        difficulty_conditions.failed_condition_like_cpp(
                            map_difficulty_id,
                            player_conditions,
                            |condition| {
                                self.represented_player_condition_context_like_cpp()
                                    .as_ref()
                                    .and_then(|context| context.as_context(self))
                                    .is_some_and(|context| {
                                        is_player_meeting_condition_like_cpp(condition, &context)
                                    })
                            },
                        )
                    })
                    .unwrap_or(0)
            };

        let access_requirement = self
            .catalogs
            .access_requirement_store
            .as_ref()
            .and_then(|store| store.get(map_id, requested_difficulty));

        let mut level_min = 0;
        let mut level_max = 0;
        let mut missing_item = 0;
        let mut missing_quest = 0;
        let mut missing_achievement = 0;

        if let Some(access_requirement) = access_requirement {
            if !self.core.realm_policy.instance_ignore_level_like_cpp {
                if access_requirement.level_min != 0
                    && crate::session::hub_ref(self).player_level_like_cpp()
                        < access_requirement.level_min
                {
                    level_min = access_requirement.level_min;
                }
                if access_requirement.level_max != 0
                    && crate::session::hub_ref(self).player_level_like_cpp()
                        > access_requirement.level_max
                {
                    level_max = access_requirement.level_max;
                }
            }

            let item_counts = self.represented_inventory_item_counts_like_cpp()?;
            if access_requirement.item != 0 {
                if item_counts
                    .get(&access_requirement.item)
                    .copied()
                    .unwrap_or(0)
                    == 0
                    && (access_requirement.item2 == 0
                        || item_counts
                            .get(&access_requirement.item2)
                            .copied()
                            .unwrap_or(0)
                            == 0)
                {
                    missing_item = access_requirement.item;
                }
            } else if access_requirement.item2 != 0
                && item_counts
                    .get(&access_requirement.item2)
                    .copied()
                    .unwrap_or(0)
                    == 0
            {
                missing_item = access_requirement.item2;
            }

            let (quest_state, hub) = crate::session::split_quest_state_ref(self);
            let quests =
                wow_world_quest::player_quest_gameplay_snapshot_like_cpp(hub, quest_state)?;
            match player_team_for_race_cpp(crate::session::hub_ref(self).player_race_like_cpp()) {
                Team::Alliance
                    if access_requirement.quest_done_a != 0
                        && !quests
                            .rewarded_quest_ids_like_cpp()
                            .contains(&access_requirement.quest_done_a) =>
                {
                    missing_quest = access_requirement.quest_done_a;
                }
                Team::Horde
                    if access_requirement.quest_done_h != 0
                        && !quests
                            .rewarded_quest_ids_like_cpp()
                            .contains(&access_requirement.quest_done_h) =>
                {
                    missing_quest = access_requirement.quest_done_h;
                }
                _ => {}
            }

            if access_requirement.completed_achievement != 0
                && !self.access_requirement_leader_has_achievement_like_cpp(
                    access_requirement.completed_achievement,
                )
            {
                missing_achievement = access_requirement.completed_achievement;
            }
        }

        if level_min != 0
            || level_max != 0
            || failed_map_difficulty_x_condition != 0
            || missing_item != 0
            || missing_quest != 0
            || missing_achievement != 0
        {
            if missing_quest != 0
                && let Some(access_requirement) = access_requirement
                && !access_requirement.quest_failed_text.is_empty()
            {
                self.core
                    .send_system_message_like_cpp(&access_requirement.quest_failed_text);
            } else if map_difficulty_has_message || failed_map_difficulty_x_condition != 0 {
                return Some((
                    TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
                    requested_difficulty,
                    failed_map_difficulty_x_condition as i32,
                ));
            } else if missing_item != 0 {
                let level_min_text = level_min.to_string();
                let item_name = self.catalogs.item_template_name_like_cpp(missing_item);
                let notify_text = trinity_sprintf_like_cpp(
                    self.trinity_string_like_cpp(
                        wow_data::LANG_LEVEL_MINREQUIRED_AND_ITEM_LIKE_CPP,
                    ),
                    &[level_min_text.as_str(), item_name],
                );
                self.core.send_notification_like_cpp(notify_text);
            } else if level_min != 0 {
                let level_min_text = level_min.to_string();
                let notify_text = trinity_sprintf_like_cpp(
                    self.trinity_string_like_cpp(wow_data::LANG_LEVEL_MINREQUIRED_LIKE_CPP),
                    &[level_min_text.as_str()],
                );
                self.core.send_notification_like_cpp(notify_text);
            }
            return Some((TRANSFER_ABORT_ERROR_LIKE_CPP, 0, 0));
        }

        None
    }
}
