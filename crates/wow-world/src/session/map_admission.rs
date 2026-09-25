// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map admission: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::trinity_sprintf_like_cpp;
use super::{Arc, HashSet, ObjectGuid, Position, TRANSFER_ABORT_DIFFICULTY_LIKE_CPP};
use super::{TRANSFER_ABORT_ERROR_LIKE_CPP, Team, WorldSession};
use super::{is_player_meeting_condition_like_cpp, player_team_for_race_cpp};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CreateMapSideEffectApplySummaryLikeCpp {
    pub player_recent_instance_sets: u32,
    pub group_recent_instance_sets: u32,
    pub skipped_group_recent_instance_sets: u32,
    pub instance_lock_creates: u32,
    pub skipped_instance_lock_creates: u32,
    pub instance_lock_instance_id_updates: u32,
    pub skipped_instance_lock_instance_id_updates: u32,
    pub pending_battleground_entry_teleports: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MMapRuntimeConfigLikeCpp {
    pub data_dir: String,
    pub enabled: bool,
    pub disabled_map_ids: HashSet<u32>,
}

pub type WaypointPathResolverLikeCpp =
    Arc<dyn Fn(u32) -> Option<wow_movement::WaypointPath> + Send + Sync>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerGridLoadOutcomeLikeCpp {
    pub map_unavailable: bool,
    pub map_created: bool,
    pub grid_loaded_now: bool,
    pub metadata_entries: usize,
    pub skipped_already_loaded: usize,
    pub skipped_should_not_spawn: usize,
    pub skipped_difficulty_mismatch: usize,
    pub stale_index_entries: usize,
    pub creature_records_added: usize,
    pub gameobject_records_added: usize,
    pub area_trigger_records_added: usize,
    pub pre_add_records_added: usize,
    pub add_to_map_errors: usize,
    pub load_record_missing: usize,
    pub creature_load_record_missing: usize,
    pub gameobject_load_record_missing: usize,
    pub area_trigger_load_record_missing: usize,
    pub legacy_creature_mirrors: usize,
}

pub type PlayerGridLoadResolverLikeCpp =
    Arc<dyn Fn(u16, Option<u32>, Position) -> PlayerGridLoadOutcomeLikeCpp + Send + Sync>;

impl Default for MMapRuntimeConfigLikeCpp {
    fn default() -> Self {
        Self {
            data_dir: "./Data".to_string(),
            enabled: true,
            disabled_map_ids: HashSet::new(),
        }
    }
}

impl MMapRuntimeConfigLikeCpp {
    pub fn pathfinding_enabled_for_map_like_cpp(&self, map_id: u32) -> bool {
        self.enabled && !self.disabled_map_ids.contains(&map_id)
    }

    pub fn should_try_pathfinding_like_cpp(
        &self,
        map_id: u32,
        owner_ignores_pathfinding: bool,
    ) -> bool {
        self.pathfinding_enabled_for_map_like_cpp(map_id) && !owner_ignores_pathfinding
    }
}

pub(in crate::session) fn create_map_instance_lock_token_like_cpp(
    owner_guid: ObjectGuid,
    entries: &wow_instances::MapDb2Entries,
    lock: &wow_instances::InstanceLock,
) -> u64 {
    fn mix(hash: &mut u64, value: u64) {
        *hash ^= value;
        *hash = hash.wrapping_mul(0x1000_0000_01b3);
    }

    let mut hash = 0xcbf2_9ce4_8422_2325;
    mix(&mut hash, owner_guid.high_value() as u64);
    mix(&mut hash, owner_guid.low_value() as u64);
    mix(&mut hash, u64::from(entries.map_id));
    mix(&mut hash, u64::from(entries.lock_id));
    mix(&mut hash, u64::from(entries.difficulty_id));
    mix(&mut hash, u64::from(lock.map_id));
    mix(&mut hash, u64::from(lock.difficulty_id));
    hash
}

pub(in crate::session) fn create_map_decision_difficulty_id_like_cpp(
    decision: &wow_map::CreateMapDecision,
) -> Option<wow_map::Difficulty> {
    match decision {
        wow_map::CreateMapDecision::Existing { difficulty_id, .. }
        | wow_map::CreateMapDecision::Create { difficulty_id, .. } => Some(*difficulty_id),
        wow_map::CreateMapDecision::Reject { .. } => None,
    }
}

pub(in crate::session) fn create_map_decision_key_like_cpp(
    decision: &wow_map::CreateMapDecision,
) -> Option<wow_map::MapKey> {
    match decision {
        wow_map::CreateMapDecision::Existing { key, .. }
        | wow_map::CreateMapDecision::Create { key, .. } => Some(*key),
        wow_map::CreateMapDecision::Reject { .. } => None,
    }
}

impl WorldSession {
    pub(in crate::session) fn access_requirement_abort_like_cpp(
        &self,
        map_id: u32,
        requested_difficulty: u8,
    ) -> Option<(u32, u8, i32)> {
        let downscaled_entries = self
            .create_map_db2_entries_like_cpp(map_id, requested_difficulty as wow_map::Difficulty)?;
        let map_difficulty_id = self
            .maps
            .difficulty_store
            .as_ref()
            .and_then(|store| store.get(map_id, downscaled_entries.difficulty_id))
            .map(|entry| entry.id)
            .unwrap_or(0);
        let map_difficulty_has_message = self
            .maps
            .difficulty_store
            .as_ref()
            .and_then(|store| store.get(map_id, downscaled_entries.difficulty_id))
            .map(|entry| !entry.message.is_empty())
            .unwrap_or(false);

        let failed_map_difficulty_x_condition =
            if self.instance_ignore_level_like_cpp || map_difficulty_id == 0 {
                0
            } else {
                self.maps
                    .difficulty_x_condition_store
                    .as_ref()
                    .zip(self.player_condition_store.as_ref())
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
            .access_requirement_store
            .as_ref()
            .and_then(|store| store.get(map_id, requested_difficulty));

        let mut level_min = 0;
        let mut level_max = 0;
        let mut missing_item = 0;
        let mut missing_quest = 0;
        let mut missing_achievement = 0;

        if let Some(access_requirement) = access_requirement {
            if !self.instance_ignore_level_like_cpp {
                if access_requirement.level_min != 0
                    && self.player_level_like_cpp() < access_requirement.level_min
                {
                    level_min = access_requirement.level_min;
                }
                if access_requirement.level_max != 0
                    && self.player_level_like_cpp() > access_requirement.level_max
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

            let quests = self.player_quest_gameplay_snapshot_like_cpp()?;
            match player_team_for_race_cpp(self.player_race_like_cpp()) {
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
                self.send_system_message_like_cpp(&access_requirement.quest_failed_text);
            } else if map_difficulty_has_message || failed_map_difficulty_x_condition != 0 {
                return Some((
                    TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
                    requested_difficulty,
                    failed_map_difficulty_x_condition as i32,
                ));
            } else if missing_item != 0 {
                let level_min_text = level_min.to_string();
                let item_name = self.item_template_name_like_cpp(missing_item);
                let notify_text = trinity_sprintf_like_cpp(
                    self.trinity_string_like_cpp(
                        wow_data::LANG_LEVEL_MINREQUIRED_AND_ITEM_LIKE_CPP,
                    ),
                    &[level_min_text.as_str(), item_name],
                );
                self.send_notification_like_cpp(notify_text);
            } else if level_min != 0 {
                let level_min_text = level_min.to_string();
                let notify_text = trinity_sprintf_like_cpp(
                    self.trinity_string_like_cpp(wow_data::LANG_LEVEL_MINREQUIRED_LIKE_CPP),
                    &[level_min_text.as_str()],
                );
                self.send_notification_like_cpp(notify_text);
            }
            return Some((TRANSFER_ABORT_ERROR_LIKE_CPP, 0, 0));
        }

        None
    }
}
