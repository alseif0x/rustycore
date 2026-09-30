//! Cast prefix through the owned LOS boundary. Resume never repeats this prefix.
use super::*;
use wow_entities::{LineOfSightEndpoint, LineOfSightOptions, LineOfSightQuery};

pub(super) struct SpellCastPending {
    pub cast: SpellCast,
    pub cooldown_now_ms: u64,
    pub cooldown: Option<SpellCooldown>,
    pub caster_hit_inert: bool,
    pub caster_uncontrolled: bool,
    pub attributes: Option<[u32; 15]>,
    pub position: Position,
    pub visibility_range: f32,
    pub from: LineOfSightEndpoint,
    pub to: LineOfSightEndpoint,
}

pub(super) enum CastPrefix {
    Rejected(SpellValidation),
    Ready(SpellCastPending),
    Pending(SpellCastPending),
}

pub(super) fn require_cast(backend: &SpellMap<'_>, cast: &SpellCast) -> Result<(), SpellValidation> {
    let command = &cast.command;
    let actor = backend.actor(command.caster_guid).ok_or(SpellValidation::MissingTarget)?;
    if !actor.is_alive() || actor.state() != CreatureAiState::InCombat
        || actor.creature.ai_ownership().combat_target != Some(command.target_guid)
        || actor.creature_spell_engagement_epoch_like_cpp() != command.engagement_epoch
    { return Err(SpellValidation::MissingTarget); }
    if !command.caster_incarnation.matches(&actor.creature) {
        return Err(SpellValidation::CasterIncarnationRejected);
    }
    Ok(())
}

