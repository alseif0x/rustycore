//! Original AI preparation; all primaries prepare before any queue consumption.
use super::*;

pub(super) fn turret(
    creature: &mut WorldCreature,
    guid: ObjectGuid,
    recipient_guid: ObjectGuid,
    map_id: u16,
    instance_id: u32,
    difficulty_id: u8,
    policies: &mut SpellPolicies<'_>,
    outcome: &mut SpellOutcome,
    pending_actions: &mut Vec<SpellAction>,
) {
    let spells = creature.creature.spells();

    if difficulty_id != 0 && spells[0] != 0 {
        // See the CombatAI gate above: TurretAI has the same
        // unhydrated difficulty-specific cast metadata.
        outcome.spell_effects_unrepresented += 1;
        return;
    }
    // C++ TurretAI only ever reads `m_spells[0]`; unrelated
    // template slots cannot put it into UNIT_STATE_CASTING.
    if has_noninstant_spell(&spells[..1], difficulty_id, policies) {
        outcome.noninstant_casts_unrepresented += 1;
        return;
    }
    if creature
        .creature
        .unit()
        .has_unit_state(UnitState::CASTING.bits())
    {
        outcome.unit_state_casting_skips += 1;
        return;
    }
    let spell_id = spells[0];
    if spell_id == 0 || !creature.can_swing() {
        return;
    }
    let Some(spell) = i32::try_from(spell_id)
        .ok()
        .and_then(|spell_id| (policies.info)(spell_id as u32, difficulty_id, true))
    else {
        outcome.missing_spell_metadata += 1;
        return;
    };

    match disable(spell_id, map_id, creature, policies) {
        SpellDisable::Enabled => {}
        SpellDisable::Disabled => {
            outcome.spells_disabled += 1;
            // C++ reaches `CastSpell` and lets `Spell::CheckCast`
            // reject the disabled spell, so BASE_ATTACK is still
            // consumed whenever the raw combat-range gate in
            // `DoSpellAttackIfReady` admitted the attempt.
            // Deciding that here would use the pre-tick legacy
            // position, so defer it to the canonical drain.
            pending_actions.push(SpellAction::TurretRejectedAttempt(SpellRejectedAttempt {
                caster_guid: guid,
                target_guid: recipient_guid,
                map_id,
                instance_id,
                engagement_epoch: creature.creature_spell_engagement_epoch_like_cpp(),
                spell_id,
                difficulty_id,
            }));
            return;
        }
        SpellDisable::Unrepresented => {
            outcome.spell_disable_context_unrepresented += 1;
            return;
        }
    }
    if !(policies.check)(SpellPreparationCheck::RuntimeHooks, spell_id, difficulty_id) {
        outcome.spell_runtime_hooks_unrepresented += 1;
        return;
    }
    if !(policies.check)(
        SpellPreparationCheck::CastingRequirements,
        spell_id,
        difficulty_id,
    ) {
        outcome.spell_casting_requirements_unrepresented += 1;
        return;
    }
    if !(policies.check)(
        SpellPreparationCheck::ShapeshiftRequirements,
        spell_id,
        difficulty_id,
    ) {
        outcome.spell_casting_requirements_unrepresented += 1;
        creature.record_swing();
        return;
    }
    if !(policies.check)(
        SpellPreparationCheck::AuraRestrictions,
        spell_id,
        difficulty_id,
    ) {
        outcome.spell_effects_unrepresented += 1;
        return;
    }
    if !cooldown_semantics(spell_id, difficulty_id, policies) {
        outcome.spell_effects_unrepresented += 1;
        return;
    }
    if combat_forbidden(spell_id, difficulty_id, policies) {
        outcome.spell_effects_unrepresented += 1;
        // TurretAI resets BASE_ATTACK after its CastSpell call
        // even when CheckCast rejects the peaceful-only spell.
        creature.record_swing();
        return;
    }
    if implicit_cost(&spell, difficulty_id, policies, creature) {
        outcome.spell_effects_unrepresented += 1;
        return;
    }
    if target_restrictions(spell_id, difficulty_id, policies) {
        outcome.spell_effects_unrepresented += 1;
        return;
    }
    match single_unit_topology(
        &spell,
        recipient_guid,
        recipient_guid,
        projectile(spell_id, difficulty_id, policies),
    ) {
        Ok(()) => {}
        Err(SpellTopologyError::NonInstant) => {
            outcome.noninstant_casts_unrepresented += 1;
            return;
        }
        Err(SpellTopologyError::ProjectileOrAmmo) => {
            outcome.spell_projectiles_unrepresented += 1;
            return;
        }
        Err(SpellTopologyError::EffectOrTarget) => {
            outcome.spell_effects_unrepresented += 1;
            return;
        }
    }
    let Ok(command) = cast_plan(
        creature,
        guid,
        recipient_guid,
        map_id,
        instance_id,
        spell_id,
        &spell,
        difficulty_id,
        policies,
    ) else {
        outcome.spell_visuals_unrepresented += 1;
        return;
    };
    pending_actions.push(SpellAction::Cast(SpellCast {
        command,
        difficulty_id,
        turret_ai: true,
    }));
    outcome.casts_ready += 1;
}
