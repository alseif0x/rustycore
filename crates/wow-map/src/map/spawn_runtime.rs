// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spawn-aware map operations and update planning.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn map_object_count(&self) -> usize {
        self.entity_world.len()
    }

    pub fn objects_to_switch_count_like_cpp(&self) -> usize {
        self.objects_to_switch.len()
    }

    pub fn pending_switch_like_cpp(&self, guid: ObjectGuid) -> Option<bool> {
        self.objects_to_switch.get(&guid).copied()
    }

    #[cfg(test)]
    pub(in crate::map) fn enqueue_object_to_switch_for_test(&mut self, guid: ObjectGuid, on: bool) {
        self.objects_to_switch.insert(guid, on);
    }

    pub fn creature_group_holder_member_count_like_cpp(&self, leader_spawn_id: SpawnId) -> usize {
        self.creature_group_holder_like_cpp
            .get(&leader_spawn_id)
            .map_or(0, HashSet::len)
    }

    /// Bounded map-owned consumer for C++ `GameEventMgr::ChangeEquipOrModel` live creature loop.
    ///
    /// Mirrors the `GetCreatureBySpawnIdStore().equal_range(spawn_id)` direction over the
    /// map-owned creature by-spawn index. This only mutates canonical `MapObjectRecord::Creature`
    /// equipment/display fields; it does not implement full `Creature::LoadEquipment`, DB2
    /// `GetCreatureModelInfo`, values/session fanout, scripts, AI, or ObjectAccessor side effects.
    pub fn change_game_event_equip_or_model_by_spawn_id_like_cpp(
        &mut self,
        spawn_id: SpawnId,
        equipment_id: u8,
        model_id: u32,
        model_info_available: bool,
    ) -> GameEventChangeEquipOrModelLiveOutcomeLikeCpp {
        let guids = self.creature_spawn_id_store_guids_like_cpp(spawn_id);
        let mut outcome = GameEventChangeEquipOrModelLiveOutcomeLikeCpp {
            spawn_id,
            indexed_guids: guids.len(),
            ..GameEventChangeEquipOrModelLiveOutcomeLikeCpp::default()
        };

        for guid in guids {
            let Some(record) = self.entity_world.get_mut(&guid) else {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            };
            let Some(creature) = record.creature_mut() else {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            };
            if creature.spawn_id() != spawn_id {
                outcome.stale_index_or_wrong_kind += 1;
                continue;
            }

            outcome.live_creatures_mutated += 1;
            creature.set_equipment_id_like_cpp(equipment_id);
            outcome.equipment_changed += 1;

            if model_id > 0 && creature.unit().data().display_id as u32 != model_id {
                if model_info_available {
                    creature.set_display_id(model_id, true, None);
                    outcome.display_changed += 1;
                } else {
                    outcome.model_validation_unavailable += 1;
                }
            }
        }

        outcome
    }

    pub(in crate::map) fn index_map_object_record_by_spawn_id_like_cpp(
        &mut self,
        record: &MapObjectRecord,
    ) {
        if let Some(creature) = record.creature() {
            let spawn_id = creature.spawn_id();
            if spawn_id != 0 {
                self.creatures_by_spawn_id
                    .entry(spawn_id)
                    .or_default()
                    .insert(creature.guid());
            }
            return;
        }

        if let Some(gameobject) = record.game_object() {
            let spawn_id = gameobject.spawn_id();
            if spawn_id != 0 {
                self.gameobjects_by_spawn_id
                    .entry(spawn_id)
                    .or_default()
                    .insert(gameobject.world().guid());
            }
            return;
        }

        if let Some(area_trigger) = record.area_trigger() {
            let spawn_id = area_trigger.spawn_id();
            if spawn_id != 0 {
                self.area_triggers_by_spawn_id
                    .entry(spawn_id)
                    .or_default()
                    .insert(area_trigger.world().guid());
            }
        }
    }

    pub(in crate::map) fn unindex_map_object_record_by_spawn_id_like_cpp(
        &mut self,
        record: &MapObjectRecord,
    ) {
        if let Some(creature) = record.creature() {
            remove_spawn_id_index_entry_like_cpp(
                &mut self.creatures_by_spawn_id,
                creature.spawn_id(),
                creature.guid(),
            );
            return;
        }

        if let Some(gameobject) = record.game_object() {
            remove_spawn_id_index_entry_like_cpp(
                &mut self.gameobjects_by_spawn_id,
                gameobject.spawn_id(),
                gameobject.world().guid(),
            );
            return;
        }

        if let Some(area_trigger) = record.area_trigger() {
            remove_spawn_id_index_entry_like_cpp(
                &mut self.area_triggers_by_spawn_id,
                area_trigger.spawn_id(),
                area_trigger.world().guid(),
            );
        }
    }

    pub(in crate::map) fn apply_creature_search_formation_like_cpp(
        &mut self,
        current_guid: ObjectGuid,
        outcome: CreatureSearchFormationOutcomeLikeCpp,
    ) {
        if !outcome.add_to_group_requested {
            return;
        }

        let Some(leader_spawn_id) = outcome.leader_spawn_id else {
            return;
        };

        let stale_member_guids = self.creature_spawn_id_store_guids_like_cpp(outcome.spawn_id);
        let group = self
            .creature_group_holder_like_cpp
            .entry(leader_spawn_id)
            .or_default();
        for stale_guid in stale_member_guids {
            if stale_guid != current_guid {
                group.remove(&stale_guid);
            }
        }
        group.insert(current_guid);
    }

    pub fn active_non_players_count_like_cpp(&self) -> usize {
        self.active_non_players_like_cpp.len()
    }

    pub fn is_active_non_player_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.active_non_players_like_cpp.contains(&guid)
    }

    pub(in crate::map) fn represented_active_non_player_sources_like_cpp(&self) -> Vec<ObjectGuid> {
        let mut guids: Vec<_> = self
            .active_non_players_like_cpp
            .iter()
            .copied()
            .filter(|guid| self.object_is_in_world(*guid))
            .collect();
        sort_dedup(&mut guids);
        guids
    }

    /// Bounded map-owned body for C++ `Spell::EffectSummonObjectWild`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:2937-2971`: launch-only spell effect resolves the
    ///   destination before this seam, creates a ready GameObject from
    ///   `effectInfo->MiscValue`, inherits phase from `m_caster`, sets respawn
    ///   seconds from positive duration, sets `SpellId`, executes the summon log,
    ///   and calls `Map::AddToMap` without owner linkage.
    /// - `SpellEffects.cpp:2973-2986`: flag-drop battleground state and linked
    ///   trap phase/respawn/spell/log are runtime side effects after AddToMap.
    ///
    /// Scope: the caller supplies an already-resolved template, position,
    /// duration and spell id. This helper does not load DB/templates, resolve
    /// spell targets/GetClosePoint, inherit real phase masks, dispatch scripts,
    /// send packets, update battleground state, or create/resolve linked traps.
    pub fn spell_effect_summon_object_wild_like_cpp(
        &mut self,
        caster_guid: ObjectGuid,
        spell_id: u32,
        template: GameObjectTemplateLifecycleRecord,
        position: Position,
        duration_ms: i32,
    ) -> SpellEffectSummonObjectWildOutcomeLikeCpp {
        let template_entry = template.entry;
        let Some(caster_record) = self.map_object_record(caster_guid) else {
            return SpellEffectSummonObjectWildOutcomeLikeCpp {
                caster_guid,
                spell_id,
                template_entry,
                status: SpellEffectSummonObjectWildStatusLikeCpp::MissingCaster,
                guid: None,
                low_guid: None,
                create_error: None,
                add_to_map: None,
                respawn_time_secs: None,
                phase_inherit_represented: false,
                execute_log_represented: false,
                owner_linked: false,
                flagdrop_type: false,
                flagdrop_player_branch_reached: false,
                flagdrop_battleground_update_represented: false,
                linked_trap_guid: None,
                linked_trap_side_effect_represented: false,
            };
        };
        let caster_is_player = caster_record.kind() == AccessorObjectKind::Player;
        let respawn_time_secs = if duration_ms > 0 {
            duration_ms / 1_000
        } else {
            0
        };
        let flagdrop_type = template.go_type == GAMEOBJECT_TYPE_FLAGDROP;

        let low_guid = match self.generate_low_guid_like_cpp(HighGuid::GameObject) {
            Ok(low) => low,
            Err(_) => {
                return SpellEffectSummonObjectWildOutcomeLikeCpp {
                    caster_guid,
                    spell_id,
                    template_entry,
                    status: SpellEffectSummonObjectWildStatusLikeCpp::LowGuidUnavailable,
                    guid: None,
                    low_guid: None,
                    create_error: None,
                    add_to_map: None,
                    respawn_time_secs: Some(respawn_time_secs),
                    phase_inherit_represented: false,
                    execute_log_represented: false,
                    owner_linked: false,
                    flagdrop_type,
                    flagdrop_player_branch_reached: false,
                    flagdrop_battleground_update_represented: false,
                    linked_trap_guid: None,
                    linked_trap_side_effect_represented: false,
                };
            }
        };
        let guid = ObjectGuid::create_world_object(
            HighGuid::GameObject,
            0,
            1,
            self.map_id as u16,
            self.instance_id,
            template_entry,
            low_guid,
        );
        let record = GameObjectCreateLifecycleRecord {
            guid,
            map_id: self.map_id,
            instance_id: self.instance_id,
            position,
            rotation: gameobject_local_rotation_from_orientation_like_cpp(position.orientation),
            anim_progress: u8::MAX,
            go_state: GoState::Ready,
            art_kit: 0,
            dynamic: true,
            spawn_id: 0,
            template,
        };

        let mut game_object = match GameObject::try_create_from_lifecycle(record) {
            Ok(game_object) => game_object,
            Err(error) => {
                return SpellEffectSummonObjectWildOutcomeLikeCpp {
                    caster_guid,
                    spell_id,
                    template_entry,
                    status: SpellEffectSummonObjectWildStatusLikeCpp::CreateFailed,
                    guid: Some(guid),
                    low_guid: Some(low_guid),
                    create_error: Some(error),
                    add_to_map: None,
                    respawn_time_secs: Some(respawn_time_secs),
                    phase_inherit_represented: false,
                    execute_log_represented: false,
                    owner_linked: false,
                    flagdrop_type,
                    flagdrop_player_branch_reached: false,
                    flagdrop_battleground_update_represented: false,
                    linked_trap_guid: None,
                    linked_trap_side_effect_represented: false,
                };
            }
        };
        game_object.set_respawn_time(i64::from(respawn_time_secs));
        game_object.set_spell_id(spell_id);
        let linked_trap_guid = game_object.linked_trap_guid_like_cpp();

        let add_to_map = self
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_game_object(game_object)
                    .expect("GameObject lifecycle create must produce a typed GameObject record"),
            )
            .ok();
        let execute_log_represented = add_to_map.is_some();

        SpellEffectSummonObjectWildOutcomeLikeCpp {
            caster_guid,
            spell_id,
            template_entry,
            status: if execute_log_represented {
                SpellEffectSummonObjectWildStatusLikeCpp::CreatedAddedToMap
            } else {
                SpellEffectSummonObjectWildStatusLikeCpp::AddToMapFailed
            },
            guid: Some(guid),
            low_guid: Some(low_guid),
            create_error: None,
            add_to_map,
            respawn_time_secs: Some(respawn_time_secs),
            phase_inherit_represented: false,
            execute_log_represented,
            owner_linked: false,
            flagdrop_type,
            flagdrop_player_branch_reached: flagdrop_type && caster_is_player,
            flagdrop_battleground_update_represented: false,
            linked_trap_guid: (!linked_trap_guid.is_empty()).then_some(linked_trap_guid),
            linked_trap_side_effect_represented: false,
        }
    }

    pub fn map_update_visit_plan_like_cpp(
        &self,
        sources: impl IntoIterator<Item = MapUpdatePlayerSources>,
        active_non_player_guids: impl IntoIterator<Item = ObjectGuid>,
        transport_guids: impl IntoIterator<Item = ObjectGuid>,
        diff_ms: u32,
    ) -> MapUpdateVisitPlan {
        let mut session_update_players = Vec::new();
        let mut player_update_guids = Vec::new();
        let mut nearby_visit_centers = Vec::new();
        let mut saw_player_source = false;

        for source in sources {
            saw_player_source = true;
            if !self.object_is_in_world(source.player_guid) {
                continue;
            }

            session_update_players.push(source.player_guid);
            player_update_guids.push(source.player_guid);
            nearby_visit_centers.push(source.player_guid);

            if let Some(viewpoint) = source.viewpoint_guid
                && self.object_is_in_world(viewpoint)
            {
                nearby_visit_centers.push(viewpoint);
            }

            push_in_world_guids(
                self,
                &mut nearby_visit_centers,
                source.far_combat_unit_guids,
            );
            push_in_world_guids(
                self,
                &mut nearby_visit_centers,
                source.far_aura_caster_guids,
            );
            push_in_world_guids(self, &mut nearby_visit_centers, source.far_summon_guids);
        }

        let mut saw_active_non_player_source = false;
        for guid in active_non_player_guids {
            saw_active_non_player_source = true;
            if self.object_is_in_world(guid) {
                nearby_visit_centers.push(guid);
            }
        }

        let mut transport_update_guids = Vec::new();
        for guid in transport_guids {
            if self.map_object(guid).is_some() {
                transport_update_guids.push(guid);
            }
        }

        sort_dedup(&mut session_update_players);
        sort_dedup(&mut player_update_guids);
        sort_dedup(&mut nearby_visit_centers);
        sort_dedup(&mut transport_update_guids);
        let process_relocation_notifies = saw_player_source || saw_active_non_player_source;

        MapUpdateVisitPlan {
            diff_ms,
            session_update_players,
            player_update_guids,
            nearby_visit_centers,
            transport_update_guids,
            process_relocation_notifies,
        }
    }
}
