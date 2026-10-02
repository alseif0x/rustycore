use wow_packet::packets::misc::{AccountMount, AccountMountUpdate};

pub const TOY_FLAG_FAVORITE_LIKE_CPP: u32 = 0x01;
pub const TOY_FLAG_HAS_FANFARE_LIKE_CPP: u32 = 0x02;

impl crate::session::state::SessionCatalogs {
    /// C++ `DB2Manager::IsToyItem`.
    pub fn is_toy_item_like_cpp(&self, item_id: u32) -> bool {
        self.toy_store
            .as_ref()
            .and_then(|store| store.get_by_item_id_like_cpp(item_id))
            .is_some()
    }

    /// C++ `std::find_if(item->Effects, spellId)` in `HandleUseToy`.
    pub fn toy_item_has_spell_effect_like_cpp(&self, item_id: u32, spell_id: i32) -> bool {
        self.items
            .effect_store
            .as_ref()
            .and_then(|store| store.effect_for_item_spell_like_cpp(item_id, spell_id))
            .is_some()
    }

    /// Bounded C++ `SpellHistory::GetCooldownDurations(spellInfo, itemId)`.
    ///
    /// C++ lets `ItemEffect` override spell/category cooldowns for item-backed
    /// casts. Rust still lacks full `_categoryCooldowns`, so this represented
    /// helper returns the longest positive item cooldown as the single spell-id
    /// keyed duration used by the current `SpellHistory` seam.
    pub fn toy_item_spell_cooldown_ms_like_cpp(
        &self,
        item_id: u32,
        spell_id: i32,
        spell_info: &wow_data::SpellInfo,
    ) -> u32 {
        if let Some(effect) = self
            .items
            .effect_store
            .as_ref()
            .and_then(|store| store.effect_for_item_spell_like_cpp(item_id, spell_id))
        {
            if effect.cooldown_msec >= 0 || effect.category_cooldown_msec >= 0 {
                return effect
                    .cooldown_msec
                    .max(effect.category_cooldown_msec)
                    .max(0) as u32;
            }
        }

        spell_info.recovery_time_ms.max(spell_info.cooldown_ms)
    }
}

impl crate::session::HubMut<'_> {
    /// C++ `Player::AddHeirloom`, called from `CollectionMgr::AddHeirloom`
    /// after the account collection accepts a new heirloom.
    pub fn add_player_heirloom_dynamic_fields_like_cpp(
        &mut self,
        item_id: u32,
        flags: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let item_id = i32::try_from(item_id).ok()?;
        self.core.mutate_canonical_player_like_cpp(|player| {
            player.add_heirloom_like_cpp(item_id, flags);
            player.values_update(true)
        })
    }

    /// C++ `Player::AddToy`, called from `CollectionMgr::AddToy` after the
    /// account collection accepts a new toy.
    pub fn add_player_toy_dynamic_field_like_cpp(
        &mut self,
        item_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let item_id = i32::try_from(item_id).ok()?;
        self.core.mutate_canonical_player_like_cpp(|player| {
            player.add_toy_like_cpp(item_id);
            player.values_update(true)
        })
    }

    /// C++ `CollectionMgr::ToyClearFanfare`.
    pub fn toy_clear_fanfare_like_cpp(&mut self, item_id: u32) -> bool {
        self.mutate_player_collection_state_like_cpp(|collections| {
            collections.update_toy_flags_like_cpp(item_id, 0, TOY_FLAG_HAS_FANFARE_LIKE_CPP)
        })
        .unwrap_or(false)
    }

    /// C++ `CollectionMgr::ToySetFavorite`.
    pub fn toy_set_favorite_like_cpp(&mut self, item_id: u32, favorite: bool) -> bool {
        self.mutate_player_collection_state_like_cpp(|collections| {
            if favorite {
                collections.update_toy_flags_like_cpp(item_id, TOY_FLAG_FAVORITE_LIKE_CPP, 0)
            } else {
                collections.update_toy_flags_like_cpp(item_id, 0, TOY_FLAG_FAVORITE_LIKE_CPP)
            }
        })
        .unwrap_or(false)
    }

    pub fn mount_set_favorite_like_cpp(&mut self, mount_spell_id: u32, is_favorite: bool) -> bool {
        let Ok(spell_id) = i32::try_from(mount_spell_id) else {
            return false;
        };
        let Some(updated_flags) = self
            .mutate_player_collection_state_like_cpp(|collections| {
                let changed = if is_favorite {
                    collections.update_mount_flags_like_cpp(spell_id, 0x01, 0)
                } else {
                    collections.update_mount_flags_like_cpp(spell_id, 0, 0x01)
                };
                changed.then_some(())?;
                collections.mounts_like_cpp().get(&spell_id).copied()
            })
            .flatten()
        else {
            return false;
        };

        self.core
            .send_packet(&AccountMountUpdate::partial(vec![AccountMount {
                spell_id,
                flags: updated_flags,
            }]));
        true
    }
}

