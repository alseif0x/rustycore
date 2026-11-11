use super::{PlayerSkills, SkillFields, SkillUpdateState, Status};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillStatusOrderError<E> {
    Source(E),
    InvalidKeySet,
}

/// Borrowed physical status membership, including inactive and Deleted entries.
/// The key is independent of the narrowed line and possibly shared field slot.
pub struct SkillStatusEntry<'a> {
    skill: u32,
    status: &'a Status,
    fields: &'a SkillFields,
}
impl<'a> SkillStatusEntry<'a> {
    pub fn skill(&self) -> u32 {
        self.skill
    }
    pub fn slot(&self) -> usize {
        usize::from(self.status.slot)
    }
    pub fn state(&self) -> SkillUpdateState {
        self.status.state
    }
    pub fn fields(&self) -> &'a SkillFields {
        self.fields
    }
}

impl PlayerSkills {
    /// Admit a read-only traversal of every physical status node. The provider
    /// receives successful full-width insertions and current map length, never
    /// field-slot order or an inferred active/dirty subset. It is called even
    /// for an empty owner. Exact-set admission does not prove native order;
    /// source-specific replay and independent order evidence remain required.
    /// Returned borrows exclude mutation while the projections are in use.
    pub fn source_statuses<E>(
        &self,
        order: impl FnOnce(&[u32], usize) -> Result<Vec<u32>, E>,
    ) -> Result<Vec<SkillStatusEntry<'_>>, SkillStatusOrderError<E>> {
        let keys =
            order(&self.insertions, self.status.len()).map_err(SkillStatusOrderError::Source)?;
        if keys.len() != self.status.len() {
            return Err(SkillStatusOrderError::InvalidKeySet);
        }
        let mut seen = HashSet::with_capacity(keys.len());
        let mut result = Vec::with_capacity(keys.len());
        for skill in keys {
            let Some(status) = self.status.get(&skill) else {
                return Err(SkillStatusOrderError::InvalidKeySet);
            };
            if !seen.insert(skill) {
                return Err(SkillStatusOrderError::InvalidKeySet);
            }
            result.push(SkillStatusEntry {
                skill,
                status,
                fields: &self.fields[usize::from(status.slot)],
            });
        }
        Ok(result)
    }
}
