//! Lazy metadata is queried at the former planning points, not preloaded.
use super::*;

pub(super) fn condition(
    id: u32,
    difficulty: u8,
    policies: &mut SpellPolicies<'_>,
) -> SpellCondition {
    (policies.condition)(id, difficulty)
}
pub(super) fn target(
    id: u32,
    spell: &SpellInfoFacts,
    difficulty: u8,
    policies: &mut SpellPolicies<'_>,
) -> SpellTarget {
    (policies.target)(id, spell, difficulty)
}
pub(super) fn disable(
    id: u32,
    map_id: u16,
    actor: &WorldCreature,
    policies: &mut SpellPolicies<'_>,
) -> SpellDisable {
    (policies.disable)(id, map_id, actor.creature.unit().world().area_id())
}
pub(super) fn cooldown_semantics(
    id: u32,
    difficulty: u8,
    policies: &mut SpellPolicies<'_>,
) -> bool {
    (policies.check)(SpellPreparationCheck::CooldownSemantics, id, difficulty)
}
pub(super) fn combat_forbidden(id: u32, difficulty: u8, policies: &mut SpellPolicies<'_>) -> bool {
    (policies.check)(SpellPreparationCheck::CombatForbidden, id, difficulty)
}
pub(super) fn target_restrictions(
    id: u32,
    difficulty: u8,
    policies: &mut SpellPolicies<'_>,
) -> bool {
    (policies.check)(SpellPreparationCheck::TargetRestrictions, id, difficulty)
}
pub(super) fn projectile(id: u32, difficulty: u8, policies: &mut SpellPolicies<'_>) -> bool {
    (policies.check)(SpellPreparationCheck::Projectile, id, difficulty)
}
pub(super) fn minimum(
    id: u32,
    _spell: &SpellInfoFacts,
    difficulty: u8,
    policies: &mut SpellPolicies<'_>,
) -> u64 {
    (policies.minimum)(id, difficulty)
}

pub fn has_noninstant_spell(
    spells: &[u32],
    difficulty: u8,
    policies: &mut SpellPolicies<'_>,
) -> bool {
    // M2.6 has no cast-completion/cancellation state. If one template slot
    // could start a non-instant Aggro/Combat cast, merely dropping that slot
    // would leave Rust free to emit another slot while C++ still owns
    // UNIT_STATE_CASTING. Suppress this creature's whole spell surface until
    // M3.1 can represent that temporal state.
    spells.iter().copied().filter(|id| *id != 0).any(|id| {
        let Some(spell) = (policies.info)(id, difficulty, true) else {
            return false;
        };
        spell.cast_time_ms != 0
            && matches!(
                (policies.condition)(id, difficulty),
                SpellCondition::Aggro | SpellCondition::Combat
            )
    })
}

pub(super) fn implicit_cost(
    spell: &SpellInfoFacts,
    difficulty: u8,
    policies: &mut SpellPolicies<'_>,
    actor: &WorldCreature,
) -> bool {
    if spell.power_costs.is_empty() || has_nonzero_power_cost(spell) {
        return false;
    }
    let Some(attributes) = (policies.attributes)(spell.spell_id, difficulty) else {
        return true;
    };
    if attributes[1] & 0x0000_0002 != 0 || attributes[4] & 0x0000_0400 != 0 {
        return true;
    }
    let auras = &actor.creature.unit().subsystems().auras;
    auras.has_aura_type_like_cpp(63)
        || auras.has_aura_type_like_cpp(
            wow_constants::spell::aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL,
        )
        || auras.has_aura_type_like_cpp(
            wow_constants::spell::aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL_PCT,
        )
}

pub(super) fn cast_plan(
    actor: &WorldCreature,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    map_id: u16,
    instance_id: u32,
    spell_id: u32,
    spell: &SpellInfoFacts,
    difficulty: u8,
    policies: &mut SpellPolicies<'_>,
) -> Result<SpellCastPlan, ()> {
    Ok(SpellCastPlan {
        caster_guid,
        target_guid,
        map_id,
        instance_id,
        spell_id: spell.spell_id,
        spell_x_spell_visual_id: (policies.visual)(spell_id, difficulty)?,
        cast_time_ms: spell.cast_time_ms,
        spell_go_cast_flags: (policies.go_flags)(spell_id, difficulty),
        engagement_epoch: actor.creature_spell_engagement_epoch_like_cpp(),
        caster_incarnation: SpellCasterIncarnation::capture(&actor.creature),
    })
}

/// The represented untriggered Spell::IsAutoActionResetSpell rule.
pub fn resets_combat_timers(
    cast_time_ms: u32,
    mut has_attribute: impl FnMut(usize, u32) -> bool,
) -> bool {
    !has_attribute(2, 0x0002_0000) && (cast_time_ms != 0 || !has_attribute(6, 0x0200_0000))
}

pub(super) fn consume_schedule(
    actor: Option<&mut WorldCreature>,
    schedule: &SpellSchedule,
) -> bool {
    let Some(actor) = actor else {
        return true;
    };
    if actor.creature_spell_engagement_epoch_like_cpp() != schedule.engagement_epoch {
        return true;
    }
    let Some(delay) = actor.random_creature_spell_delay_like_cpp(
        schedule.minimum_ms,
        schedule.minimum_ms.saturating_mul(2),
    ) else {
        return false;
    };
    actor.schedule_creature_spell_slot_after_like_cpp(schedule.slot, delay);
    true
}
