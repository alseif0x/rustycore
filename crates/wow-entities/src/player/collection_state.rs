// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player collection state.
//!
//! C++ `Player` owns `CollectionMgr` and performs the transitions itself in
//! `Entities/Player/CollectionMgr.cpp`: `AddToy` (`:102`), `AddHeirloom`
//! (`:237`), `AddMount` (`:360`), `AddItemAppearance` (`:576`, `:594`, `:732`),
//! `AddTemporaryAppearance` (`:768`) and `SetAppearanceIsFavorite` (`:828`).
//!
//! Separated from `player_gameplay_state.rs` under #763, which also closed the
//! fields to this crate's Player module: reads keep named accessors and every
//! write is a named operation the owner checks. Temporary appearances stay
//! keyed by item GUID because their lifetime follows the item, not the Player.

use std::collections::{BTreeMap, HashMap, HashSet};

use wow_core::ObjectGuid;

use crate::player_gameplay_state::{
    PlayerAccountHeirloomDataLikeCpp, PlayerFavoriteAppearanceStateLikeCpp,
};

/// Canonical mutable owner for the represented C++ `CollectionMgr` families.
///
/// The update-field mirrors themselves remain on `Player::active_data`; this
/// state owns the account collection decisions, temporary providers and dirty
/// favorite transitions that feed those fields and persistence.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlayerCollectionStateLikeCpp {
    pub(super) mounts: HashMap<i32, u8>,
    pub(super) heirlooms: BTreeMap<u32, PlayerAccountHeirloomDataLikeCpp>,
    pub(super) toys: BTreeMap<u32, u32>,
    pub(super) item_appearances: HashSet<u32>,
    pub(super) item_appearance_blocks: Vec<u32>,
    pub(super) temporary_item_appearances: HashMap<u32, HashSet<ObjectGuid>>,
    pub(super) favorite_item_appearances: HashMap<u32, PlayerFavoriteAppearanceStateLikeCpp>,
    pub(super) transmog_illusions: HashSet<u32>,
}

impl PlayerCollectionStateLikeCpp {
    // ---- reads -------------------------------------------------------------

    /// C++ `CollectionMgr::GetAccountMounts`, keyed by mount spell id.
    #[must_use]
    pub fn mounts_like_cpp(&self) -> &HashMap<i32, u8> {
        &self.mounts
    }

    /// C++ `CollectionMgr::GetAccountHeirlooms`.
    #[must_use]
    pub fn heirlooms_like_cpp(&self) -> &BTreeMap<u32, PlayerAccountHeirloomDataLikeCpp> {
        &self.heirlooms
    }

    /// C++ `CollectionMgr::GetAccountToys`.
    #[must_use]
    pub fn toys_like_cpp(&self) -> &BTreeMap<u32, u32> {
        &self.toys
    }

    #[must_use]
    pub fn item_appearances_like_cpp(&self) -> &HashSet<u32> {
        &self.item_appearances
    }

    /// The ordered appearance bit blocks; their order is the client's index.
    #[must_use]
    pub fn item_appearance_blocks_like_cpp(&self) -> &[u32] {
        &self.item_appearance_blocks
    }

    /// C++ `CollectionMgr::GetItemsProvidingTemporaryAppearance`, keyed by the
    /// appearance and holding the item GUIDs that currently provide it.
    #[must_use]
    pub fn temporary_item_appearances_like_cpp(&self) -> &HashMap<u32, HashSet<ObjectGuid>> {
        &self.temporary_item_appearances
    }

    /// Whether any item currently provides this appearance temporarily.
    #[must_use]
    pub fn has_temporary_item_appearance_like_cpp(&self, item_modified_appearance_id: u32) -> bool {
        self.temporary_item_appearances
            .contains_key(&item_modified_appearance_id)
    }

    #[must_use]
    pub fn favorite_item_appearances_like_cpp(
        &self,
    ) -> &HashMap<u32, PlayerFavoriteAppearanceStateLikeCpp> {
        &self.favorite_item_appearances
    }

    #[must_use]
    pub fn transmog_illusions_like_cpp(&self) -> &HashSet<u32> {
        &self.transmog_illusions
    }

    // ---- transitions -------------------------------------------------------

