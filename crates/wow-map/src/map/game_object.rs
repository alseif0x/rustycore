// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject, transport, scene object and area-trigger lifecycle.

use super::*;

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
        let Some(record) = self.map_objects.get(&guid) else {
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
            .map_objects
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
    /// `Map::map_objects`; this mutates only exact typed
    /// `MapObjectRecord::GameObject` records. The state write occurs before the
    /// represented `m_model && !IsTransport()` not-in-world early return, matching
    /// C++ statement order. Collision is never inferred from display/template/DB.
    pub fn set_gameobject_go_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
        state: GoState,
    ) -> GameObjectSetGoStateOutcomeLikeCpp {
        let Some(record) = self.map_objects.get(&guid) else {
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
            .map_objects
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
    /// C++ anchor: `GameObject.cpp:3683-3709`. Source-of-truth is `Map::map_objects`;
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
        let Some(record) = self.map_objects.get(&guid) else {
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
            .map_objects
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
        let Some(record) = self.map_objects.get(&guid) else {
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
            .map_objects
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
    /// Ownership: source-of-truth is canonical `Map::map_objects`. Missing,
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
    /// This snapshots canonical typed GameObject GUIDs from `Map::map_objects`
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

    /// Map-owned seam for C++ `Transport::Update(uint32 diff)` under `Map::Update`.
    ///
    /// C++ anchors:
    /// - `Map.cpp:666-785` updates object families, transport collection, then later
    ///   `SendObjectUpdates`; exact TypeContainerVisitor and `_transports` ordering is
    ///   not fully reproduced here.
    /// - `Transport.cpp:179-251` is represented only for local timers/path progress,
    ///   stop request evidence, client path-progress field, expected-map gated
    ///   200ms position-update due evidence, and stopped state/dynflag.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. Missing,
    /// non-Transport and untyped Transport-kind outcomes do not mutate state.
    /// Unlike `ObjectUpdater::Visit<T>`, the C++ `_transports` loop does not gate
    /// canonical transports on `IsInWorld`, so typed Transport records are delegated
    /// even when their embedded WorldObject is not in-world. This helper never
    /// creates fallback records, reads session/ObjectAccessor mirrors, runs
    /// scripts/AI/GameEvents, computes real spline position, teleports,
    /// spawns/removes static passengers, relocates passengers, fans out packets, or
    /// drains queues.
    pub fn update_transport_like_cpp(
        &mut self,
        transport_guid: ObjectGuid,
        diff_ms: u32,
        now_ms: u64,
    ) -> TransportUpdateOutcomeLikeCpp {
        let current_map_id = self.map_id;
        let Some(record) = self.map_object_record(transport_guid) else {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::MissingTransport,
                period_ms: None,
                path_progress_before_ms: None,
                path_progress_after_ms: None,
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        };

        if record.kind() != AccessorObjectKind::Transport {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::NotTransport,
                period_ms: None,
                path_progress_before_ms: None,
                path_progress_after_ms: None,
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        }

        let Some(transport) = record.transport() else {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::NotTransport,
                period_ms: None,
                path_progress_before_ms: None,
                path_progress_after_ms: None,
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        };

        let period_ms = transport.get_transport_period();
        let path_progress_before_ms = transport.path_progress_ms();

        let Some(record) = self.map_objects.get_mut(&transport_guid) else {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::MissingTransport,
                period_ms: Some(period_ms),
                path_progress_before_ms: Some(path_progress_before_ms),
                path_progress_after_ms: Some(path_progress_before_ms),
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        };
        let Some(transport) = record.transport_mut() else {
            return TransportUpdateOutcomeLikeCpp {
                transport_guid,
                diff_ms,
                now_ms,
                current_map_id,
                status: TransportUpdateStatusLikeCpp::NotTransport,
                period_ms: Some(period_ms),
                path_progress_before_ms: Some(path_progress_before_ms),
                path_progress_after_ms: Some(path_progress_before_ms),
                timer_ms: None,
                expected_map_matches_current_map: false,
                position_update_due: false,
                position_update_represented: false,
                just_stopped: false,
                entity_update: None,
            };
        };

        let entity_update = transport.update_like_cpp(diff_ms, now_ms, current_map_id);
        let status = if entity_update.unsupported_no_period {
            TransportUpdateStatusLikeCpp::UnsupportedNoPeriod
        } else {
            TransportUpdateStatusLikeCpp::Updated
        };
        TransportUpdateOutcomeLikeCpp {
            transport_guid,
            diff_ms,
            now_ms,
            current_map_id,
            status,
            period_ms: Some(entity_update.period_ms),
            path_progress_before_ms: Some(entity_update.old_path_progress_ms),
            path_progress_after_ms: Some(entity_update.new_path_progress_ms),
            timer_ms: entity_update.timer_ms,
            expected_map_matches_current_map: entity_update.expected_map_matches_current_map,
            position_update_due: entity_update.position_update_due,
            position_update_represented: entity_update.position_update_represented,
            just_stopped: entity_update.just_stopped,
            entity_update: Some(entity_update),
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// typed canonical Transport records only. This snapshots `MapObjectRecord`
    /// GUIDs before mutation and deliberately excludes generic `WorldObject`
    /// fallback records even when their kind is Transport.
    pub fn update_transports_like_cpp(
        &mut self,
        diff_ms: u32,
        now_ms: u64,
    ) -> TransportsUpdateSummaryLikeCpp {
        let transport_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::Transport && record.transport().is_some())
                    .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = TransportsUpdateSummaryLikeCpp::default();
        for guid in transport_guids {
            summary.visited += 1;
            let outcome = self.update_transport_like_cpp(guid, diff_ms, now_ms);
            match outcome.status {
                TransportUpdateStatusLikeCpp::Updated => summary.updated += 1,
                TransportUpdateStatusLikeCpp::UnsupportedNoPeriod => {
                    summary.unsupported_no_period += 1;
                }
                TransportUpdateStatusLikeCpp::MissingTransport => summary.missing_or_stale += 1,
                TransportUpdateStatusLikeCpp::NotTransport => summary.not_transport += 1,
                TransportUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
            if outcome.position_update_represented {
                summary.position_updates_represented += 1;
            }
            if outcome.just_stopped {
                summary.just_stopped += 1;
            }
        }

        summary
    }

    /// Map-owned seam for C++ `AreaTrigger::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `AreaTrigger.cpp:297-364` runs `WorldObject::Update(diff)`, increments
    ///   `_timeSinceCreated`, runs the non-static movement/orbit/shape branch
    ///   before duration expiry, calls `Remove(); return;` on duration expiry,
    ///   and only then runs AI update plus target-list update.
    /// - `AreaTrigger.cpp:366-372` makes `Remove()` enqueue through
    ///   `AddObjectToRemoveList()` only when the object is in world.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects and explicitly instantiates `AreaTrigger`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. This helper
    /// mutates only typed `MapObjectRecord::AreaTrigger` time/duration state and,
    /// after dropping that mutable borrow, enqueues the same GUID through the
    /// existing remove-list facade on expiry. It does not drain removal, run real
    /// movement/shape, AI, target-list runtime, ObjectAccessor/session mirrors,
    /// fanout, packets, dynamic tree, scripts, or create fallback records.
    pub fn update_area_trigger_like_cpp(
        &mut self,
        area_trigger_guid: ObjectGuid,
        elapsed_ms: u32,
    ) -> AreaTriggerUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(area_trigger_guid) else {
            return AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::MissingAreaTrigger,
                duration_before_ms: None,
                duration_after_ms: None,
                time_since_created_before_ms: None,
                time_since_created_after_ms: None,
                non_static_movement_would_run: false,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: None,
            };
        };

        if record.kind() != AccessorObjectKind::AreaTrigger {
            return AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger,
                duration_before_ms: None,
                duration_after_ms: None,
                time_since_created_before_ms: None,
                time_since_created_after_ms: None,
                non_static_movement_would_run: false,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: None,
            };
        }

        let Some(area_trigger) = record.area_trigger() else {
            return AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger,
                duration_before_ms: None,
                duration_after_ms: None,
                time_since_created_before_ms: None,
                time_since_created_after_ms: None,
                non_static_movement_would_run: false,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: None,
            };
        };

        let duration_before_ms = area_trigger.duration_ms();
        let time_since_created_before_ms = area_trigger.time_since_created_ms();
        let non_static_movement_would_run = !area_trigger.is_static_spawn();
        if !area_trigger.world().object().is_in_world() {
            return AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::NotInWorld,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_before_ms),
                time_since_created_before_ms: Some(time_since_created_before_ms),
                time_since_created_after_ms: Some(time_since_created_before_ms),
                non_static_movement_would_run: false,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: None,
            };
        }

        let (expired, duration_after_ms, time_since_created_after_ms) = {
            let Some(record) = self.map_objects.get_mut(&area_trigger_guid) else {
                return AreaTriggerUpdateOutcomeLikeCpp {
                    area_trigger_guid,
                    elapsed_ms,
                    status: AreaTriggerUpdateStatusLikeCpp::MissingAreaTrigger,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    time_since_created_before_ms: Some(time_since_created_before_ms),
                    time_since_created_after_ms: Some(time_since_created_before_ms),
                    non_static_movement_would_run: false,
                    ai_update_would_run: false,
                    target_list_update_would_run: false,
                    remove_list: None,
                };
            };
            let Some(area_trigger) = record.area_trigger_mut() else {
                return AreaTriggerUpdateOutcomeLikeCpp {
                    area_trigger_guid,
                    elapsed_ms,
                    status: AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    time_since_created_before_ms: Some(time_since_created_before_ms),
                    time_since_created_after_ms: Some(time_since_created_before_ms),
                    non_static_movement_would_run: false,
                    ai_update_would_run: false,
                    target_list_update_would_run: false,
                    remove_list: None,
                };
            };
            let expired = area_trigger.update_time_and_duration(elapsed_ms);
            (
                expired,
                area_trigger.duration_ms(),
                area_trigger.time_since_created_ms(),
            )
        };

        if expired {
            let remove_list = self.add_object_to_remove_list_like_cpp(area_trigger_guid);
            AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::ExpiredRemoveQueued,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                time_since_created_before_ms: Some(time_since_created_before_ms),
                time_since_created_after_ms: Some(time_since_created_after_ms),
                non_static_movement_would_run,
                ai_update_would_run: false,
                target_list_update_would_run: false,
                remove_list: Some(remove_list),
            }
        } else {
            AreaTriggerUpdateOutcomeLikeCpp {
                area_trigger_guid,
                elapsed_ms,
                status: AreaTriggerUpdateStatusLikeCpp::Updated,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                time_since_created_before_ms: Some(time_since_created_before_ms),
                time_since_created_after_ms: Some(time_since_created_after_ms),
                non_static_movement_would_run,
                ai_update_would_run: true,
                target_list_update_would_run: true,
                remove_list: None,
            }
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `AreaTrigger` records only.
    ///
    /// This follows the same partial ObjectUpdater seam as DynamicObject: it
    /// snapshots canonical typed AreaTrigger GUIDs from `Map::map_objects`, then
    /// delegates every GUID to `update_area_trigger_like_cpp`. It does not visit
    /// nearby cells, players/sessions, other object families, SendObjectUpdates,
    /// scripts/AI real runtime, visibility, dynamic tree, packets, DB, or mirrors.
    pub fn update_area_triggers_like_cpp(
        &mut self,
        elapsed_ms: u32,
    ) -> AreaTriggersUpdateSummaryLikeCpp {
        let area_trigger_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::AreaTrigger
                    && record.area_trigger().is_some())
                .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = AreaTriggersUpdateSummaryLikeCpp::default();
        for guid in area_trigger_guids {
            summary.visited += 1;
            let outcome = self.update_area_trigger_like_cpp(guid, elapsed_ms);
            match outcome.status {
                AreaTriggerUpdateStatusLikeCpp::Updated => summary.updated += 1,
                AreaTriggerUpdateStatusLikeCpp::ExpiredRemoveQueued => {
                    summary.expired_remove_queued += 1;
                }
                AreaTriggerUpdateStatusLikeCpp::MissingAreaTrigger => {
                    summary.missing_or_stale += 1;
                }
                AreaTriggerUpdateStatusLikeCpp::NotAreaTrigger => summary.not_area_trigger += 1,
                AreaTriggerUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
    }

    /// Map-owned seam for C++ `Conversation::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `Conversation.cpp:67-80` runs `sScriptMgr->OnConversationUpdate` before
    ///   duration handling; on expiry it calls `Remove(); return;`, otherwise it
    ///   runs `WorldObject::Update(diff)`.
    /// - `Conversation.cpp:82-87` makes `Remove()` enqueue through
    ///   `AddObjectToRemoveList()` only when the object is in world.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects and explicitly instantiates `Conversation`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. Missing,
    /// non-Conversation, and not-in-world outcomes do not mutate, enqueue, or
    /// create fallback records. This helper represents script and WorldObject
    /// update callsites as booleans only; it does not execute scripts, fanout,
    /// visibility, ObjectAccessor/session mirrors, DB writes, or remove-list drain.
    pub fn update_conversation_like_cpp(
        &mut self,
        conversation_guid: ObjectGuid,
        elapsed_ms: u32,
    ) -> ConversationUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(conversation_guid) else {
            return ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::MissingConversation,
                duration_before_ms: None,
                duration_after_ms: None,
                script_update_would_run: false,
                world_update_would_run: false,
                remove_list: None,
            };
        };

        if record.kind() != AccessorObjectKind::Conversation {
            return ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::NotConversation,
                duration_before_ms: None,
                duration_after_ms: None,
                script_update_would_run: false,
                world_update_would_run: false,
                remove_list: None,
            };
        }

        let Some(conversation) = record.conversation() else {
            return ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::NotConversation,
                duration_before_ms: None,
                duration_after_ms: None,
                script_update_would_run: false,
                world_update_would_run: false,
                remove_list: None,
            };
        };

        let duration_before_ms = conversation.duration_ms();
        if !conversation.world().object().is_in_world() {
            return ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::NotInWorld,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_before_ms),
                script_update_would_run: false,
                world_update_would_run: false,
                remove_list: None,
            };
        }

        let (expired, duration_after_ms) = {
            let Some(record) = self.map_objects.get_mut(&conversation_guid) else {
                return ConversationUpdateOutcomeLikeCpp {
                    conversation_guid,
                    elapsed_ms,
                    status: ConversationUpdateStatusLikeCpp::MissingConversation,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    script_update_would_run: false,
                    world_update_would_run: false,
                    remove_list: None,
                };
            };
            let Some(conversation) = record.conversation_mut() else {
                return ConversationUpdateOutcomeLikeCpp {
                    conversation_guid,
                    elapsed_ms,
                    status: ConversationUpdateStatusLikeCpp::NotConversation,
                    duration_before_ms: Some(duration_before_ms),
                    duration_after_ms: Some(duration_before_ms),
                    script_update_would_run: false,
                    world_update_would_run: false,
                    remove_list: None,
                };
            };
            let expired = conversation.update_duration(elapsed_ms);
            (expired, conversation.duration_ms())
        };

        if expired {
            let remove_list = self.add_object_to_remove_list_like_cpp(conversation_guid);
            ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::ExpiredRemoveQueued,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                script_update_would_run: true,
                world_update_would_run: false,
                remove_list: Some(remove_list),
            }
        } else {
            ConversationUpdateOutcomeLikeCpp {
                conversation_guid,
                elapsed_ms,
                status: ConversationUpdateStatusLikeCpp::Updated,
                duration_before_ms: Some(duration_before_ms),
                duration_after_ms: Some(duration_after_ms),
                script_update_would_run: true,
                world_update_would_run: true,
                remove_list: None,
            }
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `Conversation` records only.
    ///
    /// This snapshots canonical typed Conversation GUIDs from `Map::map_objects`,
    /// then delegates every GUID to `update_conversation_like_cpp`. It does not
    /// model exact `TypeContainerVisitor` order/cell traversal, players/sessions,
    /// other object families, `SendObjectUpdates`, real scripts, visibility,
    /// packets, DB, ObjectAccessor/session mirrors, or remove-list drain.
    pub fn update_conversations_like_cpp(
        &mut self,
        elapsed_ms: u32,
    ) -> ConversationsUpdateSummaryLikeCpp {
        let conversation_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::Conversation
                    && record.conversation().is_some())
                .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = ConversationsUpdateSummaryLikeCpp::default();
        for guid in conversation_guids {
            summary.visited += 1;
            let outcome = self.update_conversation_like_cpp(guid, elapsed_ms);
            match outcome.status {
                ConversationUpdateStatusLikeCpp::Updated => summary.updated += 1,
                ConversationUpdateStatusLikeCpp::ExpiredRemoveQueued => {
                    summary.expired_remove_queued += 1;
                }
                ConversationUpdateStatusLikeCpp::MissingConversation => {
                    summary.missing_or_stale += 1;
                }
                ConversationUpdateStatusLikeCpp::NotConversation => summary.not_conversation += 1,
                ConversationUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
    }

    /// Map-owned seam for C++ `SceneObject::Update` under `ObjectUpdater`.
    ///
    /// C++ anchors:
    /// - `SceneObject.cpp:58-71` runs `WorldObject::Update(diff)` and removes the
    ///   SceneObject when `ShouldBeRemoved()` is true.
    /// - `SceneObject.cpp:73-90` makes `Remove()` enqueue through
    ///   `AddObjectToRemoveList()` only when in world, and `ShouldBeRemoved()`
    ///   depends on `ObjectAccessor::GetUnit(owner)` plus optional Aura lookup by
    ///   spell/cast id.
    /// - `GridNotifiers.cpp:258-264,296-301` calls `Update(i_timeDiff)` only for
    ///   in-world objects and explicitly instantiates `SceneObjectMapType`.
    ///
    /// Ownership: source-of-truth is canonical `Map::map_objects`. ObjectAccessor
    /// Unit resolution and Aura lookup are represented by explicit caller-supplied
    /// booleans; this helper does not scan maps, create fallback records, fan out,
    /// send packets, write session/ObjectAccessor mirrors, or drain remove-list.
    pub fn update_scene_object_like_cpp(
        &mut self,
        scene_object_guid: ObjectGuid,
        elapsed_ms: u32,
        context: SceneObjectUpdateContextLikeCpp,
    ) -> SceneObjectUpdateOutcomeLikeCpp {
        let Some(record) = self.map_object_record(scene_object_guid) else {
            return SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::MissingSceneObject,
                owner_guid: None,
                created_by_spell_cast: None,
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: false,
                should_be_removed: false,
                remove_list: None,
            };
        };

        if record.kind() != AccessorObjectKind::SceneObject {
            return SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::NotSceneObject,
                owner_guid: None,
                created_by_spell_cast: None,
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: false,
                should_be_removed: false,
                remove_list: None,
            };
        }

        let Some(scene_object) = record.scene_object() else {
            return SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::NotSceneObject,
                owner_guid: None,
                created_by_spell_cast: None,
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: false,
                should_be_removed: false,
                remove_list: None,
            };
        };

        let owner_guid = scene_object.owner_guid();
        let created_by_spell_cast = scene_object.created_by_spell_cast();
        if !scene_object.world().object().is_in_world() {
            return SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::NotInWorld,
                owner_guid: Some(owner_guid),
                created_by_spell_cast: Some(created_by_spell_cast),
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: false,
                should_be_removed: false,
                remove_list: None,
            };
        }

        let should_be_removed =
            scene_object.should_be_removed(context.creator_exists, context.linked_aura_exists);

        if should_be_removed {
            let remove_list = self.add_object_to_remove_list_like_cpp(scene_object_guid);
            SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::RemoveQueued,
                owner_guid: Some(owner_guid),
                created_by_spell_cast: Some(created_by_spell_cast),
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: true,
                should_be_removed,
                remove_list: Some(remove_list),
            }
        } else {
            SceneObjectUpdateOutcomeLikeCpp {
                scene_object_guid,
                elapsed_ms,
                status: SceneObjectUpdateStatusLikeCpp::Updated,
                owner_guid: Some(owner_guid),
                created_by_spell_cast: Some(created_by_spell_cast),
                creator_exists: context.creator_exists,
                linked_aura_exists: context.linked_aura_exists,
                world_update_would_run: true,
                should_be_removed,
                remove_list: None,
            }
        }
    }

    /// Bounded map-owned live visitation seam for C++ `Map::Update` consuming
    /// `Trinity::ObjectUpdater` for `SceneObject` records only.
    ///
    /// This snapshots canonical typed SceneObject GUIDs from `Map::map_objects`,
    /// resolves the explicit represented ObjectAccessor/Aura context before the
    /// per-object helper, and never visits generic/untyped SceneObject records.
    pub fn update_scene_objects_like_cpp<F>(
        &mut self,
        elapsed_ms: u32,
        mut context_resolver: F,
    ) -> SceneObjectsUpdateSummaryLikeCpp
    where
        F: FnMut(ObjectGuid, &SceneObject) -> SceneObjectUpdateContextLikeCpp,
    {
        let scene_object_guids = self
            .map_objects
            .iter()
            .filter_map(|(guid, record)| {
                (record.kind() == AccessorObjectKind::SceneObject
                    && record.scene_object().is_some())
                .then_some(*guid)
            })
            .collect::<Vec<_>>();

        let mut summary = SceneObjectsUpdateSummaryLikeCpp::default();
        for guid in scene_object_guids {
            summary.visited += 1;
            let Some(context) = self
                .map_object_record(guid)
                .and_then(MapObjectRecord::scene_object)
                .map(|scene_object| context_resolver(guid, scene_object))
            else {
                let outcome = self.update_scene_object_like_cpp(
                    guid,
                    elapsed_ms,
                    SceneObjectUpdateContextLikeCpp::default(),
                );
                match outcome.status {
                    SceneObjectUpdateStatusLikeCpp::MissingSceneObject => {
                        summary.missing_or_stale += 1;
                    }
                    SceneObjectUpdateStatusLikeCpp::NotSceneObject => summary.not_scene_object += 1,
                    SceneObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
                    SceneObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                    SceneObjectUpdateStatusLikeCpp::RemoveQueued => summary.remove_queued += 1,
                }
                continue;
            };

            let outcome = self.update_scene_object_like_cpp(guid, elapsed_ms, context);
            match outcome.status {
                SceneObjectUpdateStatusLikeCpp::Updated => summary.updated += 1,
                SceneObjectUpdateStatusLikeCpp::RemoveQueued => summary.remove_queued += 1,
                SceneObjectUpdateStatusLikeCpp::MissingSceneObject => {
                    summary.missing_or_stale += 1;
                }
                SceneObjectUpdateStatusLikeCpp::NotSceneObject => summary.not_scene_object += 1,
                SceneObjectUpdateStatusLikeCpp::NotInWorld => summary.not_in_world += 1,
            }
        }

        summary
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
            if Self::gameobject_is_spawned_like_cpp(gameobject) {
                spawned_guid = Some(guid);
                break;
            }
        }

        spawned_guid
            .or(fallback_guid)
            .and_then(|guid| self.map_object_record(guid)?.game_object())
    }

    fn gameobject_is_spawned_like_cpp(gameobject: &GameObject) -> bool {
        gameobject.respawn_delay_time() == 0
            || (gameobject.respawn_time() > 0 && !gameobject.spawned_by_default())
            || (gameobject.respawn_time() == 0 && gameobject.spawned_by_default())
    }

    pub fn get_area_trigger_by_spawn_id_like_cpp(&self, spawn_id: SpawnId) -> Option<&AreaTrigger> {
        self.area_trigger_spawn_id_store_guids_like_cpp(spawn_id)
            .into_iter()
            .find_map(|guid| self.map_object_record(guid)?.area_trigger())
    }

    pub(super) fn map_record_is_unit_like_gameobject_owner_like_cpp(
        record: &MapObjectRecord,
    ) -> bool {
        matches!(
            record.kind(),
            AccessorObjectKind::Player | AccessorObjectKind::Creature | AccessorObjectKind::Pet
        ) && (record.player().is_some() || record.creature().is_some() || record.pet().is_some())
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
            .is_some_and(Self::map_record_is_unit_like_gameobject_owner_like_cpp);
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
            if let Some(record) = self.map_objects.get_mut(&owner_guid) {
                if let Some(owner) = Self::map_record_unit_mut_like_cpp(record) {
                    owner
                        .subsystems_mut()
                        .control
                        .register_owned_gameobject_like_cpp(guid);
                    registered_owned_gameobject = true;
                }
            }

            if registered_owned_gameobject {
                if let Some(game_object) = self
                    .map_objects
                    .get_mut(&guid)
                    .and_then(MapObjectRecord::game_object_mut)
                {
                    game_object.set_owner_guid_like_cpp(owner_guid);
                    owner_guid_after = game_object.owner_guid();
                }

                creature_ai_callback_represented = self
                    .map_objects
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
                .map_objects
                .get_mut(&owner_guid)
                .and_then(Self::map_record_unit_mut_like_cpp)
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
            Self::map_record_is_unit_like_gameobject_owner_like_cpp(summoner_record);
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
            if let Some(record) = self.map_objects.get_mut(&summoner_guid) {
                if let Some(owner) = Self::map_record_unit_mut_like_cpp(record) {
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
            .is_some_and(Self::map_record_is_unit_like_gameobject_owner_like_cpp);
        let slot_guid_before = self
            .map_object_record(owner_guid)
            .and_then(Self::map_record_unit_like_cpp)
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
                    .map_objects
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
                        .map_objects
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
                .map_objects
                .get_mut(&owner_guid)
                .and_then(Self::map_record_unit_mut_like_cpp)
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
            .and_then(Self::map_record_unit_like_cpp)
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
        if let Some(record) = self.map_objects.get_mut(&owner_guid) {
            if let Some(owner) = Self::map_record_unit_mut_like_cpp(record) {
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
                .map_objects
                .get_mut(&owner_guid)
                .and_then(Self::map_record_unit_mut_like_cpp)
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
            .and_then(MapObjectRecord::game_object)
            .map(|game_object| game_object.data().type_id as u32)?;

        if let Some(game_object) = self
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
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
            .map_objects
            .get_mut(&guid)
            .and_then(MapObjectRecord::game_object_mut)
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
        let record = self.map_objects.get_mut(&guid)?;
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
        self.map_objects.values().find_map(|record| {
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
        self.map_objects.get(&guid)?.transport()
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
