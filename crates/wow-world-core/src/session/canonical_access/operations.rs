//! Accessors that resolve the canonical Player owner.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use std::sync::Arc;

use wow_core::ObjectGuid;
use wow_entities::Player;

impl crate::session::state::SessionCore {
    pub fn mutate_canonical_player_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        let guid = self.player_guid()?;
        self.mutate_canonical_player_by_guid_like_cpp(guid, f)
    }

    /// Resolve this session incarnation's canonical `Player` exclusively
    /// through its generation-checked handle.
    ///
    /// Unlike the transitional GUID/map lookup helpers, this deliberately has
    /// no fallback: a stale or missing handle means that the owner is unknown.
    pub fn with_owned_player_like_cpp<R>(
        &self,
        f: impl FnOnce(&Player) -> R,
    ) -> Option<R> {
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let handle = self.player_handle_like_cpp?;
        let manager = manager.lock().ok()?;
        let result = manager.with_player_like_cpp(handle, f);
        drop(manager);
        result
    }

    /// Mutating counterpart to `with_owned_player_like_cpp`.
    pub fn with_owned_player_mut_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let handle = self.player_handle_like_cpp?;
        let mut manager = manager.lock().ok()?;
        let result = manager.with_player_mut_like_cpp(handle, f);
        drop(manager);
        result
    }

    pub fn with_owned_player_for_rest_like_cpp<R>(
        &self,
        f: impl FnOnce(&Player) -> R,
    ) -> Option<R> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            return self.canonical_player_snapshot_like_cpp(f);
        }
        self.with_owned_player_like_cpp(f)
    }

    pub fn mutate_canonical_player_by_guid_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        if let Some(handle) = self.player_handle_like_cpp
            && handle.guid() == guid
        {
            return manager.with_player_mut_like_cpp(handle, f);
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        let mut instance_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if instance_id.is_none() && managed.map().get_typed_player(guid).is_some() {
                instance_id = Some(managed.instance_id());
            }
        });
        let managed = manager.find_map_mut(map_id, instance_id.unwrap_or(0))?;
        let player = managed.map_mut().get_typed_player_mut(guid)?;
        Some(f(player))
    }

    pub fn canonical_player_has_player_flag_like_cpp(
        &self,
        guid: ObjectGuid,
        flag: u32,
    ) -> Option<bool> {
        if self.player_guid() == Some(guid) {
            let owned = self.with_owned_player_like_cpp(|player| player.has_player_flag(flag));
            if owned.is_some() {
                return owned;
            }
            #[cfg(not(any(test, feature = "test-fixtures")))]
            return None;
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.player_handle_like_cpp.is_some() {
                return None;
            }
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none() {
                result = managed
                    .map()
                    .get_typed_player(guid)
                    .map(|player| player.has_player_flag(flag));
            }
        });
        result
    }

    pub fn canonical_player_display_ids_like_cpp(&self) -> Option<(u32, u32)> {
        let guid = self.player_guid()?;
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none() {
                result = managed.map().get_typed_player(guid).map(|player| {
                    let data = player.unit().data();
                    (
                        u32::try_from(data.display_id).unwrap_or_default(),
                        u32::try_from(data.native_display_id).unwrap_or_default(),
                    )
                });
            }
        });
        result
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_player_unit_presentation_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        self.with_owned_player_mut_like_cpp(mutate)
    }

    /// Read this session's own canonical `Player`.
    ///
    /// Resolving the GUID and map key is the only session-local part; the read
    /// itself is the placement-addressed accessor a remote reader uses too
    /// (#252), so one player's canonical state cannot be reached two ways.
    pub fn canonical_player_snapshot_like_cpp<R>(
        &self,
        f: impl FnOnce(&Player) -> R,
    ) -> Option<R> {
        let guid = self.player_guid()?;
        if let (Some(manager), Some(handle)) = (
            self.canonical_map_manager.as_ref(),
            self.player_handle_like_cpp,
        ) && handle.guid() == guid
        {
            return manager.lock().ok()?.with_player_like_cpp(handle, f);
        }
        let key = self.current_canonical_player_map_key_like_cpp();
        let manager = self.canonical_map_manager.as_ref()?;
        let map_id = key
            .as_ref()
            .map(|key| key.map_id)
            .unwrap_or_else(|| u32::from(self.player_map_id_like_cpp()));
        let instance_id = key.as_ref().map(|key| key.instance_id).unwrap_or(0);
        crate::canonical_player_access::with_canonical_player_at_like_cpp(
            manager,
            guid,
            map_id,
            instance_id,
            f,
        )
    }

    pub fn canonical_player_parry_block_snapshot_like_cpp(&self) -> (bool, bool) {
        self.canonical_player_snapshot_like_cpp(|player| {
            (
                player.unit().can_parry_like_cpp(),
                player.unit().can_block_like_cpp(),
            )
        })
        .unwrap_or((false, false))
    }

    /// C++ `Player::GetWeaponProficiency` (`Player.h:1432`): the mask
    /// accumulated by the learned `SPELL_EFFECT_PROFICIENCY` spells, used by
    /// `CollectionMgr::CanAddAppearance`.
    pub fn represented_player_weapon_proficiency_like_cpp(&self) -> Option<u32> {
        self.canonical_player_snapshot_like_cpp(Player::weapon_proficiency_like_cpp)
    }

    /// Read the canonical Player's last derived equipment/stat projection.
    pub fn canonical_player_effective_combat_stats_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerEffectiveCombatStatsLikeCpp> {
        self.canonical_player_snapshot_like_cpp(|player| *player.effective_combat_stats_like_cpp())
    }

    /// Read the canonical Player's total melee attack power for consumers
    /// whose C++ counterpart calls `Unit::GetTotalAttackPowerValue`.
    ///
    /// The derived snapshot is the only production authority for this value;
    /// callers must not rebuild it from Session-owned item modifiers.
    pub fn canonical_player_total_attack_power_like_cpp(&self) -> Option<f32> {
        self.canonical_player_snapshot_like_cpp(|player| player.total_attack_power_like_cpp())
    }
}