    /// Build the collection from one loaded account payload, as C++
    /// `CollectionMgr::LoadAccountMounts`/`LoadAccountToys`/`LoadAccountHeirlooms`/
    /// `LoadAccountItemAppearances`/`LoadAccountTransmogIllusions` fill the
    /// manager together before the owner is handed the Player.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_loaded_account_parts_like_cpp(
        mounts: HashMap<i32, u8>,
        heirlooms: BTreeMap<u32, PlayerAccountHeirloomDataLikeCpp>,
        toys: BTreeMap<u32, u32>,
        item_appearances: HashSet<u32>,
        item_appearance_blocks: Vec<u32>,
        temporary_item_appearances: HashMap<u32, HashSet<ObjectGuid>>,
        favorite_item_appearances: HashMap<u32, PlayerFavoriteAppearanceStateLikeCpp>,
        transmog_illusions: HashSet<u32>,
    ) -> Self {
        Self {
            mounts,
            heirlooms,
            toys,
            item_appearances,
            item_appearance_blocks,
            temporary_item_appearances,
            favorite_item_appearances,
            transmog_illusions,
        }
    }

    /// Copy the ordered appearance bit blocks, for a caller that must own them.
    #[must_use]
    pub fn item_appearance_blocks_snapshot_like_cpp(&self) -> Vec<u32> {
        self.item_appearance_blocks.clone()
    }

    /// Copy the collected mounts, for a caller that must own them.
    #[must_use]
    pub fn mounts_snapshot_like_cpp(&self) -> HashMap<i32, u8> {
        self.mounts.clone()
    }

    /// C++ `CollectionMgr::AddToy` (`:102`) through `UpdateAccountToys`
    /// (`:140`), whose `_toys.insert` leaves an already collected toy alone;
    /// the caller learns that from the result, as C++ does from `.second`.
    pub fn add_toy_like_cpp(&mut self, item_id: u32, flags: u32) -> bool {
        if self.toys.contains_key(&item_id) {
            return false;
        }
        self.toys.insert(item_id, flags);
        true
    }

    /// Forget one toy, as the represented removal does.
    pub fn remove_toy_like_cpp(&mut self, item_id: u32) -> bool {
        self.toys.remove(&item_id).is_some()
    }

    /// Set and clear flag bits on one collected toy
    /// (C++ `ToySetFavorite` `:145` and `ToyClearFanfare` `:157`).
    pub fn update_toy_flags_like_cpp(&mut self, item_id: u32, set: u32, clear: u32) -> bool {
        let Some(flags) = self.toys.get_mut(&item_id) else {
            return false;
        };
        *flags |= set;
        *flags &= !clear;
        true
    }

    /// Install the authoritative loaded toys
    /// (`CollectionMgr::LoadAccountToys` `:113`).
    pub fn replace_toys_like_cpp(&mut self, toys: BTreeMap<u32, u32>) {
        self.toys = toys;
    }

    /// C++ `CollectionMgr::AddHeirloom` (`:237`) through
    /// `UpdateAccountHeirlooms` (`:217`), whose `_heirlooms.insert` leaves an
    /// already collected heirloom alone.
    pub fn add_heirloom_like_cpp(
        &mut self,
        item_id: u32,
        data: PlayerAccountHeirloomDataLikeCpp,
    ) -> bool {
        if self.heirlooms.contains_key(&item_id) {
            return false;
        }
        self.heirlooms.insert(item_id, data);
        true
    }

    /// C++ `CollectionMgr::UpgradeHeirloom` (`:243`) writing the upgraded row
    /// through `UpdateAccountHeirlooms` (`:217`).
    pub fn update_heirloom_like_cpp(&mut self, item_id: u32, flags: u32, bonus_id: u32) -> bool {
        let Some(data) = self.heirlooms.get_mut(&item_id) else {
            return false;
        };
        data.flags = flags;
        data.bonus_id = bonus_id;
        true
    }

    /// C++ `CollectionMgr::CheckHeirloomUpgrades` (`:278`) replacing a superseded
    /// heirloom with its successor, which starts without flags or bonus.
    pub fn replace_heirloom_like_cpp(&mut self, item_id: u32, new_item_id: u32) {
        self.heirlooms.remove(&item_id);
        self.heirlooms.insert(
            new_item_id,
            PlayerAccountHeirloomDataLikeCpp {
                flags: 0,
                bonus_id: 0,
            },
        );
    }

    /// Install the authoritative loaded heirlooms
    /// (`CollectionMgr::LoadAccountHeirlooms` `:174`).
    pub fn replace_heirlooms_like_cpp(
        &mut self,
        heirlooms: BTreeMap<u32, PlayerAccountHeirloomDataLikeCpp>,
    ) {
        self.heirlooms = heirlooms;
    }

    /// Install the authoritative loaded mounts
    /// (`CollectionMgr::LoadAccountMounts` `:330`).
    pub fn replace_mounts_like_cpp(&mut self, mounts: HashMap<i32, u8>) {
        self.mounts = mounts;
    }

    /// C++ `CollectionMgr::AddMount` (`:360`) storing one mount's flags with
    /// `_mounts.insert` (`:374`), which does not overwrite a collected mount.
    pub fn add_mount_like_cpp(&mut self, spell_id: i32, flags: u8) -> bool {
        if self.mounts.contains_key(&spell_id) {
            return false;
        }
        self.mounts.insert(spell_id, flags);
        true
    }

    /// Set and clear flag bits on one collected mount
    /// (C++ `CollectionMgr::MountSetFavorite` `:395`).
    pub fn update_mount_flags_like_cpp(&mut self, spell_id: i32, set: u8, clear: u8) -> bool {
        let Some(flags) = self.mounts.get_mut(&spell_id) else {
            return false;
        };
        *flags |= set;
        *flags &= !clear;
        true
    }

    /// C++ `CollectionMgr::AddItemAppearance` (`:576`) collecting one
    /// appearance permanently.
    ///
    /// The temporary providers of that same appearance are dropped with it: a
    /// permanent appearance supersedes the conditional transmog the temporary
    /// entry represented, and leaving the entry behind would keep publishing a
    /// conditional the Player no longer needs.
    pub fn add_item_appearance_like_cpp(&mut self, item_modified_appearance_id: u32) {
        self.item_appearances.insert(item_modified_appearance_id);
        self.temporary_item_appearances
            .remove(&item_modified_appearance_id);
    }

    /// C++ `CollectionMgr::AddTemporaryAppearance` (`:768`). The result says
    /// whether this is the first item providing that appearance, which is when
    /// the conditional transmog becomes visible.
    pub fn add_temporary_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> bool {
        let items = self
            .temporary_item_appearances
            .entry(item_modified_appearance_id)
            .or_default();
        let was_empty = items.is_empty();
        items.insert(item_guid);
        was_empty
    }

    /// C++ `CollectionMgr::RemoveTemporaryAppearance` (`:777`). The entry is erased with
    /// its last provider, and only then does the conditional transmog go.
    pub fn remove_temporary_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> bool {
        let Some(items) = self
            .temporary_item_appearances
            .get_mut(&item_modified_appearance_id)
        else {
            return false;
        };
        if !items.remove(&item_guid) || !items.is_empty() {
            return false;
        }
        self.temporary_item_appearances
            .remove(&item_modified_appearance_id);
        true
    }

    /// Mutable access to one appearance's favourite state, as C++
    /// `CollectionMgr::SetAppearanceIsFavorite` (`:828`) writes the entry it
    /// just found in `_favoriteAppearances`.
    pub fn favorite_item_appearance_entry_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
    ) -> std::collections::hash_map::Entry<'_, u32, PlayerFavoriteAppearanceStateLikeCpp> {
        self.favorite_item_appearances
            .entry(item_modified_appearance_id)
    }

    /// C++ `CollectionMgr::SaveAccountItemAppearances`
    /// (`Entities/Player/CollectionMgr.cpp:516`): the favourite states are
    /// walked in ascending appearance order, newly favourited entries are
    /// planned as inserts and become `Unchanged`, and removed entries are
    /// planned as deletes and erased. Returns `(inserts, deletes)`.
    pub fn settle_favorite_item_appearance_saves_like_cpp(&mut self) -> (Vec<u32>, Vec<u32>) {
        let mut inserts = Vec::new();
        let mut deletes = Vec::new();
        let states = self
            .favorite_item_appearances
            .iter()
            .map(|(&appearance, &state)| (appearance, state))
            .collect::<BTreeMap<_, _>>();
        for (item_modified_appearance_id, state) in states {
            match state {
                PlayerFavoriteAppearanceStateLikeCpp::New => {
                    inserts.push(item_modified_appearance_id);
                    self.favorite_item_appearances.insert(
                        item_modified_appearance_id,
                        PlayerFavoriteAppearanceStateLikeCpp::Unchanged,
                    );
                }
                PlayerFavoriteAppearanceStateLikeCpp::Removed => {
                    deletes.push(item_modified_appearance_id);
                    self.favorite_item_appearances
                        .remove(&item_modified_appearance_id);
                }
                PlayerFavoriteAppearanceStateLikeCpp::Unchanged => {}
            }
        }
        (inserts, deletes)
    }

    /// Install one loaded appearance collection: the set, its ordered bit
    /// blocks and the favourite states, which C++
    /// `CollectionMgr::LoadAccountItemAppearances` (`:461`) fills together and
    /// which are therefore authoritative together.
    pub fn install_appearance_collection_like_cpp(
        &mut self,
        item_appearances: HashSet<u32>,
        item_appearance_blocks: Vec<u32>,
        favorite_item_appearances: HashMap<u32, PlayerFavoriteAppearanceStateLikeCpp>,
    ) {
        self.item_appearances = item_appearances;
        self.item_appearance_blocks = item_appearance_blocks;
        self.favorite_item_appearances = favorite_item_appearances;
    }

    /// Install the loaded transmog illusions
    /// (`CollectionMgr::LoadAccountTransmogIllusions` `:874`).
    pub fn replace_transmog_illusions_like_cpp(&mut self, illusions: HashSet<u32>) {
        self.transmog_illusions = illusions;
    }
}
