// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pet-save transaction requests submitted by the Player lifecycle owner.

use crate::{
    PlayerPetAuraEffectLoadRowLikeCpp, PlayerPetAuraLoadRowLikeCpp,
    PlayerPetSpellChargeLoadRowLikeCpp, PlayerPetSpellCooldownLoadRowLikeCpp,
    PlayerPetStableLoadRowLikeCpp,
};

/// One pet-spell row operation selected by the native Pet save owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPetSpellSaveOperationLikeCpp {
    /// Remove one pet spell by its spell identifier.
    Delete { spell_id: u32 },
    /// Insert one pet spell with its already-selected active state.
    Insert { spell_id: u32, active: u8 },
}

/// One native Pet save database transaction.
///
/// The two variants intentionally remain separate requests. The caller
/// submits the aura/spell/history phase first and submits `PetRow` only after
/// that phase's outcome permits it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerPetSaveTransactionLikeCpp {
    /// Replace pet auras, apply selected spell operations, then replace spell
    /// cooldown and charge history.
    AurasSpellsAndHistory {
        pet_number: u32,
        auras: Vec<PlayerPetAuraLoadRowLikeCpp>,
        aura_effects: Vec<PlayerPetAuraEffectLoadRowLikeCpp>,
        spells: Vec<PlayerPetSpellSaveOperationLikeCpp>,
        cooldowns: Vec<PlayerPetSpellCooldownLoadRowLikeCpp>,
        charges: Vec<PlayerPetSpellChargeLoadRowLikeCpp>,
    },
    /// Replace the stable character_pet row using the caller-selected active
    /// slot already captured in `row.slot`.
    PetRow {
        owner_guid: u64,
        row: PlayerPetStableLoadRowLikeCpp,
    },
}