impl crate::session::HubMut<'_> {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) fn mutate_player_world_local_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerWorldLocalState) -> R,
    ) -> Option<R> {
        let mut mutate = Some(mutate);
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            mutate.take().expect("world-local mutation runs once")(
                &mut player.gameplay_state_mut().world_local,
            )
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let mut state = self
                .shared()
                .player_world_local_state_like_cpp()
                .expect("handle-less fixture world-local state");
            let result = mutate.take().expect("world-local mutation runs once")(&mut state);
            self.fixtures.identity.player_zone_id_like_cpp = state.zone_id_like_cpp();
            self.fixtures.identity.player_area_id_like_cpp = state.area_id_like_cpp();
            self.fixtures
                .identity
                .player_zone_area_authority_complete_like_cpp =
                state.has_zone_area_authority_like_cpp();
            self.fixtures.combat.player_pvp_hostile_like_cpp = state.is_pvp_hostile_like_cpp();
            self.fixtures.combat.player_pvp_end_timer_like_cpp = state.pvp_end_timer_like_cpp();
            self.fixtures.combat.player_contested_pvp_timer_like_cpp =
                state.contested_pvp_timer_like_cpp();
            self.fixtures.identity.represented_is_outdoors_like_cpp = state.is_outdoors_like_cpp();
            return Some(result);
        }
        canonical
    }

    pub fn set_player_zone_id_like_cpp(&mut self, zone_id: u32) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_zone_id_like_cpp(zone_id))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_world_local_state_like_cpp(|state| {
                    state.set_zone_id_like_cpp(zone_id);
                })
                .is_some();
        }
        canonical
    }

    pub fn set_player_area_id_like_cpp(&mut self, area_id: u32) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_area_id_like_cpp(area_id))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_world_local_state_like_cpp(|state| {
                    state.set_area_id_like_cpp(area_id);
                })
                .is_some();
        }
        canonical
    }

    pub fn set_player_world_local_zone_area_like_cpp(
        &mut self,
        zone_id: u32,
        area_id: u32,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_zone_area_like_cpp(zone_id, area_id)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_world_local_state_like_cpp(|state| {
                    state.set_zone_area_like_cpp(zone_id, area_id);
                })
                .is_some();
        }
        canonical
    }

    pub fn set_player_zone_area_authority_like_cpp(
        &mut self,
        complete: bool,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_zone_area_authority_like_cpp(complete)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_world_local_state_like_cpp(|state| {
                    state.set_zone_area_authority_like_cpp(complete);
                })
                .is_some();
        }
        canonical
    }

    pub fn set_player_pvp_hostile_like_cpp(&mut self, hostile: bool) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_pvp_hostile_like_cpp(hostile))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_world_local_state_like_cpp(|state| {
                    state.set_pvp_hostile_like_cpp(hostile);
                })
                .is_some();
        }
        canonical
    }

    pub fn set_player_pvp_end_timer_like_cpp(
        &mut self,
        end_timer: Option<i64>,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_pvp_end_timer_like_cpp(end_timer))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_world_local_state_like_cpp(|state| {
                    state.set_pvp_end_timer_like_cpp(end_timer);
                })
                .is_some();
        }
        canonical
    }

    pub(in crate::session) fn set_player_is_outdoors_like_cpp(
        &mut self,
        is_outdoors: bool,
    ) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_is_outdoors_like_cpp(is_outdoors))
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_world_local_state_like_cpp(|state| {
                    state.set_is_outdoors_like_cpp(Some(is_outdoors));
                })
                .is_some();
        }
        canonical
    }

    pub fn mutate_player_collection_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerCollectionStateLikeCpp) -> R,
    ) -> Option<R> {
        let mut state = self.shared().player_collection_state_snapshot_like_cpp()?;
        let result = mutate(&mut state);
        self.replace_player_collection_state_like_cpp(state)
            .then_some(result)
    }

    /// Apply one named canonical cinematic transition, or the handle-less test
    /// mirror that stands in for it. C++ performs these on the Player's own
    /// `CinematicMgr` (`CinematicMgr.h:39`, `CinematicMgr.cpp:46`, `:83`), never
    /// on a borrowed record.
    pub fn with_player_cinematic_state_like_cpp<R>(
        &mut self,
        mut apply: impl FnMut(&mut wow_entities::PlayerCinematicStateLikeCpp) -> R,
    ) -> Option<R> {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            apply(&mut player.gameplay_state_mut().cinematic)
        });
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.core.player_handle_like_cpp.is_none() {
            let result = apply(
                &mut self
                    .fixtures
                    .presentation
                    .represented_cinematic_state_like_cpp,
            );
            return Some(result);
        }
        None
    }

    /// Apply a heal to the canonical Player owner and return
    /// `(before, after, max, effective)`.
    pub fn apply_owned_player_heal_like_cpp(
        &mut self,
        requested_heal: u32,
    ) -> Option<(u32, u32, u32, u32)> {
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            let max_health = player
                .unit()
                .data()
                .max_health
                .clamp(1, u64::from(u32::MAX)) as u32;
            let before = player.unit().data().health.min(u64::from(max_health)) as u32;
            if !player.unit().is_alive() || before == 0 {
                return (before, before, max_health, 0);
            }
            let after = before.saturating_add(requested_heal).min(max_health);
            player.unit_mut().set_health(u64::from(after));
            (before, after, max_health, after.saturating_sub(before))
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            let max_health = self.fixtures.combat.player_max_health_like_cpp.max(1);
            let before = self.fixtures.combat.player_health_like_cpp.min(max_health);
            if !self.fixtures.combat.player_alive_like_cpp || before == 0 {
                return Some((before, before, max_health, 0));
            }
            let after = before.saturating_add(requested_heal).min(max_health);
            self.fixtures.combat.player_health_like_cpp = after;
            return Some((before, after, max_health, after.saturating_sub(before)));
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_player_battleground_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerBattlegroundState) -> R,
    ) -> Option<R> {
        let mut state = self
            .shared()
            .player_battleground_state_snapshot_like_cpp()?;
        let result = mutate(&mut state);
        self.fixtures
            .battleground
            .player_battleground_type_id_like_cpp = state.battleground_type_id_like_cpp();
        self.fixtures
            .battleground
            .player_battleground_map_id_like_cpp = state.battleground_map_id_like_cpp();
        self.fixtures
            .battleground
            .represented_battleground_status_like_cpp = state.battleground_status_like_cpp();
        self.fixtures
            .battleground
            .represented_battleground_queue_slots_like_cpp = state.queue_slots_like_cpp().to_vec();
        self.fixtures
            .battleground
            .represented_arena_team_id_invited_like_cpp = state.arena_team_id_invited_like_cpp();
        Some(result)
    }
}
