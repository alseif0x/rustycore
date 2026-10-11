//! #1263 F6-8D1 regressions: the ported phase engine owns the canonical entity.
//!
//! D1 ports the complete per-creature phase operations — the aggro/assistance
//! and taunt entries, the damage and death transitions, the `CombatAI::_events`
//! spell deadline slots and engagement epoch, the melee swing timer and the
//! creature-owned RNG draws — onto canonical
//! `Creature`/`CreatureRuntimeLikeCpp` ownership. The legacy `WorldCreature`
//! keeps delegating entry points only.
//!
//! Two contracts are asserted here:
//!
//! 1. **The engine decides and mutates through canonical ownership.** A
//!    detached legacy-only copy of the entity decides nothing for the canonical
//!    owner, and the canonical entity — not the bridge and not its packet
//!    projection — is the state every operation reads and writes.
//! 2. **State and RNG continuity across the ported boundary.** The same inputs
//!    produce the same draw sequence, the same timer consumption and the same
//!    resulting state as the legacy path did. The draws are additionally pinned
//!    against a verbatim copy of the pre-D1 decision bodies and against fixed
//!    golden values, so a future divergence cannot hide behind the delegation.

use super::*;
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

/// The seed every path in this module shares.
const D1_SEED_LIKE_CPP: u64 = 0x1263_D1;

/// The taunt aura type the application layer supplies.
///
/// `SPELL_AURA_MOD_TAUNT` lives in the static-data crate; the canonical entity
/// crate must not depend on it, so the canonical op takes it as a parameter and
/// the legacy bridge entry point supplies the constant. C++ reads the same
/// value in `Aura`/`SpellEffects`.
const D1_TAUNT_AURA_TYPE_LIKE_CPP: i32 = wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT;

fn d1_creature_like_cpp(guid: i64, min_dmg: u32, max_dmg: u32) -> WorldCreature {
    let mut creature = WorldCreature::new(
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, guid),
        12_630,
        Position::new(10.0, 10.0, 0.0, 0.0),
        1_000,
        30,
        min_dmg,
        max_dmg,
        20.0,
        100,
        14,
        0,
        0,
    );
    creature.seed_runtime_rng_like_cpp(D1_SEED_LIKE_CPP);
    creature
}

// ── Pre-D1 reference bodies ───────────────────────────────────────────────────
//
// Copied verbatim from `4e40b21dc` (`crates/wow-world-core/src/map_manager/
// combat.rs`) with the bridge receiver replaced by the canonical values it read
// through its accessors. They exist so the continuity test compares the ported
// engine against the algorithm that ran before D1, not only against the
// delegating bridge.

fn pre_d1_roll_damage_like_cpp(rng: &mut StdRng, min_dmg: u32, max_dmg: u32) -> Option<u32> {
    if min_dmg > max_dmg {
        return None;
    }
    if min_dmg == max_dmg {
        let _ = rng.next_u32();
        return Some(min_dmg);
    }
    Some(rng.gen_range(min_dmg..=max_dmg))
}

fn pre_d1_random_creature_spell_delay_like_cpp(
    rng: &mut StdRng,
    minimum_ms: u64,
    maximum_ms: u64,
) -> Option<u64> {
    if minimum_ms > maximum_ms {
        return None;
    }
    if minimum_ms == maximum_ms {
        let _ = rng.next_u32();
        return Some(minimum_ms);
    }
    Some(rng.gen_range(minimum_ms..=maximum_ms))
}

fn pre_d1_random_creature_spell_hit_roll_like_cpp(rng: &mut StdRng) -> u32 {
    rng.gen_range(0..=9_999)
}

/// The pre-D1 `record_swing` rule: the cached packet projection's attack time,
/// falling back to the live swing timer when the projection carried 0.
fn pre_d1_record_swing_timer_like_cpp(
    create_data_base_attack_time: u32,
    swing_timer_ms: u64,
) -> u64 {
    if create_data_base_attack_time > 0 {
        u64::from(create_data_base_attack_time)
    } else {
        swing_timer_ms.max(1)
    }
}

// ── (1) canonical ownership ───────────────────────────────────────────────────

