// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject, transport, scene object and area-trigger lifecycle.

use super::*;
use crate::map_rules::{
    gameobject_is_spawned_like_cpp, map_record_is_unit_like_gameobject_owner_like_cpp,
    map_record_unit_mut_like_cpp,
};

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn contains_gameobject_model_like_cpp(
        &self,
        key: RepresentedGameObjectModelKeyLikeCpp,
    ) -> bool {
        self.dynamic_tree_model_keys_like_cpp.contains(&key)
    }

    /// Represents C++ `Map::InsertGameObjectModel` -> `DynamicMapTree::insert`.
    ///
    /// The real C++ tree receives a `GameObjectModel const&`; this represented
    /// seam stores a deterministic owner-GUID key only. A duplicate key is a
    /// guarded no-op, so represented count/unbalanced state cannot drift from
    /// repeated calls with the same owner GUID.
    pub fn insert_gameobject_model_like_cpp(
        &mut self,
        key: RepresentedGameObjectModelKeyLikeCpp,
    ) -> DynamicMapTreeModelMutationOutcomeLikeCpp {
        let model_count_before = self.dynamic_tree_model_keys_like_cpp.len();
        let unbalanced_before = self.dynamic_tree_unbalanced_times_like_cpp;
        let inserted = self.dynamic_tree_model_keys_like_cpp.insert(key);

        if inserted {
            self.dynamic_tree_unbalanced_times_like_cpp = self
                .dynamic_tree_unbalanced_times_like_cpp
                .saturating_add(1);
        }

        DynamicMapTreeModelMutationOutcomeLikeCpp {
            key,
            status: if inserted {
                DynamicMapTreeModelMutationStatusLikeCpp::Inserted
            } else {
                DynamicMapTreeModelMutationStatusLikeCpp::AlreadyPresent
            },
            model_count_before,
            model_count_after: self.dynamic_tree_model_keys_like_cpp.len(),
            unbalanced_before,
            unbalanced_after: self.dynamic_tree_unbalanced_times_like_cpp,
        }
    }

    /// Represents C++ `GameObject::SetDisplayId(uint32)` over canonical map-owned state.
    ///
    /// C++ anchor: `GameObject.cpp:3817-3820`. C++ first writes
    /// `GameObjectData::DisplayID`, then calls `UpdateModel()`. This map-owned
    /// caller seam preserves that order and delegates all represented model-key
    /// side effects to `update_gameobject_model_like_cpp`.
    pub fn set_gameobject_display_id_like_cpp(
        &mut self,
        guid: ObjectGuid,
        display_id: u32,
        new_has_model: bool,
        new_is_map_object: bool,
    ) -> GameObjectSetDisplayIdOutcomeLikeCpp {
        let Some(record) = self.entity_world.get(&guid) else {
            return GameObjectSetDisplayIdOutcomeLikeCpp {
                guid,
                status: GameObjectSetDisplayIdStatusLikeCpp::MissingGameObject,
                previous_display_id: None,
                new_display_id: None,
                update_model: None,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject || record.game_object().is_none() {
            return GameObjectSetDisplayIdOutcomeLikeCpp {
                guid,
                status: GameObjectSetDisplayIdStatusLikeCpp::WrongKind,
                previous_display_id: None,
                new_display_id: None,
                update_model: None,
            };
        }

        let Some(game_object) = self
            .entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        else {
            return GameObjectSetDisplayIdOutcomeLikeCpp {
                guid,
                status: GameObjectSetDisplayIdStatusLikeCpp::WrongKind,
                previous_display_id: None,
                new_display_id: None,
                update_model: None,
            };
        };

        let previous_display_id = game_object.data().display_id;
        game_object.set_display_id(display_id);
        let new_display_id = game_object.data().display_id;

        let update_model =
            self.update_gameobject_model_like_cpp(guid, new_has_model, new_is_map_object);

        GameObjectSetDisplayIdOutcomeLikeCpp {
            guid,
            status: GameObjectSetDisplayIdStatusLikeCpp::Updated,
            previous_display_id: Some(previous_display_id),
            new_display_id: Some(new_display_id),
            update_model: Some(update_model),
        }
    }

    /// Represents C++ `GameObject::SetGoState(GOState)` over canonical map-owned state.
    ///
    /// C++ anchor: `GameObject.cpp:3771-3793`. Source-of-truth is
    /// `Map::entity_world`; this mutates only exact typed
    /// `MapObjectRecord::GameObject` records. The state write occurs before the
    /// represented `m_model && !IsTransport()` not-in-world early return, matching
    /// C++ statement order. Collision is never inferred from display/template/DB.
    pub fn set_gameobject_go_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
        state: GoState,
    ) -> GameObjectSetGoStateOutcomeLikeCpp {
        let Some(record) = self.entity_world.get(&guid) else {
            return GameObjectSetGoStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetGoStateStatusLikeCpp::MissingGameObject,
                previous_state: None,
                new_state: None,
                represented_model_present: false,
                transport_type: false,
                in_world_for_collision_branch: None,
                collision_enable: None,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject || record.game_object().is_none() {
            return GameObjectSetGoStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetGoStateStatusLikeCpp::WrongKind,
                previous_state: None,
                new_state: None,
                represented_model_present: false,
                transport_type: false,
                in_world_for_collision_branch: None,
                collision_enable: None,
            };
        }

        let Some(game_object) = self
            .entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        else {
            return GameObjectSetGoStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetGoStateStatusLikeCpp::WrongKind,
                previous_state: None,
                new_state: None,
                represented_model_present: false,
                transport_type: false,
                in_world_for_collision_branch: None,
                collision_enable: None,
            };
        };

        let previous_state = game_object.data().state;
        let represented_model_present = game_object.has_represented_gameobject_model_like_cpp();
        let transport_type = gameobject_type_is_transport_like_cpp(game_object.data().type_id);
        game_object.set_go_state(state);
        let new_state = game_object.data().state;

        let (in_world_for_collision_branch, collision_enable) =
            if represented_model_present && !transport_type {
                let in_world = game_object.world().object().is_in_world();
                if in_world {
                    let collision = game_object
                        .enable_represented_gameobject_collision_like_cpp(state == GoState::Ready);
                    (
                        Some(true),
                        Some(GameObjectCollisionEnableOutcomeLikeCpp {
                            requested_enable: collision.requested_enable,
                            represented_model_present: collision.represented_model_present,
                            previous_collision_enabled: collision.previous_collision_enabled,
                            new_collision_enabled: collision.new_collision_enabled,
                        }),
                    )
                } else {
                    (Some(false), None)
                }
            } else {
                (None, None)
            };

        GameObjectSetGoStateOutcomeLikeCpp {
            guid,
            status: GameObjectSetGoStateStatusLikeCpp::Updated,
            previous_state: Some(previous_state),
            new_state: Some(new_state),
            represented_model_present,
            transport_type,
            in_world_for_collision_branch,
            collision_enable,
        }
    }

    /// Represents C++ `GameObject::SetLootState(LootState, Unit*)` over canonical map-owned state.
    ///
    /// C++ anchor: `GameObject.cpp:3683-3709`. Source-of-truth is `Map::entity_world`;
    /// this mutates only exact typed `MapObjectRecord::GameObject` records. The `unit_guid`
    /// argument is only represented evidence for `unit->GetGUID()` and no real `Unit*` is
    /// resolved. Restock consumes explicit caller-supplied `Loot::IsChanged()` evidence; collision
    /// consumes only explicit represented `m_model` evidence and never real geometry/BIH.
    pub fn set_gameobject_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
        state: LootState,
        unit_guid: Option<ObjectGuid>,
        game_time_secs: i64,
        chest_restock_time_secs: u32,
        shared_loot_is_changed_like_cpp: bool,
    ) -> GameObjectSetLootStateOutcomeLikeCpp {
        let Some(record) = self.entity_world.get(&guid) else {
            return GameObjectSetLootStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetLootStateStatusLikeCpp::MissingGameObject,
                previous_loot_state: None,
                new_loot_state: None,
                previous_loot_state_unit_guid: None,
                new_loot_state_unit_guid: None,
                previous_restock_time: None,
                new_restock_time: None,
                ai_on_loot_state_changed_not_represented: false,
                restock_armed: false,
                represented_model_present: false,
                door_type_early_return: false,
                collision_enable: None,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject || record.game_object().is_none() {
            return GameObjectSetLootStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetLootStateStatusLikeCpp::WrongKind,
                previous_loot_state: None,
                new_loot_state: None,
                previous_loot_state_unit_guid: None,
                new_loot_state_unit_guid: None,
                previous_restock_time: None,
                new_restock_time: None,
                ai_on_loot_state_changed_not_represented: false,
                restock_armed: false,
                represented_model_present: false,
                door_type_early_return: false,
                collision_enable: None,
            };
        }

        let Some(game_object) = self
            .entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        else {
            return GameObjectSetLootStateOutcomeLikeCpp {
                guid,
                status: GameObjectSetLootStateStatusLikeCpp::WrongKind,
                previous_loot_state: None,
                new_loot_state: None,
                previous_loot_state_unit_guid: None,
                new_loot_state_unit_guid: None,
                previous_restock_time: None,
                new_restock_time: None,
                ai_on_loot_state_changed_not_represented: false,
                restock_armed: false,
                represented_model_present: false,
                door_type_early_return: false,
                collision_enable: None,
            };
        };

        let previous_loot_state = game_object.loot_state();
        let previous_loot_state_unit_guid = game_object.loot_state_unit_guid();
        let previous_restock_time = game_object.restock_time();
        let represented_model_present = game_object.has_represented_gameobject_model_like_cpp();
        let type_id = game_object.data().type_id;

        game_object.set_loot_state(state, unit_guid);

        let restock_armed = type_id == GAMEOBJECT_TYPE_CHEST as i8
            && state == LootState::Activated
            && chest_restock_time_secs > 0
            && previous_restock_time == 0
            && shared_loot_is_changed_like_cpp;
        if restock_armed {
            let restock_time = game_time_secs.saturating_add(i64::from(chest_restock_time_secs));
            game_object.set_restock_time_like_cpp(restock_time);
        }

        let door_type_early_return = type_id == GAMEOBJECT_TYPE_DOOR as i8;
        let collision_enable = if door_type_early_return || !represented_model_present {
            None
        } else {
            let collision_enabled = (game_object.data().state != GoState::Ready as i8
                && (state == LootState::Activated || state == LootState::JustDeactivated))
                || state == LootState::Ready;
            let collision =
                game_object.enable_represented_gameobject_collision_like_cpp(collision_enabled);
            Some(GameObjectCollisionEnableOutcomeLikeCpp {
                requested_enable: collision.requested_enable,
                represented_model_present: collision.represented_model_present,
                previous_collision_enabled: collision.previous_collision_enabled,
                new_collision_enabled: collision.new_collision_enabled,
            })
        };

        GameObjectSetLootStateOutcomeLikeCpp {
            guid,
            status: GameObjectSetLootStateStatusLikeCpp::Updated,
            previous_loot_state: Some(previous_loot_state),
            new_loot_state: Some(game_object.loot_state()),
            previous_loot_state_unit_guid: Some(previous_loot_state_unit_guid),
            new_loot_state_unit_guid: Some(game_object.loot_state_unit_guid()),
            previous_restock_time: Some(previous_restock_time),
            new_restock_time: Some(game_object.restock_time()),
            ai_on_loot_state_changed_not_represented: true,
            restock_armed,
            represented_model_present,
            door_type_early_return,
            collision_enable,
        }
    }

    /// Represents C++ `GameObject::UpdateModel()` over canonical map-owned state.
    ///
    /// C++ anchors: `GameObject.cpp:3867-3880`, `GameObject.cpp:4394-4399`, and
    /// `GameObject.cpp:3818-3820`. The caller supplies explicit represented
    /// `CreateModel()` output; this helper never infers model existence or
    /// map-object-ness from display id, template, type or DB. Only exact typed
    /// `MapObjectRecord::GameObject` records are mutated; missing, untyped,
    /// wrong-kind and not-in-world records are explicit no-mutation outcomes.
    pub fn update_gameobject_model_like_cpp(
        &mut self,
        guid: ObjectGuid,
        new_has_model: bool,
        new_is_map_object: bool,
    ) -> GameObjectUpdateModelOutcomeLikeCpp {
        let key = RepresentedGameObjectModelKeyLikeCpp { owner_guid: guid };
        let Some(record) = self.entity_world.get(&guid) else {
            return GameObjectUpdateModelOutcomeLikeCpp {
                guid,
                status: GameObjectUpdateModelStatusLikeCpp::MissingGameObject,
                old_model_present: false,
                old_model_registered: false,
                old_model_remove: None,
                new_has_model,
                new_is_map_object,
                new_model_insert: None,
            };
        };

        if record.kind() != AccessorObjectKind::GameObject || record.game_object().is_none() {
            return GameObjectUpdateModelOutcomeLikeCpp {
                guid,
                status: GameObjectUpdateModelStatusLikeCpp::WrongKind,
                old_model_present: false,
                old_model_registered: false,
                old_model_remove: None,
                new_has_model,
                new_is_map_object,
                new_model_insert: None,
            };
        }

        let game_object = record
            .game_object()
            .expect("exact typed GameObject record checked above");
        if !game_object.world().object().is_in_world() {
            return GameObjectUpdateModelOutcomeLikeCpp {
                guid,
                status: GameObjectUpdateModelStatusLikeCpp::NotInWorld,
                old_model_present: game_object.has_represented_gameobject_model_like_cpp(),
                old_model_registered: self.contains_gameobject_model_like_cpp(key),
                old_model_remove: None,
                new_has_model,
                new_is_map_object,
                new_model_insert: None,
            };
        }

        let old_model_present = game_object.has_represented_gameobject_model_like_cpp();
        let old_model_registered =
            old_model_present && self.contains_gameobject_model_like_cpp(key);
        let old_model_remove =
            old_model_registered.then(|| self.remove_gameobject_model_like_cpp(key));

        if let Some(game_object) = self
            .entity_world
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
        {
            // C++ removes `GO_FLAG_MAP_OBJECT`, deletes/nulls `m_model`, then
            // calls `CreateModel()`. The first call clears old map-object and
            // collision evidence; the second installs only the explicit new
            // model/map-object evidence and does not call `EnableCollision()`.
            game_object.apply_represented_gameobject_model_creation_like_cpp(false, false);
            game_object.apply_represented_gameobject_model_creation_like_cpp(
                new_has_model,
                new_is_map_object,
            );
        }

        let new_model_insert = new_has_model.then(|| self.insert_gameobject_model_like_cpp(key));

        GameObjectUpdateModelOutcomeLikeCpp {
            guid,
            status: GameObjectUpdateModelStatusLikeCpp::Updated,
            old_model_present,
            old_model_registered,
            old_model_remove,
            new_has_model,
            new_is_map_object,
            new_model_insert,
        }
    }

    /// Map-owned seam for C++ `GameObject::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:666-785` creates `Trinity::ObjectUpdater updater(t_diff)`
    ///   during `Map::Update`.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects and explicitly instantiates `GameObject`.
    /// - `GameObject.cpp:1215-1233` is represented through the entity-level
    ///   `m_despawnDelay` countdown; expiry represents `DespawnOrUnsummon(0ms,
    ///   m_despawnRespawnTime)`.
    /// - `GameObject.cpp:1575-1580` `GO_JUST_DEACTIVATED` despawns an
    ///   already-linked trap via `GetLinkedTrap()->DespawnOrUnsummon()` before
    ///   later goober/chest/generic cleanup.
    /// - `GameObject.cpp:1740-1764` `Delete()` is represented only as
    ///   `SetLootState(GO_NOT_READY)` plus `AddObjectToRemoveList()`.
    ///
    /// Ownership: source-of-truth is canonical `Map::entity_world`. Missing,
    /// non-GameObject and not-in-world outcomes do not mutate state. This helper
    /// never creates fallback records, reads session/ObjectAccessor mirrors,
    /// saves DB respawn times, runs PoolMgr, sends packets, fans out visibility,
    /// executes AI/go-type implementations, drains removal, or includes Transport
    /// records whose embedded body happens to be a GameObject.
    pub fn update_game_object_like_cpp(
        &mut self,
        game_object_guid: ObjectGuid,
        diff_ms: u32,
        game_time_secs: i64,
    ) -> GameObjectUpdateOutcomeLikeCpp {
        self.update_game_object_with_optional_pool_update_like_cpp::<fn(
            &mut Self,
            SpawnObjectType,
            SpawnId,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(
            game_object_guid,
            diff_ms,
            game_time_secs,
            None,
            None,
        )
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `GameObject` records only.
    ///
    /// This snapshots canonical typed GameObject GUIDs from `Map::entity_world`
    /// and delegates each GUID to `update_game_object_like_cpp`. C++ visits by
    /// nearby cell/active object order; this slice only adds the missing
    /// map-owned GameObject family and keeps the existing Rust family order.
    pub fn update_game_objects_like_cpp(
        &mut self,
        diff_ms: u32,
        game_time_secs: i64,
    ) -> GameObjectsUpdateSummaryLikeCpp {
        self.update_game_objects_with_optional_pool_update_like_cpp::<fn(
            &mut Self,
            SpawnObjectType,
            SpawnId,
        ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(diff_ms, game_time_secs, None, None)
    }

    pub fn gameobject_spawn_id_store_count_like_cpp(&self, spawn_id: SpawnId) -> usize {
        self.gameobjects_by_spawn_id
            .get(&spawn_id)
            .map_or(0, HashSet::len)
    }

    pub fn area_trigger_spawn_id_store_count_like_cpp(&self, spawn_id: SpawnId) -> usize {
        self.area_triggers_by_spawn_id
            .get(&spawn_id)
            .map_or(0, HashSet::len)
    }

    pub fn gameobject_spawn_id_store_guids_like_cpp(&self, spawn_id: SpawnId) -> Vec<ObjectGuid> {
        self.gameobjects_by_spawn_id
            .get(&spawn_id)
            .map(|guids| {
                let mut guids: Vec<_> = guids.iter().copied().collect();
                guids.sort();
                guids
            })
            .unwrap_or_default()
    }

    pub fn area_trigger_spawn_id_store_guids_like_cpp(&self, spawn_id: SpawnId) -> Vec<ObjectGuid> {
        self.area_triggers_by_spawn_id
            .get(&spawn_id)
            .map(|guids| {
                let mut guids: Vec<_> = guids.iter().copied().collect();
                // C++ returns the first unordered_multimap entry; Rust sorts for deterministic tests.
                guids.sort();
                guids
            })
            .unwrap_or_default()
    }

    pub fn get_gameobject_by_spawn_id_like_cpp(&self, spawn_id: SpawnId) -> Option<&GameObject> {
        let mut fallback_guid = None;
        let mut spawned_guid = None;
        for guid in self.gameobject_spawn_id_store_guids_like_cpp(spawn_id) {
            let Some(gameobject) = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::game_object)
            else {
                continue;
            };
            if gameobject.spawn_id() != spawn_id {
                continue;
            }
            fallback_guid.get_or_insert(guid);
            if gameobject_is_spawned_like_cpp(gameobject) {
                spawned_guid = Some(guid);
                break;
            }
        }

        spawned_guid
            .or(fallback_guid)
            .and_then(|guid| self.map_object_record(guid)?.game_object())
    }

    pub fn get_area_trigger_by_spawn_id_like_cpp(&self, spawn_id: SpawnId) -> Option<&AreaTrigger> {
        self.area_trigger_spawn_id_store_guids_like_cpp(spawn_id)
            .into_iter()
            .find_map(|guid| self.map_object_record(guid)?.area_trigger())
    }

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

    pub fn get_game_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(
            guid,
            &[
                AccessorObjectKind::GameObject,
                AccessorObjectKind::Transport,
            ],
        )
    }

    pub fn get_typed_game_object(&self, guid: ObjectGuid) -> Option<&GameObject> {
        let record = self.map_object_record(guid)?;
        if !matches!(
            record.kind(),
            AccessorObjectKind::GameObject | AccessorObjectKind::Transport
        ) {
            return None;
        }
        record.game_object()
    }

    pub fn get_typed_game_object_mut(&mut self, guid: ObjectGuid) -> Option<&mut GameObject> {
        let record = self.entity_world.get_mut(&guid)?;
        if !matches!(
            record.kind(),
            AccessorObjectKind::GameObject | AccessorObjectKind::Transport
        ) {
            return None;
        }
        record.game_object_mut()
    }

    pub fn get_transport(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Transport])
    }

    /// Return the typed canonical transport that currently owns a passenger.
    ///
    /// C++ `WorldObject::GetTransGUID` is backed by the object's transport
    /// movement state. The canonical Rust transport runtime owns passenger
    /// membership on `Transport`, so spell destination resolution uses this
    /// map-local lookup instead of guessing from a generic transport object.
    pub fn get_typed_transport_for_passenger_like_cpp(
        &self,
        passenger_guid: ObjectGuid,
    ) -> Option<&wow_entities::Transport> {
        self.entity_world.values().find_map(|record| {
            let transport = record.transport()?;
            (transport.passengers().contains(&passenger_guid)
                || transport.static_passengers().contains(&passenger_guid))
            .then_some(transport)
        })
    }

    pub fn get_typed_transport_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<&wow_entities::Transport> {
        self.entity_world.get(&guid)?.transport()
    }

    /// Mutably resolve a canonical transport for a movement passenger update.
    ///
    /// The map owns the `Transport` passenger set just as TrinityCore's map
    /// object owns `TransportBase::_passengers`; callers receive only the
    /// typed transport operation and never a generic object record.
    pub fn get_typed_transport_mut_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<&mut wow_entities::Transport> {
        self.entity_world.get_mut(&guid)?.transport_mut()
    }

    pub fn get_area_trigger(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::AreaTrigger])
    }

    pub fn get_scene_object(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::SceneObject])
    }

    pub fn get_conversation(&self, guid: ObjectGuid) -> Option<&WorldObject> {
        self.map_object_by_kind(guid, &[AccessorObjectKind::Conversation])
    }

    pub fn load_loaded_grid_area_trigger_records_like_cpp<L>(
        &mut self,
        coord: GridCoord,
        spawn_store: &SpawnStore,
        mut load_record: L,
    ) -> LoadedGridAreaTriggerRecordsSummaryLikeCpp
    where
        L: FnMut(&mut Self, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let Some(grid) = self.get_ngrid(coord) else {
            return LoadedGridAreaTriggerRecordsSummaryLikeCpp {
                grid_not_loaded: true,
                ..Default::default()
            };
        };
        if !grid.grid_object_data_loaded() {
            return LoadedGridAreaTriggerRecordsSummaryLikeCpp {
                grid_not_loaded: true,
                ..Default::default()
            };
        }

        let mut spawn_ids = Vec::new();
        for x in 0..MAX_NUMBER_OF_CELLS {
            for y in 0..MAX_NUMBER_OF_CELLS {
                let Some(cell) = grid.get_grid_type(x, y) else {
                    continue;
                };
                if let Some(cell_guids) = spawn_store.cell_object_guids(
                    self.map_id,
                    self.spawn_mode,
                    cell.cell_coord().get_id(),
                ) {
                    spawn_ids.extend(cell_guids.area_triggers.iter().copied());
                }
            }
        }

        let spawn_filter = self.spawn_grid_load_state_like_cpp(spawn_store);
        let mut plans = Vec::new();
        let mut summary = LoadedGridAreaTriggerRecordsSummaryLikeCpp::default();
        for spawn_id in spawn_ids {
            if self
                .get_area_trigger_by_spawn_id_like_cpp(spawn_id)
                .is_some()
            {
                summary.skipped_already_loaded += 1;
                continue;
            }
            if !spawn_filter.should_be_spawned_on_grid_load(SpawnObjectType::AreaTrigger, spawn_id)
            {
                summary.skipped_should_not_spawn += 1;
                continue;
            }
            let Some(spawn_data) = spawn_store.spawn_data(SpawnObjectType::AreaTrigger, spawn_id)
            else {
                summary.stale_index_entries += 1;
                continue;
            };
            if spawn_data.map_id != self.map_id {
                summary.stale_index_entries += 1;
                continue;
            }
            if !spawn_data.spawn_difficulties.contains(&self.spawn_mode) {
                summary.skipped_difficulty_mismatch += 1;
                continue;
            }
            summary.metadata_entries += 1;
            plans.push(spawn_id);
        }
        drop(spawn_filter);

        for spawn_id in plans {
            let Some(records) = load_record(self, SpawnObjectType::AreaTrigger, spawn_id) else {
                summary.load_record_missing += 1;
                continue;
            };
            for pre_add_record in records.pre_add_records {
                if self
                    .add_map_object_record_to_map_like_cpp(pre_add_record)
                    .is_ok()
                {
                    summary.pre_add_records_added += 1;
                } else {
                    summary.add_to_map_errors += 1;
                }
            }
            let primary_record = records.primary_record;
            let loaded_grid_primary_record = primary_record.clone();
            match self.add_map_object_record_to_map_like_cpp(primary_record) {
                Ok(_outcome) => summary
                    .loaded_grid_primary_records
                    .push(loaded_grid_primary_record),
                Err(_error) => summary.add_to_map_errors += 1,
            }
        }

        summary
    }
}
