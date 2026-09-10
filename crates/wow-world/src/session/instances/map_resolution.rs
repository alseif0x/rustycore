//! Resolving the canonical map and manager the represented Session is
//! currently attached to.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Inject the shared map manager. Call once at session creation, before login.
    pub fn set_map_manager(&mut self, mgr: crate::map_manager::SharedMapManager) {
        self.map_manager = Some(mgr);
    }
    pub fn set_canonical_map_manager(&mut self, mgr: SharedCanonicalMapManager) {
        if let Some(registry) = &self.player_registry {
            let _ = registry.bind_canonical_map_manager(Arc::clone(&mgr));
        }
        self.canonical_map_manager = Some(mgr);
    }
    /// Resolve the one canonical Player identity and move that exact value to
    /// the selected map. Existing map records predate Player handles, so the
    /// transition must adopt them before considering a new initial value.
    pub(in crate::session) fn ensure_canonical_player_owner_for_map_like_cpp(
        &mut self,
        key: wow_map::MapKey,
        position: Position,
    ) -> bool {
        if !self.ensure_canonical_player_owner_exists_like_cpp(key) {
            return false;
        }
        let Some(manager) = self.canonical_map_manager.as_ref().map(Arc::clone) else {
            return false;
        };

        let Some(handle) = self.player_handle_like_cpp else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        match manager.player_residence_like_cpp(handle) {
            Some(wow_map::PlayerResidenceLikeCpp::Active(current)) if current == key => manager
                .relocate_player_like_cpp(handle, position)
                .is_ok_and(|outcome| outcome.relocated),
            Some(wow_map::PlayerResidenceLikeCpp::Active(_)) => {
                manager.detach_player_like_cpp(handle).is_ok()
                    && manager
                        .attach_player_like_cpp(handle, key, position)
                        .is_ok()
            }
            Some(wow_map::PlayerResidenceLikeCpp::Detached) => manager
                .attach_player_like_cpp(handle, key, position)
                .is_ok(),
            None => false,
        }
    }
    /// Inject the dedicated Detour worker handle. The session only sends
    /// path requests; it never owns raw mmap/navmesh state.
    pub fn set_mmap_pathfinder_like_cpp(
        &mut self,
        pathfinder: Arc<WorldMMapPathfinderWorkerLikeCpp>,
    ) {
        self.mmap_pathfinder_like_cpp = Some(pathfinder);
    }
    pub(crate) fn has_canonical_map_manager_like_cpp(&self) -> bool {
        self.canonical_map_manager.is_some()
    }
    pub(crate) fn has_world_map_manager_like_cpp(&self) -> bool {
        self.map_manager.is_some()
    }
    pub(crate) fn visible_dynamic_objects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::DynamicObjectCreateData>> {
        let requested_map_id = u32::from(map_id);
        let player_map_key = self.current_canonical_player_map_key_like_cpp();
        let source_combat_reach = self.represented_visibility_source_combat_reach_like_cpp();
        let manager = self.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = match player_map_key {
            Some(key) if key.map_id == requested_map_id => {
                manager.find_map(key.map_id, key.instance_id)?
            }
            Some(_) => return None,
            None => manager.find_map(requested_map_id, 0)?,
        };
        let nearby = map.map().nearby_cell_guids_like_cpp(
            position.x,
            position.y,
            visibility_radius + source_combat_reach,
        );
        let mut dynamic_objects = Vec::new();

        for guid in nearby
            .world
            .dynamic_objects
            .into_iter()
            .chain(nearby.grid.dynamic_objects)
        {
            let Some(dynamic_object) = map.map().get_typed_dynamic_object(guid) else {
                continue;
            };
            let object = dynamic_object.world();
            if !object.object().is_in_world()
                || object.map_id() != u32::from(map_id)
                || !crate::session_rules::visibility_distance_allows_like_cpp(
                    position,
                    source_combat_reach,
                    &object.position(),
                    object.combat_reach(),
                    visibility_radius,
                )
            {
                continue;
            }
            dynamic_objects.push(
                crate::session_rules::dynamic_object_create_data_from_canonical_like_cpp(
                    guid,
                    dynamic_object,
                ),
            );
        }

        Some(dynamic_objects)
    }
    pub(crate) fn visible_area_triggers_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
        visibility_radius: f32,
    ) -> Option<Vec<wow_packet::packets::update::AreaTriggerCreateData>> {
        let requested_map_id = u32::from(map_id);
        let player_map_key = self.current_canonical_player_map_key_like_cpp();
        let source_combat_reach = self.represented_visibility_source_combat_reach_like_cpp();
        let viewer_phase_shift = self.represented_player_phase_shift_like_cpp();
        let manager = self.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = match player_map_key {
            Some(key) if key.map_id == requested_map_id => {
                manager.find_map(key.map_id, key.instance_id)?
            }
            Some(_) => return None,
            None => manager.find_map(requested_map_id, 0)?,
        };
        let nearby = map.map().nearby_cell_guids_like_cpp(
            position.x,
            position.y,
            visibility_radius + source_combat_reach,
        );
        let mut area_triggers = Vec::new();

        for guid in nearby.grid.area_triggers {
            let Some(create_data) = map
                .map()
                .with_area_trigger_like_cpp(guid, |area_trigger| {
                    let object = area_trigger.world();
                    if !object.object().is_in_world()
                        || object.map_id() != u32::from(map_id)
                        || !crate::session_rules::visibility_distance_allows_like_cpp(
                            position,
                            source_combat_reach,
                            &object.position(),
                            object.combat_reach(),
                            visibility_radius,
                        )
                        || !viewer_phase_shift
                            .as_ref()
                            .is_some_and(|viewer| viewer.can_see(object.phase_shift()))
                        || area_trigger.is_server_side()
                        || area_trigger.is_removed()
                    {
                        return None;
                    }
                    Some(
                        crate::entity_update_bridge::area_trigger_create_data_from_entity_like_cpp(
                            area_trigger,
                        ),
                    )
                })
                .flatten()
            else {
                continue;
            };
            area_triggers.push(create_data);
        }

        Some(area_triggers)
    }
    pub(crate) fn visible_misc_objects_from_canonical_map_like_cpp(
        &self,
        map_id: u16,
        position: &wow_core::Position,
        visibility_radius: f32,
    ) -> Option<(
        Vec<wow_packet::packets::update::CorpseCreateData>,
        Vec<wow_packet::packets::update::SceneObjectCreateData>,
        Vec<wow_packet::packets::update::ConversationCreateData>,
    )> {
        let requested_map_id = u32::from(map_id);
        let player_map_key = self.current_canonical_player_map_key_like_cpp();
        let source_combat_reach = self.represented_visibility_source_combat_reach_like_cpp();
        let viewer_phase_shift = self.represented_player_phase_shift_like_cpp();
        let manager = self.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = match player_map_key {
            Some(key) if key.map_id == requested_map_id => {
                manager.find_map(key.map_id, key.instance_id)?
            }
            Some(_) => return None,
            None => manager.find_map(requested_map_id, 0)?,
        };
        let nearby = map.map().nearby_cell_guids_like_cpp(
            position.x,
            position.y,
            visibility_radius + source_combat_reach,
        );

        let allows_world_object = |world: &wow_entities::WorldObject| {
            world.object().is_in_world()
                && world.map_id() == requested_map_id
                && viewer_phase_shift
                    .as_ref()
                    .is_some_and(|viewer| viewer.can_see(world.phase_shift()))
                && crate::session_rules::visibility_distance_allows_like_cpp(
                    position,
                    source_combat_reach,
                    &world.position(),
                    world.combat_reach(),
                    visibility_radius,
                )
        };

        let mut corpses = Vec::new();
        for guid in nearby.world.corpses.into_iter().chain(nearby.grid.corpses) {
            let Some(corpse) = map.map().get_typed_corpse(guid) else {
                continue;
            };
            if allows_world_object(corpse.world()) {
                corpses.push(
                    crate::entity_update_bridge::corpse_create_data_from_entity_like_cpp(corpse),
                );
            }
        }

        let mut scene_objects = Vec::new();
        for guid in nearby.grid.scene_objects {
            let Some(create_data) = map
                .map()
                .with_scene_object_like_cpp(guid, |scene_object| {
                    allows_world_object(scene_object.world()).then(|| {
                        crate::entity_update_bridge::scene_object_create_data_from_entity_like_cpp(
                            scene_object,
                        )
                    })
                })
                .flatten()
            else {
                continue;
            };
            scene_objects.push(create_data);
        }

        let mut conversations = Vec::new();
        for guid in nearby.grid.conversations {
            let Some(create_data) = map
                .map()
                .with_conversation_like_cpp(guid, |conversation| {
                    (!conversation.is_removed() && allows_world_object(conversation.world())).then(
                    || {
                        crate::entity_update_bridge::conversation_create_data_from_entity_like_cpp(
                            conversation,
                            &self.locale,
                        )
                    },
                )
                })
                .flatten()
            else {
                continue;
            };
            conversations.push(create_data);
        }

        Some((corpses, scene_objects, conversations))
    }
    pub(crate) fn current_canonical_player_map_key_like_cpp(&self) -> Option<wow_map::MapKey> {
        let guid = self.player_guid()?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        if let Some(handle) = self.player_handle_like_cpp
            && handle.guid() == guid
        {
            return match manager.player_residence_like_cpp(handle)? {
                wow_map::PlayerResidenceLikeCpp::Active(key) => Some(key),
                wow_map::PlayerResidenceLikeCpp::Detached => None,
            };
        }
        let mut key = None;
        let mut ambiguous = false;
        manager.do_for_all_maps(|managed| {
            if managed.map().get_typed_player(guid).is_none() {
                return;
            }
            if key.is_some() {
                ambiguous = true;
            } else {
                key = Some(wow_map::MapKey::new(
                    managed.map_id(),
                    managed.instance_id(),
                ));
            }
        });
        (!ambiguous).then_some(key).flatten()
    }
    /// Resolve object access through the player's exact canonical `Map`, as
    /// C++ `ObjectAccessor::GetCreature/GetGameObject(WorldObject const&, ...)`
    /// does through `world_object.GetMap()`. During pre-player bootstrap and
    /// focused represented tests, accept a sole map for the expected map id;
    /// multiple instances without a canonical Player fail closed.
    pub(crate) fn canonical_object_lookup_map_key_like_cpp(
        &self,
        fallback_map_id: u32,
    ) -> Option<wow_map::MapKey> {
        if let Some(map_key) = self.current_canonical_player_map_key_like_cpp() {
            return Some(map_key);
        }

        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        if let Some(player_guid) = self.player_guid() {
            let mut player_map_count = 0usize;
            manager.do_for_all_maps(|managed| {
                if managed.map().get_typed_player(player_guid).is_some() {
                    player_map_count = player_map_count.saturating_add(1);
                }
            });
            // A player temporarily visible in two canonical maps is a
            // transfer boundary, not permission to choose one by iteration
            // order. Object-owned mutations fail closed until ownership is
            // unambiguous.
            if player_map_count != 0 || self.state == SessionState::LoggedIn {
                return None;
            }
        }
        let mut fallback_key = None;
        let mut ambiguous = false;
        manager.do_for_all_maps(|managed| {
            if managed.map_id() != fallback_map_id {
                return;
            }
            if fallback_key.is_some() {
                ambiguous = true;
            } else {
                fallback_key = Some(wow_map::MapKey::new(
                    managed.map_id(),
                    managed.instance_id(),
                ));
            }
        });
        (!ambiguous).then_some(fallback_key).flatten()
    }
    pub(in crate::session) fn canonical_map_has_seer_like_object_like_cpp(
        &self,
        target: ObjectGuid,
    ) -> bool {
        if target.is_empty() {
            return false;
        }
        let Some(key) = self.current_canonical_player_map_key_like_cpp() else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        manager
            .find_map(key.map_id, key.instance_id)
            .and_then(|managed| {
                managed.map().with_world_object_by_kinds_like_cpp(
                    target,
                    crate::session_rules::represented_seer_kinds_like_cpp(),
                    |_| (),
                )
            })
            .is_some()
    }
}
