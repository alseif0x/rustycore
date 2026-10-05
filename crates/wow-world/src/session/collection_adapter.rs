// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Collection adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(in crate::session) use wow_world_inventory::DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP;
#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_inventory::RepresentedTransmogCriteriaEvent;
pub(crate) use wow_world_inventory::{
    AccountItemAppearanceSavePlanLikeCpp, AccountTransmogIllusionSavePlanLikeCpp,
};

pub(crate) use wow_world_lifecycle::{
    AccountHeirloomSaveRowLikeCpp, AccountMountSaveRowLikeCpp, AccountToySaveRowLikeCpp,
};

impl WorldSession {}

#[cfg(test)]
#[path = "../../unit_tests/session/collection_adapter/f3_shims.rs"]
mod f3_shims;
