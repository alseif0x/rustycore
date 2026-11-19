// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Map construction, lifecycle and runtime operations.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    #[allow(clippy::too_many_arguments)]
    pub fn with_hooks(
        map_id: u32,
        instance_id: u32,
        spawn_mode: Difficulty,
        grid_expiry_ms: i64,
        grid_unload: bool,
        visible_distance: f32,
        terrain: Terrain,
        lifecycle: Lifecycle,
    ) -> Self {
        Self {
            map_id,
            instance_id,
            spawn_mode,
            grid_expiry_ms,
            grid_unload,
            visible_distance,
            grids: std::iter::repeat_with(|| None)
                .take(GRID_SLOT_COUNT)
                .collect(),
            terrain,
            lifecycle,
            active_cells: HashSet::new(),
            active_non_players_like_cpp: HashSet::new(),
            personal_phase_tracker: MultiPersonalPhaseTracker::default(),
            spawn_group_state: SpawnGroupRuntimeState::new(),
            respawn_store: RespawnStoreLikeCpp::new(),
            creature_respawn_queue: Default::default(),
            pool_data: SpawnedPoolDataLikeCpp::new(),
            grid_state_unloaded: false,
            corpse_data_loaded_like_cpp: false,
            creatures_by_spawn_id: HashMap::new(),
            gameobjects_by_spawn_id: HashMap::new(),
            area_triggers_by_spawn_id: HashMap::new(),
            entity_world: EntityWorld::default(),
            map_reference_order_like_cpp: Vec::new(),
            creature_group_holder_like_cpp: HashMap::new(),
            dynamic_tree_model_keys_like_cpp: HashSet::new(),
            dynamic_tree_rebalance_timer_remaining_ms_like_cpp:
                DYNAMIC_MAP_TREE_CHECK_PERIOD_MS_LIKE_CPP,
            dynamic_tree_unbalanced_times_like_cpp: 0,
            objects_to_remove: HashSet::new(),
            pending_object_visibility_destroy_recipients_like_cpp: Vec::new(),
            objects_to_switch: HashMap::new(),
            far_spell_callbacks_like_cpp: VecDeque::new(),
            represented_far_spell_callback_execution_log_like_cpp: Vec::new(),
            creatures_to_move: Vec::new(),
            gameobjects_to_move: Vec::new(),
            dynamic_objects_to_move: Vec::new(),
            area_triggers_to_move: Vec::new(),
            creature_move_states: HashMap::new(),
            gameobject_move_states: HashMap::new(),
            dynamic_object_move_states: HashMap::new(),
            area_trigger_move_states: HashMap::new(),
            creature_move_lock: false,
            gameobject_move_lock: false,
            dynamic_object_move_lock: false,
            area_trigger_move_lock: false,
            script_schedule_like_cpp: BTreeMap::new(),
            script_schedule_lock_like_cpp: false,
            represented_executed_script_actions_like_cpp: Vec::new(),
            zone_dynamic_info_like_cpp: BTreeMap::new(),
            weather_update_timer_current_ms_like_cpp: 0,
            weather_update_timer_interval_ms_like_cpp: WEATHER_UPDATE_INTERVAL_MS_LIKE_CPP,
            guid_generators: HashMap::new(),
            creature_level_rng_like_cpp: StdRng::from_entropy(),
        }
    }

    pub const fn map_id(&self) -> u32 {
        self.map_id
    }

    pub const fn instance_id(&self) -> u32 {
        self.instance_id
    }

    pub const fn spawn_mode(&self) -> Difficulty {
        self.spawn_mode
    }

    /// Mirrors TrinityCore `urand(min, max)` (`Random.cpp:35-47`): assert
    /// `max >= min` and sample an inclusive integer range. Ownership remains on
    /// `Map` so loaded-grid runtime consumers advance one canonical RNG stream.
    pub fn urand_inclusive_like_cpp(&mut self, min: u32, max: u32) -> u32 {
        assert!(max >= min, "C++ urand requires max >= min");
        self.creature_level_rng_like_cpp.gen_range(min..=max)
    }

    /// Mirrors the floating-point random draw used by C++ weighted model selection.
    /// The caller owns the exact semantic range; this helper only keeps RNG ownership
    /// on the map, matching the loaded-grid runtime path.
    pub fn frand_exclusive_like_cpp(&mut self, min: f32, max: f32) -> f32 {
        assert!(max > min, "C++ frand-like draw requires max > min");
        self.creature_level_rng_like_cpp.gen_range(min..max)
    }

    /// Mirrors `Creature::SelectLevel` for DB/template min/max rows: fixed rows
    /// use `MinLevel` without consuming RNG; variable rows call inclusive `urand`.
    pub fn select_creature_level_like_cpp(&mut self, min_level: u8, max_level: u8) -> u8 {
        if min_level == max_level {
            return min_level;
        }
        let selected = self.urand_inclusive_like_cpp(u32::from(min_level), u32::from(max_level));
        selected as u8
    }

    #[cfg(test)]
    pub(in crate::map) fn seed_creature_level_rng_for_tests_like_cpp(&mut self, seed: u64) {
        self.creature_level_rng_like_cpp = StdRng::seed_from_u64(seed);
    }

    #[cfg(test)]
    pub(crate) fn set_dynamic_tree_model_count_for_tests_like_cpp(&mut self, model_count: u32) {
        self.dynamic_tree_model_keys_like_cpp.clear();
        for counter in 0..model_count {
            self.dynamic_tree_model_keys_like_cpp
                .insert(RepresentedGameObjectModelKeyLikeCpp {
                    owner_guid: ObjectGuid::create_player(1, i64::from(counter) + 1),
                });
        }
    }

    #[cfg(test)]
    pub(crate) fn mark_dynamic_tree_unbalanced_for_tests_like_cpp(&mut self, times: u32) {
        self.dynamic_tree_unbalanced_times_like_cpp = times;
    }

    pub fn lifecycle(&self) -> &Lifecycle {
        &self.lifecycle
    }

    pub fn far_spell_callbacks_count_like_cpp(&self) -> usize {
        self.far_spell_callbacks_like_cpp.len()
    }

    pub fn represented_far_spell_callback_execution_log_like_cpp(&self) -> &[u64] {
        &self.represented_far_spell_callback_execution_log_like_cpp
    }

    /// C++ `Map::DelayedUpdate` first block: drain `_farSpellCallbacks` FIFO before
    /// `RemoveAllObjectsInRemoveList()` (`Map.cpp:2519-2530`).
    ///
    /// Limits: this is a bounded represented seam only. It executes no real Spell,
    /// Aura, caster/ObjectAccessor lookup, session fanout, packet, script, AI, or
    /// arbitrary callback side effects. `QueueObjectRemove` is the minimal map-owned
    /// mutation used to prove same-tick ordering before the remove-list drain.
    pub fn drain_far_spell_callbacks_like_cpp(&mut self) -> FarSpellCallbackDrainSummaryLikeCpp {
        let queued_before = self.far_spell_callbacks_like_cpp.len();
        let mut summary = FarSpellCallbackDrainSummaryLikeCpp {
            queued_before,
            ..Default::default()
        };

        while let Some(callback) = self.far_spell_callbacks_like_cpp.pop_front() {
            summary.processed += 1;
            self.represented_far_spell_callback_execution_log_like_cpp
                .push(callback.id);
            match callback.action {
                RepresentedFarSpellCallbackActionLikeCpp::RecordExecution => {
                    summary.record_only += 1;
                }
                RepresentedFarSpellCallbackActionLikeCpp::QueueObjectRemove { guid } => {
                    summary.remove_queue_attempted += 1;
                    let outcome = self.add_object_to_remove_list_like_cpp(guid);
                    if outcome.queued {
                        summary.remove_queued += 1;
                    }
                    if outcome.missing_or_stale {
                        summary.remove_missing_or_stale += 1;
                    }
                    if outcome.duplicate {
                        summary.remove_duplicates += 1;
                    }
                    if outcome.unsupported_kind.is_some() {
                        summary.unsupported_remove_kinds += 1;
                    }
                }
            }
        }

        summary.queued_after = self.far_spell_callbacks_like_cpp.len();
        summary
    }

    pub fn represented_zone_dynamic_info_like_cpp(
        &self,
        zone_id: u32,
    ) -> Option<&RepresentedZoneDynamicInfoLikeCpp> {
        self.zone_dynamic_info_like_cpp.get(&zone_id)
    }

    /// C++ `WorldObject::SetWorldObject(bool)` facade owned by `Map` over the
    /// canonical `MapObjectRecord` store.
    ///
    /// C++ anchors:
    /// - `Object.cpp:910-916` returns when `!IsInWorld()`, otherwise delegates
    ///   to the owning map's `AddObjectToSwitchList(this, on)`.
    /// - `Map.cpp:2557-2572` keeps Unit validation/queue duplicate semantics in
    ///   `add_object_to_switch_list_like_cpp`; this facade does not move grid
    ///   containers or mutate temporary world-object state.
    pub fn set_world_object_like_cpp(
        &mut self,
        guid: ObjectGuid,
        on: bool,
    ) -> SetWorldObjectOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return SetWorldObjectOutcomeLikeCpp {
                guid,
                on,
                status: SetWorldObjectStatusLikeCpp::MissingOrStale,
            };
        };

        if !record.object().object().is_in_world() {
            return SetWorldObjectOutcomeLikeCpp {
                guid,
                on,
                status: SetWorldObjectStatusLikeCpp::NotInWorld,
            };
        }

        let delegated = self.add_object_to_switch_list_like_cpp(guid, on);
        SetWorldObjectOutcomeLikeCpp {
            guid,
            on,
            status: SetWorldObjectStatusLikeCpp::Delegated(delegated.status),
        }
    }

    /// Applies the request emitted by C++-shaped Unit shared-vision transitions
    /// to this map-owned `WorldObject::SetWorldObject(bool)` facade.
    ///
    /// C++ anchors:
    /// - `Unit.cpp:6489-6509` emits `SetWorldObject(true/false)` only at the
    ///   empty/non-empty shared-vision boundary.
    /// - `Object.cpp:910-916` keeps the in-world guard before map delegation.
    /// - `Map.cpp:2557-2572` owns switch-list validation/queue semantics, while
    ///   `Map.cpp:2574-2594` drains later.
    ///
    /// Ownership stays one-way: Unit emits a DTO, the map owner applies it over
    /// canonical `entity_world`/`objects_to_switch`; this method does not run the
    /// drain, rebuild missing records, fan out visibility, or wire sessions.
    pub fn apply_unit_shared_vision_set_world_object_request_like_cpp(
        &mut self,
        request: UnitSharedVisionSetWorldObjectRequestLikeCpp,
    ) -> SetWorldObjectOutcomeLikeCpp {
        self.set_world_object_like_cpp(request.unit_guid, request.on)
    }

    /// Bounded map-owned seam for the Unit-target shared-vision branch of C++
    /// `Player::SetViewpoint(WorldObject* target, bool apply)`.
    ///
    /// C++ anchors:
    /// - `Player.cpp:25344-25387` owns FarsightObject guards/mutations,
    ///   requests `UpdateVisibilityOf`, calls `Unit::Add/RemovePlayerToVision`
    ///   only for Unit targets that are not `GetVehicleBase()`, and requests
    ///   `SetSeer`.
    /// - `Unit.cpp:6489-6509` toggles Unit active state and emits
    ///   `SetWorldObject(true/false)` only at shared-vision empty boundaries.
    /// - `Object.cpp:910-916` / `Map.cpp:2557-2594` keep the SetWorldObject
    ///   map-owned switch-list enqueue/drain split.
    ///
    /// Scope: this helper mutates only canonical `Map::entity_world` typed Player
    /// and typed Creature/Pet Unit targets already in this same map. It consumes
    /// the Unit-emitted SetWorldObject DTO immediately through the Map facade, but
    /// does not drain queues, fan out visibility, implement `SetSeer`, access
    /// ObjectAccessor/session mirrors, create records, send packets, or touch DB.
    pub fn apply_player_set_viewpoint_unit_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        target_guid: ObjectGuid,
        apply: bool,
        vehicle_base_guid: Option<ObjectGuid>,
    ) -> PlayerSetViewpointOutcomeLikeCpp {
        let Some(player) = self.get_typed_player(player_guid) else {
            return player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::MissingPlayer,
                None,
                false,
                false,
            );
        };

        let current_farsight = player.active_data().farsight_object;
        if apply {
            if !current_farsight.is_empty() {
                return player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint,
                    None,
                    false,
                    false,
                );
            }
        } else if current_farsight != target_guid {
            return player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
                None,
                false,
                false,
            );
        }

        let Some(target_record) = self.map_object_record(target_guid) else {
            return player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::MissingTarget,
                None,
                false,
                false,
            );
        };
        if !matches!(
            target_record.kind(),
            AccessorObjectKind::Creature | AccessorObjectKind::Pet
        ) {
            return player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::TargetNotUnit,
                None,
                false,
                false,
            );
        }

        let vehicle_base_skip = vehicle_base_guid == Some(target_guid);
        if !vehicle_base_skip {
            let Some(target_record) = self.entity_world.get_mut(&target_guid) else {
                return player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::MissingTarget,
                    None,
                    false,
                    false,
                );
            };
            if map_record_unit_mut_like_cpp(target_record).is_none() {
                return player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::TargetNotUnit,
                    None,
                    false,
                    false,
                );
            }
        }

        let Some(player) = self.get_typed_player_mut(player_guid) else {
            return player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                PlayerSetViewpointStatusLikeCpp::MissingPlayer,
                None,
                false,
                false,
            );
        };
        player.set_farsight_object_like_cpp(if apply {
            target_guid
        } else {
            ObjectGuid::EMPTY
        });

        if vehicle_base_skip {
            return player_set_viewpoint_outcome_like_cpp(
                player_guid,
                target_guid,
                apply,
                if apply {
                    PlayerSetViewpointStatusLikeCpp::Applied
                } else {
                    PlayerSetViewpointStatusLikeCpp::Removed
                },
                None,
                apply,
                true,
            );
        }

        let request = {
            let Some(target_record) = self.entity_world.get_mut(&target_guid) else {
                return player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::MissingTarget,
                    None,
                    false,
                    false,
                );
            };
            let Some(target_unit) = map_record_unit_mut_like_cpp(target_record) else {
                return player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    target_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::TargetNotUnit,
                    None,
                    false,
                    false,
                );
            };
            if apply {
                target_unit.add_player_to_vision_like_cpp(player_guid)
            } else {
                target_unit.remove_player_from_vision_like_cpp(player_guid)
            }
            .set_world_object
        };
        let set_world_object = request.map(|request| {
            self.apply_unit_shared_vision_set_world_object_request_like_cpp(request)
        });

        player_set_viewpoint_outcome_like_cpp(
            player_guid,
            target_guid,
            apply,
            if apply {
                PlayerSetViewpointStatusLikeCpp::Applied
            } else {
                PlayerSetViewpointStatusLikeCpp::Removed
            },
            set_world_object,
            apply,
            true,
        )
    }

    /// Map-owned seam for C++ `Spell::EffectAddFarsight` ->
    /// `DynamicObject::CreateDynamicObject` -> `SetDuration` ->
    /// `SetCasterViewpoint`.
    ///
    /// C++ anchors:
    /// - `SpellEffects.cpp:2237-2261` runs only after HIT handling has selected
    ///   a Player caster, returns if the Player is not in world, creates
    ///   `DynamicObject(true)`, calls `CreateDynamicObject`, then sets duration
    ///   and caster viewpoint.
    /// - `DynamicObject.cpp:84-133` binds the object to the caster map, validates
    ///   the destination, creates a world-object GUID from map/spell/low guid,
    ///   inherits phase, sets entry/scale/update fields, marks world objects
    ///   active before AddToMap, and inserts through `Map::AddToMap`.
    /// - `DynamicObject.cpp:209-239` resolves the already-bound caster pointer for
    ///   `SetCasterViewpoint`; Rust represents that by `DynamicObject::bound_caster()`
    ///   and delegates to `apply_dynamic_object_caster_viewpoint_like_cpp`.
    ///
    /// Ownership: source-of-truth is this `Map::entity_world` for both the caster
    /// Player and the newly-created DynamicObject. Per #NEXT.R8.ENTITIES.428
    /// invariants, represented fallback paths validate all rejectable inputs before
    /// low-guid consumption so a missing/wrong caster or invalid destination leaves
    /// the Map seam unmutated; this is an explicitly bounded creation-seam guard even
    /// though C++ receives `guidlow` before `CreateDynamicObject` validates `pos`.
    /// This does not parse live Spell targets, create dummy records, register through
    /// ObjectAccessor, implement transport passenger offsets, UpdatePositionData,
    /// ZoneScript, aura/update lifecycle, real SetSeer/fanout, packets/session mirrors,
    /// DB, or spell handler wiring.
    #[allow(clippy::too_many_arguments)]
    pub fn create_farsight_dynamic_object_like_cpp(
        &mut self,
        caster_player_guid: ObjectGuid,
        spell_id: u32,
        spell_x_spell_visual_id: i32,
        dest: Position,
        radius: f32,
        duration_ms: i32,
        cast_time_ms: u64,
        realm_id: u16,
        server_id: u32,
    ) -> FarsightDynamicObjectCreateOutcomeLikeCpp {
        let early = |status| FarsightDynamicObjectCreateOutcomeLikeCpp {
            status,
            caster_player_guid,
            dynamic_object_guid: None,
            low_guid: None,
            add_to_map: None,
            caster_viewpoint: None,
        };

        let Some(caster_player) = self.get_typed_player(caster_player_guid) else {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::MissingCasterPlayer);
        };
        let caster_world = caster_player.unit().world();
        if !caster_world.object().is_in_world() {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::CasterNotInWorld);
        }
        if caster_world.map_id() != self.map_id || caster_world.instance_id() != self.instance_id {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::CasterWrongMap);
        }
        if !dest.is_valid_map_coord_like_cpp() {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::InvalidDestination);
        }
        if self.map_id > 0x1FFF {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::MapIdNotRepresentableInGuid);
        }
        let Ok(spell_id_i32) = i32::try_from(spell_id) else {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::SpellIdNotRepresentable);
        };
        let Ok(cast_time_ms_u32) = u32::try_from(cast_time_ms) else {
            return early(FarsightDynamicObjectCreateStatusLikeCpp::CastTimeNotRepresentable);
        };
        let inherited_phase_shift = caster_world.phase_shift().clone();
        let inherited_suppressed_phase_shift = caster_world.suppressed_phase_shift().clone();

        let low_guid = match self.generate_low_guid_like_cpp(HighGuid::DynamicObject) {
            Ok(low_guid) => low_guid,
            Err(error) => {
                return early(FarsightDynamicObjectCreateStatusLikeCpp::GuidSequenceError(
                    error,
                ));
            }
        };
        let dynamic_object_guid = ObjectGuid::create_world_object(
            HighGuid::DynamicObject,
            0,
            realm_id,
            self.map_id as u16,
            server_id,
            spell_id,
            low_guid,
        );

        let mut dynamic_object = DynamicObject::new(true);
        dynamic_object
            .world_mut()
            .object_mut()
            .create(dynamic_object_guid);
        if dynamic_object
            .world_mut()
            .set_map(self.map_id, self.instance_id)
            .is_err()
        {
            return FarsightDynamicObjectCreateOutcomeLikeCpp {
                status: FarsightDynamicObjectCreateStatusLikeCpp::DynamicObjectRecordError(
                    ObjectAccessorError::ObjectHasNoMap {
                        guid: dynamic_object_guid,
                    },
                ),
                caster_player_guid,
                dynamic_object_guid: Some(dynamic_object_guid),
                low_guid: Some(low_guid),
                add_to_map: None,
                caster_viewpoint: None,
            };
        }
        dynamic_object.world_mut().relocate(dest);
        *dynamic_object.world_mut().phase_shift_mut() = inherited_phase_shift;
        *dynamic_object.world_mut().suppressed_phase_shift_mut() = inherited_suppressed_phase_shift;
        dynamic_object.world_mut().object_mut().set_entry(spell_id);
        dynamic_object.world_mut().object_mut().set_scale(1.0);
        dynamic_object.set_caster_guid(caster_player_guid);
        dynamic_object.set_dynamic_object_type(DynamicObjectType::FarsightFocus);
        dynamic_object.set_spell_visual_id(spell_x_spell_visual_id);
        dynamic_object.set_spell_id(spell_id_i32);
        dynamic_object.set_radius(radius);
        dynamic_object.set_cast_time_ms(cast_time_ms_u32);
        dynamic_object.bind_to_caster(caster_player_guid);
        dynamic_object.set_duration(duration_ms);
        if dynamic_object.world().is_world_object() {
            dynamic_object.world_mut().set_active(true);
        }

        let record = match MapObjectRecord::new_dynamic_object(dynamic_object) {
            Ok(record) => record,
            Err(error) => {
                return FarsightDynamicObjectCreateOutcomeLikeCpp {
                    status: FarsightDynamicObjectCreateStatusLikeCpp::DynamicObjectRecordError(
                        error,
                    ),
                    caster_player_guid,
                    dynamic_object_guid: Some(dynamic_object_guid),
                    low_guid: Some(low_guid),
                    add_to_map: None,
                    caster_viewpoint: None,
                };
            }
        };
        let add_to_map = match self.add_map_object_record_to_map_like_cpp(record) {
            Ok(outcome) => outcome,
            Err(_error) => {
                return FarsightDynamicObjectCreateOutcomeLikeCpp {
                    status: FarsightDynamicObjectCreateStatusLikeCpp::AddToMapError,
                    caster_player_guid,
                    dynamic_object_guid: Some(dynamic_object_guid),
                    low_guid: Some(low_guid),
                    add_to_map: None,
                    caster_viewpoint: None,
                };
            }
        };
        let caster_viewpoint =
            self.apply_dynamic_object_caster_viewpoint_like_cpp(dynamic_object_guid, true);

        FarsightDynamicObjectCreateOutcomeLikeCpp {
            status: FarsightDynamicObjectCreateStatusLikeCpp::Created,
            caster_player_guid,
            dynamic_object_guid: Some(dynamic_object_guid),
            low_guid: Some(low_guid),
            add_to_map: Some(add_to_map),
            caster_viewpoint: Some(caster_viewpoint),
        }
    }

    /// Bounded map-owned caller-consumption seam for C++
    /// `DynamicObject::SetCasterViewpoint` / `RemoveCasterViewpoint`.
    ///
    /// C++ anchors:
    /// - `DynamicObject.cpp:209-225` resolves the caster from the DynamicObject's
    ///   `_caster`, calls `Player::SetViewpoint(this, apply)` only when `_caster`
    ///   is a Player, and then toggles `_isViewpoint` without checking the Player
    ///   helper's early-return result.
    /// - `DynamicObject.cpp:233-239` represents `_caster` as a previously bound
    ///   same-map Unit pointer; this helper consumes `DynamicObject::bound_caster()`
    ///   as that represented pointer equivalent and never falls back to the raw
    ///   caster GUID field or to a caller-provided Player.
    /// - `Player.cpp:25344-25387` owns FarsightObject guards/mutations,
    ///   `UpdateVisibilityOf` on apply, and `SetSeer`; DynamicObject targets do
    ///   not run the Unit shared-vision / SetWorldObject branch.
    ///
    /// Ownership: source-of-truth is canonical `Map::entity_world`. The helper
    /// first validates the typed DynamicObject record, then resolves the Player
    /// from `DynamicObject::bound_caster()` before any Player mutation. It does
    /// not create records, silently fall back from `caster_guid`, drain
    /// switch/remove lists, fan out visibility, implement full SetSeer, write
    /// session/ObjectAccessor mirrors, send packets, or touch DB.
    pub fn apply_dynamic_object_caster_viewpoint_like_cpp(
        &mut self,
        dynamic_object_guid: ObjectGuid,
        apply: bool,
    ) -> DynamicObjectCasterViewpointOutcomeLikeCpp {
        let outcome =
            |player_guid, status, player_set_viewpoint, dynamic_object_viewpoint_toggled| {
                DynamicObjectCasterViewpointOutcomeLikeCpp {
                    player_guid,
                    dynamic_object_guid,
                    apply,
                    status,
                    player_set_viewpoint,
                    dynamic_object_viewpoint_toggled,
                }
            };
        let player_outcome = |player_guid, status| {
            player_set_viewpoint_outcome_like_cpp(
                player_guid,
                dynamic_object_guid,
                apply,
                status,
                None,
                false,
                false,
            )
        };

        let Some(dynamic_object) = self
            .map_object_record(dynamic_object_guid)
            .and_then(MapObjectRecord::dynamic_object)
        else {
            return outcome(
                ObjectGuid::EMPTY,
                DynamicObjectCasterViewpointStatusLikeCpp::MissingDynamicObject,
                player_outcome(
                    ObjectGuid::EMPTY,
                    PlayerSetViewpointStatusLikeCpp::MissingTarget,
                ),
                false,
            );
        };

        let Some(player_guid) = dynamic_object.bound_caster() else {
            return outcome(
                ObjectGuid::EMPTY,
                DynamicObjectCasterViewpointStatusLikeCpp::MissingCaster,
                player_outcome(
                    ObjectGuid::EMPTY,
                    PlayerSetViewpointStatusLikeCpp::MissingPlayer,
                ),
                false,
            );
        };

        let Some(player) = self.get_typed_player(player_guid) else {
            return outcome(
                player_guid,
                DynamicObjectCasterViewpointStatusLikeCpp::CasterNotPlayer,
                player_outcome(player_guid, PlayerSetViewpointStatusLikeCpp::MissingPlayer),
                false,
            );
        };
        let current_farsight = player.active_data().farsight_object;

        let player_set_viewpoint = if apply {
            if current_farsight.is_empty() {
                if let Some(player) = self.get_typed_player_mut(player_guid) {
                    player.set_farsight_object_like_cpp(dynamic_object_guid);
                    player_set_viewpoint_outcome_like_cpp(
                        player_guid,
                        dynamic_object_guid,
                        apply,
                        PlayerSetViewpointStatusLikeCpp::Applied,
                        None,
                        true,
                        true,
                    )
                } else {
                    player_outcome(player_guid, PlayerSetViewpointStatusLikeCpp::MissingPlayer)
                }
            } else {
                player_outcome(
                    player_guid,
                    PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint,
                )
            }
        } else if current_farsight == dynamic_object_guid {
            if let Some(player) = self.get_typed_player_mut(player_guid) {
                player.set_farsight_object_like_cpp(ObjectGuid::EMPTY);
                player_set_viewpoint_outcome_like_cpp(
                    player_guid,
                    dynamic_object_guid,
                    apply,
                    PlayerSetViewpointStatusLikeCpp::Removed,
                    None,
                    false,
                    true,
                )
            } else {
                player_outcome(player_guid, PlayerSetViewpointStatusLikeCpp::MissingPlayer)
            }
        } else {
            player_outcome(
                player_guid,
                PlayerSetViewpointStatusLikeCpp::ViewpointMismatch,
            )
        };

        let mut dynamic_object_viewpoint_toggled = false;
        if let Some(record) = self.entity_world.get_mut(&dynamic_object_guid) {
            if let Some(dynamic_object) = record.dynamic_object_mut() {
                if apply {
                    dynamic_object.set_caster_viewpoint();
                } else {
                    dynamic_object.remove_caster_viewpoint();
                }
                dynamic_object_viewpoint_toggled = true;
            }
        }

        outcome(
            player_guid,
            DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved,
            player_set_viewpoint,
            dynamic_object_viewpoint_toggled,
        )
    }

    /// C++ `Map::DespawnAll` represented over map-local by-spawn indexes.
    ///
    /// C++ anchors:
    /// - `Map.cpp:2034-2055` snapshots Creature/GameObject by-spawn stores and
    ///   queues each object through `AddObjectToRemoveList`.
    /// - `Map.cpp:2547-2555` marks each queued object destroyed and runs cleanup
    ///   before insertion into the map-owned remove-list.
    /// - `Map.cpp:2574-2646` later physically drains the list.
    pub fn despawn_all_by_spawn_id_like_cpp(
        &mut self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> DespawnAllBySpawnIdOutcomeLikeCpp {
        let mut outcome = DespawnAllBySpawnIdOutcomeLikeCpp {
            object_type,
            spawn_id,
            queued: 0,
            removed: 0,
            duplicates: 0,
            stale_index_entries: 0,
            remove_errors: 0,
            unsupported_live_despawn_type: 0,
        };

        let guids = match object_type {
            SpawnObjectType::Creature => self.creature_spawn_id_store_guids_like_cpp(spawn_id),
            SpawnObjectType::GameObject => self.gameobject_spawn_id_store_guids_like_cpp(spawn_id),
            SpawnObjectType::AreaTrigger => {
                outcome.unsupported_live_despawn_type = 1;
                return outcome;
            }
        };

        for guid in guids {
            let still_matches = match object_type {
                SpawnObjectType::Creature => self
                    .map_object_record(guid)
                    .and_then(MapObjectRecord::creature)
                    .is_some_and(|creature| creature.spawn_id() == spawn_id),
                SpawnObjectType::GameObject => self
                    .map_object_record(guid)
                    .and_then(MapObjectRecord::game_object)
                    .is_some_and(|gameobject| gameobject.spawn_id() == spawn_id),
                SpawnObjectType::AreaTrigger => false,
            };
            if !still_matches {
                outcome.stale_index_entries += 1;
                continue;
            }

            let queued = self.add_object_to_remove_list_like_cpp(guid);
            if queued.missing_or_stale {
                outcome.stale_index_entries += 1;
            } else if queued.unsupported_kind.is_some() {
                outcome.unsupported_live_despawn_type += 1;
            } else if queued.duplicate {
                outcome.duplicates += 1;
            } else if queued.queued {
                outcome.queued += 1;
            }
        }

        outcome
    }
}
