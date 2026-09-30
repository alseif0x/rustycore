use super::*;
use wow_entities::AccessorObjectKind;

pub(super) struct ObjectUpdateContinuation {
    diff_ms: u32,
    nearby_object_plan: Option<crate::map::ObjectUpdatePlan>,
    now_secs: i64,
    actor_workset: Option<Vec<(ObjectGuid, crate::map::CreatureActorWitness)>>,
}

impl ObjectUpdateContinuation {
    pub(super) const fn effective_diff_ms(&self) -> u32 {
        self.diff_ms
    }

    /// Start the actor family lazily. The adjacent compatibility wrapper never
    /// reads this workset. NearbyCells retains its admitted plan's exact order;
    /// WholeTypedStores uses the existing loaded-cell Creature selection at the
    /// actual actor start, not a global or legacy actor enumeration.
    pub(super) fn actor_workset(
        &mut self,
        map: &Map,
    ) -> &[(ObjectGuid, crate::map::CreatureActorWitness)] {
        if self.actor_workset.is_none() {
            let guids = match &self.nearby_object_plan {
                Some(plan) => plan.update_guids.iter().copied().collect(),
                None => map.admitted_creature_guids_like_cpp(),
            };
            self.actor_workset = Some(guids.into_iter().filter_map(|guid| {
                map.creature_actor_witness(guid).map(|witness| (guid, witness))
            }).collect());
        }
        self.actor_workset.as_deref().expect("actor workset was initialized")
    }
}

#[cfg(test)]
#[path = "actor_tick_access/selection_tests.rs"]
mod actor_selection_tests;

