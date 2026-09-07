//! Represented session finalization: operation progress, not a database transaction.
//!
//! This coordinator owns only the obligation ledger. The Session adapter supplies
//! effects; no Session, SQL connection, mutable Player or transport lives here.
//! A step is recorded before its future is polled. Cancellation therefore leaves
//! an observable unresolved obligation and cannot turn re-entry into a replay.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizationMode {
    CharacterSelection,
    TimedLogout,
    Disconnect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum FinalizationStep {
    NativeTransfer,
    LootSettlement,
    Buyback,
    CharacterSave,
    Mounts,
    Toys,
    Heirlooms,
    Appearances,
    Illusions,
    CharacterOffline,
    Retirement,
    LogoutPublication,
    CharacterAccountOffline,
    LoginAccountOffline,
    Release,
}

const STEP_COUNT: usize = 15;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizationOutcome {
    NotAttempted,
    /// The await may have been cancelled. This is not a rollback receipt.
    InFlight,
    NoWork,
    Applied,
    DefinitelyRolledBack,
    Unknown,
    Unavailable,
    Deferred,
    RetirementFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizationDisposition {
    InProgress,
    Complete,
    RetainAndEscalate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizationReport {
    pub mode: FinalizationMode,
    pub player: Option<wow_map::PlayerHandle>,
    pub disposition: FinalizationDisposition,
    outcomes: [FinalizationOutcome; STEP_COUNT],
}

impl FinalizationReport {
    pub fn outcome(&self, step: FinalizationStep) -> FinalizationOutcome {
        self.outcomes[step as usize]
    }
}

/// One attempt, retained by its execution owner across cancellation. There is no
/// reset/retry operation. A new character-selection cycle constructs a new value
/// only after completion and release of the previous exact incarnation.
#[derive(Debug)]
pub(crate) struct SessionFinalization {
    report: FinalizationReport,
    required: [bool; STEP_COUNT],
    retained_collection: Option<wow_persistence::AccountCollectionSaveLikeCpp>,
    has_player: bool,
}

impl SessionFinalization {
    pub(crate) fn new(
        mode: FinalizationMode,
        has_player: bool,
        player: Option<wow_map::PlayerHandle>,
    ) -> Self {
        use FinalizationStep::*;
        let mut required = [false; STEP_COUNT];
        required[NativeTransfer as usize] = true;
        required[Release as usize] = true;
        if has_player {
            for step in [
                LootSettlement,
                Buyback,
                CharacterSave,
                Mounts,
                Toys,
                Heirlooms,
                Appearances,
                Illusions,
                CharacterOffline,
                Retirement,
                CharacterAccountOffline,
            ] {
                required[step as usize] = true;
            }
            required[LogoutPublication as usize] = mode != FinalizationMode::Disconnect;
        }
        required[LoginAccountOffline as usize] = mode != FinalizationMode::CharacterSelection;
        Self {
            report: FinalizationReport {
                mode,
                player,
                disposition: FinalizationDisposition::InProgress,
                outcomes: [FinalizationOutcome::NotAttempted; STEP_COUNT],
            },
            required,
            retained_collection: None,
            has_player,
        }
    }

    pub(crate) fn report(&self) -> FinalizationReport {
        self.report.clone()
    }

    pub(crate) fn is_unstarted(&self) -> bool {
        self.report.disposition == FinalizationDisposition::InProgress
            && self
                .report
                .outcomes
                .iter()
                .all(|outcome| *outcome == FinalizationOutcome::NotAttempted)
    }

    /// The application owns ordering; adapters can only execute the next admitted
    /// obligation. This is intentionally not a generic task/workflow framework.
    pub(crate) fn next_step(&self) -> Option<FinalizationStep> {
        use FinalizationStep::*;
        if self.report.disposition != FinalizationDisposition::InProgress {
            return None;
        }
        let plan: &[FinalizationStep] = if !self.has_player {
            if self.report.mode == FinalizationMode::CharacterSelection {
                &[NativeTransfer, Release]
            } else {
                &[NativeTransfer, LoginAccountOffline, Release]
            }
        } else {
            match self.report.mode {
                FinalizationMode::CharacterSelection => &[
                    NativeTransfer,
                    LootSettlement,
                    Buyback,
                    CharacterSave,
                    Mounts,
                    Toys,
                    Heirlooms,
                    Appearances,
                    Illusions,
                    CharacterOffline,
                    Retirement,
                    LogoutPublication,
                    CharacterAccountOffline,
                    Release,
                ],
                FinalizationMode::Disconnect => &[
                    NativeTransfer,
                    LootSettlement,
                    Buyback,
                    CharacterSave,
                    Mounts,
                    Toys,
                    Heirlooms,
                    Appearances,
                    Illusions,
                    CharacterOffline,
                    CharacterAccountOffline,
                    LoginAccountOffline,
                    Retirement,
                    Release,
                ],
                FinalizationMode::TimedLogout => &[
                    NativeTransfer,
                    LootSettlement,
                    Buyback,
                    CharacterSave,
                    Mounts,
                    Toys,
                    Heirlooms,
                    Appearances,
                    Illusions,
                    CharacterOffline,
                    Retirement,
                    LogoutPublication,
                    CharacterAccountOffline,
                    LoginAccountOffline,
                    Release,
                ],
            }
        };
        plan.iter()
            .copied()
            .find(|step| self.report.outcome(*step) == FinalizationOutcome::NotAttempted)
    }

    pub(crate) fn retain_collection(
        &mut self,
        request: wow_persistence::AccountCollectionSaveLikeCpp,
    ) {
        self.retained_collection = Some(request);
    }

    pub(crate) fn begin(&mut self, step: FinalizationStep) -> bool {
        if self.report.disposition != FinalizationDisposition::InProgress
            || self.next_step() != Some(step)
            || self.report.outcome(step) != FinalizationOutcome::NotAttempted
            || self
                .report
                .outcomes
                .contains(&FinalizationOutcome::InFlight)
        {
            self.report.disposition = FinalizationDisposition::RetainAndEscalate;
            return false;
        }
        self.report.outcomes[step as usize] = FinalizationOutcome::InFlight;
        true
    }

    pub(crate) fn finish(&mut self, step: FinalizationStep, outcome: FinalizationOutcome) -> bool {
        if self.report.outcome(step) != FinalizationOutcome::InFlight {
            self.report.disposition = FinalizationDisposition::RetainAndEscalate;
            return false;
        }
        self.report.outcomes[step as usize] = outcome;
        if !matches!(
            outcome,
            FinalizationOutcome::Applied | FinalizationOutcome::NoWork
        ) {
            // Intentional finalization hardening: a known failed obligation is
            // observable and retained too; it is not whole-operation success.
            self.report.disposition = FinalizationDisposition::RetainAndEscalate;
            return false;
        }
        if matches!(
            step,
            FinalizationStep::Mounts
                | FinalizationStep::Toys
                | FinalizationStep::Heirlooms
                | FinalizationStep::Appearances
                | FinalizationStep::Illusions
        ) {
            self.retained_collection = None;
        }
        true
    }

    pub(crate) fn complete(&mut self) {
        if self.report.disposition == FinalizationDisposition::InProgress
            && self
                .required
                .iter()
                .zip(self.report.outcomes.iter())
                .all(|(required, outcome)| {
                    !required
                        || matches!(
                            outcome,
                            FinalizationOutcome::Applied | FinalizationOutcome::NoWork
                        )
                })
            && !self
                .report
                .outcomes
                .contains(&FinalizationOutcome::InFlight)
        {
            self.report.disposition = FinalizationDisposition::Complete;
        } else {
            self.report.disposition = FinalizationDisposition::RetainAndEscalate;
        }
    }

    pub(crate) fn interrupt(&mut self) {
        if self.report.disposition != FinalizationDisposition::Complete {
            self.report.disposition = FinalizationDisposition::RetainAndEscalate;
        }
    }
}

impl From<wow_persistence::PersistenceOutcomeLikeCpp> for FinalizationOutcome {
    fn from(outcome: wow_persistence::PersistenceOutcomeLikeCpp) -> Self {
        match outcome {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => Self::Applied,
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { .. } => Self::DefinitelyRolledBack,
            wow_persistence::PersistenceOutcomeLikeCpp::Unknown { .. } => Self::Unknown,
        }
    }
}
