use std::collections::BTreeMap;

use tracing::warn;
use wow_core::ObjectGuid;
use wow_data::GlyphPropertiesStore;
use wow_entities::{MAX_POWERS_PER_CLASS, Player};
use wow_packet::packets::misc::NUM_ACCOUNT_DATA_TYPES;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::persistence_capabilities::loaded_character_power_snapshot_like_cpp;
use wow_world_core::session::{
    HubMut, HubRef, MAX_SPECIALIZATIONS_LIKE_CPP, TOY_FLAG_FAVORITE_LIKE_CPP,
    TOY_FLAG_HAS_FANFARE_LIKE_CPP, make_action_button_like_cpp,
    player_cuf_profile_from_packet_like_cpp, player_cuf_profile_to_packet_like_cpp,
};

use super::SessionLifecycleState;
use crate::{
    ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP, AccountDataLikeCpp, AccountHeirloomDataLikeCpp,
    GLOBAL_CACHE_MASK_LIKE_CPP, PER_CHARACTER_CACHE_MASK_LIKE_CPP,
};

impl SessionLifecycleState {
    pub fn record_loaded_action_button_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        index: u8,
        action: u32,
        action_type: u8,
    ) -> bool {
        hub.represented_set_action_button_like_cpp(
            index,
            make_action_button_like_cpp(action, action_type),
        )
    }

    /// C++ `CollectionMgr::LoadAccountHeirlooms`.
    pub fn load_represented_account_heirlooms_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        heirloom_rows: impl IntoIterator<Item = (u32, u32)>,
    ) {
        let mut heirlooms = BTreeMap::new();
        for (item_id, flags) in heirloom_rows {
            let bonus_id = match hub.catalogs.heirloom_store.as_ref() {
                Some(store) => {
                    let Some(heirloom) = store.get_by_item_id_like_cpp(item_id) else {
                        continue;
                    };
                    super::collections::heirloom_bonus_for_flags_like_cpp(heirloom, flags)
                }
                None => 0,
            };
            heirlooms.insert(item_id, AccountHeirloomDataLikeCpp { flags, bonus_id });
        }
        let _ = hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.replace_heirlooms_like_cpp(heirlooms);
        });
    }

    /// C++ `CollectionMgr::LoadAccountToys`.
    pub fn load_represented_account_toys_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
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
        let _ = hub.mutate_player_collection_state_like_cpp(|collections| {
            collections.replace_toys_like_cpp(toys);
        });
    }

    pub fn load_character_reputation_rows_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        rows: impl IntoIterator<Item = wow_progression::mgr::CharacterReputationRowLikeCpp>,
    ) -> bool {
        let Some(faction_store) = hub.catalogs.faction_store().cloned() else {
            return false;
        };
        let friendship_rep_reaction_store = hub.catalogs.friendship_rep_reaction_store().cloned();
        let paragon_reputation_store = hub.catalogs.paragon_reputation_store.as_ref().cloned();
        let race = hub.shared().player_race_like_cpp();
        let class = hub.shared().player_class_like_cpp();

        hub.mutate_reputation_mgr_like_cpp(|mgr| {
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

    pub fn mark_represented_glyphs_loaded_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let _ = hub.mark_glyphs_loaded_like_cpp();
    }

    pub fn represented_talents_loaded_like_cpp(&self, hub: HubRef<'_>) -> bool {
        hub.player_talent_runtime_snapshot_like_cpp()
            .is_some_and(|runtime| runtime.talents_loaded_like_cpp())
    }

    /// Borrow the required process catalog; tests supply explicit fixture data.
    pub fn load_represented_glyph_row_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
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

        let previous = hub
            .shared()
            .player_talent_runtime_snapshot_like_cpp()
            .and_then(|runtime| runtime.glyph_like_cpp(talent_group, glyph_slot));
        if previous != Some(glyph_id) {
            hub.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        hub.install_loaded_glyph_like_cpp(talent_group, glyph_slot, glyph_id)
    }

    /// C++ `Player::InitStatsForLevel` repairs an invalid persisted XP value
    /// after deriving `ActivePlayerData::NextLevelXP` for the loaded level.
    pub fn clamp_loaded_player_xp_to_next_level_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let (Some(player_xp), Some(next_level_xp)) = (
            hub.shared().resolved_player_xp_like_cpp(),
            hub.shared().resolved_player_next_level_xp_like_cpp(),
        ) else {
            return;
        };
        if player_xp >= next_level_xp {
            hub.set_player_xp_like_cpp(next_level_xp.saturating_sub(1));
        }
    }

    /// Get the player loading GUID.
    pub fn player_loading(&self) -> Option<ObjectGuid> {
        self.player_loading
    }

    pub fn set_player_loading(&mut self, guid: Option<ObjectGuid>) {
        self.player_loading = guid;
    }

    pub fn load_tutorials_data_values_like_cpp(&mut self, values: Option<[u32; 8]>) {
        self.tutorials_like_cpp = values.unwrap_or([0; 8]);
        self.tutorials_loaded_from_db_like_cpp = values.is_some();
        self.tutorials_loaded_coherently_like_cpp = true;
        self.tutorials_changed_like_cpp = false;
    }

    pub async fn load_player_account_data_like_cpp(&mut self, hub: &mut HubMut<'_>, guid: ObjectGuid) {
        self.load_account_data_like_cpp(hub, guid, PER_CHARACTER_CACHE_MASK_LIKE_CPP)
            .await;
    }

    pub async fn load_account_data_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        mask: u32,
    ) {
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
                account_id: hub.core.account_id,
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
                account = hub.core.account_id,
                mask, "LoadAccountData skipped: session account-state port unavailable"
            );
            return;
        };

        let rows = match port.load_account_data_like_cpp(scope).await {
            wow_persistence::SessionAccountDataLoadOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::SessionAccountDataLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = hub.core.account_id,
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

    pub fn set_loaded_player_name_like_cpp(&mut self, hub: &mut HubMut<'_>, name: String) {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().world_mut().set_name(name.clone());
            })
            .is_some();
        if canonical {
            hub.core.player_identity_bootstrap_like_cpp = None;
            return;
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        if hub.core.player_handle_like_cpp.is_some() {
            return;
        }
        hub.core
            .player_identity_bootstrap_like_cpp
            .get_or_insert_default()
            .name = Some(name.clone());
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            hub.fixtures.identity.player_name = Some(name);
        }
    }

    pub fn set_loaded_player_flags_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        player_flags: u32,
    ) {
        hub.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let _canonical = hub
            .core
            .mutate_canonical_player_like_cpp(|player| {
                player.replace_all_player_flags(player_flags)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.player_flags_test_fixture_like_cpp
                .represented_loaded_player_flags_like_cpp = Some(player_flags);
            self.player_flags_test_fixture_like_cpp
                .represented_loaded_player_flags_ex_like_cpp
                .get_or_insert(0);
            self.player_flags_test_fixture_like_cpp
                .represented_loaded_player_flags_applied_like_cpp = _canonical;
        }
    }

    pub fn set_loaded_player_flags_ex_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        player_flags_ex: u32,
    ) {
        hub.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        let _canonical = hub
            .core
            .mutate_canonical_player_like_cpp(|player| {
                player.replace_all_player_flags_ex(player_flags_ex)
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.player_flags_test_fixture_like_cpp
                .represented_loaded_player_flags_ex_like_cpp = Some(player_flags_ex);
            self.player_flags_test_fixture_like_cpp
                .represented_loaded_player_flags_applied_like_cpp = _canonical;
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn apply_loaded_player_flags_to_canonical_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let Some(player_flags) = self
            .player_flags_test_fixture_like_cpp
            .represented_loaded_player_flags_like_cpp
        else {
            return;
        };
        let player_flags_ex = self
            .player_flags_test_fixture_like_cpp
            .represented_loaded_player_flags_ex_like_cpp
            .unwrap_or(0);
        if hub
            .core
            .mutate_canonical_player_like_cpp(|player| {
                player.replace_all_player_flags(player_flags);
                player.replace_all_player_flags_ex(player_flags_ex);
            })
            .is_some()
        {
            self.player_flags_test_fixture_like_cpp
                .represented_loaded_player_flags_applied_like_cpp = true;
        }
    }

    pub fn set_loaded_player_powers_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        powers: [i32; MAX_POWERS_PER_CLASS],
    ) {
        let _canonical = hub.core.with_owned_player_mut_for_power_like_cpp(|player| {
            let max_power = player.unit().data().max_power;
            player
                .unit_mut()
                .replace_create_power_arrays_like_cpp(powers.map(|value| value.max(0)), max_power);
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if _canonical.is_some() || hub.core.player_handle_like_cpp.is_none() {
            hub.fixtures.combat.represented_player_powers_like_cpp =
                loaded_character_power_snapshot_like_cpp(powers);
        }
    }

    pub fn resolved_player_skill_records_loaded_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<bool> {
        let canonical = hub
            .core
            .with_owned_player_like_cpp(Player::skill_records_loaded_like_cpp);
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && hub.core.player_handle_like_cpp.is_none() {
            return Some(
                hub.fixtures
                    .progression
                    .player_skill_test_fixture_like_cpp
                    .player_skill_records_loaded_like_cpp,
            );
        }
        canonical
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_skill_records_loaded_like_cpp(&self, hub: HubRef<'_>) -> bool {
        self.resolved_player_skill_records_loaded_like_cpp(hub)
            .expect("test Player skill owner must resolve")
    }

    pub fn mark_represented_action_buttons_loaded_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let _canonical = hub
            .core
            .with_owned_player_mut_like_cpp(Player::mark_action_buttons_loaded_like_cpp)
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            hub.fixtures
                .presentation
                .represented_action_buttons_loaded_like_cpp = true;
        }
    }

    pub fn mark_represented_cuf_profiles_loaded_like_cpp(&mut self, hub: &mut HubMut<'_>) {
        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.mark_cuf_profiles_loaded_like_cpp();
        });
        if canonical.is_some() {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            hub.fixtures.presentation.cuf_profiles_loaded_like_cpp = true;
        }
    }

    pub fn load_represented_cuf_profile_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        id: u8,
        profile: wow_packet::packets::misc::CufProfile,
    ) -> bool {
        let index = usize::from(id);
        if index >= wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP {
            return false;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        let fixture_profile = profile.clone();
        let profile = player_cuf_profile_from_packet_like_cpp(profile);
        let canonical = hub.core.with_owned_player_mut_like_cpp(|player| {
            player.save_cuf_profile_like_cpp(index, Some(profile));
        });
        if canonical.is_some() {
            return true;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            if hub.fixtures.presentation.cuf_profiles_like_cpp.len()
                != wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP
            {
                hub.fixtures.presentation.cuf_profiles_like_cpp =
                    vec![None; wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP];
            }
            hub.fixtures.presentation.cuf_profiles_like_cpp[index] = Some(fixture_profile);
            return true;
        }
        false
    }

    pub fn represented_load_cuf_profiles_packet_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<wow_packet::packets::misc::LoadCufProfiles> {
        let canonical = hub.core.with_owned_player_like_cpp(|player| {
            wow_packet::packets::misc::LoadCufProfiles {
                profiles: player
                    .gameplay_state()
                    .cuf_profiles
                    .iter()
                    .filter_map(|profile| {
                        profile.as_ref().map(player_cuf_profile_to_packet_like_cpp)
                    })
                    .collect(),
            }
        });
        if canonical.is_some() {
            return canonical;
        }

        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some(wow_packet::packets::misc::LoadCufProfiles {
                profiles: hub
                    .fixtures
                    .presentation
                    .cuf_profiles_like_cpp
                    .iter()
                    .filter_map(Clone::clone)
                    .collect(),
            });
        }
        None
    }

    pub fn set_loaded_player_customizations_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        customizations: Vec<wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate>,
    ) {
        let choices = customizations
            .iter()
            .map(|choice| wow_entities::PlayerCustomizationChoice {
                option_id: choice.option_id,
                choice_id: choice.choice_id,
            })
            .collect();
        let _ = hub.core.mutate_canonical_player_like_cpp(|player| {
            player.hydrate_customizations_like_cpp(choices);
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.loaded_player_customizations_like_cpp = Box::new(customizations);
        }
    }
}
