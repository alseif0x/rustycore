// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Collection adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{HeirloomEntry, WorldSession};

#[cfg(any(test, feature = "test-fixtures"))]
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

impl WorldSession {}

#[cfg(test)]
#[path = "../../unit_tests/session/collection_adapter/f3_shims.rs"]
mod f3_shims;
