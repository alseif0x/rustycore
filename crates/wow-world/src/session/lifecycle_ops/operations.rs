//! Represented login, logout, account and shutdown operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_session_account_state_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp>,
    ) {
        self.lifecycle.set_session_account_state_port_like_cpp(port)
    }
    pub fn set_battlenet_account_id(&mut self, battlenet_account_id: u32) {
        self.core.account_state.battlenet_account_id = battlenet_account_id;
    }
    pub fn battlenet_account_id(&self) -> u32 {
        self.core.battlenet_account_id()
    }
    pub(crate) fn account_heirloom_active_player_rows_like_cpp(&self) -> Vec<(i32, u32)> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_heirloom_active_player_rows_like_cpp(hub)
    }
    pub fn send_account_heirlooms_like_cpp(&self) {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.send_account_heirlooms_like_cpp(hub)
    }
    pub(crate) fn add_account_heirloom_like_cpp(&mut self, item_id: u32, flags: u32) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.add_account_heirloom_like_cpp(&mut hub, item_id, flags)
    }
    pub(crate) fn upgrade_account_heirloom_like_cpp(
        &mut self,
        item_id: u32,
        cast_item: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.upgrade_account_heirloom_like_cpp(&mut hub, item_id, cast_item)
    }
    pub(crate) fn account_toy_active_player_rows_like_cpp(&self) -> Vec<i32> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_toy_active_player_rows_like_cpp(hub)
    }
    pub fn send_account_toys_like_cpp(&self) {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.send_account_toys_like_cpp(hub)
    }
    pub(crate) fn has_account_toy_like_cpp(&self, item_id: u32) -> bool {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.has_account_toy_like_cpp(hub, item_id)
    }
    pub(crate) fn add_account_toy_like_cpp(
        &mut self,
        item_id: u32,
        is_favorite: bool,
        has_fanfare: bool,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.add_account_toy_like_cpp(&mut hub, item_id, is_favorite, has_fanfare)
    }
    pub(in crate::session) fn represented_battle_pet_login_spell_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.represented_battle_pet_login_spell_source_is_empty_like_cpp(hub)
    }
    pub(crate) fn apply_represented_first_login_flag_if_needed_like_cpp(&mut self) -> bool {
        const AT_LOGIN_FIRST_LIKE_CPP: u16 = 0x020;

        if !self
            .resolved_represented_at_login_flags_like_cpp()
            .is_some_and(|flags| (flags & AT_LOGIN_FIRST_LIKE_CPP) != 0)
        {
            return false;
        }

        self.remove_represented_at_login_flag_like_cpp(AT_LOGIN_FIRST_LIKE_CPP, false)
    }
    pub(crate) fn ensure_login_player_controller_like_cpp(
        &mut self,
        guid: ObjectGuid,
        name: String,
        position: wow_core::Position,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) -> bool {
        if self.core.player_handle_like_cpp.is_none()
            && !self.player_bootstrap_attached_for_test_like_cpp()
        {
            self.attach_player_controller_like_cpp(SessionPlayerController::new(
                guid, name, position, map_id, race, class, level, gender,
            ));
            true
        } else {
            self.set_player_guid(Some(guid));
            self.set_loaded_player_name_like_cpp(name);
            self.set_loaded_player_identity_like_cpp(map_id, race, class, level, gender);
            crate::session::hub_mut(self).set_player_map_position_like_cpp(map_id, position);
            crate::session::hub_mut(self).set_fall_information_like_cpp(0, position.z);
            false
        }
    }
    pub(crate) fn set_account_mounts_like_cpp(&mut self, mounts: Vec<AccountMount>) {
        let mounts = mounts
            .into_iter()
            .map(|mount| (mount.spell_id, mount.flags))
            .collect();
        let _ =
            crate::session::hub_mut(self).mutate_player_collection_state_like_cpp(|collections| {
                collections.replace_mounts_like_cpp(mounts);
            });
        self.expand_account_mount_faction_definitions_like_cpp();
        self.learn_account_mount_spells_like_cpp();
    }
    pub(in crate::session) fn add_account_mount_with_faction_counterpart_like_cpp(
        &mut self,
        spell_id: i32,
        flags: u8,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.add_account_mount_with_faction_counterpart_like_cpp(&mut hub, spell_id, flags)
    }
    pub(in crate::session) fn expand_account_mount_faction_definitions_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.expand_account_mount_faction_definitions_like_cpp(&mut hub)
    }
    #[cfg(test)]
    pub(crate) fn account_mounts_like_cpp(&self) -> &HashMap<i32, u8> {
        &self.fixtures.collections.account_mounts_like_cpp
    }
    pub(crate) fn account_mount_rows_like_cpp(&self) -> Vec<AccountMount> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_mount_rows_like_cpp(hub)
    }
    /// Account mounts for the per-mount `CollectionMgr::LoadMounts` login
    /// publications. C++ keeps every valid DB2 row in the collection, but
    /// suppresses this partial update when the mount's PlayerCondition fails.
    pub(crate) fn account_mount_login_partial_rows_like_cpp(&self) -> Vec<AccountMount> {
        self.account_mount_rows_like_cpp()
            .into_iter()
            .filter(|mount| {
                let Ok(spell_id) = u32::try_from(mount.spell_id) else {
                    return false;
                };
                self.catalogs
                    .mount_store
                    .as_ref()
                    .and_then(|store| store.get_by_source_spell_id_like_cpp(spell_id))
                    .is_none_or(|entry| {
                        self.represented_meets_player_condition_id_like_cpp(
                            entry.player_condition_id,
                        )
                    })
            })
            .collect()
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn set_represented_at_login_flags_like_cpp(&mut self, flags: u16) -> bool {
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_at_login_flags_like_cpp(flags))
            .is_some();
        #[cfg(test)]
        if !canonical && self.core.player_handle_like_cpp.is_none() {
            return self
                .mutate_player_persistent_capability_state_like_cpp(|state| {
                    state.at_login_flags = flags;
                })
                .is_some();
        }
        canonical
    }
    pub(crate) fn resolved_represented_at_login_flags_like_cpp(&self) -> Option<u16> {
        self.player_persistent_capability_state_snapshot_like_cpp()
            .map(|state| state.at_login_flags)
    }
    #[cfg(test)]
    pub(crate) fn represented_at_login_flags_like_cpp(&self) -> u16 {
        self.resolved_represented_at_login_flags_like_cpp()
            .expect("test Player persistent-capability owner must resolve")
    }
    pub fn kick(&mut self, reason: &str) {
        self.core.kick(reason)
    }
}

