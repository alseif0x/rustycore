// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Collection adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{HeirloomEntry, WorldSession};

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_inventory::RepresentedTransmogCriteriaEvent;
pub(crate) use wow_world_inventory::{
    AccountItemAppearanceSavePlanLikeCpp, AccountTransmogIllusionSavePlanLikeCpp,
};
pub(in crate::session) use wow_world_inventory::DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP;

pub(crate) use wow_world_lifecycle::{
    AccountHeirloomSaveRowLikeCpp, AccountMountSaveRowLikeCpp, AccountToySaveRowLikeCpp,
};

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
