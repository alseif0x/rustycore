//! Canonical writes used by the spell-removal operation.
use super::super::PlayerSpellRuntimeState;
use super::{SpellUnlearnOwnerOutcome, SpellUnlearnOwnerStep};

impl PlayerSpellRuntimeState {
    pub fn apply_unlearn_step(
        &mut self,
        step: SpellUnlearnOwnerStep,
    ) -> SpellUnlearnOwnerOutcome {
        match step {
            SpellUnlearnOwnerStep::Forget { spell_id, preserve_complete } => {
                let forgotten = self.forget_known_spell_like_cpp(spell_id);
                let was_dependent = forgotten.was_dependent;
                if preserve_complete {
                    if was_dependent {
                        self.remove_row_like_cpp(spell_id);
                    } else if let Some(row) = self.row_mut_like_cpp(spell_id) {
                        row.active = false;
                        row.disabled = false;
                        row.dependent = false;
                        row.favorite = false;
                        row.state = crate::PlayerSpellLoadState::Removed;
                    }
                }
                SpellUnlearnOwnerOutcome::Forgotten { was_dependent }
            }
            SpellUnlearnOwnerStep::DropOverridesAndTrait { spell_id } => {
                self.remove_override_spell_entry_like_cpp(spell_id);
                SpellUnlearnOwnerOutcome::TraitDefinition(
                    self.take_trait_definition_id_like_cpp(spell_id),
                )
            }
        }
    }
}
