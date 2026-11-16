//! 02245dcd Player.cpp:21772-21824. Prepare one isolated spell-save operation.
use super::order::SpellBookOrderError;
use super::{PlayerSpellBook, PlayerSpellEntry, PlayerSpellState, SpellBookMutation};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellSaveAction {
    DeletePrimary {
        spell: u32,
    },
    InsertPrimary {
        spell: u32,
        active: bool,
        disabled: bool,
    },
    DeleteFavorite {
        spell: u32,
    },
    InsertFavorite {
        spell: u32,
    },
}

/// Opaque operation token plus immutable statements; it is deliberately non-Clone.
pub struct SpellSaveBatch {
    operation: u64,
    actions: Vec<SpellSaveAction>,
}
impl SpellSaveBatch {
    pub fn actions(&self) -> &[SpellSaveAction] {
        &self.actions
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SpellSavePrepareError<E> {
    PendingOperation,
    SequenceExhausted,
    Order(SpellBookOrderError<E>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellSaveFinishError {
    NoPending,
    WrongOperation,
}

#[derive(Clone, Copy)]
struct CapturedEntry {
    spell: u32,
    state: PlayerSpellState,
    active: bool,
    dependent: bool,
    disabled: bool,
    favorite: bool,
}
impl CapturedEntry {
    fn new(spell: u32, entry: PlayerSpellEntry) -> Self {
        Self {
            spell,
            state: entry.state,
            active: entry.active,
            dependent: entry.dependent,
            disabled: entry.disabled,
            favorite: entry.favorite,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FinishKind {
    Upsert { inserted_primary: bool },
    DeleteOnly,
    Removed,
}

struct PendingSpellRow {
    spell: u32,
    captured: Option<CapturedEntry>,
    finish: FinishKind,
}

pub(super) struct PendingSpellSave {
    operation: u64,
    captured_obligations: Vec<u32>,
    rows: Vec<PendingSpellRow>,
}

impl PlayerSpellBook {
    /// Plan from complete native membership before advancing sequence, storing
    /// pending facts, erasing Removed entries, or changing any dirty state.
    pub fn prepare_save<E>(
        &mut self,
        order: impl FnOnce(&[SpellBookMutation], usize) -> Result<Vec<u32>, E>,
    ) -> Result<SpellSaveBatch, SpellSavePrepareError<E>> {
        if self.pending_save.is_some() {
            return Err(SpellSavePrepareError::PendingOperation);
        }
        let next_sequence = self
            .save_sequence
            .checked_add(1)
            .ok_or(SpellSavePrepareError::SequenceExhausted)?;
        let operation = self.save_sequence;

        let ordered = self
            .source_entries(order)
            .map_err(SpellSavePrepareError::Order)?;
        let captured_entries = ordered
            .iter()
            .map(|(spell, entry)| (*spell, CapturedEntry::new(*spell, **entry)))
            .collect::<Vec<_>>();
        drop(ordered);

        let live = captured_entries
            .iter()
            .map(|(spell, _)| *spell)
            .collect::<HashSet<_>>();
        let obligations = self.primary_delete_obligations.clone();
        let obligation_set = obligations.iter().copied().collect::<HashSet<_>>();
        let mut actions = Vec::new();
        let mut rows = Vec::new();
        let mut erase_removed = Vec::new();

        // Persisted deletes for IDs without a current node precede native rows.
        for &spell in &obligations {
            if !live.contains(&spell) {
                actions.push(SpellSaveAction::DeletePrimary { spell });
                rows.push(PendingSpellRow {
                    spell,
                    captured: None,
                    finish: FinishKind::DeleteOnly,
                });
            }
        }

        for &(spell, captured) in &captured_entries {
            let has_delete_obligation = obligation_set.contains(&spell);
            match captured.state {
                PlayerSpellState::Removed => {
                    actions.push(SpellSaveAction::DeletePrimary { spell });
                    rows.push(PendingSpellRow {
                        spell,
                        captured: Some(captured),
                        finish: FinishKind::Removed,
                    });
                    erase_removed.push(spell);
                }
                PlayerSpellState::Temporary if has_delete_obligation => {
                    actions.push(SpellSaveAction::DeletePrimary { spell });
                    rows.push(PendingSpellRow {
                        spell,
                        captured: Some(captured),
                        finish: FinishKind::DeleteOnly,
                    });
                }
                PlayerSpellState::Temporary => {}
                PlayerSpellState::Unchanged if !has_delete_obligation => {}
                PlayerSpellState::Unchanged | PlayerSpellState::Changed | PlayerSpellState::New => {
                    if captured.state == PlayerSpellState::New && has_delete_obligation {
                        actions.push(SpellSaveAction::DeletePrimary { spell });
                    } else if captured.state == PlayerSpellState::Changed
                        || captured.state == PlayerSpellState::Unchanged
                    {
                        actions.push(SpellSaveAction::DeletePrimary { spell });
                    }
                    append_upsert_actions(&mut actions, captured);
                    rows.push(PendingSpellRow {
                        spell,
                        captured: Some(captured),
                        finish: FinishKind::Upsert {
                            inserted_primary: !captured.dependent,
                        },
                    });
                }
            }
        }

        let pending = PendingSpellSave {
            operation,
            captured_obligations: obligations,
            rows,
        };
        self.save_sequence = next_sequence;
        for spell in erase_removed {
            self.erase(spell);
        }
        self.pending_save = Some(pending);

        Ok(SpellSaveBatch { operation, actions })
    }

    /// Confirm only this book's matching operation. The outer receipt still
    /// owns Player identity, group confirmation and COMMIT proof.
    pub(in crate::player) fn confirm_save(
        &mut self,
        batch: SpellSaveBatch,
    ) -> Result<(), SpellSaveFinishError> {
        let Some(pending) = self.pending_save.as_ref() else {
            return Err(SpellSaveFinishError::NoPending);
        };
        if pending.operation != batch.operation {
            return Err(SpellSaveFinishError::WrongOperation);
        }
        let pending = self.pending_save.take().expect("checked pending operation");

        for spell in pending.captured_obligations {
            self.remove_primary_delete_obligation(spell);
        }
        for row in pending.rows {
            match row.finish {
                FinishKind::Upsert { inserted_primary } => {
                    let Some(current) = self.entries.get(&row.spell).copied() else {
                        if inserted_primary {
                            self.add_primary_delete_obligation(row.spell);
                        }
                        continue;
                    };
                    if current.state == PlayerSpellState::Temporary {
                        if inserted_primary {
                            self.add_primary_delete_obligation(row.spell);
                        }
                        continue;
                    }
                    if current.state == PlayerSpellState::Removed {
                        continue;
                    }
                    let captured = row.captured.expect("upsert rows capture persisted facts");
                    let exact = current.state == captured.state
                        && current.active == captured.active
                        && current.dependent == captured.dependent
                        && current.disabled == captured.disabled
                        && current.favorite == captured.favorite;
                    self.entries.get_mut(&row.spell).unwrap().state = if exact {
                        PlayerSpellState::Unchanged
                    } else {
                        PlayerSpellState::Changed
                    };
                }
                FinishKind::DeleteOnly | FinishKind::Removed => {
                    if let Some(current) = self.entries.get_mut(&row.spell) {
                        if matches!(
                            current.state,
                            PlayerSpellState::Unchanged
                                | PlayerSpellState::New
                                | PlayerSpellState::Changed
                        ) {
                            current.state = PlayerSpellState::New;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Known rollback/not-started retains gameplay membership and dirty tags;
    /// Removed nodes stay erased and gain retry-only primary DELETE obligations.
    pub(in crate::player) fn rollback_save(
        &mut self,
        batch: SpellSaveBatch,
    ) -> Result<(), SpellSaveFinishError> {
        let Some(pending) = self.pending_save.as_ref() else {
            return Err(SpellSaveFinishError::NoPending);
        };
        if pending.operation != batch.operation {
            return Err(SpellSaveFinishError::WrongOperation);
        }
        let pending = self.pending_save.take().expect("checked pending operation");
        for row in pending.rows {
            if row.finish == FinishKind::Removed {
                self.add_primary_delete_obligation(row.spell);
            }
        }
        Ok(())
    }

    pub(super) fn pending_captured_new(&self, spell: u32) -> bool {
        self.pending_save.as_ref().is_some_and(|pending| {
            pending.rows.iter().any(|row| {
                row.spell == spell
                    && matches!(row.finish, FinishKind::Upsert { .. })
                    && row
                        .captured
                        .is_some_and(|entry| entry.state == PlayerSpellState::New)
            })
        })
    }

    pub(super) fn pending_protects_unconfirmed_intent(&self, spell: u32) -> bool {
        self.pending_save.as_ref().is_some_and(|pending| {
            pending
                .rows
                .iter()
                .any(|row| row.spell == spell && matches!(row.finish, FinishKind::Upsert { .. }))
        })
    }

    fn add_primary_delete_obligation(&mut self, spell: u32) {
        if !self.primary_delete_obligations.contains(&spell) {
            self.primary_delete_obligations.push(spell);
        }
    }

    fn remove_primary_delete_obligation(&mut self, spell: u32) {
        if let Some(index) = self
            .primary_delete_obligations
            .iter()
            .position(|&candidate| candidate == spell)
        {
            self.primary_delete_obligations.remove(index);
        }
    }
}

fn append_upsert_actions(actions: &mut Vec<SpellSaveAction>, entry: CapturedEntry) {
    if !entry.dependent {
        actions.push(SpellSaveAction::InsertPrimary {
            spell: entry.spell,
            active: entry.active,
            disabled: entry.disabled,
        });
    }
    actions.push(SpellSaveAction::DeleteFavorite { spell: entry.spell });
    if entry.favorite {
        actions.push(SpellSaveAction::InsertFavorite { spell: entry.spell });
    }
}
