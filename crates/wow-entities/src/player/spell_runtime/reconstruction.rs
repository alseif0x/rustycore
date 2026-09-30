//! Ordered reconstruction of loaded spell dependencies and send-only ranks.
//! The borrowed known-spell buffer remains the caller's existing projection.

use super::PlayerSpellRuntimeState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadedSpellDependency {
    pub spell_id: u32,
    pub overrides_spell_id: u32,
    pub active: bool,
    pub auto_learned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadedSpellStep {
    Dependencies(u32),
    Flags(i32),
    Override { overridden: i32, replacement: i32 },
    Done(usize),
}

pub enum LoadedSpellInput<I> {
    Dependencies(I),
    Applied,
}

#[derive(Clone, Copy)]
enum Phase {
    Root,
    Lookup(u32),
    Rows,
    Flags(i32),
    ConsiderOverride,
    Override(i32, i32),
    Done,
}

/// One root copy and one lazy row iterator at a time; no player authority.
pub struct LoadedSpellReconstruction<I> {
    pending: Vec<i32>,
    index: usize,
    added: usize,
    rows: Option<I>,
    current: Option<(LoadedSpellDependency, i32)>,
    phase: Phase,
}

impl<I: Iterator<Item = LoadedSpellDependency>> LoadedSpellReconstruction<I> {
    pub fn new(roots: &[i32]) -> Self {
        Self {
            pending: roots.to_vec(),
            index: 0,
            added: 0,
            rows: None,
            current: None,
            phase: Phase::Root,
        }
    }

    pub fn step(&mut self) -> LoadedSpellStep {
        loop {
            match self.phase {
                Phase::Root => {
                    if self.index == self.pending.len() {
                        self.phase = Phase::Done;
                        continue;
                    }
                    let spell_id = self.pending[self.index];
                    self.index += 1;
                    let Ok(spell_id) = u32::try_from(spell_id) else {
                        continue;
                    };
                    self.phase = Phase::Lookup(spell_id);
                }
                Phase::Lookup(id) => return LoadedSpellStep::Dependencies(id),
                Phase::Rows => {
                    let Some(row) = self.rows.as_mut().expect("resolved dependency rows").next() else {
                        self.rows = None;
                        self.phase = Phase::Root;
                        continue;
                    };
                    let Ok(spell_id) = i32::try_from(row.spell_id) else {
                        continue;
                    };
                    self.current = Some((row, spell_id));
                    self.phase = if !row.auto_learned && row.active {
                        Phase::Flags(spell_id)
                    } else {
                        Phase::ConsiderOverride
                    };
                }
                Phase::Flags(id) => return LoadedSpellStep::Flags(id),
                Phase::ConsiderOverride => {
                    let (row, replacement) = self.current.expect("current dependency");
                    if row.active && row.overrides_spell_id != 0 {
                        if let Ok(overridden) = i32::try_from(row.overrides_spell_id) {
                            self.phase = Phase::Override(overridden, replacement);
                            continue;
                        }
                    }
                    self.current = None;
                    self.phase = Phase::Rows;
                }
                Phase::Override(overridden, replacement) => {
                    return LoadedSpellStep::Override { overridden, replacement };
                }
                Phase::Done => return LoadedSpellStep::Done(self.added),
            }
        }
    }

    pub fn advance(&mut self, input: LoadedSpellInput<I>, known_spells: &mut Vec<i32>) {
        match (&self.phase, input) {
            (Phase::Lookup(_), LoadedSpellInput::Dependencies(rows)) => {
                self.rows = Some(rows);
                self.phase = Phase::Rows;
            }
            (Phase::Flags(spell_id), LoadedSpellInput::Applied) => {
                // The caller's single dependent/favorite writer has already run,
                // including when that writer failed to reach a canonical owner.
                if !known_spells.contains(spell_id) {
                    known_spells.push(*spell_id);
                    self.pending.push(*spell_id);
                    self.added += 1;
                }
                self.phase = Phase::ConsiderOverride;
            }
            (Phase::Override(_, _), LoadedSpellInput::Applied) => {
                self.current = None;
                self.phase = Phase::Rows;
            }
            _ => unreachable!("input must answer the current loaded spell step"),
        }
    }
}

impl PlayerSpellRuntimeState {
    /// Preserve the loaded send projection; do not modify canonical spell rows.
    pub fn deactivate_lower_loaded_ranks(
        known_spells: &mut Vec<i32>,
        mut next_rank: impl FnMut(u32) -> u32,
    ) -> usize {
        let known_set: std::collections::HashSet<i32> = known_spells.iter().copied().collect();
        let before = known_spells.len();
        known_spells.retain(|spell_id| {
            let Ok(mut next_spell_id) = u32::try_from(*spell_id) else {
                return true;
            };

            loop {
                next_spell_id = next_rank(next_spell_id);
                if next_spell_id == 0 {
                    return true;
                }

                if let Ok(next_spell_i32) = i32::try_from(next_spell_id)
                    && known_set.contains(&next_spell_i32)
                {
                    return false;
                }
            }
        });

        before - known_spells.len()
    }
}
