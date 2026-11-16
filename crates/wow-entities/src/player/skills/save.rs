use super::{PlayerSkills, SkillStatusOrderError, SkillUpdateState};

#[cfg(test)]
#[path = "../../../unit_tests/player/skills/save/mod.rs"]
mod tests;

/// One dirty native status row, projected in admitted source traversal order.
/// `value` and `maximum` are copied directly from the physical status slot,
/// including for Deleted entries. SQL key narrowing belongs to its adapter.
#[derive(Debug, PartialEq, Eq)]
pub struct SkillSaveRow {
    skill: u32,
    state: SkillUpdateState,
    value: u16,
    maximum: u16,
    profession_slot: i8,
    slot: u16,
    line: u16,
    step: u16,
}

impl SkillSaveRow {
    pub fn skill(&self) -> u32 {
        self.skill
    }

    pub fn state(&self) -> SkillUpdateState {
        self.state
    }

    pub fn value(&self) -> u16 {
        self.value
    }

    pub fn maximum(&self) -> u16 {
        self.maximum
    }

    pub fn profession_slot(&self) -> i8 {
        self.profession_slot
    }
}

/// Opaque, non-Clone save capture. It carries no Player incarnation or commit
/// proof; the owner must retain its existing receipt and ACK only confirmed
/// expected skill-group commits.
pub struct SkillSaveBatch {
    rows: Vec<SkillSaveRow>,
}

impl SkillSaveBatch {
    pub fn rows(&self) -> &[SkillSaveRow] {
        &self.rows
    }
}

impl PlayerSkills {
    /// Capture dirty rows after admitting the complete native status traversal.
    /// The order provider sees all physical keys, including clean and Deleted
    /// entries, before unchanged rows are filtered from the returned batch.
    pub fn capture_save<E>(
        &self,
        order: impl FnOnce(&[u32], usize) -> Result<Vec<u32>, E>,
    ) -> Result<SkillSaveBatch, SkillStatusOrderError<E>> {
        let rows = self
            .source_statuses(order)?
            .into_iter()
            .filter_map(|entry| {
                let state = entry.state();
                (state != SkillUpdateState::Unchanged).then(|| {
                    let fields = entry.fields();
                    SkillSaveRow {
                        skill: entry.skill(),
                        state,
                        value: fields.rank(),
                        maximum: fields.maximum(),
                        profession_slot: self
                            .profession_slot(entry.skill())
                            .map_or(-1, |slot| slot as i8),
                        slot: entry.slot() as u16,
                        line: fields.line(),
                        step: fields.step(),
                    }
                })
            })
            .collect();
        Ok(SkillSaveBatch { rows })
    }

    /// Reconcile one confirmed skills-group commit. The Player save owner must
    /// first validate its incarnation, expected group and full-save fence.
    pub(in crate::player) fn acknowledge_save(&mut self, batch: SkillSaveBatch) {
        for row in batch.rows {
            let Some(current_status) = self.status.get(&row.skill) else {
                continue;
            };
            if current_status.slot != row.slot {
                continue;
            }

            let current_state = current_status.state;
            let current_fields = self.fields[usize::from(current_status.slot)];
            let current_profession_slot = self
                .profession_slot(row.skill)
                .map_or(-1, |slot| slot as i8);
            let matches_capture = current_state == row.state
                && current_fields.line() == row.line
                && current_fields.step() == row.step
                && current_fields.rank() == row.value
                && current_fields.maximum() == row.maximum
                && current_profession_slot == row.profession_slot;

            let acknowledged_state = if matches_capture {
                SkillUpdateState::Unchanged
            } else {
                match row.state {
                    SkillUpdateState::New | SkillUpdateState::Changed => match current_state {
                        SkillUpdateState::Deleted => SkillUpdateState::Deleted,
                        SkillUpdateState::New | SkillUpdateState::Changed => {
                            SkillUpdateState::Changed
                        }
                        SkillUpdateState::Unchanged
                            if row.state == SkillUpdateState::New && current_fields.rank() == 0 =>
                        {
                            SkillUpdateState::Deleted
                        }
                        SkillUpdateState::Unchanged => SkillUpdateState::Changed,
                    },
                    SkillUpdateState::Deleted => match current_state {
                        SkillUpdateState::New | SkillUpdateState::Changed => SkillUpdateState::New,
                        SkillUpdateState::Deleted | SkillUpdateState::Unchanged => current_state,
                    },
                    SkillUpdateState::Unchanged => continue,
                }
            };

            self.status
                .get_mut(&row.skill)
                .expect("captured status remains present during ACK")
                .state = acknowledged_state;
        }
    }
}
