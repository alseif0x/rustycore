// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item-effect event contracts retained by Session inventory.

#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;
use wow_entities::ApplyEnchantmentEffectAction;

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedItemModsReapplyEventLikeCpp {
    pub item_guid: ObjectGuid,
    pub slot: u8,
    pub apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedItemBonusActionLikeCpp {
    pub item_guid: ObjectGuid,
    pub slot: u8,
    pub action: ApplyEnchantmentEffectAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedItemSetSpellEventLikeCpp {
    pub item_set_id: u32,
    pub spell_entry_id: u32,
    pub spell_id: u32,
    pub threshold: u8,
    pub apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentedItemSetAuraRefreshEventLikeCpp {
    pub item_set_id: u32,
    pub spell_entry_id: u32,
    pub spell_id: u32,
    pub apply: bool,
    pub form_change: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedCombatStatRecalculationLikeCpp {
    Expertise {
        attack: wow_constants::WeaponAttackType,
    },
    Rating {
        combat_rating: u8,
    },
}
