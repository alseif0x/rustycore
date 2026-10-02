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
