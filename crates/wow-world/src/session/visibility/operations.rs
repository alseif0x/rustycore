//! Represented visibility and phase state at the Session boundary.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    fn player_session_never_visible_for_seer_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.player_logout_like_cpp || self.player_loading == Some(guid)
    }
    fn apply_player_session_visibility_detection_like_cpp(
        player: &mut Player,
        never_visible_for_seer: bool,
        seer_can_never_see_target: bool,
    ) {
        player
            .unit_mut()
            .set_never_visible_for_seer_like_cpp(never_visible_for_seer);
        player
            .unit_mut()
            .set_seer_can_never_see_target_like_cpp(seer_can_never_see_target);
    }
    pub(crate) fn sync_current_player_session_visibility_detection_like_cpp(&mut self) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let never_visible_for_seer = self.player_session_never_visible_for_seer_like_cpp(guid);
        let seer_can_never_see_target = self.player_can_never_see_target_like_cpp();
        let _ = self.mutate_canonical_player_by_guid_like_cpp(guid, |player| {
            Self::apply_player_session_visibility_detection_like_cpp(
                player,
                never_visible_for_seer,
                seer_can_never_see_target,
            );
        });
    }
    fn object_id_visibility_conditions_met_like_cpp(
        &self,
        target: &WorldObject,
        seer: &WorldObject,
    ) -> bool {
        let Some(condition_store) = self.condition_store.as_ref() else {
            return true;
        };

        let area_table_store = self.area_table_store.as_ref().map(Arc::clone);
        crate::conditions::is_object_meeting_visibility_by_object_id_conditions_like_cpp(
            condition_store,
            target.object().type_id() as u32,
            target.object().entry(),
            Some(seer),
            |condition, source_info| {
                crate::conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |area_id, required_area_id| {
                        area_table_store.as_ref().is_some_and(|store| {
                            store.is_in_area_like_cpp(area_id, required_area_id)
                        })
                    },
                )
                .value()
                .unwrap_or(false)
            },
        )
    }
    pub(in crate::session) fn apply_target_visibility_context_for_current_player_like_cpp(
        &self,
        target_unit: &mut wow_entities::Unit,
        seer_unit: &mut wow_entities::Unit,
        moved_unit_guid: Option<ObjectGuid>,
        current_group_guid: Option<u64>,
    ) {
        target_unit.set_object_id_visibility_conditions_met_like_cpp(
            self.object_id_visibility_conditions_met_like_cpp(
                target_unit.world(),
                seer_unit.world(),
            ),
        );

        let target_guid = target_unit.world().object().guid();
        if moved_unit_guid == Some(target_guid) {
            seer_unit.set_seer_can_always_see_target_like_cpp(true);
        }

        let private_owner = target_unit.private_object_owner_like_cpp();
        if !private_owner.is_empty() {
            seer_unit.set_seer_group_visible_for_private_owner_like_cpp(
                self.current_player_is_in_group_guid_like_cpp(current_group_guid, private_owner),
            );
        }

        let owner_group_visible = target_unit
            .subsystems()
            .control
            .charmer_or_owner_guid()
            .is_some_and(|owner_guid| {
                self.current_player_is_group_visible_for_owner_like_cpp(
                    current_group_guid,
                    owner_guid,
                )
            });
        target_unit.set_target_owner_group_visible_for_seer_like_cpp(owner_group_visible);
    }
    pub(crate) fn visible_other_players_from_registry_like_cpp(
        &self,
        map_id: u16,
        position: &Position,
        visibility_radius: f32,
    ) -> Vec<(ObjectGuid, PlayerVisibilityCreateSnapshot)> {
        let Some(player_guid) = self.player_guid() else {
            return Vec::new();
        };
        let Some(registry) = &self.player_registry else {
            return Vec::new();
        };
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let source_combat_reach = self.represented_visibility_source_combat_reach_like_cpp();

        registry
            .player_visibility_create_candidates(
                player_guid,
                map_id,
                instance_id,
                *position,
                source_combat_reach,
                visibility_radius,
            )
            .into_iter()
            .filter(|candidate| {
                self.canonical_player_phase_visible_like_cpp(map_id, instance_id, candidate.guid)
                    == Some(true)
            })
            .map(|candidate| (candidate.guid, candidate))
            .collect()
    }
    /// Require the target to exist in the canonical map and apply the same
    /// phase gate as C++ `VisibleNotifier::Visit(PlayerMapType&)`. Missing map
    /// state returns `None` and fails closed at the caller: C++ cannot visit a
    /// registry-only player.
    fn canonical_player_phase_visible_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        target_guid: ObjectGuid,
    ) -> Option<bool> {
        // Resolve the viewer before taking the map lock; the handle resolver
        // takes the same manager and must not recurse into that mutex.
        let player_phase_shift = self.represented_player_phase_shift_like_cpp()?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(u32::from(map_id), instance_id)?;
        let target = map.map().get_typed_player(target_guid)?;
        Some(player_phase_shift.can_see(target.unit().world().phase_shift()))
    }
    pub fn set_phase_store(&mut self, store: Arc<PhaseStore>) {
        self.phase_store = Some(store);
    }
    pub(crate) fn represented_player_phase_shift_like_cpp(&self) -> Option<PhaseShift> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.unit().world().phase_shift().clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_player_phase_shift.clone());
        }
        canonical
    }
    pub(crate) fn set_represented_player_phase_shift_like_cpp(
        &mut self,
        phase_shift: PhaseShift,
    ) -> bool {
        let mut phase_shift = Some(phase_shift);
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                *player.unit_mut().world_mut().phase_shift_mut() =
                    phase_shift.take().expect("phase mutation runs once");
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_player_phase_shift =
                phase_shift.take().expect("fixture phase remains available");
            return true;
        }
        false
    }
    pub(crate) fn can_see_phase_shift_like_cpp(&self, other: &PhaseShift) -> bool {
        self.represented_player_phase_shift_like_cpp()
            .is_some_and(|phase_shift| phase_shift.can_see(other))
    }
    pub(crate) fn resolved_visible_resting_like_cpp(&self) -> Option<bool> {
        // RestMgr::SetRestFlag/RemoveRestFlag (RestMgr.cpp:99-125): the
        // mask and Player flag belong to the same Player. Read them together.
        let canonical = self.with_owned_player_for_rest_like_cpp(|player| {
            let rest = player.rest_state_like_cpp();
            if rest.location_initialized {
                rest.rest_flag_mask != 0
            } else {
                (player.data().player_flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0
            }
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let rest = self.player_rest_state_snapshot_like_cpp()?;
            if rest.location_initialized {
                return Some(rest.rest_flag_mask != 0);
            }
            return Some(
                self.represented_loaded_player_flags_like_cpp
                    .map(|flags| (flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0)
                    .unwrap_or(rest.rest_flag_mask != 0),
            );
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn represented_visible_resting_like_cpp(&self) -> bool {
        self.resolved_visible_resting_like_cpp()
            .expect("test Player rest owner must resolve")
    }
    /// Send represented `ActivePlayerData::FarsightObject` VALUES update after
    /// the canonical AddFarsight `Player::SetViewpoint(..., true)` success.
    ///
    /// C++ anchors: `Player::SetViewpoint` writes
    /// `UF::ActivePlayerData::FarsightObject`; `ActivePlayerData::WriteUpdate`
    /// emits the field under parent block `changesMask[0]` and field bit 26.
    pub(in crate::session) fn send_active_player_farsight_object_values_update_like_cpp(
        &self,
        player_guid: ObjectGuid,
        farsight_guid: ObjectGuid,
    ) {
        use wow_packet::packets::update::{ActivePlayerDataValuesUpdate, UpdateObject};

        let mut data = ActivePlayerDataValuesUpdate::default();
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 0);
        set_active_player_update_bit_like_cpp(&mut data.active_player_data_mask, 26);
        data.farsight_object = farsight_guid;
        self.send_packet(&UpdateObject::full_active_player_values_update(
            player_guid,
            self.player_map_id_like_cpp(),
            data,
        ));
    }
    #[cfg(test)]
    pub(crate) fn represented_seer_guid_like_cpp(&self) -> Option<ObjectGuid> {
        self.represented_seer_guid_like_cpp
    }
    fn current_canonical_player_farsight_object_value_like_cpp(&self) -> Option<ObjectGuid> {
        let guid = self.player_guid()?;
        let key = self.current_canonical_player_map_key_like_cpp()?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        Some(
            manager
                .find_map(key.map_id, key.instance_id)?
                .map()
                .get_typed_player(guid)?
                .active_data()
                .farsight_object,
        )
    }
    pub(in crate::session) fn current_canonical_farsight_object_like_cpp(
        &self,
    ) -> Option<ObjectGuid> {
        let value = self.current_canonical_player_farsight_object_value_like_cpp()?;
        (!value.is_empty()).then_some(value)
    }
    /// Consume the represented `Player::SetViewpoint(target, false)`/`SetSeer(this)`
    /// side effect after canonical DynamicObject viewpoint removal has already
    /// cleared the map-owned Player `ActivePlayerData::FarsightObject`.
    ///
    /// Ownership remains one-way: canonical map Player state is the source of
    /// truth; this helper only mirrors canonical empty farsight into the
    /// session-local represented `m_seer` and represented VALUES packet.
    pub(crate) fn sync_represented_farsight_clear_from_canonical_like_cpp(&mut self) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(seer_guid) = self.represented_seer_guid_like_cpp else {
            return false;
        };
        if seer_guid.is_empty() || seer_guid == player_guid {
            return false;
        }

        let Some(canonical_farsight_object) =
            self.current_canonical_player_farsight_object_value_like_cpp()
        else {
            return false;
        };
        if !canonical_farsight_object.is_empty() {
            return false;
        }

        self.represented_seer_guid_like_cpp = Some(player_guid);
        self.send_active_player_farsight_object_values_update_like_cpp(
            player_guid,
            ObjectGuid::EMPTY,
        );
        self.last_visibility_pos = None;
        true
    }
    pub(in crate::session) fn represented_seer_kinds_like_cpp() -> &'static [AccessorObjectKind] {
        &[
            AccessorObjectKind::Player,
            AccessorObjectKind::Creature,
            AccessorObjectKind::Pet,
            AccessorObjectKind::DynamicObject,
        ]
    }
    pub(in crate::session) fn visibility_distance_allows_like_cpp(
        source_position: &Position,
        source_combat_reach: f32,
        target_position: &Position,
        target_combat_reach: f32,
        sight_range: f32,
    ) -> bool {
        let max_distance =
            sight_range + source_combat_reach.max(0.0) + target_combat_reach.max(0.0);
        source_position.distance_2d_sq(target_position) < max_distance * max_distance
    }
    pub(crate) async fn force_update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) {
        self.last_visibility_pos = None;
        self.update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
            .await;
    }
    pub(crate) fn clear_pending_visibility_refresh_like_cpp(&self) {
        self.visibility_refresh_pending_like_cpp
            .store(false, Ordering::Release);
    }
    pub(crate) async fn flush_pending_visibility_refresh_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if self
            .visibility_refresh_pending_like_cpp
            .swap(false, Ordering::AcqRel)
        {
            self.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
                .await;
        }
    }
    #[cfg(test)]
    pub(crate) async fn force_update_visibility_like_cpp(&mut self) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.force_update_visibility_with_catalogs_like_cpp(&catalogs)
            .await;
    }
}