impl ManagedMap {
    pub(super) fn update_after_sessions_with_creature_owner_and_selection_like_cpp<L>(
        &mut self,
        diff_ms: u32,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        load_record: Option<&mut L>,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
        object_update_selection: MapObjectUpdateSelectionLikeCpp,
    ) where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let continuation = self.prepare_object_update(diff_ms, object_update_selection);
        self.run_creature_phase(&continuation, creature_update_owner);
        self.finish_object_update(continuation, pool_update, load_record);
    }

    pub(super) fn prepare_object_update(
        &mut self,
        diff_ms: u32,
        object_update_selection: MapObjectUpdateSelectionLikeCpp,
    ) -> ObjectUpdateContinuation {
        let nearby_object_plan = match object_update_selection {
            MapObjectUpdateSelectionLikeCpp::WholeTypedStores => None,
            MapObjectUpdateSelectionLikeCpp::NearbyCells => Some(
                self.runtime
                    .map
                    .object_update_plan_for_current_tick_like_cpp(diff_ms),
            ),
        };

        self.last_dynamic_objects_update_summary = if let Some(plan) = &nearby_object_plan {
            let guids = plan
                .update_guids
                .iter()
                .copied()
                .filter(|guid| {
                    self.runtime
                        .map
                        .map_object_record(*guid)
                        .is_some_and(|record| {
                            record.kind() == AccessorObjectKind::DynamicObject
                                && record.dynamic_object().is_some()
                        })
                })
                .collect::<Vec<_>>();
            self.runtime
                .map
                .update_dynamic_objects_for_guids_like_cpp(guids, diff_ms)
        } else {
            self.runtime.map.update_dynamic_objects_like_cpp(diff_ms)
        };
        let now_secs = game_time_now_secs_i64();
        ObjectUpdateContinuation {
            diff_ms,
            nearby_object_plan,
            now_secs,
            actor_workset: None,
        }
    }

    pub(super) fn run_creature_phase(
        &mut self,
        continuation: &ObjectUpdateContinuation,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
    ) {
        let diff_ms = continuation.diff_ms;
        let nearby_object_plan = &continuation.nearby_object_plan;
        let now_secs = continuation.now_secs;
        self.last_creature_update_owner = creature_update_owner;
        // Partial C++ ObjectUpdater seam: after DynamicObject, visit only the
        // represented map-owned Creature family in this slice. Default context is
        // honest represented runtime only: no real AI/combat/threat/fanout.
        self.last_creatures_update_summary = match creature_update_owner {
            MapCreatureUpdateOwnerLikeCpp::CanonicalMap => {
                if let Some(plan) = nearby_object_plan {
                    let guids = plan
                        .update_guids
                        .iter()
                        .copied()
                        .filter(|guid| {
                            self.runtime
                                .map
                                .map_object_record(*guid)
                                .is_some_and(|record| {
                                    matches!(
                                        record.kind(),
                                        AccessorObjectKind::Creature | AccessorObjectKind::Pet
                                    )
                                })
                        })
                        .collect::<Vec<_>>();
                    self.runtime.map.update_creatures_for_guids_like_cpp(
                        guids,
                        diff_ms,
                        now_secs,
                        |_guid, _creature| CreatureRuntimeUpdateContext::default(),
                    )
                } else {
                    self.runtime.map.update_creatures_like_cpp(
                        diff_ms,
                        now_secs,
                        |_guid, _creature| CreatureRuntimeUpdateContext::default(),
                    )
                }
            }
            MapCreatureUpdateOwnerLikeCpp::ExternalRuntime => {
                // The legacy/session owner already advances this transition.
                // Do not mutate a canonical shadow and discard its plan: that
                // would advance timers without applying the corresponding
                // C++ effects or fanout.
                CreatureUpdateSummaryLikeCpp::default()
            }
        };
    }

    pub(super) fn finish_object_update<L>(
        &mut self,
        continuation: ObjectUpdateContinuation,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        load_record: Option<&mut L>,
    ) where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let ObjectUpdateContinuation {
            diff_ms,
            nearby_object_plan,
            now_secs,
            actor_workset: _,
        } = continuation;
        // C++ Unit::Update advances timed PvP combat references for both
        // players and creatures. The canonical map owns both sides here, so
        // expire them once per map tick and purge the reciprocal relation.
        self.last_expired_pvp_combat_refs_like_cpp = self
            .runtime
            .map
            .update_all_pvp_combat_refs_like_cpp(diff_ms);
        // Partial C++ ObjectUpdater seam: after Creature, visit represented
        // map-owned GameObject records. C++ real order is TypeContainerVisitor
        // nearby-cell/active-object traversal; this Rust insertion only adds the
        // missing family and leaves AI/go-type/per-player/packet/DB gaps open.
        self.last_game_objects_update_summary = {
            let selected_game_objects = nearby_object_plan.as_ref().map(|plan| {
                plan.update_guids
                    .iter()
                    .copied()
                    .filter(|guid| {
                        self.runtime
                            .map
                            .map_object_record(*guid)
                            .is_some_and(|record| {
                                record.kind() == AccessorObjectKind::GameObject
                                    && record.game_object().is_some()
                            })
                    })
                    .collect::<Vec<_>>()
            });
            match (selected_game_objects, pool_update, load_record) {
                (Some(guids), Some((spawn_store, pool_mgr)), Some(load_record)) => self
                    .runtime
                    .map
                    .update_game_objects_for_guids_with_optional_pool_update_like_cpp(
                        guids,
                        diff_ms,
                        now_secs,
                        Some((spawn_store, pool_mgr)),
                        Some(load_record),
                    ),
                (Some(guids), Some((spawn_store, pool_mgr)), None) => self
                    .runtime
                    .map
                    .update_game_objects_for_guids_with_optional_pool_update_like_cpp::<fn(
                        &mut Map,
                        SpawnObjectType,
                        SpawnId,
                    ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(
                        guids,
                        diff_ms,
                        now_secs,
                        Some((spawn_store, pool_mgr)),
                        None,
                    ),
                (Some(guids), None, _) => self
                    .runtime
                    .map
                    .update_game_objects_for_guids_with_optional_pool_update_like_cpp::<fn(
                        &mut Map,
                        SpawnObjectType,
                        SpawnId,
                    ) -> Option<LoadedGridRespawnRecordsLikeCpp>>(
                        guids, diff_ms, now_secs, None, None,
                    ),
                (None, Some((spawn_store, pool_mgr)), Some(load_record)) => self
                    .runtime
                    .map
                    .update_game_objects_with_pool_update_loaded_grid_records_like_cpp(
                        diff_ms,
                        now_secs,
                        spawn_store,
                        pool_mgr,
                        load_record,
                    ),
                (None, Some((spawn_store, pool_mgr)), None) => self
                    .runtime
                    .map
                    .update_game_objects_with_pool_update_like_cpp(
                        diff_ms,
                        now_secs,
                        spawn_store,
                        pool_mgr,
                    ),
                (None, None, _) => self
                    .runtime
                    .map
                    .update_game_objects_like_cpp(diff_ms, now_secs),
            }
        };
        // Partial C++ transport seam: after the represented GameObject/ObjectUpdater
        // family and before later represented families, visit typed canonical
        // Transports. This does not reproduce exact C++ cell visitor ordering nor
        // full `_transports` runtime (AI/scripts/spline/teleport/fanout/passengers).
        let now_ms = game_time_now_ms_u64();
        self.last_transports_update_summary =
            self.runtime.map.update_transports_like_cpp(diff_ms, now_ms);
        // Partial C++ ObjectUpdater seam: after Transport, visit only the
        // represented map-owned AreaTrigger family in this slice. Other families,
        // nearby-cell traversal, player/session updates, fanout and scripts stay
        // explicit remaining gaps.
        self.last_area_triggers_update_summary = if let Some(plan) = &nearby_object_plan {
            let guids = plan
                .update_guids
                .iter()
                .copied()
                .filter(|guid| {
                    self.runtime
                        .map
                        .map_object_record(*guid)
                        .is_some_and(|record| {
                            record.kind() == AccessorObjectKind::AreaTrigger
                                && record.area_trigger().is_some()
                        })
                })
                .collect::<Vec<_>>();
            self.runtime
                .map
                .update_area_triggers_for_guids_like_cpp(guids, diff_ms)
        } else {
            self.runtime.map.update_area_triggers_like_cpp(diff_ms)
        };
        // Partial C++ ObjectUpdater seam: visit represented map-owned
        // Conversation records after AreaTrigger for this Rust slice. Exact
        // TypeContainerVisitor ordering/cell traversal, real scripts,
        // SendObjectUpdates and fanout remain explicit gaps.
        self.last_conversations_update_summary = if let Some(plan) = &nearby_object_plan {
            let guids = plan
                .update_guids
                .iter()
                .copied()
                .filter(|guid| {
                    self.runtime
                        .map
                        .map_object_record(*guid)
                        .is_some_and(|record| {
                            record.kind() == AccessorObjectKind::Conversation
                                && record.conversation().is_some()
                        })
                })
                .collect::<Vec<_>>();
            self.runtime
                .map
                .update_conversations_for_guids_like_cpp(guids, diff_ms)
        } else {
            self.runtime.map.update_conversations_like_cpp(diff_ms)
        };
        // Partial C++ ObjectUpdater seam: visit represented map-owned
        // SceneObject records after Conversation for this Rust slice. Real
        // ObjectAccessor::GetUnit and Aura lookup by spell/cast id are not present
        // yet, so the live manager default is conservative and does not remove
        // SceneObjects merely because that runtime is absent.
        self.last_scene_objects_update_summary = if let Some(plan) = &nearby_object_plan {
            let guids = plan
                .update_guids
                .iter()
                .copied()
                .filter(|guid| {
                    self.runtime
                        .map
                        .map_object_record(*guid)
                        .is_some_and(|record| {
                            record.kind() == AccessorObjectKind::SceneObject
                                && record.scene_object().is_some()
                        })
                })
                .collect::<Vec<_>>();
            self.runtime.map.update_scene_objects_for_guids_like_cpp(
                guids,
                diff_ms,
                |_guid, scene_object| {
                    SceneObjectUpdateContextLikeCpp::represented_default_for(scene_object)
                },
            )
        } else {
            self.runtime
                .map
                .update_scene_objects_like_cpp(diff_ms, |_guid, scene_object| {
                    SceneObjectUpdateContextLikeCpp::represented_default_for(scene_object)
                })
        };
        // C++ calls `Map::SendObjectUpdates()` after ObjectUpdater/Transport/
        // SceneObject-style visitation and before scripts/weather/personal phase
        // (`Map.cpp:777-798`). Rust consumes only represented map-owned
        // `m_objectUpdated`/changed-mask state here; no `UpdateDataMapType`,
        // session packets, visible-player iteration, or direct fanout is built.
        self.last_send_object_updates_summary_like_cpp =
            self.runtime.map.send_object_updates_like_cpp();
        // C++ then drains `m_scriptSchedule` under `i_scriptLock` before weather
        // and personal phase (`Map.cpp:777-798`, `MapScripts.cpp:311-321`).
        // Rust records due represented actions only; no script commands,
        // ObjectAccessor/session/fanout/DB/weather side effects are executed.
        self.last_script_schedule_process_summary_like_cpp = self
            .runtime
            .map
            .process_script_schedule_update_order_like_cpp(now_secs);
        // C++ updates `_weatherUpdateTimer` immediately after script schedule and
        // before `GetMultiPersonalPhaseTracker().Update(this, t_diff)`
        // (`Map.cpp:777-798`). Rust represents only the map-owned timer and
        // `_zoneDynamicInfo.DefaultWeather` update/reset seam; WeatherMgr, RNG,
        // script hooks, player fanout, packets, DB and zone messages remain gaps.
        self.last_weather_update_summary_like_cpp =
            self.runtime.map.update_weather_like_cpp(diff_ms);
        // C++ calls `GetMultiPersonalPhaseTracker().Update(this, t_diff)` after
        // SendObjectUpdates/scripts/weather and before later move/remove drains.
        // Rust consumes the existing map-owned tracker here as a represented seam
        // only: GUID expiry -> AddObjectToRemoveList, without claiming exact full
        // update ordering, visibility fanout, DB, scripts, or dynamic-tree parity.
        self.last_personal_phase_tracker_update_summary = self
            .runtime
            .map
            .update_personal_phase_tracker_like_cpp(diff_ms);
        // C++ `Map::Update` immediately drains Creature, GameObject, and
        // AreaTrigger move-lists after personal-phase tracker update and before
        // `ProcessRelocationNotifies(t_diff)` (`Map.cpp:797-805`). Rust keeps
        // `Map` as the sole owner of canonical object and queue state here; the
        // live manager only orchestrates order and stores summaries. DynamicObject
        // is intentionally not drained by this live path because C++ `Map::Update`
        // does not call a DynamicObject move-list drain.
        self.last_live_move_list_drain_summary = LiveMoveListDrainSummaryLikeCpp {
            creature: self.runtime.map.move_all_creatures_in_move_list_like_cpp(),
            game_object: self
                .runtime
                .map
                .move_all_game_objects_in_move_list_like_cpp(),
            area_trigger: self
                .runtime
                .map
                .move_all_area_triggers_in_move_list_like_cpp(),
        };
        // C++ `Map::Update` calls `ProcessRelocationNotifies(t_diff)`
        // immediately after the live Creature/GameObject/AreaTrigger move-list
        // drains and only when player/active-non-player sources exist
        // (`Map.cpp:797-805`). Rust consumes only the existing map-owned
        // represented helper here: marked-cell selection, relocation timer
        // selection/reset, delayed relocation plan selection, and notify flag
        // reset over canonical map state. It does not claim real notifier side
        // effects, packets, ObjectAccessor/session fanout, AI, dynamic tree, or
        // exact full visitor parity.
        self.last_process_relocation_notifies_outcome_like_cpp = self
            .runtime
            .map
            .process_live_relocation_notifies_like_cpp(diff_ms, DEFAULT_VISIBILITY_NOTIFY_PERIOD);
        // C++ `Map::Update` tail immediately follows ProcessRelocationNotifies:
        // `sScriptMgr->OnMapUpdate(this, t_diff)` then the `map_creatures` and
        // `map_gameobjects` metrics (`Map.cpp:804-815`). Rust records only the
        // boundary invocation and typed canonical counts from `Map::entity_world`;
        // no real ScriptMgr dispatch, script callbacks, Prometheus/telemetry,
        // ObjectAccessor, DB, or fanout side effects are claimed.
        self.last_map_update_tail_summary_like_cpp = MapUpdateTailSummaryLikeCpp {
            script_hook: MapUpdateScriptHookSummaryLikeCpp {
                invoked: true,
                diff_ms,
                map_id: self.runtime.map.map_id(),
                instance_id: self.runtime.map.instance_id(),
                kind: self.kind,
                script_dispatch_represented: false,
            },
            metrics: self.runtime.map.map_update_metrics_like_cpp(),
        };
    }
}
