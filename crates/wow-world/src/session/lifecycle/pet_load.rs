// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Per-character results retained for the represented asynchronous
//! `PetLoadQueryHolder` callback (`Pet.cpp:157-203,386-437`).
//!
//! This is session lifecycle state, not Player or live Pet gameplay state. A
//! new character load replaces the holder; the typed Pet consumes its rows
//! when Map storage materializes that Pet.

use std::collections::HashMap;

use super::super::{
    CharacterPetAuraEffectRowLikeCpp, CharacterPetAuraRowLikeCpp,
    CharacterPetDeclinedNamesRowLikeCpp, CharacterPetSpellChargeRowLikeCpp,
    CharacterPetSpellCooldownRowLikeCpp, CharacterPetSpellRowLikeCpp,
};

#[derive(Debug, Default)]
pub(in crate::session) struct PetLoadQueryHolderRowsLikeCpp {
    pub(in crate::session) spells: HashMap<u32, Vec<CharacterPetSpellRowLikeCpp>>,
    pub(in crate::session) spell_cooldowns: HashMap<u32, Vec<CharacterPetSpellCooldownRowLikeCpp>>,
    pub(in crate::session) spell_charges: HashMap<u32, Vec<CharacterPetSpellChargeRowLikeCpp>>,
    pub(in crate::session) auras: HashMap<u32, Vec<CharacterPetAuraRowLikeCpp>>,
    pub(in crate::session) aura_effects: HashMap<u32, Vec<CharacterPetAuraEffectRowLikeCpp>>,
    pub(in crate::session) declined_names: HashMap<u32, CharacterPetDeclinedNamesRowLikeCpp>,
}

impl PetLoadQueryHolderRowsLikeCpp {
    pub(in crate::session) fn reset(&mut self) {
        *self = Self::default();
    }
}
