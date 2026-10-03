//! Player.cpp:31011-31029. One canonical replacement membership/history.
use super::{PlayerSpellBook, SpellBookMutation, SpellBookOrderError};
use std::collections::HashSet;
#[derive(Default)]
pub(super) struct OverrideSet {
    spells: HashSet<u32>,
    // Per-group witness: deleting the outer node retires its bucket history.
    history: Vec<SpellBookMutation>,
}
impl PlayerSpellBook {
    /// Exact-key admission for a transient native unordered_set replay. No
    /// sorted/Rust/set insertion order or PlayerSpell active/known filtering.
    /// Empty outer groups do not exist; an absent group never calls the provider.
    pub fn source_override_spells<E>(
        &self,
        original: u32,
        order: impl FnOnce(&[SpellBookMutation], usize) -> Result<Vec<u32>, E>,
    ) -> Result<Vec<u32>, SpellBookOrderError<E>> {
        let Some(set) = self.overrides.get(&original) else {
            return Ok(vec![]);
        };
        let keys = order(&set.history, set.spells.len()).map_err(SpellBookOrderError::Source)?;
        if keys.len() != set.spells.len() {
            return Err(SpellBookOrderError::InvalidKeySet);
        }
        let mut seen = HashSet::with_capacity(keys.len());
        for &id in &keys {
            if !set.spells.contains(&id) || !seen.insert(id) {
                return Err(SpellBookOrderError::InvalidKeySet);
            }
        }
        Ok(keys)
    }
    pub fn has_override_spell(&self, original: u32, replacement: u32) -> bool {
        self.overrides
            .get(&original)
            .is_some_and(|set| set.spells.contains(&replacement))
    }
    pub fn add_override_spell(&mut self, original: u32, replacement: u32) {
        let set = self.overrides.entry(original).or_default();
        if set.spells.insert(replacement) {
            set.history.push(SpellBookMutation::Insert(replacement));
        }
    }
    pub fn remove_override_spell(&mut self, original: u32, replacement: u32) {
        let Some(set) = self.overrides.get_mut(&original) else {
            return;
        };
        if set.spells.remove(&replacement) {
            set.history.push(SpellBookMutation::Erase(replacement));
        }
        if set.spells.is_empty() {
            self.overrides.remove(&original);
        }
    }
}
