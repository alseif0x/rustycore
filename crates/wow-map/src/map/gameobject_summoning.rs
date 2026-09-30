// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject summon and slot-replacement lifecycle.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Bounded map-owned representation of C++ `WorldObject::SummonGameObject`.
    ///
    /// C++ anchors:
    /// - `Object.cpp:2067-2090`: `WorldObject::SummonGameObject(entry, pos,
    ///   rot, respawnTime, summonType)` requires an in-world summoner, creates
    ///   a ready dynamic GameObject from the already-resolved template,
    ///   inherits phase, sets respawn time, either calls `ToUnit()->AddGameObject`
    ///   for Player / Unit + `GO_SUMMON_TIMED_OR_CORPSE_DESPAWN`, or marks the
    ///   object not spawned by default, then calls `Map::AddToMap`.
    /// - `GameObject.cpp:1187-1200`: `GameObject::CreateGameObject` delegates
    ///   to `GameObject::Create` and returns null on missing template/create
    ///   failure.
    ///
    /// Scope: the caller supplies an already-resolved template, position and
    /// respawn seconds. This helper does not load DB/templates, compute
    /// `GetClosePoint`, inherit real phase masks, dispatch scripts, send
    /// packets, create linked traps, or emit spell execute logs.
    pub fn world_object_summon_gameobject_like_cpp(
        &mut self,
        summoner_guid: ObjectGuid,
        template: GameObjectTemplateLifecycleRecord,
        position: Position,
        respawn_time_secs: i64,
        summon_type: GameObjectSummonTypeLikeCpp,
    ) -> WorldObjectSummonGameObjectOutcomeLikeCpp {
        let template_entry = template.entry;
        let Some(summoner_record) = self.map_object_record(summoner_guid) else {
            return WorldObjectSummonGameObjectOutcomeLikeCpp {
                summoner_guid,
                template_entry,
                summon_type,
                status: WorldObjectSummonGameObjectStatusLikeCpp::MissingSummoner,
                guid: None,
                low_guid: None,
                create_error: None,
                add_to_map: None,
                add_owner: None,
                respawn_time_secs,
                phase_inherit_represented: false,
                spawned_by_default_forced_false: false,
            };
        };
        if !summoner_record.object().object().is_in_world() {
            return WorldObjectSummonGameObjectOutcomeLikeCpp {
                summoner_guid,
                template_entry,
                summon_type,
                status: WorldObjectSummonGameObjectStatusLikeCpp::SummonerNotInWorld,
                guid: None,
                low_guid: None,
                create_error: None,
                add_to_map: None,
                add_owner: None,
                respawn_time_secs,
                phase_inherit_represented: false,
                spawned_by_default_forced_false: false,
            };
        }
        let summoner_is_player = summoner_record.kind() == AccessorObjectKind::Player;
        let summoner_is_unit_like =
            summoner_record.is_unit_owner();
        let should_add_to_owner = summoner_is_player
            || (summoner_is_unit_like
                && summon_type == GameObjectSummonTypeLikeCpp::TimedOrCorpseDespawn);

        let low_guid = match self.generate_low_guid_like_cpp(HighGuid::GameObject) {
            Ok(low) => low,
            Err(_) => {
                return WorldObjectSummonGameObjectOutcomeLikeCpp {
                    summoner_guid,
                    template_entry,
                    summon_type,
                    status: WorldObjectSummonGameObjectStatusLikeCpp::LowGuidUnavailable,
                    guid: None,
                    low_guid: None,
                    create_error: None,
                    add_to_map: None,
                    add_owner: None,
                    respawn_time_secs,
                    phase_inherit_represented: false,
                    spawned_by_default_forced_false: false,
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
                return WorldObjectSummonGameObjectOutcomeLikeCpp {
                    summoner_guid,
                    template_entry,
                    summon_type,
                    status: WorldObjectSummonGameObjectStatusLikeCpp::CreateFailed,
                    guid: Some(guid),
                    low_guid: Some(low_guid),
                    create_error: Some(error),
                    add_to_map: None,
                    add_owner: None,
                    respawn_time_secs,
                    phase_inherit_represented: false,
                    spawned_by_default_forced_false: false,
                };
            }
        };
        game_object.set_respawn_time(respawn_time_secs);

        let mut add_owner = None;
        let mut spawned_by_default_forced_false = false;
        if should_add_to_owner {
            game_object.set_owner_guid_like_cpp(summoner_guid);
            let mut registered_owned_gameobject = false;
            let mut creature_ai_callback_represented = false;
            if let Some(mut record) = self.entity_world.get_mut(&summoner_guid) {
                if let Some(owner) = record.reborrow().unit_mut() {
                    owner
                        .subsystems_mut()
                        .control
                        .register_owned_gameobject_like_cpp(guid);
                    registered_owned_gameobject = true;
                }
                creature_ai_callback_represented = match record.kind() {
                    AccessorObjectKind::Creature => record
                        .creature_mut()
                        .map(|creature| {
                            creature
                                .unit_mut()
                                .subsystems_mut()
                                .ai
                                .just_summoned_gameobject_like_cpp()
                        })
                        .unwrap_or(false),
                    AccessorObjectKind::Pet => record
                        .pet_mut()
                        .map(|pet| {
                            pet.creature_mut()
                                .unit_mut()
                                .subsystems_mut()
                                .ai
                                .just_summoned_gameobject_like_cpp()
                        })
                        .unwrap_or(false),
                    _ => false,
                };
            }
            add_owner = Some(GameObjectAddToOwnerOutcomeLikeCpp {
                guid,
                owner_guid: summoner_guid,
                owner_found_as_unit_like: summoner_is_unit_like,
                gameobject_found: true,
                owner_guid_before: ObjectGuid::EMPTY,
                owner_guid_after: summoner_guid,
                gameobject_owner_empty_before: true,
                registered_owned_gameobject,
                owner_guid_set: registered_owned_gameobject,
                cooldown_start_represented: false,
                creature_ai_callback_represented,
            });
        } else {
            game_object.set_spawned_by_default(false);
            spawned_by_default_forced_false = true;
        }

        let add_to_map = self
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_game_object(game_object)
                    .expect("GameObject lifecycle create must produce a typed GameObject record"),
            )
            .ok();
        let status = if add_to_map.is_some() {
            WorldObjectSummonGameObjectStatusLikeCpp::CreatedAddedToMap
        } else {
            WorldObjectSummonGameObjectStatusLikeCpp::AddToMapFailed
        };

        WorldObjectSummonGameObjectOutcomeLikeCpp {
            summoner_guid,
            template_entry,
            summon_type,
            status,
            guid: Some(guid),
            low_guid: Some(low_guid),
            create_error: None,
            add_to_map,
            add_owner,
            respawn_time_secs,
            phase_inherit_represented: false,
            spawned_by_default_forced_false,
        }
    }

    /// Bounded map-owned pre-create cleanup for C++ `Spell::EffectSummonObject`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:3548-3563`: before creating the replacement object,
    ///   clear the existing `m_ObjectSlot[slot]`; if the old GameObject exists,
    ///   null its spell id in the recast case, call `Unit::RemoveGameObject(obj,
    ///   true)`, then clear the slot.
    /// - `Unit.cpp:5213-5251`: pointer-overload removal clears owner/list/slot,
    ///   removes spell auras when `GetSpellId() != 0`, emits the represented AI
    ///   despawn boundary, then `SetRespawnTime(0); Delete()` when `del=true`.
    ///
    /// Scope: this represents only the old-slot cleanup before a new object is
    /// created. It does not create the new GameObject, write the replacement
    /// slot, inherit phase, execute scripts, send packets, or emit cooldown
    /// events.
    pub fn gameobject_prepare_owner_slot_for_summon_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        slot: usize,
        spell_id: u32,
    ) -> GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp {
        let owner_found_as_unit_like = self
            .map_object_record(owner_guid)
            .is_some_and(|record| record.is_unit_owner());
        let slot_guid_before = self
            .map_object_record(owner_guid)
            .and_then(|record| record.unit())
            .and_then(|owner| {
                owner
                    .subsystems()
                    .control
                    .gameobject_slots
                    .get(slot)
                    .copied()
            })
            .unwrap_or(ObjectGuid::EMPTY);

        let mut gameobject_found = false;
        let mut recast_spell_id_cleared = false;
        let mut unit_pointer_owner_match = false;
        let mut remove_from_owner = None;
        let mut respawn_time_cleared = false;
        let mut delete_outcome = None;
        let mut slot_cleared = false;

        if owner_found_as_unit_like && !slot_guid_before.is_empty() {
            gameobject_found = self
                .map_object_record(slot_guid_before)
                .and_then(|record| record.game_object())
                .is_some();

            if gameobject_found {
                unit_pointer_owner_match = self
                    .map_object_record(slot_guid_before)
                    .and_then(|record| record.game_object())
                    .is_some_and(|gameobject| gameobject.owner_guid() == owner_guid);
                if let Some(gameobject) = self
                    .entity_world
                    .get_mut(&slot_guid_before)
                    .and_then(ObjectMut::game_object_mut)
                {
                    if gameobject.spell_id() == spell_id {
                        gameobject.set_spell_id(0);
                        recast_spell_id_cleared = true;
                    }
                }

                if unit_pointer_owner_match {
                    remove_from_owner =
                        self.gameobject_remove_from_owner_like_cpp(slot_guid_before);
                    if let Some(gameobject) = self
                        .entity_world
                        .get_mut(&slot_guid_before)
                        .and_then(ObjectMut::game_object_mut)
                    {
                        gameobject.set_respawn_time(0);
                        respawn_time_cleared = true;
                    }
                    delete_outcome = self.gameobject_delete_like_cpp(slot_guid_before);
                }
            }

            if let Some(owner) = self
                .entity_world
                .get_mut(&owner_guid)
                .and_then(|record| record.unit_mut())
            {
                slot_cleared = owner
                    .subsystems_mut()
                    .control
                    .set_gameobject_slot(slot, ObjectGuid::EMPTY);
            }
        }

        GameObjectPrepareOwnerSlotForSummonOutcomeLikeCpp {
            owner_guid,
            slot,
            spell_id,
            owner_found_as_unit_like,
            slot_guid_before,
            slot_had_guid: !slot_guid_before.is_empty(),
            gameobject_found,
            recast_spell_id_cleared,
            unit_pointer_owner_match,
            remove_from_owner,
            respawn_time_cleared,
            delete_outcome,
            slot_cleared,
            cooldown_event_represented: false,
        }
    }

    /// Bounded map-owned body for C++ `Spell::EffectSummonObject`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:3565-3597`: after old-slot cleanup and destination
    ///   resolution, create a ready GameObject from `effectInfo->MiscValue`,
    ///   inherit phase, copy caster faction/level, set respawn from spell
    ///   duration, set `SpellId`, call `Unit::AddGameObject`, execute the
    ///   summon-object log boundary, add to map, then write `m_ObjectSlot[slot]`.
    /// - `GameObject.cpp:179-229`: `GameObject::Create` binds the object to the
    ///   map/position/rotation/template before `Map::AddToMap`.
    ///
    /// Scope: the caller supplies an already-resolved template, destination and
    /// duration. This helper does not load DB/templates, resolve spell targets,
    /// inherit real phase masks, execute scripts, send packets, or emit cooldown
    /// events.
    pub fn gameobject_summon_object_for_owner_slot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        slot: usize,
        spell_id: u32,
        template: GameObjectTemplateLifecycleRecord,
        position: Position,
        duration_ms: i32,
    ) -> GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
        let template_entry = template.entry;
        let Some(owner) = self
            .map_object_record(owner_guid)
            .and_then(|record| record.unit())
        else {
            return GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
                owner_guid,
                slot,
                spell_id,
                template_entry,
                status: GameObjectSummonObjectForOwnerSlotStatusLikeCpp::MissingOwner,
                guid: None,
                low_guid: None,
                create_error: None,
                add_to_map: None,
                add_owner_slot: None,
                respawn_time_secs: None,
                caster_faction: None,
                caster_level: None,
                phase_inherit_represented: false,
                execute_log_represented: false,
                cooldown_event_represented: false,
            };
        };

        let caster_faction = owner.data().faction_template.max(0) as u32;
        let caster_level = owner.data().level.max(0) as u32;
        let respawn_time_secs = if duration_ms > 0 {
            duration_ms / 1_000
        } else {
            0
        };
        let low_guid = match self.generate_low_guid_like_cpp(HighGuid::GameObject) {
            Ok(low) => low,
            Err(_) => {
                return GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
                    owner_guid,
                    slot,
                    spell_id,
                    template_entry,
                    status: GameObjectSummonObjectForOwnerSlotStatusLikeCpp::LowGuidUnavailable,
                    guid: None,
                    low_guid: None,
                    create_error: None,
                    add_to_map: None,
                    add_owner_slot: None,
                    respawn_time_secs: Some(respawn_time_secs),
                    caster_faction: Some(caster_faction),
                    caster_level: Some(caster_level),
                    phase_inherit_represented: false,
                    execute_log_represented: false,
                    cooldown_event_represented: false,
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
        let rotation = gameobject_local_rotation_from_orientation_like_cpp(position.orientation);
        let record = GameObjectCreateLifecycleRecord {
            guid,
            map_id: self.map_id,
            instance_id: self.instance_id,
            position,
            rotation,
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
                return GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
                    owner_guid,
                    slot,
                    spell_id,
                    template_entry,
                    status: GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreateFailed,
                    guid: Some(guid),
                    low_guid: Some(low_guid),
                    create_error: Some(error),
                    add_to_map: None,
                    add_owner_slot: None,
                    respawn_time_secs: Some(respawn_time_secs),
                    caster_faction: Some(caster_faction),
                    caster_level: Some(caster_level),
                    phase_inherit_represented: false,
                    execute_log_represented: false,
                    cooldown_event_represented: false,
                };
            }
        };
        game_object.set_faction(caster_faction);
        game_object.set_level(caster_level);
        game_object.set_respawn_time(i64::from(respawn_time_secs));
        game_object.set_spell_id(spell_id);
        game_object.set_owner_guid_like_cpp(owner_guid);

        let mut registered_owned_gameobject = false;
        let mut creature_ai_callback_represented = false;
        if let Some(mut record) = self.entity_world.get_mut(&owner_guid) {
            if let Some(owner) = record.reborrow().unit_mut() {
                owner
                    .subsystems_mut()
                    .control
                    .register_owned_gameobject_like_cpp(guid);
                registered_owned_gameobject = true;
            }
            creature_ai_callback_represented = match record.kind() {
                AccessorObjectKind::Creature => record
                    .creature_mut()
                    .map(|creature| {
                        creature
                            .unit_mut()
                            .subsystems_mut()
                            .ai
                            .just_summoned_gameobject_like_cpp()
                    })
                    .unwrap_or(false),
                AccessorObjectKind::Pet => record
                    .pet_mut()
                    .map(|pet| {
                        pet.creature_mut()
                            .unit_mut()
                            .subsystems_mut()
                            .ai
                            .just_summoned_gameobject_like_cpp()
                    })
                    .unwrap_or(false),
                _ => false,
            };
        }
        let add_owner = GameObjectAddToOwnerOutcomeLikeCpp {
            guid,
            owner_guid,
            owner_found_as_unit_like: true,
            gameobject_found: true,
            owner_guid_before: ObjectGuid::EMPTY,
            owner_guid_after: owner_guid,
            gameobject_owner_empty_before: true,
            registered_owned_gameobject,
            owner_guid_set: registered_owned_gameobject,
            cooldown_start_represented: false,
            creature_ai_callback_represented,
        };

        let add_to_map = self
            .add_map_object_record_to_map_like_cpp(
                MapObjectRecord::new_game_object(game_object)
                    .expect("GameObject lifecycle create must produce a typed GameObject record"),
            )
            .ok();
        let add_owner_slot = if add_to_map.is_some() {
            let mut slot_previous_guid = ObjectGuid::EMPTY;
            let mut slot_set = false;
            if let Some(owner) = self
                .entity_world
                .get_mut(&owner_guid)
                .and_then(|record| record.unit_mut())
            {
                if let Some(previous) = owner
                    .subsystems()
                    .control
                    .gameobject_slots
                    .get(slot)
                    .copied()
                {
                    slot_previous_guid = previous;
                }
                slot_set = owner
                    .subsystems_mut()
                    .control
                    .set_gameobject_slot(slot, guid);
            }
            Some(GameObjectAddToOwnerSlotOutcomeLikeCpp {
                add_owner,
                slot,
                slot_previous_guid,
                slot_set,
            })
        } else {
            None
        };
        let execute_log_represented = add_owner_slot.as_ref().is_some_and(|outcome| {
            outcome.add_owner.registered_owned_gameobject && outcome.slot_set
        });

        GameObjectSummonObjectForOwnerSlotOutcomeLikeCpp {
            owner_guid,
            slot,
            spell_id,
            template_entry,
            status: if execute_log_represented {
                GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreatedAddedAndSlotted
            } else {
                GameObjectSummonObjectForOwnerSlotStatusLikeCpp::AddToMapOrOwnerFailed
            },
            guid: Some(guid),
            low_guid: Some(low_guid),
            create_error: None,
            add_to_map,
            add_owner_slot,
            respawn_time_secs: Some(respawn_time_secs),
            caster_faction: Some(caster_faction),
            caster_level: Some(caster_level),
            phase_inherit_represented: false,
            execute_log_represented,
            cooldown_event_represented: false,
        }
    }

    /// Bounded map-owned representation of C++ `GameObject::Delete()`.
    ///
    /// C++ anchors:
    /// - `GameObject.cpp:1740-1764`: `SetLootState(GO_NOT_READY)`,
    ///   `RemoveFromOwner()`, optional capture-point packet, `SendGameObjectDespawn()`,
    ///   GO state reset for non-transports, override flag restore, then PoolMgr or
    ///   `AddObjectToRemoveList()`.
    /// - `Map.cpp:2547-2555`: `AddObjectToRemoveList()` is the physical-removal
    ///   handoff; extraction happens later in `RemoveAllObjectsInRemoveList()`.
    pub(super) fn gameobject_delete_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp> {
        let go_type = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(|record| record.game_object())
            .map(|game_object| game_object.data().type_id as u32)?;

        if let Some(game_object) = self
            .entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::game_object_mut)
        {
            // `GameObject::Delete` queues physical removal without calling
            // `ClearLoot`. Terminally detach only the async authority here:
            // this prevents both Arc-held claims and an async generator from
            // reactivating the deleted lifetime while preserving C++'s
            // interim object fields until remove-list drain.
            game_object.loot_authority_like_cpp().detach_like_cpp();
            game_object.set_loot_state(LootState::NotReady, None);
        }
        let remove_from_owner = self.gameobject_remove_from_owner_like_cpp(guid);
        let capture_point_packet_represented = go_type == GAMEOBJECT_TYPE_CAPTURE_POINT;
        let despawn_packet_represented = true;

        let (go_state_ready, flags_restored) = self
            .entity_world
            .get_mut(&guid)
            .and_then(ObjectMut::game_object_mut)
            .map(|game_object| {
                let go_state_ready = go_type != GAMEOBJECT_TYPE_TRANSPORT;
                if go_state_ready {
                    game_object.set_go_state(GoState::Ready);
                }
                let flags_restored = game_object.restore_represented_baseline_flags_like_cpp();
                (go_state_ready, flags_restored)
            })
            .unwrap_or((false, false));

        let remove_list = self.add_object_to_remove_list_like_cpp(guid);
        Some(GameObjectDeleteOutcomeLikeCpp {
            guid,
            remove_from_owner,
            capture_point_packet_represented,
            despawn_packet_represented,
            go_state_ready,
            flags_restored,
            pool_update_represented: false,
            pool_update_plan: None,
            pool_update_error: None,
            pool_update_summary: None,
            remove_list: Some(remove_list),
        })
    }

    pub(super) fn gameobject_delete_from_update_with_optional_loader_like_cpp<L>(
        &mut self,
        guid: ObjectGuid,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        load_record: Option<&mut L>,
    ) -> Option<GameObjectDeleteOutcomeLikeCpp>
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        match pool_update {
            Some((spawn_store, pool_mgr)) => match load_record {
                Some(loader) => self
                    .gameobject_delete_with_pool_update_loaded_grid_records_like_cpp(
                        guid,
                        spawn_store,
                        pool_mgr,
                        |_, _| 0.0,
                        |_candidates, count| (0..count).collect(),
                        loader,
                    ),
                None => self.gameobject_delete_with_pool_update_like_cpp(
                    guid,
                    spawn_store,
                    pool_mgr,
                    |_, _| 0.0,
                    |_candidates, count| (0..count).collect(),
                ),
            },
            None => self.gameobject_delete_like_cpp(guid),
        }
    }
fn gameobject_local_rotation_from_orientation_like_cpp(orientation: f32) -> [f32; 4] {
    let half = orientation * 0.5;
    [0.0, 0.0, half.sin(), half.cos()]
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

}
