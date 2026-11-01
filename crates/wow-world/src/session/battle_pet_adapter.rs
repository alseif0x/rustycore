// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battle pet adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.
//! The outcome types, the stats application and the uncage `DestroyItem`
//! phase moved to `wow_world_inventory::battle_pet_session` (#1363 slice 7b).
//! The battle-pet XP-percent aura moved to `wow_world_spell` (#1363 slice 7c).

#[cfg(test)]
use super::{ObjectGuid, RepresentedBattlePetCageItemLikeCpp};

#[cfg(test)]
use std::sync::atomic::{AtomicI64, Ordering};

#[cfg(test)]
pub(crate) static NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP: AtomicI64 = AtomicI64::new(1);

#[cfg(test)]
pub(crate) fn next_represented_battle_pet_guid_like_cpp() -> ObjectGuid {
    let counter = NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP.fetch_add(1, Ordering::Relaxed);
    ObjectGuid::create_global(wow_core::guid::HighGuid::BattlePet, 0, counter)
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetCageOutcomeLikeCpp {
    Caged(RepresentedBattlePetCageItemLikeCpp),
    NoJournalLock,
    UnknownPet,
    NotTradable,
    InBattleSlot,
    Damaged,
    InventoryUnavailable,
    StoreFailed,
}

#[cfg(test)]
#[path = "../../unit_tests/session/battle_pet_adapter/f3_shims.rs"]
mod f3_shims;
