//! Canonical Player spell state. Neither SQL rows nor a native container own it.
//! 02245dcd Player.h:204-232,352; Player.cpp:3130-3160,3448-3458,3736-3757.
//! Learning requires the real Player effect executor; save/world are incomplete.
mod cast;
mod learning;
mod order;
mod overrides;
#[cfg(test)]
mod tests;
pub use cast::{CastOverrideAura, CastSpellAuras, CastSpellError, CastSpellResult};
pub use learning::{
    AddPlayerSpell, LearnPlayerSpell, RemovePlayerSpell, SpellBookMessage, SpellLearnCriterion,
    SpellLearningEffects, SpellLearningError, SpellLearningSourceError, SpellLearningSources,
};
pub use order::SpellBookOrderError;
use std::collections::HashMap;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerSpellState {
    #[default]
    Unchanged = 0,
    Changed = 1,
    New = 2,
    Removed = 3,
    Temporary = 4,
}

/// Source signed 24/8-bit fields, narrowed under the pinned GNU contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerSpellTrait {
    definition: i32,
    rank: i8,
}
impl PlayerSpellTrait {
    pub fn new(definition: i32, rank: i32) -> Self {
        Self {
            definition: (definition << 8) >> 8,
            rank: rank as i8,
        }
    }
    pub fn definition_id(self) -> i32 {
        self.definition
    }
    pub fn rank(self) -> i8 {
        self.rank
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerSpellEntry {
    state: PlayerSpellState,
    active: bool,
    dependent: bool,
    disabled: bool,
    favorite: bool,
    trait_data: Option<PlayerSpellTrait>,
}
impl PlayerSpellEntry {
    pub fn state(&self) -> PlayerSpellState {
        self.state
    }
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn dependent(&self) -> bool {
        self.dependent
    }
    pub fn disabled(&self) -> bool {
        self.disabled
    }
    pub fn favorite(&self) -> bool {
        self.favorite
    }
    pub fn trait_data(&self) -> Option<PlayerSpellTrait> {
        self.trait_data
    }
}

/// Successful node membership changes, not Player state or a persistence log.
/// Replaying these recreates the source unordered_map's bucket/link history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellBookMutation {
    Insert(u32),
    Erase(u32),
}

#[derive(Default)]
pub struct PlayerSpellBook {
    entries: HashMap<u32, PlayerSpellEntry>,
    membership: Vec<SpellBookMutation>,
    overrides: HashMap<u32, overrides::OverrideSet>,
}
impl PlayerSpellBook {
    pub fn spell(&self, id: u32) -> Option<&PlayerSpellEntry> {
        self.entries.get(&id)
    }
    /// Physical node count, including Removed, disabled and temporary entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn has_spell(&self, id: u32) -> bool {
        self.spell(id)
            .is_some_and(|entry| entry.state != PlayerSpellState::Removed && !entry.disabled)
    }
    pub fn has_active_spell(&self, id: u32) -> bool {
        self.spell(id).is_some_and(|entry| {
            entry.state != PlayerSpellState::Removed && entry.active && !entry.disabled
        })
    }

    /// No definition lookup or zero-ID filter exists in the source operation.
    pub fn add_temporary_spell(&mut self, id: u32) {
        if self.entries.contains_key(&id) {
            return;
        }
        let (entry, inserted) = self.try_emplace(id);
        debug_assert!(inserted);
        entry.state = PlayerSpellState::Temporary;
        entry.active = true;
        entry.dependent = false;
        entry.disabled = false;
    }
    pub fn remove_temporary_spell(&mut self, id: u32) {
        if self
            .spell(id)
            .is_some_and(|entry| entry.state == PlayerSpellState::Temporary)
        {
            self.erase(id);
        }
    }
    /// Even assigning the existing favorite dirties an Unchanged source entry.
    pub fn set_spell_favorite(&mut self, id: u32, favorite: bool) {
        let Some(entry) = self.entries.get_mut(&id) else {
            return;
        };
        entry.favorite = favorite;
        if entry.state == PlayerSpellState::Unchanged {
            entry.state = PlayerSpellState::Changed;
        }
    }

    // All future book writers use these private node operations. Payload-only
    // mutations must not record insertion/erase or reconstruct bucket history.
    fn try_emplace(&mut self, id: u32) -> (&mut PlayerSpellEntry, bool) {
        match self.entries.entry(id) {
            std::collections::hash_map::Entry::Occupied(entry) => (entry.into_mut(), false),
            std::collections::hash_map::Entry::Vacant(entry) => {
                self.membership.push(SpellBookMutation::Insert(id));
                (entry.insert(PlayerSpellEntry::default()), true)
            }
        }
    }
    fn erase(&mut self, id: u32) {
        if self.entries.remove(&id).is_some() {
            self.membership.push(SpellBookMutation::Erase(id));
        }
    }
}
