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

    pub fn apply_represented_gameobject_loot_release_like_cpp(
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

    /// C++ `GameObject::IsWithinDistInMap` gate for `HandleAutostoreLootItemOpcode`:
    /// the represented GameObject position, display box and lock range, using
    /// the same canonical/represented resolution as the World loot facade.
    pub(super) fn represented_gameobject_can_autostore_loot_item_like_cpp(
        &self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(state) = self.represented_gameobject_loot_state_like_cpp(guid) else {
            return false;
        };

        // C++ ref: LootHandler.cpp HandleAutostoreLootItemOpcode skips distance
        // for owned GameObjects and GAMEOBJECT_TYPE_FISHINGHOLE. DB spawns do
        // not carry CreatedBy; apply the owner exception only when runtime GO
        // state explicitly recorded GetOwnerGUID.
        if state.owner_guid == Some(player_guid)
            || state.go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE as u8)
        {
            return true;
        }

        match (self.hub_ref_like_cpp().player_position_like_cpp(), state.position) {
            (Some(player), Some(position)) => {
                let radius = wow_world_entities::represented_gameobject_interaction_distance_like_cpp(
                    state.go_type,
                    state.interact_radius_override,
                );
                let radius = self
                    .represented_gameobject_spell_lock_range_like_cpp(state.lock_id)
                    .unwrap_or(radius);
                if let Some(display_info) = self
                    .catalogs_like_cpp()
                    .gameobject_display_info_store()
                    .and_then(|store| {
                        state
                            .display_id
                            .and_then(|display_id| store.get(display_id))
                    })
                {
                    wow_world_entities::represented_gameobject_display_box_contains_like_cpp(
                        position,
                        player,
                        display_info,
                        state.scale,
                        state.rotation,
                        radius,
                    )
                } else {
                    player.is_within_dist(&position, radius)
                }
            }
            _ => true,
        }
    }

    /// C++ `GameObject::GetLockRange`-style spell-lock gate: the largest range
    /// among the spells the owner can use to open the lock.
    fn represented_gameobject_spell_lock_range_like_cpp(
        &self,
        lock_id: Option<u32>,
    ) -> Option<f32> {
        let lock_id = lock_id?;
        let catalogs = self.catalogs_like_cpp();
        let lock = catalogs.lock_store()?.get(lock_id)?;
        for i in 0..wow_data::lock::MAX_LOCK_CASE {
            let lock_type = lock.lock_type[i];
            if lock_type == 0 {
                continue;
            }

            if lock_type == LOCK_KEY_SPELL_LIKE_CPP {
                if let Some(range) = catalogs.represented_spell_max_range_like_cpp(lock.index[i]) {
                    return Some(range);
                }
            }

            if lock_type != LOCK_KEY_SKILL_LIKE_CPP {
                break;
            }

            for spell_id in self.represented_known_spells_like_cpp() {
                let Some(spell) = catalogs.spell_store().and_then(|store| store.get(spell_id))
                else {
                    continue;
                };
                let can_open_lock = spell.effects().iter().any(|effect| {
                    effect.effect == SPELL_EFFECT_OPEN_LOCK_LIKE_CPP
                        && effect.effect_misc_value_1 == lock.index[i]
                        && effect.effect_base_points >= i32::from(lock.skill[i])
                });
                if can_open_lock {
                    if let Some(range) = catalogs.represented_spell_max_range_like_cpp(spell_id) {
                        return Some(range);
                    }
                }
            }
        }

        None
    }

    /// Canonical known spells, with the same absent-owner fixture fallback the
    /// World spell-state adapter applies at its original read point.
    fn represented_known_spells_like_cpp(&self) -> Vec<i32> {
        let core = self.core_like_cpp();
        let canonical = core.with_owned_player_like_cpp(|player| {
            player.spell_runtime_like_cpp().known_spells_like_cpp().to_vec()
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && core.player_handle_like_cpp.is_none() {
            let runtime = wow_world_spell::canonical_player_spell_runtime_like_cpp(
                self.spell_state.represented_spell_runtime_fixture_like_cpp(),
            );
            return runtime.known_spells_like_cpp().to_vec();
        }
        canonical.unwrap_or_default()
    }

    /// C++ `Loot::GetOwnerGUID`-backed represented GameObject state used by the
    /// release gates. Position and owner prefer the canonical map object and
    /// fall back to the represented runtime state, matching the World facade.
    fn represented_gameobject_loot_state_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectLootStateLikeCpp> {
        if !guid.is_game_object() {
            return None;
        }

        let hub = self.hub_ref_like_cpp();
        let canonical_position = self.loot.canonical_map_object_position_for_loot_like_cpp(
            hub,
            guid,
            &[
                wow_entities::AccessorObjectKind::GameObject,
                wow_entities::AccessorObjectKind::Transport,
            ],
        );
        let canonical_owner = self
            .loot
            .canonical_gameobject_owner_for_loot_like_cpp(hub, guid);
        let represented_state = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid);
        if canonical_position.is_none()
            && represented_state.and_then(|state| state.position).is_none()
            && !hub.core.client_visible_guids_like_cpp.contains(&guid)
        {
            return None;
        }

        Some(RepresentedGameObjectLootStateLikeCpp {
            position: canonical_position
                .or_else(|| represented_state.and_then(|state| state.position)),
            display_id: represented_state.and_then(|state| state.display_id),
            scale: represented_state.map(|state| state.scale).unwrap_or(1.0),
            rotation: represented_state
                .map(|state| state.rotation)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
            go_type: represented_state.and_then(|state| state.go_type),
            interact_radius_override: represented_state
                .and_then(|state| state.interact_radius_override),
            lock_id: represented_state.and_then(|state| state.lock_id),
            owner_guid: canonical_owner
                .or_else(|| represented_state.and_then(|state| state.owner_guid)),
        })
    }

    /// C++ `GameObject::OnLootRelease` GATHERING_NODE branch: after
    /// `SetGoStateFor(GO_STATE_ACTIVE, looter)` the object publishes one
    /// viewer-projected `DynamicFlags` values update.
    pub(super) fn send_gathering_node_loot_release_dynamic_flags_update_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) {
        let core = self.core_like_cpp();
        if !core.client_visible_guids_like_cpp.contains(&guid) {
            return;
        }
        let Some(access) = core.canonical_gameobject_access_like_cpp(guid) else {
            return;
        };
        let Some(state) = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid)
        else {
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
            core.player_map_id_like_cpp(),
            packet_update,
        );
        self.send_packet(&update);
    }

    /// C++ `ViewerDependentValue<UF::ObjectData::DynamicFlagsTag>::GetValue`
    /// for the releasing player, assembled from the release context's
    /// split borrows exactly as the World facade assembles it.
    fn represented_gameobject_dynamic_flags_for_player_like_cpp(
        &self,
        entry: u32,
        state: &wow_world_entities::RepresentedGameObjectUseState,
    ) -> u32 {
        let core = self.core_like_cpp();
        let conditions = self.represented_condition_projection_like_cpp(core);
        let eligibility = crate::QuestEligibilityCx::new(
            core.quest_eligibility_access_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_race,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_class,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.identity.player_level,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self
                    .fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_records_like_cpp,
                #[cfg(any(test, feature = "test-fixtures"))]
                &self.fixtures.progression.reputation_state_like_cpp,
            ),
            self.quest_state,
            self.catalogs_like_cpp(),
            &conditions,
            cfg!(test),
        );
        let owner = core.quest_objective_access_like_cpp();
        let inventory_access = core.owned_inventory_access_like_cpp();
        crate::quest::represented_gameobject_dynamic_flags_for_player_like_cpp(
            &owner,
            self.catalogs_like_cpp(),
            self.quest_state,
            &*self.inventory,
            &inventory_access,
            &eligibility,
            self.player_guid(),
            self.hub_ref_like_cpp().player_is_game_master_like_cpp() == Some(true),
            entry,
            state,
            cfg!(test),
        )
    }

    /// Inert condition projection for the release's own readonly pass. Built on
    /// demand so the context can keep its mutable inventory borrow.
    fn represented_condition_projection_like_cpp<'b>(
        &'b self,
        core: &'b SessionCore,
    ) -> crate::PlayerConditionProjectionCxLikeCpp<'b> {
        #[cfg(any(test, feature = "test-fixtures"))]
        let player = core.player_condition_access_with_selected_fixture_refs_like_cpp(
            wow_world_core::session::PlayerConditionFixtureRefsLikeCpp::new(
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
                &self.fixtures.identity.player_level,
                &self.fixtures.identity.player_gender,
                &self
                    .fixtures
                    .progression
                    .represented_primary_specialization_id_like_cpp,
                &self.fixtures.movement.player_position,
                &self.fixtures.identity.player_zone_id_like_cpp,
                &self.fixtures.identity.player_area_id_like_cpp,
                &self
                    .fixtures
                    .identity
                    .player_zone_area_authority_complete_like_cpp,
                &self.fixtures.combat.player_pvp_hostile_like_cpp,
                &self.fixtures.combat.player_pvp_end_timer_like_cpp,
                &self.fixtures.combat.player_contested_pvp_timer_like_cpp,
                &self.fixtures.identity.represented_is_outdoors_like_cpp,
                &self.fixtures.combat.player_health_like_cpp,
                &self.fixtures.combat.player_max_health_like_cpp,
                &self.fixtures.combat.player_alive_like_cpp,
                &self.fixtures.vehicles.taxi_destinations_like_cpp,
                &self.fixtures.vehicles.taxi_flight_state_like_cpp,
                &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
                &self.fixtures.vehicles.taxi_mounted_like_cpp,
                &self.fixtures.auras.visible_auras,
                &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                &self
                    .fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                &self
                    .fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_records_like_cpp,
            ),
        );
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let player = core.player_condition_access_with_selected_fixture_refs_like_cpp();

        crate::PlayerConditionProjectionCxLikeCpp::new(
            player,
            &*self.inventory,
            self.social,
            self.catalogs_like_cpp()
                .chr_specialization_store()
                .map(std::sync::Arc::as_ref),
            #[cfg(any(test, feature = "test-fixtures"))]
            self.spell_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.quest_state,
            #[cfg(any(test, feature = "test-fixtures"))]
            self.instances,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.battleground,
            self.catalogs_like_cpp()
                .inventory_valuation_catalog_view_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.reputation_state_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .battleground
                .represented_battleground_status_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_complete_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.combat.in_combat,
            #[cfg(any(test, feature = "test-fixtures"))]
            cfg!(test),
        )
    }
}

/// Canonical/represented GameObject facts read by the release gates.
struct RepresentedGameObjectLootStateLikeCpp {
    position: Option<wow_core::Position>,
    display_id: Option<u32>,
    scale: f32,
    rotation: [f32; 4],
    go_type: Option<u8>,
    interact_radius_override: Option<u32>,
    lock_id: Option<u32>,
    owner_guid: Option<ObjectGuid>,
}
