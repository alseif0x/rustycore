// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Collection adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{AccountMount, AccountMountUpdate, HeirloomEntry, TOY_FLAG_FAVORITE_LIKE_CPP};
use super::{TOY_FLAG_HAS_FANFARE_LIKE_CPP, WorldSession, collections};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedTransmogCriteriaEvent {
    LearnAnyTransmogInSlot {
        equipment_slot: u32,
        item_modified_appearance_id: u32,
    },
    CollectTransmogSetFromGroup {
        transmog_set_group_id: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AccountItemAppearanceSavePlanLikeCpp {
    pub(crate) appearance_blocks: Vec<(u32, u32)>,
    pub(crate) favorite_inserts: Vec<u32>,
    pub(crate) favorite_deletes: Vec<u32>,
}

impl AccountItemAppearanceSavePlanLikeCpp {
    pub(crate) fn is_empty(&self) -> bool {
        self.appearance_blocks.is_empty()
            && self.favorite_inserts.is_empty()
            && self.favorite_deletes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AccountTransmogIllusionSavePlanLikeCpp {
    pub(crate) illusion_blocks: Vec<(u32, u32)>,
}

impl AccountTransmogIllusionSavePlanLikeCpp {
    pub(crate) fn is_empty(&self) -> bool {
        self.illusion_blocks.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccountMountSaveRowLikeCpp {
    pub(crate) bnet_account_id: u32,
    pub(crate) mount_spell_id: u32,
    pub(crate) flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccountToySaveRowLikeCpp {
    pub(crate) bnet_account_id: u32,
    pub(crate) item_id: u32,
    pub(crate) is_favorite: bool,
    pub(crate) has_fanfare: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AccountHeirloomSaveRowLikeCpp {
    pub(crate) bnet_account_id: u32,
    pub(crate) item_id: u32,
    pub(crate) flags: u32,
}

pub(in crate::session) const DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP: [u32; 7] = [
    3,  // Lifestealing
    13, // Crusader
    22, // Striking
    23, // Agility
    34, // Hide Weapon Enchant
    43, // Beastslayer
    44, // Titanguard
];

pub(in crate::session) fn heirloom_bonus_for_flags_like_cpp(
    heirloom: &HeirloomEntry,
    flags: u32,
) -> u32 {
    for upgrade_level in (0..heirloom.upgrade_item_id.len()).rev() {
        if flags & (1_u32 << upgrade_level) != 0 {
            return u32::from(heirloom.upgrade_item_bonus_list_id[upgrade_level]);
        }
    }

    0
}

impl WorldSession {
    pub(in crate::session) fn player_collection_state_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerCollectionStateLikeCpp> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.gameplay_state().collections.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_player_collection_state_like_cpp());
        }
        canonical
    }

    /// Collect the session's represented collection fields into the canonical
    /// collection state, as C++ hands the loaded `CollectionMgr` to the Player.
    #[cfg(test)]
    pub(in crate::session) fn represented_player_collection_state_like_cpp(
        &self,
    ) -> wow_entities::PlayerCollectionStateLikeCpp {
        wow_entities::PlayerCollectionStateLikeCpp::from_loaded_account_parts_like_cpp(
            self.account_mounts_like_cpp.clone(),
            self.represented_account_heirlooms_like_cpp.clone(),
            self.represented_account_toys_like_cpp.clone(),
            self.represented_item_appearances_like_cpp.clone(),
            self.represented_item_appearance_blocks_like_cpp.clone(),
            self.represented_temporary_item_appearances_like_cpp.clone(),
            self.represented_favorite_item_appearances_like_cpp.clone(),
            self.represented_transmog_illusions_like_cpp.clone(),
        )
    }

    pub(in crate::session) fn replace_player_collection_state_like_cpp(
        &mut self,
        state: wow_entities::PlayerCollectionStateLikeCpp,
    ) -> bool {
        let canonical = self
            .mutate_canonical_player_like_cpp(|player| {
                player.install_collection_state_like_cpp(state.clone());
            })
            .is_some();
        #[cfg(test)]
        {
            self.account_mounts_like_cpp = state.mounts_like_cpp().clone();
            self.represented_account_heirlooms_like_cpp = state.heirlooms_like_cpp().clone();
            self.represented_account_toys_like_cpp = state.toys_like_cpp().clone();
            self.represented_item_appearances_like_cpp = state.item_appearances_like_cpp().clone();
            self.represented_item_appearance_blocks_like_cpp =
                state.item_appearance_blocks_snapshot_like_cpp();
            self.represented_temporary_item_appearances_like_cpp =
                state.temporary_item_appearances_like_cpp().clone();
            self.represented_favorite_item_appearances_like_cpp =
                state.favorite_item_appearances_like_cpp().clone();
            self.represented_transmog_illusions_like_cpp =
                state.transmog_illusions_like_cpp().clone();
            if self.player_handle_like_cpp.is_none() {
                return true;
            }
        }
        canonical
    }

    /// C++ `Player::AddHeirloom`, called from `CollectionMgr::AddHeirloom`
    /// after the account collection accepts a new heirloom.
    pub(crate) fn add_player_heirloom_dynamic_fields_like_cpp(
        &mut self,
        item_id: u32,
        flags: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let item_id = i32::try_from(item_id).ok()?;
        self.mutate_canonical_player_like_cpp(|player| {
            player.add_heirloom_like_cpp(item_id, flags);
            player.values_update(true)
        })
    }

    /// C++ `DB2Manager::IsToyItem`.
    pub(crate) fn is_toy_item_like_cpp(&self, item_id: u32) -> bool {
        self.toy_store
            .as_ref()
            .and_then(|store| store.get_by_item_id_like_cpp(item_id))
            .is_some()
    }

    /// C++ `std::find_if(item->Effects, spellId)` in `HandleUseToy`.
    pub(crate) fn toy_item_has_spell_effect_like_cpp(&self, item_id: u32, spell_id: i32) -> bool {
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
    pub(crate) fn toy_item_spell_cooldown_ms_like_cpp(
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

    /// C++ `Player::AddToy`, called from `CollectionMgr::AddToy` after the
    /// account collection accepts a new toy.
    pub(crate) fn add_player_toy_dynamic_field_like_cpp(
        &mut self,
        item_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let item_id = i32::try_from(item_id).ok()?;
        self.mutate_canonical_player_like_cpp(|player| {
            player.add_toy_like_cpp(item_id);
            player.values_update(true)
        })
    }

    /// C++ `CollectionMgr::ToyClearFanfare`.
    pub(crate) fn toy_clear_fanfare_like_cpp(&mut self, item_id: u32) -> bool {
        self.mutate_player_collection_state_like_cpp(|collections| {
            collections.update_toy_flags_like_cpp(item_id, 0, TOY_FLAG_HAS_FANFARE_LIKE_CPP)
        })
        .unwrap_or(false)
    }

    /// C++ `CollectionMgr::ToySetFavorite`.
    pub(crate) fn toy_set_favorite_like_cpp(&mut self, item_id: u32, favorite: bool) -> bool {
        self.mutate_player_collection_state_like_cpp(|collections| {
            if favorite {
                collections.update_toy_flags_like_cpp(item_id, TOY_FLAG_FAVORITE_LIKE_CPP, 0)
            } else {
                collections.update_toy_flags_like_cpp(item_id, 0, TOY_FLAG_FAVORITE_LIKE_CPP)
            }
        })
        .unwrap_or(false)
    }

    pub(crate) fn mount_set_favorite_like_cpp(
        &mut self,
        mount_spell_id: u32,
        is_favorite: bool,
    ) -> bool {
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

        self.send_packet(&AccountMountUpdate::partial(vec![AccountMount {
            spell_id,
            flags: updated_flags,
        }]));
        true
    }
}