#[test]
fn canonical_creature_phase_engine_decides_and_mutates_through_canonical_ownership_like_cpp() {
    let victim = ObjectGuid::create_player(1, 12_631);
    let mut legacy = d1_creature_like_cpp(12_632, 7, 9);
    let projection_before = legacy.create_data.clone();

    // A detached legacy-only copy: the same bytes the bridge holds, but not the
    // object the canonical owner holds.
    let mut detached = legacy.creature.clone();
    detached.enter_combat(victim);
    detached.advance_runtime_clock_like_cpp(2_000);
    detached.record_swing();
    assert_eq!(
        detached.ai_ownership().combat_target,
        Some(victim),
        "the detached copy decides for itself"
    );
    assert_eq!(
        detached.runtime_like_cpp().runtime_elapsed_ms_like_cpp(),
        2_000
    );
    assert!(
        !detached.can_swing(),
        "the detached copy consumed its own swing timer"
    );

    assert_eq!(
        legacy.creature.ai_ownership().combat_target,
        None,
        "a legacy-only copy decides nothing for the canonical owner"
    );
    assert_eq!(
        legacy
            .creature
            .runtime_like_cpp()
            .runtime_elapsed_ms_like_cpp(),
        0,
        "a legacy-only copy advances no canonical clock"
    );
    assert_eq!(
        legacy.creature.ai_ownership().last_swing_ms,
        0,
        "a legacy-only copy records no canonical swing"
    );
    assert_eq!(
        legacy.creature.creature_spell_engagement_epoch_like_cpp(),
        0,
        "a legacy-only copy opens no canonical engagement epoch"
    );

    // The canonical entity is the state the operations read and write.
    legacy.creature.enter_combat(victim);
    legacy.creature.advance_runtime_clock_like_cpp(2_000);
    legacy.creature.record_swing();

    assert_eq!(
        legacy.creature.ai_ownership().combat_target,
        Some(victim),
        "the canonical owner's object is the one that enters combat"
    );
    assert_eq!(legacy.state(), wow_entities::CreatureAiState::InCombat);
    assert_eq!(
        legacy
            .creature
            .runtime_like_cpp()
            .runtime_elapsed_ms_like_cpp(),
        2_000,
        "the creature-local clock lives on the canonical runtime state"
    );
    assert_eq!(
        legacy.creature.ai_ownership().last_swing_ms,
        2_000,
        "the swing record lands on the canonical AI ownership"
    );
    assert_eq!(
        legacy.creature.ai_ownership().swing_timer_ms,
        u64::from(wow_entities::BASE_ATTACK_TIME_LIKE_CPP),
        "the canonical swing timer is rearmed from the unit's base attack time"
    );
    assert!(
        !legacy.can_swing(),
        "the canonical swing timer consumption is what the bridge observes"
    );
    assert_eq!(
        legacy.creature.creature_spell_engagement_epoch_like_cpp(),
        1,
        "entering a new engagement opens exactly one canonical epoch"
    );
    assert_eq!(
        legacy.create_data.movement_flags, projection_before.movement_flags,
        "the phase operations keep no movement state on the packet projection"
    );
    assert_eq!(
        legacy.create_data.health, projection_before.health,
        "the phase operations keep no health state on the packet projection"
    );
    assert_eq!(
        legacy.create_data.base_attack_time, projection_before.base_attack_time,
        "the phase operations keep no swing-timer state on the packet projection"
    );
}

// ── (2) state and RNG continuity ──────────────────────────────────────────────

/// One full phase-operation sequence, driven through the legacy bridge entry
/// points.
fn d1_drive_legacy_path_like_cpp(
    creature: &mut WorldCreature,
    victim: ObjectGuid,
    caster: ObjectGuid,
) -> Vec<Option<u64>> {
    let mut draws = Vec::new();
    creature.advance_runtime_clock_like_cpp(1_500);
    creature.enter_combat(victim);
    creature.record_failed_swing_retry_like_cpp();
    creature.advance_runtime_clock_like_cpp(200);
    for _ in 0..5 {
        draws.push(creature.roll_damage().map(u64::from));
    }
    for _ in 0..3 {
        draws.push(creature.random_creature_spell_delay_like_cpp(1_000, 5_000));
    }
    draws.push(creature.random_creature_spell_delay_like_cpp(2_500, 2_500));
    for _ in 0..3 {
        draws.push(
            creature
                .random_creature_spell_hit_roll_like_cpp()
                .map(u64::from),
        );
    }
    creature.schedule_creature_spell_slot_after_like_cpp(0, 500);
    creature.schedule_creature_spell_slot_after_like_cpp(3, 2_500);
    creature.advance_runtime_clock_like_cpp(1_000);
    let _ = creature.first_due_creature_spell_slot_like_cpp();
    creature.record_swing();
    let _ = creature.apply_taunt_aura_like_cpp(caster, 100, 1, 2_000);
    let _ = creature.schedule_assistance_like_cpp(victim, vec![caster], 750);
    creature.advance_runtime_clock_like_cpp(1_000);
    let _ = creature.take_due_assistance_like_cpp();
    let _ = creature.expire_taunt_auras_if_due_like_cpp();
    let _ = creature.take_damage_before_death_state_like_cpp(250);
    let _ = creature.reset_combat();
    // Terminal draw whose invalid range tombstones exact RNG authority; it runs
    // last so every earlier draw stays comparable.
    draws.push(creature.roll_damage().map(u64::from));
    draws
}

