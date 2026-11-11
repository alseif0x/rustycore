//! 02245dcd Player.cpp:3227-3444. Removal is not an atomic/save transaction.
use super::super::PlayerSpellState as State;
use super::{
    AddPlayerSpell, LearningError, PlayerSpellBook, RemovePlayerSpell, SpellBookMessage,
    SpellLearningEffects, SpellLearningSource, effect, source,
};
use std::collections::BTreeSet;

use crate::{SpellBookDefinition, SpellLearningError, SpellLearningSourceError};
impl PlayerSpellBook {
    pub fn remove_spell<S: SpellLearningSource, E: SpellLearningEffects<S>>(
        &mut self,
        request: RemovePlayerSpell,
        sources: &S,
        effects: &mut E,
    ) -> Result<(), LearningError<E::Error, S>> {
        self.remove_inner(request, sources, effects, &mut BTreeSet::new())
    }
    fn remove_inner<S: SpellLearningSource, E: SpellLearningEffects<S>>(
        &mut self,
        request: RemovePlayerSpell,
        sources: &S,
        effects: &mut E,
        path: &mut BTreeSet<u32>,
    ) -> Result<(), LearningError<E::Error, S>> {
        let Some(entry) = self.spell(request.spell) else {
            return Ok(());
        };
        if entry.state == State::Removed
            || (request.disabled && entry.disabled)
            || entry.state == State::Temporary
        {
            return Ok(());
        }
        // Internal next/required recursion happens before own mutation/effects.
        // An eligible node repeating on this path is a source nonterminating
        // dependency cycle. External aura/skill reentry starts a fresh path and
        // is not rejected merely for having the same spell ID.
        if !path.insert(request.spell) {
            return Err(source(SpellLearningSourceError::RecursiveRemovalCycle));
        }
        let result = self.remove_apply(request, sources, effects, path);
        path.remove(&request.spell);
        result
    }
    fn remove_apply<S: SpellLearningSource, E: SpellLearningEffects<S>>(
        &mut self,
        request: RemovePlayerSpell,
        sources: &S,
        effects: &mut E,
        path: &mut BTreeSet<u32>,
    ) -> Result<(), LearningError<E::Error, S>> {
        let next = sources.next_spell_in_chain(request.spell);
        if next != 0 {
            // AssertSpellInfo precedes HasSpell, including an unknown next rank.
            let next_info = super::facts::spell(sources, next)
                .map_err(source)?
                .ok_or_else(|| source(SpellLearningSourceError::MissingAssertedDefinition))?;
            if self.has_spell(next) && next_info.custom_attributes() & 0x00800000 == 0 {
                let mut higher = RemovePlayerSpell::new(next);
                higher.disabled = request.disabled;
                higher.learn_low_rank = false;
                self.remove_inner(higher, sources, effects, path)?;
            }
        }
        for id in sources
            .spells_requiring(request.spell)
            .expect("admitted required phase")
        {
            let mut dependent = RemovePlayerSpell::new(id);
            dependent.disabled = request.disabled;
            self.remove_inner(dependent, sources, effects, path)?;
        }
        let Some(before) = self.spell(request.spell).copied() else {
            return Ok(());
        };
        // Re-search only: the source does NOT repeat its state/disabled guards
        // here, even if a completed child callback already marked it Removed.
        if request.disabled {
            let current = self.entries.get_mut(&request.spell).unwrap();
            current.disabled = true;
            if current.state != State::New {
                current.state = State::Changed;
            }
        } else if before.state == State::New {
            self.erase(request.spell);
        } else {
            self.entries.get_mut(&request.spell).unwrap().state = State::Removed;
        }
        effect(effects.remove_owned_aura(self, request.spell))?;
        let view = super::facts::spell(sources, request.spell)
            .map_err(source)?
            .ok_or_else(|| source(SpellLearningSourceError::MissingAssertedDefinition))?;
        for index in 0..view.effect_count() {
            effect(effects.remove_pet_aura_if_defined(self, request.spell, index as u8))?;
        }
        if super::facts::primary_profession_first(sources, &view) {
            let free = effects.free_profession_points().wrapping_add(1);
            if free <= effects.max_primary_professions() {
                effect(effects.set_free_profession_points(free))?;
            }
        }
        self.remove_spell_skills(request.spell, sources, effects)?;
        for node in sources
            .spell_learn_nodes(request.spell)
            .expect("admitted learning phase")
        {
            let other_teaches = sources
                .spell_learned_by(node.spell)
                .expect("admitted reverse phase")
                .any(|other| {
                    other.source != request.spell && other.active && self.has_spell(other.source)
                });
            if other_teaches {
                continue;
            }
            let mut dependent = RemovePlayerSpell::new(node.spell);
            dependent.disabled = request.disabled;
            self.remove_inner(dependent, sources, effects, path)?;
            if node.overrides_spell != 0 {
                self.remove_override_spell(node.overrides_spell, node.spell);
            }
        }
        let previous = sources.previous_spell_in_chain(request.spell);
        let mut activated = false;
        if previous != 0 && before.active && sources.spell_rank_node(request.spell).is_some() {
            if let Some(entry) = self.entries.get_mut(&previous) {
                if entry.dependent != before.dependent {
                    entry.dependent = before.dependent;
                    if entry.state != State::New {
                        entry.state = State::Changed;
                    }
                }
                if !entry.active && request.learn_low_rank {
                    let mut lower = AddPlayerSpell::learned(previous, true, entry.dependent);
                    lower.learning = false;
                    lower.disabled = entry.disabled;
                    if self.add_spell(lower, sources, effects)? {
                        // No IsInWorld gate here in source, unlike AddSpell's rank loop.
                        effect(effects.publish(
                            self,
                            SpellBookMessage::Superseded {
                                old: request.spell,
                                new: previous,
                            },
                        ))?;
                        activated = true;
                    }
                }
            }
        }
        if let Some(trait_data) = before.trait_data {
            if let Some(original) = effects
                .trait_override(trait_data.definition_id())
                .map_err(SpellLearningError::Effect)?
            {
                self.remove_override_spell(original, request.spell);
            }
        }
        self.overrides.remove(&request.spell);
        let passive = view.attributes()[0] & 0x40 != 0;
        if effects.can_titan_grip() && passive && view.has_effect(155) {
            let penalty = effects.titan_grip_penalty_spell();
            effect(effects.remove_auras_due_to_spell(self, penalty))?;
            effect(effects.disable_titan_grip())?;
        }
        if effects.can_dual_wield() && passive && view.has_effect(40) {
            effect(effects.disable_dual_wield())?;
        }
        if effects.offhand_check_at_unlearn() {
            effect(effects.auto_unequip_offhand(self))?;
        }
        if !activated {
            effect(effects.publish(
                self,
                SpellBookMessage::Unlearned {
                    spell: request.spell,
                    suppress_messaging: request.suppress_messaging,
                },
            ))?;
        }
        Ok(())
    }
}
