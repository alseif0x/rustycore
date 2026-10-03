//! Represented load and hydration at the Session boundary.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn mark_represented_void_storage_loaded_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.mark_represented_void_storage_loaded_like_cpp(&mut hub)
    }
    /// Match C++ `Player::LoadFromDB`: locked characters do not consume the
    /// prepared void-storage result, but still own a coherent empty vault that
    /// can be unlocked and saved during this session.
    pub(crate) fn prepare_represented_void_storage_login_load_like_cpp(&mut self) -> bool {
        self.clear_represented_void_storage_like_cpp();
        let should_load_rows = self.void_storage_is_unlocked_like_cpp();
        if !should_load_rows {
            self.mark_represented_void_storage_loaded_like_cpp();
        }
        should_load_rows
    }
    pub(crate) fn load_represented_void_storage_row_like_cpp(
        &mut self,
        slot: u8,
        item: RepresentedVoidStorageItemLikeCpp,
    ) -> bool {
        let slot_index = usize::from(slot);
        if item.item_id == 0
            || slot_index >= wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP
            || self.item_storage_template(item.item_entry).is_none()
        {
            return false;
        }
        let item_entry = item.item_entry;
        let canonical = self.core.with_owned_player_mut_like_cpp(|player| {
            player.load_void_storage_item_like_cpp(slot, item.clone())
        });
        let inserted = match canonical {
            Some(inserted) => inserted,
            None => {
                #[cfg(test)]
                {
                    if self.core.player_handle_like_cpp.is_none() {
                        self.inventory
                            .insert_loaded_void_storage_item_for_test_like_cpp(slot_index, &item)
                    } else {
                        false
                    }
                }
                #[cfg(not(test))]
                {
                    false
                }
            }
        };
        if !inserted {
            return false;
        }
        // C++ `_LoadVoidStorage` initializes `BonusData` from the void item
        // instance and calls `CollectionMgr::AddItemAppearance`; this DB shape
        // only carries the fixed-level modifier, so the effective appearance
        // modifier remains the template default zero.
        let _ = self.add_item_appearance_for_item_like_cpp(item_entry, 0);
        true
    }
    pub async fn load_instance_time_restrictions_like_cpp(&mut self) {
        crate::session::cx_lifecycle(self)
            .load_instance_time_restrictions_like_cpp()
            .await
    }
    pub(crate) fn load_character_reputation_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = wow_progression::mgr::CharacterReputationRowLikeCpp>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_character_reputation_rows_like_cpp(&mut hub, rows)
    }
    /// C++ `Player::_LoadGroup` overwrites the loaded player difficulties with
    /// the current group values because the leader may change them while the
    /// member is offline.
    pub(crate) fn load_represented_group_difficulties_like_cpp(&mut self) -> bool {
        let (Some(group_guid), Some(group_registry)) = (
            self.resolved_group_guid_like_cpp(),
            self.core.directory.group_registry.as_ref(),
        ) else {
            return false;
        };

        let preferences = {
            let Some(group) = group_registry.get(&group_guid) else {
                return false;
            };
            (
                group.dungeon_difficulty_id,
                group.raid_difficulty_id,
                group.legacy_raid_difficulty_id,
            )
        };

        self.replace_player_difficulty_preferences_like_cpp(
            preferences.0,
            preferences.1,
            preferences.2,
        )
    }
    /// C++ `Player::SetGroup(group, subgroup)` stores the subgroup on the
    /// player's `GroupReference`; `Player::GetSubGroup()` reads it from there.
    pub(crate) fn load_represented_group_subgroup_like_cpp(&mut self) -> bool {
        let (Some(group_guid), Some(player_guid), Some(group_registry)) = (
            self.resolved_group_guid_like_cpp(),
            self.player_guid(),
            self.core.directory.group_registry.as_ref(),
        ) else {
            let _ = self.set_owned_player_group_like_cpp(None);
            return false;
        };

        let Some(group) = group_registry.get(&group_guid) else {
            let _ = self.set_owned_player_group_like_cpp(None);
            return false;
        };
        let Some(slot) = group.member_slot_like_cpp(player_guid) else {
            let _ = self.set_owned_player_group_like_cpp(None);
            return false;
        };

        self.set_owned_player_group_like_cpp(Some((group_guid, slot.subgroup)))
    }
    /// C++ `Player::_LoadGroup` resolves `CHAR_SEL_GROUP_MEMBER.guid` through
    /// `sGroupMgr->GetGroupByDbStoreId` before attaching the player to the
    /// already-loaded group.
    pub(crate) fn load_represented_group_by_db_store_id_like_cpp(
        &mut self,
        db_store_id: u32,
    ) -> bool {
        let Some(group_registry) = self.core.directory.group_registry.as_ref() else {
            let _ = self.set_owned_player_group_like_cpp(None);
            return false;
        };
        let Some(group_guid) = group_guid_by_db_store_id_like_cpp(db_store_id) else {
            let _ = self.set_owned_player_group_like_cpp(None);
            return false;
        };
        if !group_registry.contains_key(&group_guid) {
            let _ = self.set_owned_player_group_like_cpp(None);
            return false;
        }

        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(subgroup) = group_registry.get(&group_guid).and_then(|group| {
            group
                .member_slot_like_cpp(player_guid)
                .map(|slot| slot.subgroup)
        }) else {
            let _ = self.set_owned_player_group_like_cpp(None);
            return false;
        };
        if !self.set_owned_player_group_like_cpp(Some((group_guid, subgroup))) {
            return false;
        }
        self.load_represented_group_difficulties_like_cpp()
    }
    pub(crate) fn load_represented_xp_rest_bonus_like_cpp(
        &mut self,
        rest_state: u8,
        rest_bonus: f32,
    ) {
        // C++ `RestMgr::LoadRestBonus` restores both DB values verbatim. It does
        // not clamp or recompute the state until a later `AddRestBonus` reaches
        // `SetRestBonus` (for example when offline time is applied).
        // C++-created rows only contain the declared PlayerRestState values.
        // Normalize legacy Rust rows that persisted the old invalid value 0,
        // while preserving every valid DB state verbatim like LoadRestBonus.
        let rest_state = if wow_entities::valid_player_rest_state_like_cpp(rest_state) {
            rest_state
        } else {
            REST_STATE_NORMAL_LIKE_CPP
        };
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.clear_represented_rest_flags_for_character_load_like_cpp();
            let _ = self.mutate_player_rest_state_like_cpp(|state| {
                state.install_loaded_rest_like_cpp(rest_state, rest_bonus);
            });
            return;
        }
        let _ = self.core.with_owned_player_mut_like_cpp(|player| {
            player.load_xp_rest_bonus_like_cpp(rest_state, rest_bonus);
        });
    }
    #[cfg(test)]
    fn clear_represented_rest_flags_for_character_load_like_cpp(&mut self) {
        let loaded_resting = self
            .core
            .canonical_player_snapshot_like_cpp(|player| player.data().player_flags)
            .is_some_and(|flags| (flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0);
        let _canonical = self.with_owned_player_mut_for_rest_like_cpp(|player| {
            let mut state = player.rest_state_like_cpp().clone();
            state.reset_location_tracking_like_cpp();
            player.replace_rest_state_like_cpp(state);
            if loaded_resting {
                player.set_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
            } else {
                player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
            }
        });
        #[cfg(test)]
        if _canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_flag_mask_like_cpp = 0;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_location_initialized_like_cpp = false;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_defer_rest_flag_sync_like_cpp = false;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_deferred_rest_flag_update_dirty_like_cpp = false;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_inn_area_trigger_id_like_cpp = 0;
            self.fixtures
                .progression
                .rest_mgr_test_fixture_like_cpp
                .represented_rest_time_secs_like_cpp = 0;
        }
    }
    pub(crate) fn load_represented_explored_zones_like_cpp(&mut self, input: &str) -> usize {
        let blocks = parse_explored_zones_db_string_like_cpp(input);
        let Some(previous) = self.player_explored_zones_snapshot_like_cpp() else {
            return 0;
        };
        let changed = previous != blocks;

        let canonical = self
            .core
            .mutate_canonical_player_like_cpp(|player| {
                let applied = player.set_explored_zones_blocks_like_cpp(&blocks);
                (applied > 0).then(|| player.values_update(true))
            })
            .flatten();
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            self.instances
                .replace_represented_explored_zones_for_test_like_cpp(blocks);
        }
        if let Some(update) = canonical {
            self.core.send_player_values_update_like_cpp(&update);
        }

        if changed {
            blocks.iter().filter(|block| **block != 0).count()
        } else {
            0
        }
    }
    pub(crate) fn mark_represented_talents_loaded_like_cpp(&mut self) {
        let _ = crate::session::hub_mut(self).mark_talents_loaded_like_cpp();
        self.refresh_represented_talent_points_like_cpp();
    }
    pub(crate) fn represented_talents_loaded_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.represented_talents_loaded_like_cpp(hub)
    }
    pub(crate) fn clamp_loaded_player_xp_to_next_level_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.clamp_loaded_player_xp_to_next_level_like_cpp(&mut hub)
    }
    /// Set the player loading GUID (ConnectTo flow).
    pub fn set_player_loading(&mut self, guid: Option<ObjectGuid>) {
        self.lifecycle.set_player_loading(guid);
        self.sync_current_player_session_visibility_detection_like_cpp();
    }
    pub fn player_loading(&self) -> Option<ObjectGuid> {
        self.lifecycle.player_loading()
    }
    pub async fn load_tutorials_data_like_cpp(&mut self) {
        self.lifecycle.begin_tutorials_load_like_cpp();

        let Some(port) = self
            .lifecycle
            .session_account_state_port_like_cpp()
            .map(std::sync::Arc::clone)
        else {
            warn!(
                account = self.core.account_id,
                "LoadTutorialsData skipped: session account-state port unavailable"
            );
            return;
        };

        match port.load_tutorials_like_cpp(self.core.account_id).await {
            wow_persistence::SessionTutorialsLoadOutcomeLikeCpp::Loaded(values) => {
                self.lifecycle.load_tutorials_data_values_like_cpp(values);
            }
            wow_persistence::SessionTutorialsLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.core.account_id,
                    "LoadTutorialsData query failed: {reason}"
                );
            }
        }
    }
    pub async fn load_global_account_data_like_cpp(&mut self) {
        self.load_account_data_like_cpp(ObjectGuid::EMPTY, GLOBAL_CACHE_MASK_LIKE_CPP)
            .await;
    }
    pub async fn load_player_account_data_like_cpp(&mut self, guid: ObjectGuid) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .load_player_account_data_like_cpp(&mut hub, guid)
            .await
    }
    async fn load_account_data_like_cpp(&mut self, guid: ObjectGuid, mask: u32) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_account_data_like_cpp(&mut hub, guid, mask).await
    }
    pub(crate) fn set_loaded_player_name_like_cpp(&mut self, name: String) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_name_like_cpp(&mut hub, name)
    }
    pub(crate) fn set_loaded_player_identity_like_cpp(
        &mut self,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) {
        let initialize_reputation = crate::session::hub_ref(self).player_race_like_cpp() != race
            || crate::session::hub_ref(self).player_class_like_cpp() != class
            || self
                .core
                .with_owned_player_like_cpp(|player| {
                    player.reputation_like_cpp().faction_count_like_cpp() == 0
                })
                .unwrap_or(true);
        if self.core.player_map_id_like_cpp() != map_id
            || crate::session::hub_ref(self).player_race_like_cpp() != race
            || crate::session::hub_ref(self).player_class_like_cpp() != class
            || crate::session::hub_ref(self).player_gender_like_cpp() != gender
        {
            self.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.core.current_map_id = map_id;
        let gray_level = crate::session::hub_ref(self).gray_level(level);
        let canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_race_class_gender(race, class, crate::session::gender_from_u8(gender));
                player.set_level_and_gray_level_like_cpp(level, gray_level);
            })
            .is_some();
        if canonical {
            self.core.player_identity_bootstrap_like_cpp = None;
        } else {
            #[cfg(not(test))]
            if self.core.player_handle_like_cpp.is_some() {
                return;
            }
            self.core.player_identity_bootstrap_like_cpp =
                Some(super::PlayerIdentityBootstrapLikeCpp {
                    name: crate::session::hub_ref(self).player_name_like_cpp(),
                    race,
                    class,
                    level,
                    gender,
                });
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                self.fixtures.identity.player_race = race;
                self.fixtures.identity.player_class = class;
                self.fixtures.identity.player_level = level;
                self.fixtures.identity.player_gender = gender;
            }
        }
        crate::session::hub_mut(self).set_player_faction_for_race_like_cpp(race);
        if initialize_reputation {
            crate::session::hub_mut(self).initialize_reputation_mgr_like_cpp();
        }
        self.refresh_represented_talent_points_like_cpp();
    }
    pub(crate) fn set_loaded_player_flags_like_cpp(&mut self, player_flags: u32) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_flags_like_cpp(&mut hub, player_flags)
    }
    pub(crate) fn set_loaded_player_flags_ex_like_cpp(&mut self, player_flags_ex: u32) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_flags_ex_like_cpp(&mut hub, player_flags_ex)
    }
    pub(crate) fn set_loaded_player_powers_like_cpp(
        &mut self,
        powers: [i32; MAX_POWERS_PER_CLASS],
    ) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_powers_like_cpp(&mut hub, powers)
    }
    pub(crate) fn loaded_action_buttons_snapshot_like_cpp(
        &self,
    ) -> Option<[u32; wow_packet::packets::misc::MAX_ACTION_BUTTONS]> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| {
                player
                    .action_buttons_loaded_like_cpp()
                    .then(|| player.action_buttons_snapshot_like_cpp())
            })
            .flatten();
        #[cfg(test)]
        if canonical.is_none()
            && self.core.player_handle_like_cpp.is_none()
            && self
                .fixtures
                .presentation
                .represented_action_buttons_loaded_like_cpp
        {
            return Some(
                self.fixtures
                    .presentation
                    .represented_action_buttons_like_cpp,
            );
        }
        canonical
    }
    pub(crate) fn load_represented_cuf_profile_like_cpp(
        &mut self,
        id: u8,
        profile: wow_packet::packets::misc::CufProfile,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_represented_cuf_profile_like_cpp(&mut hub, id, profile)
    }
    pub(crate) fn represented_load_cuf_profiles_packet_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::LoadCufProfiles> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.represented_load_cuf_profiles_packet_like_cpp(hub)
    }
    pub(crate) fn set_loaded_player_customizations_like_cpp(
        &mut self,
        customizations: Vec<wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate>,
    ) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.set_loaded_player_customizations_like_cpp(&mut hub, customizations)
    }
    /// C++ `Player::LoadFromDB` parses `knownTitles` as 32-bit words and
    /// validates `chosenTitle` against `Player::HasTitle` before setting it.
    pub(crate) fn load_represented_character_titles_like_cpp(
        &mut self,
        known_titles: &str,
        chosen_title: u32,
    ) {
        let mut known_title_ids = BTreeSet::new();
        for (word_index, token) in known_titles.split_whitespace().enumerate() {
            let word = token.parse::<u64>().unwrap_or(0) & u64::from(u32::MAX);
            for bit_index in 0..32_u32 {
                if (word & (1_u64 << bit_index)) != 0 {
                    known_title_ids.insert((word_index as u32) * 32 + bit_index);
                }
            }
        }

        let chosen_title = if chosen_title != 0 && !known_title_ids.contains(&chosen_title) {
            0
        } else {
            chosen_title
        };
        let chosen_title = i32::try_from(chosen_title).unwrap_or(0);
        let _canonical = self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_known_titles_like_cpp(known_title_ids.clone());
                player.set_chosen_title_like_cpp(chosen_title);
            })
            .is_some();
        #[cfg(test)]
        if !_canonical && self.core.player_handle_like_cpp.is_none() {
            self.quest_state
                .quest_test_fixture_like_cpp
                .represented_known_titles_like_cpp = known_title_ids.into_iter().collect();
            self.quest_state
                .quest_test_fixture_like_cpp
                .represented_chosen_title_like_cpp = chosen_title;
        }
    }
}

