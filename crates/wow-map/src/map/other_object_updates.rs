// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Transport, AreaTrigger, Conversation and SceneObject update phases.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
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
    /// Ownership: source-of-truth is canonical `Map::entity_world`. Missing,
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

        let Some(record) = self.entity_world.get_mut(&transport_guid) else {
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
            .entity_world
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
    /// Ownership: source-of-truth is canonical `Map::entity_world`. This helper
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
            let Some(record) = self.entity_world.get_mut(&area_trigger_guid) else {
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
    /// snapshots canonical typed AreaTrigger GUIDs from `Map::entity_world`, then
    /// delegates every GUID to `update_area_trigger_like_cpp`. It does not visit
    /// nearby cells, players/sessions, other object families, SendObjectUpdates,
    /// scripts/AI real runtime, visibility, dynamic tree, packets, DB, or mirrors.
    pub fn update_area_triggers_like_cpp(
        &mut self,
        elapsed_ms: u32,
    ) -> AreaTriggersUpdateSummaryLikeCpp {
        let area_trigger_guids = self
            .entity_world
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
    /// Ownership: source-of-truth is canonical `Map::entity_world`. Missing,
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
            let Some(record) = self.entity_world.get_mut(&conversation_guid) else {
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
    /// This snapshots canonical typed Conversation GUIDs from `Map::entity_world`,
    /// then delegates every GUID to `update_conversation_like_cpp`. It does not
    /// model exact `TypeContainerVisitor` order/cell traversal, players/sessions,
    /// other object families, `SendObjectUpdates`, real scripts, visibility,
    /// packets, DB, ObjectAccessor/session mirrors, or remove-list drain.
    pub fn update_conversations_like_cpp(
        &mut self,
        elapsed_ms: u32,
    ) -> ConversationsUpdateSummaryLikeCpp {
        let conversation_guids = self
            .entity_world
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
    /// Ownership: source-of-truth is canonical `Map::entity_world`. ObjectAccessor
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
    /// This snapshots canonical typed SceneObject GUIDs from `Map::entity_world`,
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
            .entity_world
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
}
