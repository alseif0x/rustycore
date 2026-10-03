use super::super::PlayerSpellState as State;
use super::existing::Existing;
use super::{
    AddPlayerSpell, LearnPlayerSpell, PlayerSpellBook, SpellLearnCriterion, SpellLearningEffects,
    SpellLearningError, SpellLearningSourceError, SpellLearningSources, effect, source,
};

impl PlayerSpellBook {
    /// Source AddSpell's bool means active client admission, not durable save.
    /// Mandatory effects run immediately and can reenter this same book. There
    /// is no blanket spell-ID recursion guard: previous-rank/skill auto-learning
    /// can legitimately insert the original spell before its outer try_emplace.
    pub fn add_spell<E: SpellLearningEffects>(
        &mut self,
        mut request: AddPlayerSpell,
        sources: &SpellLearningSources<'_>,
        effects: &mut E,
    ) -> Result<bool, SpellLearningError<E::Error>> {
        let view = sources.spell(request.spell).map_err(source)?;
        let valid = view.is_some()
            && sources
                .spells
                .spell_is_valid(request.spell, 0, sources.items)
                .map_err(|error| source(SpellLearningSourceError::Validity(error)))?;
        if !valid {
            if !effects.is_in_world() && !request.learning {
                // Source deletes this spell for ALL characters on invalid load.
                // This task grants no global destructive DB-cleanup authority.
                return Err(source(SpellLearningSourceError::UnauthorizedGlobalCleanup));
            }
            return Ok(false);
        }
        let view = view.expect("validated definition");
        let (state, disabled_case) = match self.existing_spell(&mut request, sources, effects)? {
            Existing::Done(value) => return Ok(value),
            Existing::Continue {
                state,
                disabled_case,
            } => (state, disabled_case),
        };
        let mut superseded_old = false;
        if !disabled_case {
            let previous = sources.spells.previous_spell_in_chain(request.spell);
            if previous != 0 {
                if !effects.is_in_world() || request.disabled {
                    let mut prior = AddPlayerSpell::learned(previous, request.active, true);
                    prior.disabled = request.disabled;
                    prior.from_skill = request.from_skill;
                    let _ = self.add_spell(prior, sources, effects)?;
                } else {
                    self.learn_spell(
                        LearnPlayerSpell::new(previous, true, request.from_skill),
                        sources,
                        effects,
                    )?;
                }
            }
            let (current, inserted) = self.try_emplace(request.spell);
            current.state = if inserted { state } else { State::Changed };
            current.active = request.active;
            current.dependent = request.dependent;
            current.disabled = request.disabled;
            current.favorite = request.favorite;
            current.trait_data = request.trait_data;
            if current.active
                && !current.disabled
                && sources.spells.spell_rank_node(request.spell).is_some()
            {
                superseded_old = self.replace_ranks(request.spell, sources, effects)?;
            }
            if self.spell(request.spell).unwrap().disabled {
                return Ok(false);
            }
        }
        let cast = if !request.loading
            && view.custom_attributes() & 0x00800000 != 0
            && view.has_effect(36)
        {
            true
        } else if view.fields().attributes[0] & 0x40 != 0 {
            self.passive_learn(&view, effects)?
        } else {
            view.has_effect(44) || view.fields().attributes[1] & 0x80000000 != 0
        };
        if cast {
            effect(effects.cast_triggered(self, request.spell))?;
            if view.has_effect(44) {
                return Ok(false);
            }
        }
        if let Some(trait_data) = request.trait_data {
            if let Some(original) = effects
                .trait_override(trait_data.definition_id())
                .map_err(SpellLearningError::Effect)?
            {
                if original != 0 {
                    self.add_override_spell(original, request.spell);
                }
            }
        }
        let free = effects.free_profession_points();
        if free != 0 && sources.primary_profession_first(&view) {
            effect(effects.set_free_profession_points(free - 1))?;
        }
        self.add_spell_skills(request, sources, effects)?;
        for node in sources
            .spells
            .spell_learn_nodes(request.spell)
            .expect("admitted learning phase")
        {
            if !node.auto_learned {
                if !effects.is_in_world() || !node.active {
                    let _ = self.add_spell(
                        AddPlayerSpell::learned(node.spell, node.active, true),
                        sources,
                        effects,
                    )?;
                } else {
                    self.learn_spell(LearnPlayerSpell::new(node.spell, true, 0), sources, effects)?;
                }
            }
            if node.overrides_spell != 0 && node.active {
                self.add_override_spell(node.overrides_spell, node.spell);
            }
        }
        if !effects.is_player_loading() {
            for ability in sources
                .spells
                .skill_line_abilities(request.spell)
                .expect("admitted ability map")
            {
                effect(effects.update_criterion(
                    self,
                    SpellLearnCriterion::TradeskillSkillLine,
                    u32::from(ability.skill_line),
                ))?;
                effect(effects.update_criterion(
                    self,
                    SpellLearnCriterion::SpellFromSkillLine,
                    u32::from(ability.skill_line),
                ))?;
            }
            effect(effects.update_criterion(
                self,
                SpellLearnCriterion::LearnOrKnowSpell,
                request.spell,
            ))?;
        }
        if effects
            .has_mount_definition(request.spell)
            .map_err(SpellLearningError::Effect)?
        {
            let loading = !effects.is_in_world();
            effect(effects.add_mount(self, request.spell, loading))?;
        }
        // Captured flags, not current entry.active (source lower-rank quirk).
        Ok(request.active && !request.disabled && !superseded_old)
    }
}