impl crate::session::LifecycleCx<'_> {
    #[cfg(test)]
    pub(in crate::session) fn load_instance_time_restriction_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = (u32, u64)>,
    ) {
        self.instances
            .replace_represented_instance_reset_times_for_test_like_cpp(rows);
    }

    pub async fn load_instance_time_restrictions_like_cpp(&mut self) {
        let _ = self
            .instances
            .replace_instance_reset_times_like_cpp(&mut self.hub, []);

        let Some(port) = self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
            warn!(
                account = self.hub.core.account_id,
                "LoadInstanceTimeRestrictions skipped: Player lifecycle port unavailable"
            );
            return;
        };

        let rows = match port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions {
                    account_id: self.hub.core.account_id,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::InstanceTimeRestrictions(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.hub.core.account_id,
                    "LoadInstanceTimeRestrictions query failed: {reason}"
                );
                return;
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.hub.core.account_id,
                    "Player lifecycle port returned the wrong auxiliary login data for instance time restrictions"
                );
                return;
            }
        };

        let _ = self.instances.replace_instance_reset_times_like_cpp(
            &mut self.hub,
            rows.into_iter()
                .map(|row| (row.instance_id, row.release_time)),
        );
    }

    /// C++ `Player::LoadFromDB` applies `CheckLoaded*DifficultyID` to the raw
    /// `characters` columns before any login packets are sent.
    pub(crate) fn load_represented_player_difficulties_like_cpp(
        &mut self,
        dungeon_difficulty_id: u32,
        raid_difficulty_id: u32,
        legacy_raid_difficulty_id: u32,
    ) {
        let Some(store) = self.hub.catalogs.difficulty_store() else {
            let _ = self
                .instances
                .replace_player_difficulty_preferences_like_cpp(
                    &mut self.hub,
                    DIFFICULTY_NORMAL_LIKE_CPP,
                    DIFFICULTY_NORMAL_RAID_LIKE_CPP,
                    DIFFICULTY_10_N_LIKE_CPP,
                );
            return;
        };

        let dungeon_difficulty_id =
            store.check_loaded_dungeon_difficulty_id_like_cpp(dungeon_difficulty_id);
        let raid_difficulty_id = store.check_loaded_raid_difficulty_id_like_cpp(raid_difficulty_id);
        let legacy_raid_difficulty_id =
            store.check_loaded_legacy_raid_difficulty_id_like_cpp(legacy_raid_difficulty_id);

        let _ = self
            .instances
            .replace_player_difficulty_preferences_like_cpp(
                &mut self.hub,
                dungeon_difficulty_id,
                raid_difficulty_id,
                legacy_raid_difficulty_id,
            );
    }
}

impl crate::session::LifecycleCxRef<'_> {
    pub(crate) fn represented_void_storage_loaded_like_cpp(&self) -> Option<bool> {
        self.inventory
            .with_owned_void_storage_like_cpp(self.hub, |_, loaded| loaded)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/persistence/load/f3_shims.rs"]
mod f3_shims;
