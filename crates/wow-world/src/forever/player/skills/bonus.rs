//! 02245dcd Player.cpp:5699-5714 / Player.h:2420-2422.
//! Real canonical field mutation, with source pre-order child propagation.
use super::{PlayerSkills, SkillUpdateState};
use std::collections::BTreeSet;
use wow_data::forever_birth::BirthCatalog;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillBonusError {
    SignedAdditionOverflow,
    RecursiveChildCycle,
}

impl PlayerSkills {
    /// Execute ModifySkillBonus, including all live descendants, on the one
    /// canonical field array. Permanent is source `talent=true`. This does not
    /// mark skill save states or alter pure rank/step/cap/starting rank.
    ///
    /// Inactive/deleted/missing nodes prune their entire subtree, as in C++.
    /// No SkillLine existence, category, parent-rank or skill-zero gate is added.
    /// The final immutable BirthCatalog supplies its source-ordered child index.
    /// Undefined signed addition or an active recursion cycle returns an error
    /// retaining the completed mutation prefix: neither is an atomic rollback
    /// or a source success. The caller must discard an unadmitted construction
    /// on such an error, not continue learning/saving/publishing it.
    pub fn modify_bonus(
        &mut self,
        birth: &BirthCatalog,
        skill: u32,
        delta: i32,
        permanent: bool,
    ) -> Result<(), SkillBonusError> {
        let mut active = BTreeSet::new();
        let mut frames = Vec::new();
        let mut pending = Some(skill);
        loop {
            if let Some(skill) = pending.take() {
                // Unlike HasSkill, the source does not reject skill ID zero.
                if let Some(status) = self.status.get(&skill)
                    && status.state != SkillUpdateState::Deleted
                    && self.fields[usize::from(status.slot)].rank != 0
                {
                    if !active.insert(skill) {
                        return Err(SkillBonusError::RecursiveChildCycle);
                    }
                    let field = &mut self.fields[usize::from(status.slot)];
                    let previous = if permanent {
                        i32::from(field.permanent_bonus)
                    } else {
                        i32::from(field.temporary_bonus)
                    };
                    let value = previous
                        .checked_add(delta)
                        .ok_or(SkillBonusError::SignedAdditionOverflow)?;
                    // Both source setters take uint16; TempBonus stores int16.
                    // Preserve narrowing bits, never saturate or clamp bonuses.
                    if permanent {
                        field.permanent_bonus = value as u16;
                    } else {
                        field.temporary_bonus = value as u16 as i16;
                    }
                    frames.push((skill, birth.child_lines(skill)));
                }
            }
            // Explicit borrowed iterators avoid native-stack exhaustion and any
            // cloned raw rows, memoization, speculative child execution or cap.
            while let Some((skill, children)) = frames.last_mut() {
                if let Some(child) = children.next() {
                    pending = Some(child.id);
                    break;
                }
                active.remove(skill);
                frames.pop();
            }
            if pending.is_none() {
                return Ok(());
            }
        }
    }
}
