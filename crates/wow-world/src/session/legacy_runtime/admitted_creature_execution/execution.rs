// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The admitted execution itself: one clock, one due event, one swing.

use super::*;

/// Run the isolated admitted creature execution once.
///
/// Order, mirroring the admitted tick's own phases:
/// 1. the session barrier and the whole admission fence, before any mutation;
/// 2. the single-owner claim of every admitted transition;
/// 3. one clock advancement by the saved diff, one due cast consumption, one
///    swing commit and its canonical effect per admitted creature;
/// 4. the deferred publication batch, returned to the caller.
#[must_use]
pub fn run_admitted_creature_execution_isolated_like_cpp(
    admission: &AdmittedCreatureExecutionLikeCpp,
    canonical_map_manager: &SharedCanonicalMapManager,
    lease: &SharedCreatureExecutionLeaseLikeCpp,
) -> IsolatedCreatureExecutionOutcomeLikeCpp {
    let owner = CreatureExecutionOwnerLikeCpp::CanonicalAdmitted;
    let Ok(mut manager) = canonical_map_manager.lock() else {
        return IsolatedCreatureExecutionOutcomeLikeCpp::refused_like_cpp(
            CreatureExecutionAdmissionLikeCpp::RefusedSessionBarrier {
                expected_epoch: admission.tick_epoch,
                current: wow_map::MapTickCoordinationStateLikeCpp::Idle,
            },
            owner,
        );
    };
    let fenced = admission.fence_like_cpp(&manager);
    if !fenced.is_admitted_like_cpp() {
        return IsolatedCreatureExecutionOutcomeLikeCpp::refused_like_cpp(fenced, owner);
    }
    // Claim every transition before executing any of them: a partially
    // admitted transition must not advance the diff of its siblings.
    let Ok(mut lease) = lease.lock() else {
        return IsolatedCreatureExecutionOutcomeLikeCpp::refused_like_cpp(
            CreatureExecutionAdmissionLikeCpp::RefusedOwnerLease { held_by: owner },
            owner,
        );
    };
    for object in &admission.objects {
        let transition = admission.transition_like_cpp(object);
        if let Some(held_by) = lease.owner_like_cpp(transition) {
            return IsolatedCreatureExecutionOutcomeLikeCpp::refused_like_cpp(
                CreatureExecutionAdmissionLikeCpp::RefusedOwnerLease { held_by },
                owner,
            );
        }
    }
    for object in &admission.objects {
        let transition = admission.transition_like_cpp(object);
        let _ = lease.claim_like_cpp(transition, owner);
    }
    drop(lease);

    let mut outcome = IsolatedCreatureExecutionOutcomeLikeCpp::refused_like_cpp(
        CreatureExecutionAdmissionLikeCpp::Admitted,
        owner,
    );
    for object in &admission.objects {
        let Some(map) = manager.find_map_mut(object.map_id, object.instance_id) else {
            continue;
        };
        let execution =
            execute_admitted_creature_transition_like_cpp(map.map_mut(), admission, object);
        outcome.clock_advanced_ms = outcome
            .clock_advanced_ms
            .saturating_add(u64::from(admission.diff_ms));
        outcome.due_casts += execution.due_casts;
        outcome.swing_damage = outcome.swing_damage.saturating_add(execution.swing_damage);
        outcome.effects_consumed += execution.effects_consumed;
        outcome.deferred_publication.extend(execution.publication);
    }
    outcome
}

#[derive(Debug, Default)]
struct AdmittedCreatureExecutionArmLikeCpp {
    due_casts: usize,
    swing_damage: u64,
    effects_consumed: usize,
    publication: Vec<crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand>,
}