/// The same sequence driven against a bare canonical entity.
fn d1_drive_canonical_path_like_cpp(
    creature: &mut Creature,
    victim: ObjectGuid,
    caster: ObjectGuid,
) -> Vec<Option<u64>> {
    let mut draws = Vec::new();
    creature.advance_runtime_clock_like_cpp(1_500);
    creature.enter_combat(victim);
    creature.record_failed_swing_retry_like_cpp();
    creature.advance_runtime_clock_like_cpp(200);
    for _ in 0..5 {
        draws.push(creature.roll_damage().map(u64::from));
    }
    for _ in 0..3 {
        draws.push(creature.random_creature_spell_delay_like_cpp(1_000, 5_000));
    }
    draws.push(creature.random_creature_spell_delay_like_cpp(2_500, 2_500));
    for _ in 0..3 {
        draws.push(
            creature
                .random_creature_spell_hit_roll_like_cpp()
                .map(u64::from),
        );
    }
    creature.schedule_creature_spell_slot_after_like_cpp(0, 500);
    creature.schedule_creature_spell_slot_after_like_cpp(3, 2_500);
    creature.advance_runtime_clock_like_cpp(1_000);
    let _ = creature.first_due_creature_spell_slot_like_cpp();
    creature.record_swing();
    let _ = creature.apply_taunt_aura_like_cpp(caster, 100, 1, 2_000, D1_TAUNT_AURA_TYPE_LIKE_CPP);
    let _ = creature.schedule_assistance_like_cpp(victim, vec![caster], 750);
    creature.advance_runtime_clock_like_cpp(1_000);
    let _ = creature.take_due_assistance_like_cpp();
    let _ = creature.expire_taunt_auras_if_due_like_cpp();
    let _ = creature.take_damage_before_death_state_like_cpp(250);
    let _ = creature.reset_combat();
    draws.push(creature.roll_damage().map(u64::from));
    draws
}

/// Every observable the sequence produces, read off canonical ownership only.
#[derive(Debug, PartialEq)]
struct D1ObservablesLikeCpp {
    elapsed_ms: u64,
    last_swing_ms: u64,
    swing_timer_ms: u64,
    engagement_epoch: u64,
    spell_slots: [Option<u64>; wow_entities::MAX_CREATURE_SPELLS],
    combat_target: Option<ObjectGuid>,
    assistance_called: bool,
    pending_assistance: usize,
    active_taunts: usize,
    health: u64,
    death_state: wow_constants::DeathState,
    rng_authority_complete: bool,
    motion_master_ticks: u64,
}

fn d1_observables_like_cpp(creature: &Creature) -> D1ObservablesLikeCpp {
    let runtime = creature.runtime_like_cpp();
    D1ObservablesLikeCpp {
        elapsed_ms: runtime.runtime_elapsed_ms_like_cpp(),
        last_swing_ms: creature.ai_ownership().last_swing_ms,
        swing_timer_ms: creature.ai_ownership().swing_timer_ms,
        engagement_epoch: runtime.creature_spell_engagement_epoch_like_cpp,
        spell_slots: runtime.creature_spell_due_at_ms_like_cpp,
        combat_target: creature.ai_ownership().combat_target,
        assistance_called: runtime.assistance_called_like_cpp,
        pending_assistance: runtime.pending_assistance_like_cpp.len(),
        active_taunts: runtime.active_taunts_like_cpp.len(),
        health: creature.current_health(),
        death_state: creature.unit().death_state(),
        rng_authority_complete: runtime.runtime_rng_authority_complete_like_cpp(),
        motion_master_ticks: runtime.runtime_motion_master_ticks_like_cpp(),
    }
}

#[test]
fn canonical_phase_engine_preserves_state_and_rng_continuity_like_cpp() {
    let victim = ObjectGuid::create_player(1, 12_640);
    let caster = ObjectGuid::create_player(1, 12_641);
    let mut legacy = d1_creature_like_cpp(12_642, 7, 11);
    let mut canonical = legacy.creature.clone();
    canonical
        .runtime_like_cpp_mut()
        .seed_runtime_rng_like_cpp(D1_SEED_LIKE_CPP);

    // The two paths start from the same canonical state and the same stream.
    assert_eq!(
        d1_observables_like_cpp(&canonical),
        d1_observables_like_cpp(&legacy.creature)
    );

    let legacy_draws = d1_drive_legacy_path_like_cpp(&mut legacy, victim, caster);
    let canonical_draws = d1_drive_canonical_path_like_cpp(&mut canonical, victim, caster);

    assert_eq!(
        canonical_draws, legacy_draws,
        "the ported engine must reproduce the legacy draw sequence, in order"
    );
    assert_eq!(
        d1_observables_like_cpp(&canonical),
        d1_observables_like_cpp(&legacy.creature),
        "the ported engine must consume the same timers and land the same state"
    );
}

