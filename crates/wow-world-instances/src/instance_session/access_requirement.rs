// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `Map::PlayerCannotEnter`'s access-requirement step and the
//! `InstanceTemplate` leader-achievement check (#1363 slice 12d; moved from the
//! World `session/map_admission.rs` and `session/collections/operations.rs`,
//! with the `trinity_sprintf_like_cpp` formatter from
//! `session/connection_identity.rs`). Each host call is the same-named World
//! operation the body called before the move.

use wow_constants::unit::Team;
use wow_world_core::session::state::hub_support::player_team_for_race_cpp;
use wow_world_social::SocialSessionHostLikeCpp;
use wow_world_spell::TalentHostLikeCpp;

use super::{TRANSFER_ABORT_DIFFICULTY_LIKE_CPP, TRANSFER_ABORT_ERROR_LIKE_CPP};
use crate::GroupSessionHostLikeCpp;

pub fn access_requirement_abort_like_cpp<H>(
    host: &H,
    map_id: u32,
    requested_difficulty: u8,
) -> Option<(u32, u8, i32)>
where
    H: GroupSessionHostLikeCpp + SocialSessionHostLikeCpp + TalentHostLikeCpp + ?Sized,
{
    let (instances, hub) = host.split_instances_ref();
    let downscaled_entries = instances.create_map_db2_entries_like_cpp(
        hub,
        map_id,
        requested_difficulty as wow_map::Difficulty,
    )?;
    let map_difficulty_id = host
        .hub_ref()
        .catalogs
        .maps
        .difficulty_store
        .as_ref()
        .and_then(|store| store.get(map_id, downscaled_entries.difficulty_id))
        .map(|entry| entry.id)
        .unwrap_or(0);
    let map_difficulty_has_message = host
        .hub_ref()
        .catalogs
        .maps
        .difficulty_store
        .as_ref()
        .and_then(|store| store.get(map_id, downscaled_entries.difficulty_id))
        .map(|entry| !entry.message.is_empty())
        .unwrap_or(false);

    let failed_map_difficulty_x_condition =
        if host.core().realm_policy.instance_ignore_level_like_cpp || map_difficulty_id == 0 {
            0
        } else {
            host.hub_ref()
                .catalogs
                .maps
                .difficulty_x_condition_store
                .as_ref()
                .zip(host.hub_ref().catalogs.player_condition_store.as_ref())
                .and_then(|(difficulty_conditions, player_conditions)| {
                    difficulty_conditions.failed_condition_like_cpp(
                        map_difficulty_id,
                        player_conditions,
                        |condition| host.represented_player_meets_condition_like_cpp(condition),
                    )
                })
                .unwrap_or(0)
        };

    let access_requirement = host
        .hub_ref()
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
        if !host.core().realm_policy.instance_ignore_level_like_cpp {
            if access_requirement.level_min != 0
                && host.hub_ref().player_level_like_cpp() < access_requirement.level_min
            {
                level_min = access_requirement.level_min;
            }
            if access_requirement.level_max != 0
                && host.hub_ref().player_level_like_cpp() > access_requirement.level_max
            {
                level_max = access_requirement.level_max;
            }
        }

        let item_counts = host.represented_inventory_item_counts_like_cpp()?;
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

        let quests = SocialSessionHostLikeCpp::player_quest_gameplay_snapshot_like_cpp(host)?;
        match player_team_for_race_cpp(host.hub_ref().player_race_like_cpp()) {
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
            && !access_requirement_leader_has_achievement_like_cpp(
                host,
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
            host.core()
                .send_system_message_like_cpp(&access_requirement.quest_failed_text);
        } else if map_difficulty_has_message || failed_map_difficulty_x_condition != 0 {
            return Some((
                TRANSFER_ABORT_DIFFICULTY_LIKE_CPP,
                requested_difficulty,
                failed_map_difficulty_x_condition as i32,
            ));
        } else if missing_item != 0 {
            let level_min_text = level_min.to_string();
            let item_name = host
                .hub_ref()
                .catalogs
                .item_template_name_like_cpp(missing_item);
            let notify_text = trinity_sprintf_like_cpp(
                TalentHostLikeCpp::trinity_string_like_cpp(
                    host,
                    wow_data::LANG_LEVEL_MINREQUIRED_AND_ITEM_LIKE_CPP,
                ),
                &[level_min_text.as_str(), item_name],
            );
            host.core().send_notification_like_cpp(notify_text);
        } else if level_min != 0 {
            let level_min_text = level_min.to_string();
            let notify_text = trinity_sprintf_like_cpp(
                TalentHostLikeCpp::trinity_string_like_cpp(
                    host,
                    wow_data::LANG_LEVEL_MINREQUIRED_LIKE_CPP,
                ),
                &[level_min_text.as_str()],
            );
            host.core().send_notification_like_cpp(notify_text);
        }
        return Some((TRANSFER_ABORT_ERROR_LIKE_CPP, 0, 0));
    }

    None
}

pub fn access_requirement_leader_has_achievement_like_cpp<H: GroupSessionHostLikeCpp + ?Sized>(
    host: &H,
    achievement_id: u32,
) -> bool {
    if achievement_id == 0 {
        return true;
    }

    let Some(player_guid) = host.core().player_guid else {
        return false;
    };
    let leader_guid = crate::group_membership::resolved_group_guid_like_cpp(host)
        .and_then(|group_guid| {
            host.core()
                .directory
                .group_registry
                .as_ref()?
                .get(&group_guid)
        })
        .map(|group| group.leader_guid)
        .unwrap_or(player_guid);
    if leader_guid == player_guid {
        return host
            .hub_ref()
            .completed_achievement_ids_snapshot_like_cpp()
            .is_some_and(|achievements| achievements.contains(&achievement_id));
    }

    host.core()
        .player_registry
        .as_ref()
        .is_some_and(|registry| {
            registry.connected_player_has_achievement(leader_guid, achievement_id)
        })
}

fn trinity_sprintf_like_cpp(format: &str, args: &[&str]) -> String {
    let mut output = String::with_capacity(format.len());
    let mut chars = format.chars().peekable();
    let mut arg_idx = 0usize;

    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }

        match chars.next() {
            Some('%') => output.push('%'),
            Some('u' | 'd' | 's') => {
                if let Some(arg) = args.get(arg_idx) {
                    output.push_str(arg);
                    arg_idx += 1;
                }
            }
            Some(other) => {
                output.push('%');
                output.push(other);
            }
            None => output.push('%'),
        }
    }

    output
}
