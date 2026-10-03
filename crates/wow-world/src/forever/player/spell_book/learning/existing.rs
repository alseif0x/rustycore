use super::super::PlayerSpellState as State;
use super::{
    AddPlayerSpell, PlayerSpellBook, SpellBookMessage, SpellLearningEffects, SpellLearningError,
    SpellLearningSources, effect, source,
};
pub(super) enum Existing {
    Done(bool),
    Continue { state: State, disabled_case: bool },
}
impl PlayerSpellBook {
    pub(super) fn existing_spell<E: SpellLearningEffects>(
        &mut self,
        request: &mut AddPlayerSpell,
        sources: &SpellLearningSources<'_>,
        effects: &mut E,
    ) -> Result<Existing, SpellLearningError<E::Error>> {
        let mut state = if request.learning {
            State::New
        } else {
            State::Unchanged
        };
        let Some(entry) = self.spell(request.spell).copied() else {
            return Ok(Existing::Continue {
                state,
                disabled_case: false,
            });
        };
        if entry.state == State::Temporary {
            self.remove_temporary_spell(request.spell);
            return Ok(Existing::Continue {
                state,
                disabled_case: false,
            });
        }
        let mut next_active = 0;
        if sources.spells.spell_rank_node(request.spell).is_some() {
            let next = sources.spells.next_spell_in_chain(request.spell);
            if next != 0 && self.has_spell(next) {
                request.active = false;
                next_active = next;
            }
        }
        if entry.state != State::Removed
            && entry.active == request.active
            && entry.dependent == request.dependent
            && entry.disabled == request.disabled
        {
            if !effects.is_in_world() && !request.learning {
                self.entries.get_mut(&request.spell).unwrap().state = State::Unchanged;
            }
            return Ok(Existing::Done(false)); // Trait/favorite unchanged on this path.
        }
        let mut dependent_set = false;
        if entry.state != State::Removed && !entry.dependent && request.dependent {
            let current = self.entries.get_mut(&request.spell).unwrap();
            current.dependent = true;
            if current.state != State::New {
                current.state = State::Changed;
            }
            dependent_set = true;
        }
        if entry.trait_data != request.trait_data {
            if let Some(trait_data) = entry.trait_data {
                if let Some(original) = effects
                    .trait_override(trait_data.definition_id())
                    .map_err(SpellLearningError::Effect)?
                {
                    self.remove_override_spell(original, request.spell); // Includes zero.
                }
            }
            self.entries.get_mut(&request.spell).unwrap().trait_data = request.trait_data;
        }
        self.entries.get_mut(&request.spell).unwrap().favorite = request.favorite;
        if entry.active != request.active && entry.state != State::Removed && !entry.disabled {
            let current = self.entries.get_mut(&request.spell).unwrap();
            current.active = request.active;
            if !effects.is_in_world() && !request.learning && !dependent_set {
                current.state = State::Unchanged;
            } else if current.state != State::New {
                current.state = State::Changed;
            }
            if request.active {
                if sources.passive(request.spell).map_err(source)? {
                    let view = sources
                        .spell(request.spell)
                        .map_err(source)?
                        .expect("admitted spell");
                    if self.passive_learn(&view, effects)? {
                        effect(effects.cast_triggered(self, request.spell))?;
                    }
                }
            } else if effects.is_in_world() {
                let message = if next_active != 0 {
                    SpellBookMessage::Superseded {
                        old: request.spell,
                        new: next_active,
                    }
                } else {
                    SpellBookMessage::Unlearned {
                        spell: request.spell,
                        suppress_messaging: false,
                    }
                };
                effect(effects.publish(self, message))?;
            }
            return Ok(Existing::Done(request.active));
        }
        if entry.disabled != request.disabled && entry.state != State::Removed {
            let current = self.entries.get_mut(&request.spell).unwrap();
            if current.state != State::New {
                current.state = State::Changed;
            }
            current.disabled = request.disabled;
            return Ok(if request.disabled {
                Existing::Done(false)
            } else {
                Existing::Continue {
                    state,
                    disabled_case: true,
                }
            });
        }
        match self.spell(request.spell).unwrap().state {
            State::Unchanged => return Ok(Existing::Done(false)),
            State::Removed => {
                self.erase(request.spell);
                state = State::Changed;
            }
            _ => {
                if !effects.is_in_world() && !request.learning && !dependent_set {
                    self.entries.get_mut(&request.spell).unwrap().state = State::Unchanged;
                }
                return Ok(Existing::Done(false));
            }
        }
        Ok(Existing::Continue {
            state,
            disabled_case: false,
        })
    }
}
