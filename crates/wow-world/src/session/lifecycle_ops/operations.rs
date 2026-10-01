//! Represented login, logout, account and shutdown operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

fn account_heirloom_update_opcode_resolved_like_cpp() -> bool {
    <wow_packet::packets::misc::AccountHeirloomUpdate as wow_packet::ServerPacket>::OPCODE
        != ServerOpcodes::UpdateCapturePoint
}

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

impl crate::session::state::SessionLifecycleState {
    pub fn set_session_account_state_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp>,
    ) {
        self.persistence_ports_like_cpp
            .admission
            .session_account_state = Some(port);
    }

    /// C++ `CollectionMgr::SaveAccountHeirlooms`.
    pub(crate) fn account_heirloom_rows_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Vec<(u32, u32)> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .heirlooms_like_cpp()
                    .iter()
                    .map(|(item_id, data)| (*item_id, data.flags))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `CollectionMgr::GetHeirloomBonus`.
    #[cfg(test)]
    pub(crate) fn account_heirloom_bonus_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        item_id: u32,
    ) -> u32 {
        hub.fixtures
            .collections
            .represented_account_heirlooms_like_cpp
            .get(&item_id)
            .map(|data| data.bonus_id)
            .unwrap_or(0)
    }

    /// C++ `CollectionMgr::GetAccountHeirlooms` full update payload.
    pub(crate) fn account_heirloom_packet_rows_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Vec<AccountHeirloom> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .heirlooms_like_cpp()
                    .iter()
                    .filter_map(|(item_id, data)| {
                        Some(AccountHeirloom {
                            item_id: i32::try_from(*item_id).ok()?,
                            flags: data.flags,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `CollectionMgr::LoadHeirlooms` active-player create data order.
    pub(crate) fn account_heirloom_active_player_rows_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Vec<(i32, u32)> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .heirlooms_like_cpp()
                    .iter()
                    .filter_map(|(item_id, data)| Some((i32::try_from(*item_id).ok()?, data.flags)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `WorldPackets::Misc::AccountHeirloomUpdate` full login update.
    pub fn send_account_heirlooms_like_cpp(&self, hub: crate::session::HubRef<'_>) {
        if !account_heirloom_update_opcode_resolved_like_cpp() {
            warn!(
                "Skipping AccountHeirloomUpdate: legacy C++ opcode is unresolved 0xBADD for 54261"
            );
            return;
        }

        hub.core.send_packet(&AccountHeirloomUpdate::full(
            self.account_heirloom_packet_rows_like_cpp(hub),
        ));
    }

    /// C++ `CollectionMgr::AddHeirloom` / `UpdateAccountHeirlooms`.
    pub(crate) fn add_account_heirloom_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        item_id: u32,
        flags: u32,
    ) -> bool {
        hub.mutate_player_collection_state_like_cpp(|collections| {
            collections
                .add_heirloom_like_cpp(item_id, AccountHeirloomDataLikeCpp { flags, bonus_id: 0 })
        })
        .unwrap_or(false)
    }

    /// C++ `CollectionMgr::UpgradeHeirloom`.
    pub(crate) fn upgrade_account_heirloom_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        item_id: u32,
        cast_item: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let heirloom = hub
            .catalogs
            .heirloom_store
            .as_ref()?
            .get_by_item_id_like_cpp(item_id)?
            .clone();
        let current_flags = hub
            .shared()
            .player_collection_state_snapshot_like_cpp()?
            .heirlooms_like_cpp()
            .get(&item_id)?
            .flags;
        let active_item_id = i32::try_from(item_id).ok()?;
        let active_offset = hub.core.mutate_canonical_player_like_cpp(|player| {
            player
                .heirlooms_like_cpp()
                .iter()
                .position(|&heirloom_item_id| heirloom_item_id == active_item_id)
        })??;

        let mut flags = current_flags;
        let mut bonus_id = 0_u32;
        for (upgrade_level, &upgrade_item_id) in heirloom.upgrade_item_id.iter().enumerate() {
            if upgrade_item_id == cast_item {
                flags |= 1_u32 << upgrade_level;
                bonus_id = u32::from(heirloom.upgrade_item_bonus_list_id[upgrade_level]);
            }
        }

        let update = hub.core.mutate_canonical_player_like_cpp(|player| {
            player
                .set_heirloom_flags_like_cpp(active_offset, flags)
                .then(|| player.values_update(true))
        })??;

        hub.mutate_player_collection_state_like_cpp(|collections| {
            collections
                .update_heirloom_like_cpp(item_id, flags, bonus_id)
                .then_some(())
        })??;
        Some(update)
    }

    /// C++ `CollectionMgr::SaveAccountToys`.
    pub(crate) fn account_toy_rows_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Vec<(u32, bool, bool)> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .toys_like_cpp()
                    .iter()
                    .map(|(item_id, flags)| {
                        (
                            *item_id,
                            (*flags & TOY_FLAG_FAVORITE_LIKE_CPP) != 0,
                            (*flags & TOY_FLAG_HAS_FANFARE_LIKE_CPP) != 0,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `CollectionMgr::GetAccountToys` full update payload.
    pub(crate) fn account_toy_packet_rows_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Vec<AccountToy> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .toys_like_cpp()
                    .iter()
                    .map(|(item_id, flags)| AccountToy {
                        item_id: *item_id,
                        is_favorite: (flags & TOY_FLAG_FAVORITE_LIKE_CPP) != 0,
                        has_fanfare: (flags & TOY_FLAG_HAS_FANFARE_LIKE_CPP) != 0,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `CollectionMgr::LoadToys` active-player create data order.
    pub(crate) fn account_toy_active_player_rows_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Vec<i32> {
        hub.player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .toys_like_cpp()
                    .keys()
                    .filter_map(|item_id| i32::try_from(*item_id).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// C++ `WorldPackets::Toy::AccountToyUpdate` full login update.
    pub fn send_account_toys_like_cpp(&self, hub: crate::session::HubRef<'_>) {
        hub.core.send_packet(&AccountToyUpdate::full(
            self.account_toy_packet_rows_like_cpp(hub),
        ));
    }

    /// C++ `CollectionMgr::HasToy`.
    pub(crate) fn has_account_toy_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        item_id: u32,
    ) -> bool {
        hub.player_collection_state_snapshot_like_cpp()
            .is_some_and(|collections| collections.toys_like_cpp().contains_key(&item_id))
    }

    /// C++ `CollectionMgr::AddToy` / `UpdateAccountToys`.
    pub(crate) fn add_account_toy_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        item_id: u32,
        is_favorite: bool,
        has_fanfare: bool,
    ) -> bool {
        let mut flags = 0_u32;
        if is_favorite {
            flags |= TOY_FLAG_FAVORITE_LIKE_CPP;
        }
        if has_fanfare {
            flags |= TOY_FLAG_HAS_FANFARE_LIKE_CPP;
        }
        hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.add_toy_like_cpp(item_id, flags)
        })
        .unwrap_or(false)
    }

    /// C++ login learns spell 125610 when battle-pet slot zero is unlocked.
    /// The account-wide owner is authoritative only after its complete load;
    /// isolated tests must explicitly publish the equivalent three-slot
    /// snapshot instead of relying on the constructor's locked defaults.
    pub(in crate::session) fn represented_battle_pet_login_spell_source_is_empty_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> bool {
        let slots = if let Some(attachment) = &self.battle_pet_account_attachment_like_cpp {
            attachment
                .owner_like_cpp()
                .journal_like_cpp(attachment.lease_id_like_cpp(), hub.core.player_guid())
                .slots
        } else {
            #[cfg(not(test))]
            return false;
            #[cfg(test)]
            {
                if !hub
                    .fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_slots_authority_complete_like_cpp
                {
                    return false;
                }
                hub.fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pet_slots_like_cpp
                    .iter()
                    .map(RepresentedBattlePetSlotLikeCpp::packet_slot_like_cpp)
                    .collect()
            }
        };

        slots.len() == BATTLE_PET_SLOT_COUNT_LIKE_CPP
            && slots
                .iter()
                .enumerate()
                .all(|(index, slot)| usize::from(slot.index) == index)
            && slots.first().is_some_and(|slot| slot.locked)
    }

    /// C++ `ScriptMgr::OnPlayerLogin` (`ScriptMgr.cpp:2052-2055`), invoked
    /// once after a completed login (`CharacterHandler.cpp:1452`).
    ///
    /// Modules receive an immutable snapshot and return effects; the batch is
    /// validated as a whole before anything is applied, so an invalid effect
    /// discards the batch instead of half-applying it. A rejected batch is
    /// logged and the login continues: a module must not be able to fail a
    /// player's login.
    pub(crate) fn dispatch_module_player_login_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
        registry: &wow_module_api::ModuleRegistry,
        first_login: bool,
    ) {
        if registry.is_empty() {
            return;
        }
        let Some(guid) = hub.core.player_guid() else {
            return;
        };
        let snapshot = wow_module_api::PlayerLoginSnapshot {
            guid,
            name: hub.player_name_like_cpp().unwrap_or_default(),
            race: hub.player_race_like_cpp(),
            class: hub.player_class_like_cpp(),
            level: hub.player_level_like_cpp(),
            map_id: hub.core.player_map_id_like_cpp(),
            first_login,
        };
        match registry.dispatch_player_login(&snapshot) {
            Ok(effects) => {
                for (module, effect) in effects.iter() {
                    match effect {
                        wow_module_api::PlayerLoginEffect::SendSystemMessageSelf { text } => {
                            debug!(module = %module, "module login message");
                            hub.core.send_system_message_like_cpp(text);
                        }
                    }
                }
            }
            Err(error) => {
                warn!(%error, "module login effect batch rejected; no effect applied");
            }
        }
    }

    pub(crate) fn account_data_like_cpp(&self, data_type: u8) -> Option<&AccountDataLikeCpp> {
        self.account_data_like_cpp.get(usize::from(data_type))
    }

    pub(crate) fn account_data_times_like_cpp(
        &self,
        player_guid: ObjectGuid,
        mask: u32,
    ) -> wow_packet::packets::misc::AccountDataTimes {
        let mut times = [0i64; NUM_ACCOUNT_DATA_TYPES];
        for (index, account_data) in self.account_data_like_cpp.iter().enumerate() {
            if mask & (1u32 << index) != 0 {
                times[index] = account_data.time;
            }
        }

        wow_packet::packets::misc::AccountDataTimes::for_times(player_guid, times)
    }

    pub(crate) fn set_account_data_like_cpp(
        &mut self,
        data_type: u8,
        time: i64,
        data: String,
    ) -> bool {
        let Some(account_data) = self.account_data_like_cpp.get_mut(usize::from(data_type)) else {
            return false;
        };

        account_data.time = time;
        account_data.data = data;
        true
    }

    pub(in crate::session) fn add_account_mount_with_faction_counterpart_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        spell_id: i32,
        flags: u8,
    ) -> bool {
        self.add_account_mount_like_cpp(hub, spell_id, flags, true)
    }

    fn add_account_mount_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
        spell_id: i32,
        flags: u8,
        include_faction_counterpart: bool,
    ) -> bool {
        let Ok(spell_id_u32) = u32::try_from(spell_id) else {
            return false;
        };
        if hub.catalogs.mount_store.as_ref().is_some_and(|store| {
            store
                .get_by_source_spell_id_like_cpp(spell_id_u32)
                .is_none()
        }) {
            return false;
        }

        if include_faction_counterpart
            && let Some(other_faction_spell_id) = hub
                .catalogs
                .mount_definition_store_like_cpp
                .as_ref()
                .and_then(|store| store.other_faction_spell_id_like_cpp(spell_id_u32))
            && let Ok(other_faction_spell_id) = i32::try_from(other_faction_spell_id)
        {
            self.add_account_mount_like_cpp(hub, other_faction_spell_id, flags, false);
        }

        hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.add_mount_like_cpp(spell_id, flags)
        })
        .unwrap_or(false)
    }

    pub(in crate::session) fn expand_account_mount_faction_definitions_like_cpp(
        &mut self,
        hub: &mut crate::session::HubMut<'_>,
    ) {
        let Some(mounts) = hub
            .shared()
            .player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .mounts_snapshot_like_cpp()
                    .into_iter()
                    .collect::<Vec<_>>()
            })
        else {
            return;
        };
        for (spell_id, flags) in mounts {
            self.add_account_mount_with_faction_counterpart_like_cpp(hub, spell_id, flags);
        }
    }

    pub(crate) fn account_mount_rows_like_cpp(
        &self,
        hub: crate::session::HubRef<'_>,
    ) -> Vec<AccountMount> {
        let Some(mut mounts) = hub
            .player_collection_state_snapshot_like_cpp()
            .map(|collections| {
                collections
                    .mounts_like_cpp()
                    .iter()
                    .map(|(spell_id, flags)| AccountMount {
                        spell_id: *spell_id,
                        flags: *flags,
                    })
                    .collect::<Vec<_>>()
            })
        else {
            return Vec::new();
        };
        mounts.sort_by_key(|mount| mount.spell_id);
        mounts
    }

    #[cfg(test)]
    pub(crate) fn represented_at_login_flag_removals_like_cpp(
        &self,
    ) -> &[RepresentedAtLoginFlagRemovalLikeCpp] {
        &self.represented_at_login_flag_removals_like_cpp
    }
}

impl crate::session::state::SessionCore {
    pub fn battlenet_account_id(&self) -> u32 {
        self.account_state.battlenet_account_id
    }

    /// Kick the session (mark as disconnecting).
    pub fn kick(&mut self, reason: &str) {
        warn!(
            "Kicking account {} ({}): {reason}",
            self.account_id, self.account_name
        );
        self.state = SessionState::Disconnecting;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/lifecycle_ops/operations/f3_shims.rs"]
mod f3_shims;
