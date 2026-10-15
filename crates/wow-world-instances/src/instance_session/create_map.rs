// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The player and group side effects of a `MapManager::CreateMap` decision
//! (#1363 slice 11b; moved from the World `session/instances/map_key.rs`, with
//! its summary from `session/map_admission.rs`).

use wow_core::ObjectGuid;

use super::InstanceSessionHostLikeCpp;
use super::admission::set_active_instance_lock_instance_id_side_effect_like_cpp;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreateMapSideEffectApplySummaryLikeCpp {
    pub player_recent_instance_sets: u32,
    pub group_recent_instance_sets: u32,
    pub skipped_group_recent_instance_sets: u32,
    pub instance_lock_creates: u32,
    pub skipped_instance_lock_creates: u32,
    pub instance_lock_instance_id_updates: u32,
    pub skipped_instance_lock_instance_id_updates: u32,
    pub pending_battleground_entry_teleports: u32,
}

pub fn apply_create_map_side_effects_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    map_id: u32,
    decision: &wow_map::CreateMapDecision,
) -> CreateMapSideEffectApplySummaryLikeCpp {
    let side_effects = match decision {
        wow_map::CreateMapDecision::Existing { side_effects, .. }
        | wow_map::CreateMapDecision::Create { side_effects, .. }
        | wow_map::CreateMapDecision::Reject { side_effects } => side_effects,
    };
    let decision_difficulty_id = match decision {
        wow_map::CreateMapDecision::Existing { difficulty_id, .. }
        | wow_map::CreateMapDecision::Create { difficulty_id, .. } => Some(*difficulty_id),
        wow_map::CreateMapDecision::Reject { .. } => None,
    };

    let mut summary = CreateMapSideEffectApplySummaryLikeCpp::default();
    for side_effect in side_effects {
        match *side_effect {
            wow_map::CreateMapSideEffect::SetPlayerRecentInstance { instance_id } => {
                if {
                    let (state, mut hub) = host.split_instances_mut();
                    state.set_represented_player_recent_instance_like_cpp(
                        &mut hub,
                        map_id,
                        instance_id,
                    )
                } {
                    summary.player_recent_instance_sets += 1;
                }
            }
            wow_map::CreateMapSideEffect::SetGroupRecentInstance {
                owner_guid_counter,
                instance_id,
            } => {
                let owner_guid = i64::try_from(owner_guid_counter)
                    .ok()
                    .map(|counter| ObjectGuid::create_player(1, counter));
                let updated = host
                    .resolved_group_guid_like_cpp()
                    .zip(owner_guid)
                    .and_then(|(group_guid, owner_guid)| {
                        host.core()
                            .directory
                            .group_registry
                            .as_ref()?
                            .set_recent_instance_transition_like_cpp(
                                group_guid,
                                map_id,
                                owner_guid,
                                instance_id,
                            )
                            .ok()
                    })
                    .is_some();
                if updated {
                    summary.group_recent_instance_sets += 1;
                } else {
                    summary.skipped_group_recent_instance_sets += 1;
                }
            }
            wow_map::CreateMapSideEffect::CreateInstanceLockForNewInstance {
                owner_guid_counter,
                instance_id,
            } => {
                let owner_guid = i64::try_from(owner_guid_counter)
                    .ok()
                    .map(|counter| ObjectGuid::create_player(1, counter));
                let created = decision_difficulty_id
                    .zip(owner_guid)
                    .and_then(|(difficulty_id, owner_guid)| {
                        let (s, h) = host.split_instances_ref();
                        s.create_instance_lock_for_new_instance_side_effect_like_cpp(
                            h,
                            map_id,
                            difficulty_id,
                            owner_guid,
                            instance_id,
                        )
                    })
                    .is_some();
                if created {
                    summary.instance_lock_creates += 1;
                } else {
                    summary.skipped_instance_lock_creates += 1;
                }
            }
            wow_map::CreateMapSideEffect::SetInstanceLockInstanceId { instance_id } => {
                let updated = decision_difficulty_id
                    .and_then(|difficulty_id| {
                        set_active_instance_lock_instance_id_side_effect_like_cpp(
                            host,
                            map_id,
                            difficulty_id,
                            instance_id,
                        )
                    })
                    .is_some();
                if updated {
                    summary.instance_lock_instance_id_updates += 1;
                } else {
                    summary.skipped_instance_lock_instance_id_updates += 1;
                }
            }
            wow_map::CreateMapSideEffect::TeleportToBattlegroundEntryPoint => {
                summary.pending_battleground_entry_teleports += 1;
            }
        }
    }

    summary
}
