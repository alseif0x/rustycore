// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot release transitions and close-out operations.

use super::*;

mod owner;

impl WorldSession {
    pub(in crate::handlers::loot) fn apply_represented_gameobject_loot_release_like_cpp(
        &mut self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
        selected_pool_looted: bool,
        mut whole_object_fully_looted: bool,
        authoritative_release: Option<&AuthoritativeLootReleaseLikeCpp>,
    ) {
        let go_type = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.go_type)
            .map(u32::from);
        let represented_chest_restock_time_secs = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.chest_restock_time_secs)
            .unwrap_or_default();
        let represented_personal_loot_uses_after_release = self
            .represented_gameobject_use_states
            .get(&guid)
            .map(|state| state.personal_loot_uses.saturating_add(1))
            .unwrap_or(1);
        // C++ `FishingHole.MaxOpens` is still template evidence from the represented
        // GO value; the use counter source-of-truth is canonical `GameObject::use_times`
        // when the canonical GameObject can be mutated.
        let represented_fishing_hole_max_opens = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.fishing_hole_max_opens);
        let canonical_fishing_hole_release = (go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE))
            .then(|| {
                self.release_canonical_fishing_hole_like_cpp(
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
                    self.set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation_like_cpp(
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
                    self.set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
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
                self.set_canonical_gameobject_loot_state_like_cpp(
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
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
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

    pub(in crate::handlers::loot) fn hide_represented_gameobject_for_player_after_loot_release_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) {
        let Some(map_id) = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.per_player_despawn_until.map(|_| state.map_id))
            .flatten()
        else {
            return;
        };
        if !self.client_visible_guids_like_cpp.remove(&guid) {
            return;
        }
        self.send_packet(&UpdateObject::out_of_range_objects(vec![guid], map_id));
    }

    pub(in crate::handlers::loot) fn send_gathering_node_loot_release_dynamic_flags_update_like_cpp(
        &self,
        guid: ObjectGuid,
    ) {
        if !self.client_visible_guids_like_cpp.contains(&guid) {
            return;
        }
        let Some(access) = self.canonical_gameobject_access_like_cpp(guid) else {
            return;
        };
        let Some(state) = self.represented_gameobject_use_states.get(&guid) else {
            return;
        };
        if state.go_type.map(u32::from) != Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
            return;
        }
        let dynamic_flags =
            self.represented_gameobject_dynamic_flags_for_player_like_cpp(access.entry, state);
        let packet_update = wow_packet::packets::update::GameObjectDataValuesUpdate {
            changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
            object_data: Some(wow_packet::packets::update::ObjectDataValuesUpdate {
                changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
                object_data_mask: 0x05,
                entry_id: 0,
                dynamic_flags,
                scale: 0.0,
            }),
            game_object_data_mask: 0,
            state_world_effect_ids: Vec::new(),
            enable_doodad_sets: Vec::new(),
            enable_doodad_sets_update_mask: None,
            world_effects: Vec::new(),
            world_effects_update_mask: None,
            display_id: 0,
            spell_visual_id: 0,
            state_spell_visual_id: 0,
            spawn_tracking_state_anim_id: 0,
            spawn_tracking_state_anim_kit_id: 0,
            created_by: ObjectGuid::EMPTY,
            guild_guid: ObjectGuid::EMPTY,
            flags: 0,
            parent_rotation: [0.0; 4],
            faction_template: 0,
            level: 0,
            state: 0,
            type_id: 0,
            percent_health: 0,
            art_kit: 0,
            custom_param: 0,
        };
        let update = UpdateObject::game_object_values_update(
            guid,
            self.player_map_id_like_cpp(),
            packet_update,
        );
        self.send_packet(&update);
    }

}