/// The single-creature execution of one admitted transition.
///
/// C++ `Creature::Update` advances the creature's own clock exactly once per
/// visited object (`Unit::Update(p_time)`), consumes at most one due
/// `CombatAI::_events` entry per update (`EventMap::ExecuteEvent`) and commits
/// at most one `DoMeleeAttackIfReady` swing. Every draw comes from the
/// creature-owned stream F6-8D1 ported, so a second owner that ran first would
/// shift the sequence and be observable.
fn execute_admitted_creature_transition_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    admission: &AdmittedCreatureExecutionLikeCpp,
    object: &AdmittedCreatureExecutionObjectLikeCpp,
) -> AdmittedCreatureExecutionArmLikeCpp {
    use wow_packet::packets::combat::{HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_HIT};

    let mut arm = AdmittedCreatureExecutionArmLikeCpp::default();
    let guid = object.creature_guid;
    // Scope 1: the creature-owned operations. The clock, the due event, the
    // damage roll and the swing rearm all read and write one canonical entity,
    // in the F6-8D1 order.
    let committed = map.with_creature_mut_like_cpp(guid, |creature| {
        // One diff per admitted transition.
        creature.advance_runtime_clock_like_cpp(admission.diff_ms);
        // At most one due event per update, exactly as
        // `EventMap::ExecuteEvent`.
        let mut due_casts = 0;
        if let Some(slot) = creature.first_due_creature_spell_slot_like_cpp() {
            creature.clear_creature_spell_slot_like_cpp(slot);
            let _ = creature.random_creature_spell_hit_roll_like_cpp();
            due_casts += 1;
        }
        if !creature.can_swing() {
            return (due_casts, None);
        }
        let Some(damage) = creature.roll_damage() else {
            // `DoMeleeAttackIfReady` rearms BASE_ATTACK even when the roll
            // cannot produce a represented result.
            creature.record_swing();
            return (due_casts, None);
        };
        let Some(victim_guid) = creature.unit().subsystems().combat.current_victim_guid else {
            creature.record_swing();
            return (due_casts, None);
        };
        creature.record_swing();
        (due_casts, Some((victim_guid, damage.max(1))))
    });
    let Some((due_casts, committed)) = committed else {
        return arm;
    };
    arm.due_casts += due_casts;
    let Some((victim_guid, damage)) = committed else {
        return arm;
    };

    // Scope 2: the one canonical effect of the swing.
    let Some(victim_state) = apply_admitted_swing_damage_like_cpp(map, victim_guid, damage) else {
        return arm;
    };
    arm.swing_damage = u64::from(damage);
    arm.effects_consumed += 1;
    arm.publication.push(
        crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
            attacker_guid: guid,
            victim_guid,
            map_id: object.map_id.clamp(0, u32::from(u16::MAX)) as u16,
            instance_id: object.instance_id,
            damage,
            over_damage: victim_state.over_damage,
            target_level: victim_state.target_level,
            victim_health_after: victim_state.health_after,
            victim_health_state_revision_after: victim_state.revision_after,
            hit_info: HIT_INFO_AFFECTS_VICTIM,
            victim_state: VICTIM_STATE_HIT,
            original_damage: damage,
            absorbed: 0,
            mana_spent: 0,
            absorb_consumptions: Vec::new(),
            split_combat_log_packets: Vec::new(),
            self_share_health_updates: Vec::new(),
        },
    );
    arm
}

/// The canonical observable one admitted swing commits on its victim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AdmittedSwingVictimStateLikeCpp {
    health_after: u64,
    revision_after: u64,
    target_level: u8,
    over_damage: i32,
}

/// Apply the one admitted swing to the canonical victim.
///
/// Player victims write their canonical health and advance the shared
/// health-state revision; creature victims go through the ported
/// `take_damage_before_death_state_like_cpp` transition so the death state and
/// the revision timeline move exactly once.
fn apply_admitted_swing_damage_like_cpp(
    map: &mut wow_map::ManagedMapInnerLikeCpp,
    victim_guid: ObjectGuid,
    damage: u32,
) -> Option<AdmittedSwingVictimStateLikeCpp> {
    if victim_guid.is_player() {
        let victim = map.get_typed_player_mut(victim_guid)?;
        let health_before = victim.unit().data().health;
        victim
            .unit_mut()
            .set_health(health_before.saturating_sub(u64::from(damage)));
        let unit = victim.unit();
        return Some(AdmittedSwingVictimStateLikeCpp {
            health_after: unit.data().health,
            revision_after: unit.health_state_revision_like_cpp(),
            target_level: unit.data().level.clamp(0, i32::from(u8::MAX)) as u8,
            over_damage: if damage as u64 >= health_before {
                damage as i32 - health_before as i32
            } else {
                -1
            },
        });
    }

    let victim = map.get_typed_creature_mut(victim_guid)?;
    let health_before = victim.unit().data().health;
    let died = victim.take_damage_before_death_state_like_cpp(damage);
    let unit = victim.unit();
    Some(AdmittedSwingVictimStateLikeCpp {
        health_after: unit.data().health,
        revision_after: unit.health_state_revision_like_cpp(),
        target_level: unit.data().level.clamp(0, i32::from(u8::MAX)) as u8,
        over_damage: if died {
            damage as i32 - health_before as i32
        } else {
            -1
        },
    })
}