#[test]
fn canonical_phase_engine_reproduces_the_pre_d1_draw_sequence_like_cpp() {
    // The same draws, in the same order, from the same seed, through the
    // verbatim pre-D1 bodies.
    let mut reference = StdRng::seed_from_u64(D1_SEED_LIKE_CPP);
    let mut reference_draws: Vec<Option<u64>> = Vec::new();
    for _ in 0..5 {
        reference_draws.push(pre_d1_roll_damage_like_cpp(&mut reference, 7, 11).map(u64::from));
    }
    for _ in 0..3 {
        reference_draws.push(pre_d1_random_creature_spell_delay_like_cpp(
            &mut reference,
            1_000,
            5_000,
        ));
    }
    reference_draws.push(pre_d1_random_creature_spell_delay_like_cpp(
        &mut reference,
        2_500,
        2_500,
    ));
    for _ in 0..3 {
        reference_draws.push(Some(u64::from(
            pre_d1_random_creature_spell_hit_roll_like_cpp(&mut reference),
        )));
    }

    let mut canonical = d1_creature_like_cpp(12_650, 7, 11).creature;
    canonical
        .runtime_like_cpp_mut()
        .seed_runtime_rng_like_cpp(D1_SEED_LIKE_CPP);
    let mut engine_draws: Vec<Option<u64>> = Vec::new();
    for _ in 0..5 {
        engine_draws.push(canonical.roll_damage().map(u64::from));
    }
    for _ in 0..3 {
        engine_draws.push(canonical.random_creature_spell_delay_like_cpp(1_000, 5_000));
    }
    engine_draws.push(canonical.random_creature_spell_delay_like_cpp(2_500, 2_500));
    for _ in 0..3 {
        engine_draws.push(
            canonical
                .random_creature_spell_hit_roll_like_cpp()
                .map(u64::from),
        );
    }

    assert_eq!(
        engine_draws, reference_draws,
        "the canonical engine must reproduce the pre-D1 draw sequence exactly"
    );
    assert_eq!(
        engine_draws,
        vec![
            Some(10),
            Some(11),
            Some(7),
            Some(7),
            Some(7),
            Some(2_841),
            Some(4_392),
            Some(1_140),
            Some(2_500),
            Some(8_492),
            Some(9_366),
            Some(9_095),
        ],
        "the pre-D1 draw sequence is pinned by value"
    );
}

#[test]
fn canonical_swing_timer_resolution_matches_the_pre_d1_projection_rule_like_cpp() {
    // The pre-D1 `record_swing` read the cached packet projection, whose value
    // `create_data_from_canonical_like_cpp` derives from this exact unit field
    // with the C++ 0 -> BASE_ATTACK_TIME clamp.
    let mut loaded = d1_creature_like_cpp(12_660, 5, 9);
    loaded
        .creature
        .unit_mut()
        .set_base_attack_time_like_cpp(WeaponAttackType::BaseAttack, 1_400);
    let projection = WorldCreature::create_data_from_canonical_like_cpp(&loaded.creature);
    assert_eq!(
        loaded.creature.base_attack_time_like_cpp(),
        pre_d1_record_swing_timer_like_cpp(
            projection.base_attack_time,
            loaded.creature.ai_ownership().swing_timer_ms
        ),
        "a loaded base attack time resolves identically on both paths"
    );

    // The bare canonical default is 0; the C++ clamp resolves it to
    // BASE_ATTACK_TIME on both paths, because the projection clamps it too.
    let mut bare = d1_creature_like_cpp(12_661, 5, 9);
    let projection = WorldCreature::create_data_from_canonical_like_cpp(&bare.creature);
    assert_eq!(
        projection.base_attack_time,
        wow_entities::BASE_ATTACK_TIME_LIKE_CPP
    );
    assert_eq!(
        bare.creature.base_attack_time_like_cpp(),
        pre_d1_record_swing_timer_like_cpp(
            projection.base_attack_time,
            bare.creature.ai_ownership().swing_timer_ms
        ),
        "the zero-base default resolves through the same clamp"
    );
    // Consume the rule once so the projection is demonstrably the read source.
    bare.record_swing();
    assert_eq!(
        bare.creature.ai_ownership().swing_timer_ms,
        u64::from(wow_entities::BASE_ATTACK_TIME_LIKE_CPP)
    );
}
