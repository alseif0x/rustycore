//! Represented load and hydration at the Session boundary.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn mark_represented_void_storage_loaded_like_cpp(&mut self) {
        let _ = self.with_owned_void_storage_mut_like_cpp(|_, loaded| *loaded = true);
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
        let slot = usize::from(slot);
        if item.item_id == 0
            || slot >= wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP
            || self.item_storage_template(item.item_entry).is_none()
        {
            return false;
        }
        let item_entry = item.item_entry;
        let inserted = self
            .with_owned_void_storage_mut_like_cpp(|items, _| {
                if items[slot].is_some()
                    || items
                        .iter()
                        .flatten()
                        .any(|loaded| loaded.item_id == item.item_id)
                {
                    return false;
                }
                items[slot] = Some(item.clone());
                true
            })
            .unwrap_or(false);
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
    pub(crate) fn represented_void_storage_loaded_like_cpp(&self) -> Option<bool> {
        self.with_owned_void_storage_like_cpp(|_, loaded| loaded)
    }
    #[cfg(test)]
    pub(in crate::session) fn load_instance_time_restriction_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = (u32, u64)>,
    ) {
        self.represented_instance_reset_times_like_cpp.clear();
        for (instance_id, release_time) in rows {
            self.represented_instance_reset_times_like_cpp
                .entry(instance_id)
                .or_insert(release_time);
        }
    }
    pub async fn load_instance_time_restrictions_like_cpp(&mut self) {
        self.represented_instance_reset_times_like_cpp.clear();

        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            warn!(
                account = self.account_id,
                "LoadInstanceTimeRestrictions skipped: Player lifecycle port unavailable"
            );
            return;
        };

        let rows = match port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions {
                    account_id: self.account_id,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::InstanceTimeRestrictions(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    "LoadInstanceTimeRestrictions query failed: {reason}"
                );
                return;
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.account_id,
                    "Player lifecycle port returned the wrong auxiliary login data for instance time restrictions"
                );
                return;
            }
        };

        for row in rows {
            self.represented_instance_reset_times_like_cpp
                .entry(row.instance_id)
                .or_insert(row.release_time);
        }
    }
    /// C++ `CollectionMgr::LoadAccountHeirlooms`.
    pub(crate) fn load_represented_account_heirlooms_like_cpp(
        &mut self,
        heirloom_rows: impl IntoIterator<Item = (u32, u32)>,
    ) {
        let mut heirlooms = BTreeMap::new();
        for (item_id, flags) in heirloom_rows {
            let bonus_id = match self.heirloom_store.as_ref() {
                Some(store) => {
                    let Some(heirloom) = store.get_by_item_id_like_cpp(item_id) else {
                        continue;
                    };
                    heirloom_bonus_for_flags_like_cpp(heirloom, flags)
                }
                None => 0,
            };
            heirlooms.insert(item_id, AccountHeirloomDataLikeCpp { flags, bonus_id });
        }
        let _ = self.mutate_player_collection_state_like_cpp(|collections| {
            collections.heirlooms = heirlooms;
        });
    }
    /// C++ `CollectionMgr::LoadAccountToys`.
    pub(crate) fn load_represented_account_toys_like_cpp(
        &mut self,
        toy_rows: impl IntoIterator<Item = (u32, bool, bool)>,
    ) {
        let mut toys = BTreeMap::new();
        for (item_id, is_favorite, has_fanfare) in toy_rows {
            let mut flags = 0_u32;
            if is_favorite {
                flags |= TOY_FLAG_FAVORITE_LIKE_CPP;
            }
            if has_fanfare {
                flags |= TOY_FLAG_HAS_FANFARE_LIKE_CPP;
            }
            toys.insert(item_id, flags);
        }
        let _ = self.mutate_player_collection_state_like_cpp(|collections| {
            collections.toys = toys;
        });
    }
    pub(crate) fn load_character_reputation_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = crate::reputation::mgr::CharacterReputationRowLikeCpp>,
    ) -> bool {
        let Some(faction_store) = self.faction_store().cloned() else {
            return false;
        };
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().cloned();
        let paragon_reputation_store = self.paragon_reputation_store.as_ref().cloned();
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();

        self.mutate_reputation_mgr_like_cpp(|mgr| {
            mgr.load_from_db_like_cpp(
                rows,
                faction_store.as_ref(),
                friendship_rep_reaction_store.as_deref(),
                paragon_reputation_store.as_deref(),
                race,
                class,
            );
        })
        .is_some()
    }
    /// C++ `Player::LoadFromDB` applies `CheckLoaded*DifficultyID` to the raw
    /// `characters` columns before any login packets are sent.
    pub(crate) fn load_represented_player_difficulties_like_cpp(
        &mut self,
        dungeon_difficulty_id: u32,
        raid_difficulty_id: u32,
        legacy_raid_difficulty_id: u32,
    ) {
        let Some(store) = self.difficulty_store() else {
            let _ = self.replace_player_difficulty_preferences_like_cpp(
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

        let _ = self.replace_player_difficulty_preferences_like_cpp(
            dungeon_difficulty_id,
            raid_difficulty_id,
            legacy_raid_difficulty_id,
        );
    }
    /// C++ `Player::_LoadGroup` overwrites the loaded player difficulties with
    /// the current group values because the leader may change them while the
    /// member is offline.
    pub(crate) fn load_represented_group_difficulties_like_cpp(&mut self) -> bool {
        let (Some(group_guid), Some(group_registry)) = (
            self.resolved_group_guid_like_cpp(),
            self.group_registry.as_ref(),
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
            self.group_registry.as_ref(),
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
        let Some(group_registry) = self.group_registry.as_ref() else {
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
        let rest_state = if crate::session_rules::valid_player_rest_state_like_cpp(rest_state) {
            rest_state
        } else {
            REST_STATE_NORMAL_LIKE_CPP
        };
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.clear_represented_rest_flags_for_character_load_like_cpp();
            let _ = self.mutate_player_rest_state_like_cpp(|state| {
                state.rest_state = rest_state;
                state.rest_bonus = rest_bonus;
            });
            return;
        }
        let _ = self.with_owned_player_mut_like_cpp(|player| {
            player.load_xp_rest_bonus_like_cpp(rest_state, rest_bonus);
        });
    }
    #[cfg(test)]
    fn clear_represented_rest_flags_for_character_load_like_cpp(&mut self) {
        let loaded_resting = self
            .canonical_player_snapshot_like_cpp(|player| player.data().player_flags)
            .is_some_and(|flags| (flags & PLAYER_FLAGS_RESTING_LIKE_CPP) != 0);
        let _canonical = self.with_owned_player_mut_for_rest_like_cpp(|player| {
            let mut state = player.rest_state_like_cpp().clone();
            state.rest_flag_mask = 0;
            state.location_initialized = false;
            state.defer_flag_sync = false;
            state.deferred_flag_update_dirty = false;
            state.inn_area_trigger_id = 0;
            state.rest_time_secs = 0;
            player.replace_rest_state_like_cpp(state);
            if loaded_resting {
                player.set_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
            } else {
                player.remove_player_flag(PLAYER_FLAGS_RESTING_LIKE_CPP);
            }
        });
        #[cfg(test)]
        if _canonical.is_none() && self.player_handle_like_cpp.is_none() {
            self.represented_rest_flag_mask_like_cpp = 0;
            self.represented_rest_location_initialized_like_cpp = false;
            self.represented_defer_rest_flag_sync_like_cpp = false;
            self.represented_deferred_rest_flag_update_dirty_like_cpp = false;
            self.represented_inn_area_trigger_id_like_cpp = 0;
            self.represented_rest_time_secs_like_cpp = 0;
        }
    }
    pub(crate) fn mark_represented_glyphs_loaded_like_cpp(&mut self) {
        let _ = self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.glyphs_loaded = true;
        });
    }
    pub(crate) fn load_represented_explored_zones_like_cpp(&mut self, input: &str) -> usize {
        let blocks = parse_explored_zones_db_string_like_cpp(input);
        let Some(previous) = self.player_explored_zones_snapshot_like_cpp() else {
            return 0;
        };
        let changed = previous != blocks;

        let canonical = self
            .mutate_canonical_player_like_cpp(|player| {
                let applied = player.set_explored_zones_blocks_like_cpp(&blocks);
                (applied > 0).then(|| player.values_update(true))
            })
            .flatten();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_explored_zones_like_cpp = blocks;
        }
        if let Some(update) = canonical {
            self.send_player_values_update_like_cpp(&update);
        }

        if changed {
            blocks.iter().filter(|block| **block != 0).count()
        } else {
            0
        }
    }
    pub(crate) fn mark_represented_talents_loaded_like_cpp(&mut self) {
        let _ = self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.talents_loaded = true;
        });
        self.refresh_represented_talent_points_like_cpp();
    }
    pub(crate) fn represented_talents_loaded_like_cpp(&self) -> bool {
        self.player_talent_runtime_snapshot_like_cpp()
            .is_some_and(|runtime| runtime.talents_loaded)
    }
    pub(crate) fn load_represented_talent_row_like_cpp(
        &mut self,
        talent_tabs: &TalentTabStore,
        talent_id: u32,
        rank: u8,
        talent_group: u8,
    ) -> bool {
        let talent_group_index = usize::from(talent_group);
        if talent_group_index >= MAX_SPECIALIZATIONS_LIKE_CPP {
            return false;
        }

        let Some(talent) = self
            .talent_store()
            .and_then(|store| store.get(talent_id))
            .cloned()
        else {
            return false;
        };

        let Some(talent_tab) = talent_tabs.get(u32::from(talent.tab_id)) else {
            return false;
        };

        let Some(class_mask) = player_class_mask_for_talent_like_cpp(self.player_class_like_cpp())
        else {
            return false;
        };

        let Ok(talent_class_mask) = u32::try_from(talent_tab.class_mask) else {
            return false;
        };
        if (class_mask & talent_class_mask) == 0 {
            return false;
        }

        let rank_index = usize::from(rank);
        let Some(spell_id) = talent.spell_rank.get(rank_index).copied() else {
            return false;
        };
        if spell_id <= 0 {
            return false;
        }

        if !self.represented_spell_valid_for_talent_like_cpp(spell_id) {
            return false;
        }

        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.talent_groups[talent_group_index].insert(talent_id, rank);
        })
        .is_some()
    }
    /// Borrow the required process catalog; tests supply explicit fixture data.
    pub(crate) fn load_represented_glyph_row_like_cpp(
        &mut self,
        glyph_properties: &GlyphPropertiesStore,
        talent_group: u8,
        glyph_slot: u8,
        glyph_id: u16,
    ) -> bool {
        let talent_group_index = usize::from(talent_group);
        if talent_group_index >= MAX_SPECIALIZATIONS_LIKE_CPP {
            return false;
        }

        let glyph_slot_index = usize::from(glyph_slot);
        if glyph_slot_index >= wow_packet::packets::misc::MAX_GLYPH_SLOT_INDEX_LIKE_CPP {
            return false;
        }

        if glyph_id != 0 && glyph_properties.get(u32::from(glyph_id)).is_none() {
            return false;
        }

        let previous = self
            .player_talent_runtime_snapshot_like_cpp()
            .map(|runtime| runtime.glyph_groups[talent_group_index][glyph_slot_index]);
        if previous != Some(glyph_id) {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.glyph_groups[talent_group_index][glyph_slot_index] = glyph_id;
        })
        .is_some()
    }
    /// C++ `Player::InitStatsForLevel` repairs an invalid persisted XP value
    /// after deriving `ActivePlayerData::NextLevelXP` for the loaded level.
    pub(crate) fn clamp_loaded_player_xp_to_next_level_like_cpp(&mut self) {
        let (Some(player_xp), Some(next_level_xp)) = (
            self.resolved_player_xp_like_cpp(),
            self.resolved_player_next_level_xp_like_cpp(),
        ) else {
            return;
        };
        if player_xp >= next_level_xp {
            self.set_player_xp_like_cpp(next_level_xp.saturating_sub(1));
        }
    }
    /// Set the player loading GUID (ConnectTo flow).
    pub fn set_player_loading(&mut self, guid: Option<ObjectGuid>) {
        self.player_loading = guid;
        self.sync_current_player_session_visibility_detection_like_cpp();
    }
    /// Get the player loading GUID.
    pub fn player_loading(&self) -> Option<ObjectGuid> {
        self.player_loading
    }
    pub(crate) fn load_tutorials_data_values_like_cpp(&mut self, values: Option<[u32; 8]>) {
        self.tutorials_like_cpp = values.unwrap_or([0; 8]);
        self.tutorials_loaded_from_db_like_cpp = values.is_some();
        self.tutorials_loaded_coherently_like_cpp = true;
        self.tutorials_changed_like_cpp = false;
    }
    pub async fn load_tutorials_data_like_cpp(&mut self) {
        self.tutorials_like_cpp = [0; 8];
        self.tutorials_loaded_from_db_like_cpp = false;
        self.tutorials_loaded_coherently_like_cpp = false;
        self.tutorials_changed_like_cpp = false;

        let Some(port) = self
            .persistence_ports_like_cpp
            .admission
            .session_account_state
            .clone()
        else {
            warn!(
                account = self.account_id,
                "LoadTutorialsData skipped: session account-state port unavailable"
            );
            return;
        };

        match port.load_tutorials_like_cpp(self.account_id).await {
            wow_persistence::SessionTutorialsLoadOutcomeLikeCpp::Loaded(values) => {
                self.load_tutorials_data_values_like_cpp(values);
            }
            wow_persistence::SessionTutorialsLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
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
        self.load_account_data_like_cpp(guid, PER_CHARACTER_CACHE_MASK_LIKE_CPP)
            .await;
    }
    async fn load_account_data_like_cpp(&mut self, guid: ObjectGuid, mask: u32) {
        debug_assert_eq!(
            GLOBAL_CACHE_MASK_LIKE_CPP | PER_CHARACTER_CACHE_MASK_LIKE_CPP,
            ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP
        );

        for index in 0..NUM_ACCOUNT_DATA_TYPES {
            if mask & (1u32 << index) != 0 {
                self.account_data_like_cpp[index] = AccountDataLikeCpp::default();
            }
        }

        let scope = if mask == GLOBAL_CACHE_MASK_LIKE_CPP {
            wow_persistence::SessionAccountDataScopeLikeCpp::Global {
                account_id: self.account_id,
            }
        } else {
            wow_persistence::SessionAccountDataScopeLikeCpp::Character {
                guid_low: guid.counter() as u64,
            }
        };

        let Some(port) = self
            .persistence_ports_like_cpp
            .admission
            .session_account_state
            .clone()
        else {
            warn!(
                account = self.account_id,
                mask, "LoadAccountData skipped: session account-state port unavailable"
            );
            return;
        };

        let rows = match port.load_account_data_like_cpp(scope).await {
            wow_persistence::SessionAccountDataLoadOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::SessionAccountDataLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.account_id,
                    mask, "LoadAccountData query failed: {reason}"
                );
                return;
            }
        };

        let table_name = if mask == GLOBAL_CACHE_MASK_LIKE_CPP {
            "account_data"
        } else {
            "character_account_data"
        };
        for row in rows {
            let data_type = row.data_type;
            if usize::from(data_type) >= NUM_ACCOUNT_DATA_TYPES {
                warn!(
                    table = table_name,
                    data_type, "LoadAccountData ignored invalid account data type like C++"
                );
            } else if mask & (1u32 << data_type) == 0 {
                warn!(
                    table = table_name,
                    data_type,
                    "LoadAccountData ignored account data type inappropriate for table like C++"
                );
            } else {
                self.account_data_like_cpp[usize::from(data_type)].time = row.time;
                self.account_data_like_cpp[usize::from(data_type)].data = row.data;
            }
        }
    }
    pub(crate) fn set_loaded_player_name_like_cpp(&mut self, name: String) {
        self.player_name = Some(name);
    }
    pub(crate) fn set_loaded_player_identity_like_cpp(
        &mut self,
        map_id: u16,
        race: u8,
        class: u8,
        level: u8,
        gender: u8,
    ) {
        let initialize_reputation = self.player_race_like_cpp() != race
            || self.player_class_like_cpp() != class
            || self
                .with_owned_player_like_cpp(|player| player.gameplay_state().reputations.is_empty())
                .unwrap_or(true);
        if self.player_map_id_like_cpp() != map_id
            || self.player_race_like_cpp() != race
            || self.player_class_like_cpp() != class
            || self.player_gender_like_cpp() != gender
        {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        self.current_map_id = map_id;
        self.player_race = race;
        self.player_class = class;
        self.player_level = level;
        self.player_gender = gender;
        self.set_player_faction_for_race_like_cpp(race);
        if initialize_reputation {
            self.initialize_reputation_mgr_like_cpp();
        }
        self.refresh_represented_talent_points_like_cpp();
    }
    pub(crate) fn set_loaded_player_flags_like_cpp(&mut self, player_flags: u32) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let _canonical = self
            .mutate_canonical_player_like_cpp(|player| {
                player.replace_all_player_flags(player_flags)
            })
            .is_some();
        #[cfg(test)]
        {
            self.represented_loaded_player_flags_like_cpp = Some(player_flags);
            self.represented_loaded_player_flags_ex_like_cpp
                .get_or_insert(0);
            self.represented_loaded_player_flags_applied_like_cpp = _canonical;
        }
    }
    pub(crate) fn set_loaded_player_flags_ex_like_cpp(&mut self, player_flags_ex: u32) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let _canonical = self
            .mutate_canonical_player_like_cpp(|player| {
                player.replace_all_player_flags_ex(player_flags_ex)
            })
            .is_some();
        #[cfg(test)]
        {
            self.represented_loaded_player_flags_ex_like_cpp = Some(player_flags_ex);
            self.represented_loaded_player_flags_applied_like_cpp = _canonical;
        }
    }
    #[cfg(test)]
    pub(crate) fn apply_loaded_player_flags_to_canonical_like_cpp(&mut self) {
        let Some(player_flags) = self.represented_loaded_player_flags_like_cpp else {
            return;
        };
        let player_flags_ex = self
            .represented_loaded_player_flags_ex_like_cpp
            .unwrap_or(0);
        if self
            .mutate_canonical_player_like_cpp(|player| {
                player.replace_all_player_flags(player_flags);
                player.replace_all_player_flags_ex(player_flags_ex);
            })
            .is_some()
        {
            self.represented_loaded_player_flags_applied_like_cpp = true;
        }
    }
    pub(crate) fn set_loaded_player_powers_like_cpp(
        &mut self,
        powers: [i32; MAX_POWERS_PER_CLASS],
    ) {
        let _canonical = self.with_owned_player_mut_for_power_like_cpp(|player| {
            let max_power = player.unit().data().max_power;
            player
                .unit_mut()
                .replace_create_power_arrays_like_cpp(powers.map(|value| value.max(0)), max_power);
        });
        #[cfg(test)]
        if _canonical.is_some() || self.player_handle_like_cpp.is_none() {
            self.represented_player_powers_like_cpp =
                loaded_character_power_snapshot_like_cpp(powers);
        }
    }
    pub(crate) fn resolved_player_skill_records_loaded_like_cpp(&self) -> Option<bool> {
        let canonical = self.with_owned_player_like_cpp(Player::skill_records_loaded_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.player_skill_records_loaded_like_cpp);
        }
        canonical
    }
    #[cfg(test)]
    pub(crate) fn player_skill_records_loaded_like_cpp(&self) -> bool {
        self.resolved_player_skill_records_loaded_like_cpp()
            .expect("test Player skill owner must resolve")
    }
    pub(crate) fn mark_represented_action_buttons_loaded_like_cpp(&mut self) {
        let _canonical = self
            .with_owned_player_mut_like_cpp(Player::mark_action_buttons_loaded_like_cpp)
            .is_some();
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_action_buttons_loaded_like_cpp = true;
        }
    }
    pub(crate) fn loaded_action_buttons_snapshot_like_cpp(
        &self,
    ) -> Option<[u32; wow_packet::packets::misc::MAX_ACTION_BUTTONS]> {
        let canonical = self
            .with_owned_player_like_cpp(|player| {
                player
                    .action_buttons_loaded_like_cpp()
                    .then(|| player.action_buttons_snapshot_like_cpp())
            })
            .flatten();
        #[cfg(test)]
        if canonical.is_none()
            && self.player_handle_like_cpp.is_none()
            && self.represented_action_buttons_loaded_like_cpp
        {
            return Some(self.represented_action_buttons_like_cpp);
        }
        canonical
    }
    pub(crate) fn record_loaded_action_button_like_cpp(
        &mut self,
        index: u8,
        action: u32,
        action_type: u8,
    ) -> bool {
        self.represented_set_action_button_like_cpp(
            index,
            make_action_button_like_cpp(action, action_type),
        )
    }
    pub(crate) fn mark_represented_cuf_profiles_loaded_like_cpp(&mut self) {
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            player.gameplay_state_mut().cuf_profiles_loaded = true;
        });
        if canonical.is_some() {
            return;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.cuf_profiles_loaded_like_cpp = true;
        }
    }
    pub(crate) fn load_represented_cuf_profile_like_cpp(
        &mut self,
        id: u8,
        profile: wow_packet::packets::misc::CufProfile,
    ) -> bool {
        let index = usize::from(id);
        if index >= wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP {
            return false;
        }

        #[cfg(test)]
        let fixture_profile = profile.clone();
        let profile = player_cuf_profile_from_packet_like_cpp(profile);
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            let profiles = &mut player.gameplay_state_mut().cuf_profiles;
            if profiles.len() != wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP {
                *profiles = vec![None; wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP];
            }
            profiles[index] = Some(profile);
        });
        if canonical.is_some() {
            return true;
        }

        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            if self.cuf_profiles_like_cpp.len()
                != wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP
            {
                self.cuf_profiles_like_cpp =
                    vec![None; wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP];
            }
            self.cuf_profiles_like_cpp[index] = Some(fixture_profile);
            return true;
        }
        false
    }
    pub(crate) fn represented_load_cuf_profiles_packet_like_cpp(
        &self,
    ) -> Option<wow_packet::packets::misc::LoadCufProfiles> {
        let canonical =
            self.with_owned_player_like_cpp(|player| wow_packet::packets::misc::LoadCufProfiles {
                profiles: player
                    .gameplay_state()
                    .cuf_profiles
                    .iter()
                    .filter_map(|profile| {
                        profile.as_ref().map(player_cuf_profile_to_packet_like_cpp)
                    })
                    .collect(),
            });
        if canonical.is_some() {
            return canonical;
        }

        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return Some(wow_packet::packets::misc::LoadCufProfiles {
                profiles: self
                    .cuf_profiles_like_cpp
                    .iter()
                    .filter_map(Clone::clone)
                    .collect(),
            });
        }
        None
    }
    pub(crate) fn set_loaded_player_customizations_like_cpp(
        &mut self,
        customizations: Vec<wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate>,
    ) {
        let choices = customizations
            .iter()
            .map(|choice| wow_entities::PlayerCustomizationChoice {
                option_id: choice.option_id,
                choice_id: choice.choice_id,
            })
            .collect();
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.gameplay_state_mut().customizations = choices;
        });
        #[cfg(test)]
        {
            self.loaded_player_customizations_like_cpp = Box::new(customizations);
        }
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
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_known_titles_like_cpp(known_title_ids.clone());
                player.set_chosen_title_like_cpp(chosen_title);
            })
            .is_some();
        #[cfg(test)]
        if !_canonical && self.player_handle_like_cpp.is_none() {
            self.represented_known_titles_like_cpp = known_title_ids.into_iter().collect();
            self.represented_chosen_title_like_cpp = chosen_title;
        }
    }
}
