// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject summoning and owner-slot operations.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    /// Bounded map-owned representation of C++ `Unit::AddGameObject(GameObject*)`.
    ///
    /// C++ anchors:
    /// - `Unit.cpp:5192-5209`: if the object exists and has no owner, append to
    ///   `m_gameObj`, set `CreatedBy` to the Unit GUID, optionally start
    ///   event-based cooldown, and dispatch `CreatureAI::JustSummonedGameobject`.
    /// - `Object.cpp:2067-2090` and `SpellEffects.cpp:3238/3590/4456-4482`:
    ///   summon/create paths call this helper for the owning Unit before or
    ///   around `Map::AddToMap`.
    ///
    /// Scope: this does not create objects, insert into object slots, start
    /// cooldowns, execute scripts/SmartAI, send packets, or touch DB. Slot
    /// assignment is path-specific in C++ (`Spell::EffectSummonObject`) and
    /// remains a caller concern.
    pub fn gameobject_add_to_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        guid: ObjectGuid,
    ) -> GameObjectAddToOwnerOutcomeLikeCpp {
        let owner_found_as_unit_like = self
            .map_object_record(owner_guid)
            .is_some_and(map_record_is_unit_like_gameobject_owner_like_cpp);
        let (gameobject_found, owner_guid_before) = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::GameObject)
            .and_then(MapObjectRecord::game_object)
            .map(|game_object| (true, game_object.owner_guid()))
            .unwrap_or((false, ObjectGuid::EMPTY));
        let gameobject_owner_empty_before = gameobject_found && owner_guid_before.is_empty();

        let mut registered_owned_gameobject = false;
        let mut owner_guid_after = owner_guid_before;
        let mut creature_ai_callback_represented = false;

        if owner_found_as_unit_like && gameobject_owner_empty_before {
            if let Some(record) = self.entity_world.get_mut(&owner_guid) {
                if let Some(owner) = map_record_unit_mut_like_cpp(record) {
                    owner
                        .subsystems_mut()
                        .control
                        .register_owned_gameobject_like_cpp(guid);
                    registered_owned_gameobject = true;
                }
            }

            if registered_owned_gameobject {
                if let Some(game_object) = self
                    .entity_world
                    .get_mut(&guid)
                    .and_then(MapObjectRecord::game_object_mut)
                {
                    game_object.set_owner_guid_like_cpp(owner_guid);
                    owner_guid_after = game_object.owner_guid();
                }

                creature_ai_callback_represented = self
                    .entity_world
                    .get_mut(&owner_guid)
                    .map(|record| match record.kind() {
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
                    })
                    .unwrap_or(false);
            }
        }

        GameObjectAddToOwnerOutcomeLikeCpp {
            guid,
            owner_guid,
            owner_found_as_unit_like,
            gameobject_found,
            owner_guid_before,
            owner_guid_after,
            gameobject_owner_empty_before,
            registered_owned_gameobject,
            owner_guid_set: owner_guid_after == owner_guid && owner_guid_before != owner_guid,
            cooldown_start_represented: false,
            creature_ai_callback_represented,
        }
    }

    /// Bounded map-owned tail for C++ `Spell::EffectSummonObject`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:3548-3563`: the caller clears any previous
    ///   `m_ObjectSlot[slot]` and deletes the old GameObject before creating
    ///   the replacement.
    /// - `SpellEffects.cpp:3590-3597`: after `Unit::AddGameObject(go)` and
    ///   `Map::AddToMap(go)`, the caster writes `m_ObjectSlot[slot]`.
    ///
    /// Scope: this helper represents only the post-create owner link and final
    /// slot assignment for an already map-owned GameObject. It does not create
    /// the GameObject, clear/delete an old slot occupant, compute spell
    /// duration/location, inherit phase, or send packets.
    pub fn gameobject_add_to_owner_slot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        guid: ObjectGuid,
        slot: usize,
    ) -> GameObjectAddToOwnerSlotOutcomeLikeCpp {
        let add_owner = self.gameobject_add_to_owner_like_cpp(owner_guid, guid);
        let mut slot_previous_guid = ObjectGuid::EMPTY;
        let mut slot_set = false;

        if add_owner.registered_owned_gameobject {
            if let Some(owner) = self
                .entity_world
                .get_mut(&owner_guid)
                .and_then(map_record_unit_mut_like_cpp)
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
        }

        GameObjectAddToOwnerSlotOutcomeLikeCpp {
            add_owner,
            slot,
            slot_previous_guid,
            slot_set,
        }
    }

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
            map_record_is_unit_like_gameobject_owner_like_cpp(summoner_record);
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
            if let Some(record) = self.entity_world.get_mut(&summoner_guid) {
                if let Some(owner) = map_record_unit_mut_like_cpp(record) {
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
            .is_some_and(map_record_is_unit_like_gameobject_owner_like_cpp);
        let slot_guid_before = self
            .map_object_record(owner_guid)
            .and_then(map_record_unit_like_cpp)
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
                .and_then(MapObjectRecord::game_object)
                .is_some();

            if gameobject_found {
                unit_pointer_owner_match = self
                    .map_object_record(slot_guid_before)
                    .and_then(MapObjectRecord::game_object)
                    .is_some_and(|gameobject| gameobject.owner_guid() == owner_guid);
                if let Some(gameobject) = self
                    .entity_world
                    .get_mut(&slot_guid_before)
                    .and_then(MapObjectRecord::game_object_mut)
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
                        .and_then(MapObjectRecord::game_object_mut)
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
                .and_then(map_record_unit_mut_like_cpp)
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
            .and_then(map_record_unit_like_cpp)
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
        if let Some(record) = self.entity_world.get_mut(&owner_guid) {
            if let Some(owner) = map_record_unit_mut_like_cpp(record) {
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
                .and_then(map_record_unit_mut_like_cpp)
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
}
