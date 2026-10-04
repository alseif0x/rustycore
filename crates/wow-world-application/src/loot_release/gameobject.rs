// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;
use std::time::{Duration, Instant};
use wow_entities::{GoState, LootState};
use wow_packet::packets::update::UpdateObject;

impl LootReleaseCxLikeCpp<'_> {
    pub(super) fn queue_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let access = self.owner.transitions_like_cpp();
        let Some(routing) = access.chest_routing_like_cpp() else {
            return 0;
        };
        let Some(state) = self.world_entities.represented_gameobject_use_state_like_cpp(gameobject_guid) else {
            return 0;
        };
        let Some(source) = state.chest_loot_source else {
            return 0;
        };
        let command = wow_world_core::session::mailbox::SyncChestGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: access.player_map_id_like_cpp(),
            instance_id: access.loot_instance_id_like_cpp(),
            go_type: state.go_type.unwrap_or(GAMEOBJECT_TYPE_CHEST as u8),
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            chest_loot_id: source.loot_id,
            chest_personal_loot_id: source.personal_loot_id,
            chest_push_loot_id: source.push_loot_id,
            chest_quest_id: source.chest_quest_id,
            chest_restock_time_secs: source.chest_restock_time_secs,
            chest_consumable: source.chest_consumable,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        };
        routing.queue_like_cpp(command)
    }

    pub(super) fn apply_represented_gameobject_loot_release_like_cpp(
        &mut self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
        selected_pool_looted: bool,
        mut whole_object_fully_looted: bool,
        authoritative_release: Option<&AuthoritativeLootReleaseLikeCpp>,
    ) {
        let go_type = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid)
            .and_then(|state| state.go_type)
            .map(u32::from);
        let represented_chest_restock_time_secs = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid)
            .and_then(|state| state.chest_restock_time_secs)
            .unwrap_or_default();
        let represented_personal_loot_uses_after_release = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid)
            .map(|state| state.personal_loot_uses.saturating_add(1))
            .unwrap_or(1);
        // C++ `FishingHole.MaxOpens` is still template evidence from the represented
        // GO value; the use counter source-of-truth is canonical `GameObject::use_times`
        // when the canonical GameObject can be mutated.
        let represented_fishing_hole_max_opens = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid)
            .and_then(|state| state.fishing_hole_max_opens);
        let canonical_fishing_hole_release = (go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE))
            .then(|| {
                self.owner.transitions_like_cpp().release_canonical_fishing_hole_like_cpp(
                    guid,
                    represented_fishing_hole_max_opens,
                )
            })
            .flatten();
        let canonical_fishing_hole_use_count_after_release = canonical_fishing_hole_release
            .as_ref()
            .map(|(use_count, _, _)| *use_count);

        let guarded_global_transition_attempted = selected_pool_looted
            && whole_object_fully_looted
            && !matches!(
                go_type,
                Some(GAMEOBJECT_TYPE_FISHING_NODE)
                    | Some(GAMEOBJECT_TYPE_FISHING_HOLE)
                    | Some(GAMEOBJECT_TYPE_GATHERING_NODE)
            )
            && authoritative_release.is_some();
        let guarded_global_transition = authoritative_release
            .filter(|_| guarded_global_transition_attempted)
            .and_then(|release| {
                if release.require_no_viewers {
                    self.owner.transitions_like_cpp().set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation_like_cpp(
                        guid,
                        &release.authority,
                        release.object_generation,
                        release.lifecycle_revision,
                        LootState::JustDeactivated,
                        None,
                        represented_chest_restock_time_secs,
                        false,
                    )
                } else {
                    self.owner.transitions_like_cpp().set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
                        guid,
                        &release.authority,
                        release.object_generation,
                        release.lifecycle_revision,
                        LootState::JustDeactivated,
                        None,
                        represented_chest_restock_time_secs,
                        false,
                    )
                }
            });
        if guarded_global_transition_attempted && guarded_global_transition.is_none() {
            // An upsert/install/replacement won the serialization point after
            // close. Its new pool must keep the object globally active.
            whole_object_fully_looted = false;
        }

        let canonical_loot_state_request = match go_type {
            Some(GAMEOBJECT_TYPE_FISHING_NODE) => Some((LootState::JustDeactivated, None, false)),
            Some(GAMEOBJECT_TYPE_FISHING_HOLE) if canonical_fishing_hole_release.is_some() => None,
            Some(GAMEOBJECT_TYPE_FISHING_HOLE) => {
                let use_count_after_release = canonical_fishing_hole_use_count_after_release
                    .unwrap_or(represented_personal_loot_uses_after_release);
                let state = if represented_fishing_hole_max_opens
                    .is_some_and(|max_opens| use_count_after_release >= max_opens)
                {
                    LootState::JustDeactivated
                } else {
                    LootState::Ready
                };
                Some((state, None, false))
            }
            Some(GAMEOBJECT_TYPE_GATHERING_NODE) if selected_pool_looted => None,
            _ if guarded_global_transition_attempted => None,
            _ if selected_pool_looted && whole_object_fully_looted => {
                Some((LootState::JustDeactivated, None, false))
            }
            _ if selected_pool_looted => None,
            _ => Some((LootState::Activated, Some(player_guid), true)),
        };
        let requested_loot_state_outcome = canonical_loot_state_request.and_then(
            |(loot_state, unit_guid, shared_loot_is_changed_like_cpp)| {
                self.owner.transitions_like_cpp().set_canonical_gameobject_loot_state_like_cpp(
                    guid,
                    loot_state,
                    unit_guid,
                    represented_chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
            },
        );
        let canonical_applied_loot_state = if guarded_global_transition.is_some() {
            Some((LootState::JustDeactivated, None))
        } else if let Some((_, state, _)) = canonical_fishing_hole_release.as_ref() {
            Some((*state, None))
        } else {
            canonical_loot_state_request.map(|(state, unit_guid, _)| (state, unit_guid))
        };
        let canonical_loot_state_updated = guarded_global_transition
            .as_ref()
            .or_else(|| {
                canonical_fishing_hole_release
                    .as_ref()
                    .map(|(_, _, outcome)| outcome)
            })
            .or(requested_loot_state_outcome.as_ref())
            .is_some_and(|outcome| {
                outcome.status == wow_map::map::GameObjectSetLootStateStatusLikeCpp::Updated
            });

        let state = self
            .world_entities
            .ensure_represented_gameobject_use_state_like_cpp(guid);
        if canonical_loot_state_updated {
            if let Some((loot_state, unit_guid)) = canonical_applied_loot_state {
                state.loot_state = Some(loot_state);
                state.loot_state_unit_guid = unit_guid.unwrap_or(ObjectGuid::EMPTY);
                if loot_state == LootState::Activated
                    && go_type == Some(GAMEOBJECT_TYPE_CHEST)
                    && state.chest_consumable == Some(false)
                    && state.chest_restock_until.is_none()
                    && state
                        .chest_restock_time_secs
                        .is_some_and(|restock_time| restock_time != 0)
                {
                    let restock_secs = state.chest_restock_time_secs.unwrap_or_default();
                    state.chest_restock_until =
                        Some(Instant::now() + Duration::from_secs(u64::from(restock_secs)));
                }
            }
        } else {
            match go_type {
                Some(GAMEOBJECT_TYPE_FISHING_NODE) => {
                    state.loot_state = Some(LootState::JustDeactivated);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                Some(GAMEOBJECT_TYPE_FISHING_HOLE) => {
                    state.personal_loot_uses = state.personal_loot_uses.saturating_add(1);
                    state.loot_state = if state
                        .fishing_hole_max_opens
                        .is_some_and(|max_opens| state.personal_loot_uses >= max_opens)
                    {
                        Some(LootState::JustDeactivated)
                    } else {
                        Some(LootState::Ready)
                    };
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                Some(GAMEOBJECT_TYPE_GATHERING_NODE) if selected_pool_looted => {}
                Some(GAMEOBJECT_TYPE_CHEST)
                    if selected_pool_looted
                        && whole_object_fully_looted
                        && state.chest_consumable == Some(false)
                        && state
                            .chest_personal_loot_id
                            .is_none_or(|loot_id| loot_id == 0)
                        && state
                            .chest_restock_time_secs
                            .is_some_and(|restock_time| restock_time != 0) =>
                {
                    let restock_secs = state.chest_restock_time_secs.unwrap_or_default();
                    state.loot_state = Some(LootState::NotReady);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                    state.chest_restock_until =
                        Some(Instant::now() + Duration::from_secs(u64::from(restock_secs)));
                }
                _ if selected_pool_looted && whole_object_fully_looted => {
                    state.loot_state = Some(LootState::JustDeactivated);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                _ if selected_pool_looted => {}
                _ => {
                    state.loot_state = Some(LootState::Activated);
                    state.loot_state_unit_guid = player_guid;
                    if go_type == Some(GAMEOBJECT_TYPE_CHEST)
                        && state.chest_consumable == Some(false)
                        && state.chest_restock_until.is_none()
                        && state
                            .chest_restock_time_secs
                            .is_some_and(|restock_time| restock_time != 0)
                    {
                        let restock_secs = state.chest_restock_time_secs.unwrap_or_default();
                        state.chest_restock_until =
                            Some(Instant::now() + Duration::from_secs(u64::from(restock_secs)));
                    }
                }
            }
        }
        if canonical_loot_state_updated && go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE) {
            state.personal_loot_uses = canonical_fishing_hole_use_count_after_release
                .unwrap_or(represented_personal_loot_uses_after_release);
        }
        if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) && selected_pool_looted {
            state.go_state = Some(GoState::Active);
        }
        if go_type == Some(GAMEOBJECT_TYPE_CHEST)
            && selected_pool_looted
            && state.chest_consumable == Some(false)
            && state
                .chest_personal_loot_id
                .is_some_and(|loot_id| loot_id != 0)
        {
            let delay_secs = state
                .chest_restock_time_secs
                .filter(|restock_time| *restock_time != 0)
                .unwrap_or(wow_entities::DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS);
            state.per_player_despawn_secs = Some(delay_secs);
            state.per_player_despawn_until =
                Some(Instant::now() + Duration::from_secs(u64::from(delay_secs)));
            state.per_player_state_player_guid = Some(player_guid);
        }
    }

    pub(super) fn hide_represented_gameobject_for_player_after_loot_release_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) {
        let Some(map_id) = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid)
            .and_then(|state| state.per_player_despawn_until.map(|_| state.map_id))
            .flatten()
        else {
            return;
        };
        if !self.owner.retire_client_visible_guid_like_cpp(guid) {
            return;
        }
        self.owner.publication_like_cpp().send_packet(&UpdateObject::out_of_range_objects(vec![guid], map_id));
    }
}