impl crate::session::HubMut<'_> {
    pub fn replace_player_collection_state_like_cpp(
        &mut self,
        state: wow_entities::PlayerCollectionStateLikeCpp,
    ) -> bool {
        let canonical = self
            .core
            .mutate_canonical_player_like_cpp(|player| {
                player.install_collection_state_like_cpp(state.clone());
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures.collections.account_mounts_like_cpp = state.mounts_like_cpp().clone();
            self.fixtures
                .collections
                .represented_account_heirlooms_like_cpp = state.heirlooms_like_cpp().clone();
            self.fixtures.collections.represented_account_toys_like_cpp =
                state.toys_like_cpp().clone();
            self.fixtures
                .collections
                .represented_item_appearances_like_cpp = state.item_appearances_like_cpp().clone();
            self.fixtures
                .collections
                .represented_item_appearance_blocks_like_cpp =
                state.item_appearance_blocks_snapshot_like_cpp();
            self.fixtures
                .collections
                .represented_temporary_item_appearances_like_cpp =
                state.temporary_item_appearances_like_cpp().clone();
            self.fixtures
                .collections
                .represented_favorite_item_appearances_like_cpp =
                state.favorite_item_appearances_like_cpp().clone();
            self.fixtures
                .collections
                .represented_transmog_illusions_like_cpp =
                state.transmog_illusions_like_cpp().clone();
            if self.core.player_handle_like_cpp.is_none() {
                return true;
            }
        }
        canonical
    }
}

impl crate::session::HubRef<'_> {
    pub fn player_collection_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerCollectionStateLikeCpp> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(|player| player.gameplay_state().collections.clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(self.represented_player_collection_state_like_cpp());
        }
        canonical
    }

    /// Collect the session's represented collection fields into the canonical
    /// collection state, as C++ hands the loaded `CollectionMgr` to the Player.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_player_collection_state_like_cpp(
        &self,
    ) -> wow_entities::PlayerCollectionStateLikeCpp {
        wow_entities::PlayerCollectionStateLikeCpp::from_loaded_account_parts_like_cpp(
            self.fixtures.collections.account_mounts_like_cpp.clone(),
            self.fixtures
                .collections
                .represented_account_heirlooms_like_cpp
                .clone(),
            self.fixtures
                .collections
                .represented_account_toys_like_cpp
                .clone(),
            self.fixtures
                .collections
                .represented_item_appearances_like_cpp
                .clone(),
            self.fixtures
                .collections
                .represented_item_appearance_blocks_like_cpp
                .clone(),
            self.fixtures
                .collections
                .represented_temporary_item_appearances_like_cpp
                .clone(),
            self.fixtures
                .collections
                .represented_favorite_item_appearances_like_cpp
                .clone(),
            self.fixtures
                .collections
                .represented_transmog_illusions_like_cpp
                .clone(),
        )
    }
}
