//! Post-LOS admission, one hit roll, canonical cooldown and GUID commit.
use super::*;
use super::validation::SpellCastPending;

pub(super) struct SpellPublicationState { pub command: SpellCastPlan, pub hit: SpellHit }
pub(super) type CastResult = (SpellValidation, Option<(SpellCompletion, SpellPublicationState)>);

pub(super) fn finish_cast(backend: &mut SpellMap<'_>, pending: SpellCastPending,
    visible: bool, policies: &mut SpellPolicies<'_>) -> CastResult {
    if !visible { return (SpellValidation::LosRejected, None); }
    let command = &pending.cast.command;
    let map = backend.map().unwrap();
    let Some(caster) = map.get_typed_creature(command.caster_guid) else { return (SpellValidation::MissingTarget, None); };
    let Some(victim) = map.get_typed_player(command.target_guid) else { return (SpellValidation::MissingTarget, None); };
    if let Some(attributes) = pending.attributes
        && !target_is_valid(caster, victim, &attributes, policies)
    { return (SpellValidation::TargetRejected, None); }
    let represented = (|| {
        let spell_id = u32::try_from(command.spell_id).ok()?;
        let spell = (policies.info)(spell_id, pending.cast.difficulty_id, false)?;
        let metadata = (policies.hit_metadata)(command.spell_id, pending.cast.difficulty_id)?;
        let attributes = pending.attributes?;
        let active_indices: Vec<_> = spell.effects.iter()
            .filter(|effect| effect.effect != 0 && !rules::is_noop(effect.effect))
            .map(|effect| effect.effect_index).collect();
        let victim_inert = victim.unit().subsystems().auras.has_complete_spell_hit_inert_aura_authority_like_cpp();
        let target_has_no_vehicle_kit = victim.unit().subsystems().vehicle.kit.is_none();
        let behind = !victim.unit().world().has_in_arc(std::f32::consts::PI, caster.unit().world(), 2.0);
        if !pending.caster_hit_inert || !pending.caster_uncontrolled || !victim_inert
            || !target_has_no_vehicle_kit || !behind { return None; }
        let profile = represented_hit_profile(&metadata, &active_indices, attributes)?;
        Some((profile, cast_log(caster, &spell, pending.cast.difficulty_id)))
    })();
    let (profile, log) = represented.map_or((None, None), |(profile, log)| (Some(profile), log));
    if !pending.cast.turret_ai && resets_combat_timers(command.cast_time_ms,
        |word, attribute| (policies.has_attribute)(command.spell_id, pending.cast.difficulty_id, word, attribute))
    { backend.actor_mut(command.caster_guid).unwrap().record_swing(); }
    let Some(profile) = profile else {
        backend.actor_mut(command.caster_guid).unwrap().invalidate_runtime_rng_authority_like_cpp();
        return (SpellValidation::HitResultUnrepresented, None);
    };
    let Some(log) = log else {
        backend.actor_mut(command.caster_guid).unwrap().invalidate_runtime_rng_authority_like_cpp();
        return (SpellValidation::HitResultUnrepresented, None);
    };
    let Some(roll) = backend.actor_mut(command.caster_guid).unwrap().random_creature_spell_hit_roll_like_cpp() else {
        return (SpellValidation::RuntimeRngAuthorityRejected, None);
    };
    let Some(hit) = resolve_hit_profile(profile, Some(roll)) else {
        backend.actor_mut(command.caster_guid).unwrap().invalidate_runtime_rng_authority_like_cpp();
        return (SpellValidation::HitResultUnrepresented, None);
    };
    if let Some(profile) = pending.cooldown
        && !profile.passive && (profile.recovery_time_ms != 0 || profile.category_recovery_time_ms != 0)
    {
        backend.map_mut().unwrap().get_typed_creature_mut(command.caster_guid).unwrap()
            .unit_mut().subsystems_mut().spells.history.start_cooldown(pending.cooldown_now_ms,
                profile.spell_id, 0, profile.recovery_time_ms, profile.category_id,
                profile.category_recovery_time_ms, false);
    }
    let counter = backend.map_mut().unwrap().generate_low_guid_like_cpp(HighGuid::Cast)
        .expect("Cast is a supported map GUID sequence");
    let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 3, command.caster_guid.realm_id(),
        command.map_id, 0, u32::try_from(command.spell_id).unwrap_or_default(), counter);
    let completion = SpellCompletion { caster_guid: command.caster_guid, target_guid: command.target_guid,
        map_id: command.map_id, instance_id: command.instance_id, spell_id: command.spell_id,
        spell_x_spell_visual_id: command.spell_x_spell_visual_id, cast_time_ms: command.cast_time_ms,
        spell_go_cast_flags: command.spell_go_cast_flags, cast_id, hit, position: pending.position,
        visibility_range: pending.visibility_range, log };
    // Canonical APP appends outside its guard BEFORE the HIT tombstone/next
    // Schedule. Legacy APP retains the complete PRE guards through append.
    // The continuation owns the original command, not an actor or RNG copy.
    (SpellValidation::Ready(hit), Some((completion, SpellPublicationState {
        command: pending.cast.command, hit,
    })))
}
