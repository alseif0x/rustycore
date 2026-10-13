// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented spellbook at the session boundary (#1363 slice 9a).
//!
//! C++ `Player::LearnSpell`, `Player::RemoveSpell` (with its dependent,
//! superseded and lower-rank spells), the spell-map reads and the spell-row,
//! trait and override bookkeeping run on the Player. Their Rust bodies now
//! live in this crate as free functions over [`SpellbookHostLikeCpp`]; each
//! host method is the same-named `WorldSession` field split or operation,
//! called at the exact point the body called it before the move.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use wow_data::{
    SkillRangeTypeLikeCpp, SpellLearnSkillLookupLikeCpp, SpellLearnSkillNodeLikeCpp,
    SpellLearnSpellNodeLikeCpp, TalentTabStore,
};
use wow_world_core::session::{HubMut, HubRef, SessionHubHostLikeCpp};

use crate::{
    RepresentedPlayerSpellLikeCpp, SessionSpellState, canonical_player_spell_record_like_cpp,
    represented_player_spell_record_like_cpp,
};

mod acquisition;
mod effects;
mod runtime;
mod spellbook;

pub use acquisition::*;
pub use effects::*;
pub use runtime::*;
pub use spellbook::*;

/// The World session as the spellbook's host. The supertrait lends the hub.
pub trait SpellbookHostLikeCpp: SessionHubHostLikeCpp {
    /// The World `spell_state` field.
    fn spell_state(&self) -> &SessionSpellState;
    /// The World `split_spell_state_ref` split.
    fn split_spell_state_ref(&self) -> (&SessionSpellState, HubRef<'_>);
    /// The World `split_spell_state_mut` split.
    fn split_spell_state_mut(&mut self) -> (&mut SessionSpellState, HubMut<'_>);
    /// The World spell-map read (C++ `Player::GetSpellMap`). It stays on the
    /// session because it passes the World `cfg!(test)` fixture flag.
    fn with_player_spell_runtime_like_cpp<R>(
        &self,
        query: impl FnOnce(&wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R>;
    /// The World spell-map transition dispatch (#754), kept on the session
    /// for the same reason.
    fn mutate_player_spell_runtime_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R>;
    fn represented_auto_unequip_offhand_if_need_like_cpp(&mut self, force: bool) -> bool;
    fn remove_represented_auras_due_to_spell_like_cpp(&mut self, spell_id: i32) -> usize;
    fn remove_auras_with_interrupt_flags_like_cpp(&mut self, flags: u32, flags2: u32) -> usize;
}
