use super::{PlayerSpellBook, PlayerSpellEntry, SpellBookMutation};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellBookOrderError<E> {
    Source(E),
    InvalidKeySet,
}

impl PlayerSpellBook {
    /// Borrow canonical entries in the admitted target container order. Rust
    /// HashMap iteration, sorted IDs and insertion order are not substitutes.
    /// The numeric-only provider retains no Player state. Exact-set admission
    /// detects bad output; independent native QA must establish order fidelity.
    ///
    /// This is a read-only traversal, NOT a live reentrant AddSpell rank cursor.
    /// Mutation is excluded while returned entry borrows exist. The source rank
    /// loop itself only changes payloads, but publication reaches OnPacketSend
    /// script hooks. Its future writer must resolve reentry/invalidation, not
    /// hold these borrows across arbitrary learning or node mutations.
    pub fn source_entries<E>(
        &self,
        order: impl FnOnce(&[SpellBookMutation], usize) -> Result<Vec<u32>, E>,
    ) -> Result<Vec<(u32, &PlayerSpellEntry)>, SpellBookOrderError<E>> {
        let keys =
            order(&self.membership, self.entries.len()).map_err(SpellBookOrderError::Source)?;
        if keys.len() != self.entries.len() {
            return Err(SpellBookOrderError::InvalidKeySet);
        }
        let mut seen = HashSet::with_capacity(keys.len());
        let mut result = Vec::with_capacity(keys.len());
        for id in keys {
            let Some(entry) = self.entries.get(&id) else {
                return Err(SpellBookOrderError::InvalidKeySet);
            };
            if !seen.insert(id) {
                return Err(SpellBookOrderError::InvalidKeySet);
            }
            result.push((id, entry));
        }
        Ok(result)
    }
}