impl crate::session::LifecycleCx<'_> {
    /// C++ `CollectionMgr::CheckHeirloomUpgrades`.
    pub(crate) fn check_account_heirloom_upgrades_like_cpp(
        &mut self,
        item_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let heirloom_store = Arc::clone(self.hub.catalogs.heirloom_store.as_ref()?);
        let heirloom = heirloom_store.get_by_item_id_like_cpp(item_id)?;
        self.hub
            .shared()
            .player_collection_state_snapshot_like_cpp()?
            .heirlooms_like_cpp()
            .get(&item_id)?;

        let mut heirloom_item_id = u32::try_from(heirloom.static_upgraded_item_id).ok()?;
        let mut new_item_id = 0_u32;
        while let Some(heirloom_diff) = heirloom_store.get_by_item_id_like_cpp(heirloom_item_id) {
            let diff_item_id = u32::try_from(heirloom_diff.item_id).ok()?;
            if self
                .inventory
                .represented_player_has_default_item_entry_like_cpp(self.hub.shared(), diff_item_id)
            {
                new_item_id = diff_item_id;
            }

            let Some(heirloom_sub_item_id) = u32::try_from(heirloom_diff.static_upgraded_item_id)
                .ok()
                .and_then(|static_item_id| {
                    heirloom_store
                        .get_by_item_id_like_cpp(static_item_id)
                        .and_then(|heirloom_sub| u32::try_from(heirloom_sub.item_id).ok())
                })
            else {
                break;
            };
            heirloom_item_id = heirloom_sub_item_id;
        }

        if new_item_id == 0 {
            return None;
        }

        let active_item_id = i32::try_from(item_id).ok()?;
        let active_new_item_id = i32::try_from(new_item_id).ok()?;
        let active_offset = self.hub.core.mutate_canonical_player_like_cpp(|player| {
            player
                .heirlooms_like_cpp()
                .iter()
                .position(|&heirloom_item_id| heirloom_item_id == active_item_id)
        })??;

        let update = self.hub.core.mutate_canonical_player_like_cpp(|player| {
            let set_item = player.set_heirloom_like_cpp(active_offset, active_new_item_id);
            let set_flags = player.set_heirloom_flags_like_cpp(active_offset, 0);
            (set_item && set_flags).then(|| player.values_update(true))
        })??;

        self.hub
            .mutate_player_collection_state_like_cpp(|collections| {
                collections.replace_heirloom_like_cpp(item_id, new_item_id);
            })?;
        Some(update)
    }
}


#[cfg(test)]
#[path = "../../../unit_tests/session/lifecycle_ops/operations/f3_shims.rs"]
mod f3_shims;