pub(super) fn prepare_cast(backend: &mut SpellMap<'_>, cast: SpellCast,
    policies: &mut SpellPolicies<'_>, terrain_enabled: bool) -> CastPrefix {
    if let Err(error) = require_cast(backend, &cast) { return CastPrefix::Rejected(error); }
    let command = &cast.command;
    let actor = backend.actor(command.caster_guid).unwrap();
    let cooldown_now_ms = actor.runtime_elapsed_ms_like_cpp();
    let cooldown = u32::try_from(command.spell_id).ok()
        .and_then(|id| (policies.cooldown)(id, cast.difficulty_id));
    let caster_hit_inert = actor.creature.unit().subsystems().auras
        .has_complete_spell_hit_inert_aura_authority_like_cpp();
    let control = &actor.creature.unit().subsystems().control;
    let caster_uncontrolled = control.owner_guid.is_none() && control.charmer_guid.is_none()
        // CharmInfo remains GCD-bearing even with temporarily empty GUIDs.
        // Keep that controlled surface outside this stock-AI slice.
        && !control.controlled_by_player && !control.has_charm_info();
    let Some(map) = backend.map() else { return CastPrefix::Rejected(SpellValidation::MissingTarget); };
    let Some(caster) = map.get_typed_creature(command.caster_guid) else {
        return CastPrefix::Rejected(SpellValidation::MissingTarget);
    };
    if !command.caster_incarnation.matches(caster) {
        return CastPrefix::Rejected(SpellValidation::CasterIncarnationRejected);
    }
    let Some(victim) = map.get_typed_player(command.target_guid) else {
        return CastPrefix::Rejected(SpellValidation::MissingTarget);
    };
    if !caster.unit().is_alive() || !victim.unit().is_alive() {
        return CastPrefix::Rejected(SpellValidation::MissingTarget);
    }
    let position = caster.unit().world().position();
    // Snapshot fanout from the same current body used for admission, at the
    // original pre-LOS point; never reuse AI-selection position or clone Actor.
    let visibility_range = caster.unit().world().get_visibility_range(map);
    let Ok(spell_id) = u32::try_from(command.spell_id) else {
        return CastPrefix::Rejected(SpellValidation::MissingTarget);
    };
    let Some(range) = (policies.range)(spell_id, cast.difficulty_id) else {
        return CastPrefix::Rejected(SpellValidation::OutOfRange);
    };
    let reach_sum = caster.unit().world().combat_reach().max(0.0)
        + victim.unit().world().combat_reach().max(0.0);
    let melee_range = (reach_sum + 4.0 / 3.0).max(5.0);
    let mut minimum = range.minimum.max(0.0);
    let mut maximum = range.maximum.max(0.0);
    let turret_maximum = maximum + reach_sum;
    if range.flags & 0x01 != 0 {
        minimum = 0.0;
        maximum = melee_range;
    } else {
        if range.flags & 0x02 != 0 { minimum += melee_range; }
        else if minimum > 0.0 { minimum += reach_sum; }
        maximum += reach_sum;
    }
    let caster_movement = caster.unit().movement_flags_like_cpp();
    let victim_movement = victim.unit().movement_flags_like_cpp();
    if caster_movement.intersects(MovementFlag::MASK_MOVING)
        && victim_movement.intersects(MovementFlag::MASK_MOVING)
        && !caster_movement.contains(MovementFlag::WALKING)
        && !victim_movement.contains(MovementFlag::WALKING)
    { maximum += 8.0 / 3.0; }
    let distance_sq = caster.unit().world().position().distance_sq(&victim.unit().world().position());
    if cast.turret_ai && distance_sq >= turret_maximum * turret_maximum {
        return CastPrefix::Rejected(SpellValidation::OutOfRange);
    }
    // UnitAI resets BASE_ATTACK after only its strict raw-max gate. This
    // precedes both canonical history and Spell::CheckRange/CheckCast.
    if cast.turret_ai { backend.actor_mut(command.caster_guid).unwrap().record_swing(); }
    let map = backend.map().unwrap();
    let caster = map.get_typed_creature(command.caster_guid).unwrap();
    if cooldown.is_some_and(|profile| !profile.passive && caster.unit().subsystems().spells.history
        .has_cooldown(profile.spell_id, profile.category_id, cooldown_now_ms))
    { return CastPrefix::Rejected(SpellValidation::CooldownRejected); }
    if distance_sq > maximum * maximum || (minimum > 0.0 && distance_sq < minimum * minimum) {
        return CastPrefix::Rejected(SpellValidation::OutOfRange);
    }
    let attributes = (policies.attributes)(command.spell_id, cast.difficulty_id);
    let ignores_los = attributes.is_some_and(|attributes| attributes[2] & 0x0000_0004 != 0);
    let mut from = LineOfSightEndpoint::raw_position(position);
    let mut to = LineOfSightEndpoint::raw_position(position);
    if !ignores_los {
        let victim = map.get_typed_player(command.target_guid).unwrap();
        let source = caster.unit().world();
        let target = victim.unit().world();
        if !source.is_in_map(target) || !source.has_current_map()
            || source.map_id() != map.map_id() || source.instance_id() != map.instance_id() {
            return CastPrefix::Rejected(SpellValidation::LosRejected);
        }
        let query = LineOfSightQuery::to_object_like_cpp(source, target, LineOfSightOptions::default());
        from = query.from;
        to = query.to;
    }
    let pending = SpellCastPending { cast, cooldown_now_ms, cooldown, caster_hit_inert,
        caster_uncontrolled, attributes, position, visibility_range, from, to };
    if !ignores_los && terrain_enabled { CastPrefix::Pending(pending) }
    else { CastPrefix::Ready(pending) }
}

pub(super) fn rejected_attempt(backend: &mut SpellMap<'_>, attempt: SpellRejectedAttempt,
    policies: &mut SpellPolicies<'_>) -> bool {
    let Some(actor) = backend.actor(attempt.caster_guid) else { return false; };
    if !actor.is_alive() || actor.state() != CreatureAiState::InCombat
        || actor.creature.ai_ownership().combat_target != Some(attempt.target_guid)
        || actor.creature_spell_engagement_epoch_like_cpp() != attempt.engagement_epoch
    { return false; }
    let Some(range) = (policies.range)(attempt.spell_id, attempt.difficulty_id) else { return false; };
    let Some(map) = backend.map() else { return false; };
    let Some(caster) = map.creature_transform_vitals_snapshot_like_cpp(attempt.caster_guid) else { return false; };
    let Some(victim) = map.get_typed_player(attempt.target_guid) else { return false; };
    if !caster.is_alive || !victim.unit().is_alive() { return false; }
    let reach_sum = caster.combat_reach.max(0.0) + victim.unit().world().combat_reach().max(0.0);
    let maximum = range.maximum.max(0.0) + reach_sum;
    if !(caster.position.distance_sq(&victim.unit().world().position()) < maximum * maximum) { return false; }
    backend.actor_mut(attempt.caster_guid).unwrap().record_swing();
    true
}
