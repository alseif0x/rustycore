// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1263 F6-8D3a-1b: the deferred half of the creature spell phase.
//!
//! The selection half (`creature_spell_tick.rs`) records, in C++ order, what
//! each creature's `CombatAI::UpdateAI` / `TurretAI::UpdateAI` would do next:
//! a `DoCast` to validate and publish, an `EventMap::ScheduleEvent` re-arm, or a
//! TurretAI attempt whose rejected `CastSpell` still resets BASE_ATTACK. This
//! module drains those actions through **one** body,
//! [`drain_creature_spell_actions_like_cpp`], against the owner that holds the
//! caster:
//!
//! - the legacy bridge ([`LegacyCreatureSpellActionOwnerLikeCpp`]) keeps the
//!   lock scopes it always had: a re-arm takes only the legacy write guard, a
//!   cast or turret attempt takes the canonical guard and then the legacy one;
//! - the admitted canonical executor ([`CanonicalCreatureSpellActionOwnerLikeCpp`])
//!   runs every action on the canonical manager it already holds locked, where
//!   the caster is the canonical incarnation itself.

use super::*;
use wow_world_entities::creature_spell_admission::{
    CanonicalCreatureSpellCastersLikeCpp, apply_turret_rejected_cast_attempt_on_manager_like_cpp,
};

pub(super) struct PendingCreatureSpellCastLikeCpp {
    pub(super) command: CreatureSpellCastPlanLikeCpp,
    pub(super) difficulty_id: u8,
    pub(super) turret_ai: bool,
}

pub(super) struct PendingCreatureSpellScheduleLikeCpp {
    pub(super) caster_guid: ObjectGuid,
    pub(super) map_id: u16,
    pub(super) instance_id: u32,
    pub(super) engagement_epoch: u64,
    pub(super) slot: usize,
    pub(super) minimum_ms: u64,
}

pub(super) enum PendingCreatureSpellActionLikeCpp {
    Cast(PendingCreatureSpellCastLikeCpp),
    Schedule(PendingCreatureSpellScheduleLikeCpp),
    /// A TurretAI attempt C++ would still have made through `CastSpell`,
    /// whose swing reset depends only on the raw combat-range gate.
    TurretRejectedAttempt(TurretRejectedCastAttemptLikeCpp),
}

/// Who holds the caster while the deferred spell actions run.
pub(super) trait CreatureSpellActionOwnerLikeCpp {
    /// The caster of a deferred `ScheduleEvent` re-arm; `None` when it is gone.
    fn with_schedule_caster_like_cpp<R>(
        &mut self,
        schedule: &PendingCreatureSpellScheduleLikeCpp,
        operation: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R>;
    /// A TurretAI rejected attempt; `None` when no canonical manager exists.
    fn turret_rejected_attempt_like_cpp(
        &mut self,
        attempt: &TurretRejectedCastAttemptLikeCpp,
        config: &LegacyCreatureAggroConfigLikeCpp,
    ) -> Option<bool>;
    /// Validate and append one cast; `None` when no canonical manager exists.
    fn validate_cast_like_cpp(
        &mut self,
        cast: &PendingCreatureSpellCastLikeCpp,
        config: &LegacyCreatureAggroConfigLikeCpp,
        plan: &mut RuntimePlan,
    ) -> Option<CreatureSpellCastValidationResultLikeCpp>;
}

/// The legacy bridge: the caster is the legacy representation, and every
/// action takes the guards it took before F6-8D3a-1b.
pub(super) struct LegacyCreatureSpellActionOwnerLikeCpp<'a> {
    pub(super) legacy_map_manager: &'a crate::map_manager::SharedMapManager,
    pub(super) canonical_map_manager: Option<&'a SharedCanonicalMapManager>,
}

impl CreatureSpellActionOwnerLikeCpp for LegacyCreatureSpellActionOwnerLikeCpp<'_> {
    fn with_schedule_caster_like_cpp<R>(
        &mut self,
        schedule: &PendingCreatureSpellScheduleLikeCpp,
        operation: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R> {
        let mut manager = self
            .legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        manager
            .find_creature_mut(schedule.map_id, schedule.instance_id, schedule.caster_guid)
            .map(|creature| operation(&mut creature.creature))
    }

    fn turret_rejected_attempt_like_cpp(
        &mut self,
        attempt: &TurretRejectedCastAttemptLikeCpp,
        config: &LegacyCreatureAggroConfigLikeCpp,
    ) -> Option<bool> {
        Some(apply_turret_rejected_cast_attempt_like_cpp(
            self.canonical_map_manager?,
            self.legacy_map_manager,
            attempt,
            config,
        ))
    }

    fn validate_cast_like_cpp(
        &mut self,
        cast: &PendingCreatureSpellCastLikeCpp,
        config: &LegacyCreatureAggroConfigLikeCpp,
        plan: &mut RuntimePlan,
    ) -> Option<CreatureSpellCastValidationResultLikeCpp> {
        Some(validate_and_append_creature_spell_cast_like_cpp(
            self.canonical_map_manager?,
            self.legacy_map_manager,
            &cast.command,
            cast.difficulty_id,
            cast.turret_ai,
            config,
            plan,
        ))
    }
}

/// The admitted canonical executor: the caster is the canonical incarnation on
/// the manager the executor holds locked for the whole transition.
pub(super) struct CanonicalCreatureSpellActionOwnerLikeCpp<'a> {
    pub(super) manager: &'a mut wow_map::MapManager,
}

impl CreatureSpellActionOwnerLikeCpp for CanonicalCreatureSpellActionOwnerLikeCpp<'_> {
    fn with_schedule_caster_like_cpp<R>(
        &mut self,
        schedule: &PendingCreatureSpellScheduleLikeCpp,
        operation: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> Option<R> {
        self.manager
            .find_map_mut(u32::from(schedule.map_id), schedule.instance_id)?
            .map_mut()
            .with_creature_mut_like_cpp(schedule.caster_guid, operation)
    }

    fn turret_rejected_attempt_like_cpp(
        &mut self,
        attempt: &TurretRejectedCastAttemptLikeCpp,
        config: &LegacyCreatureAggroConfigLikeCpp,
    ) -> Option<bool> {
        Some(apply_turret_rejected_cast_attempt_on_manager_like_cpp(
            self.manager,
            &mut CanonicalCreatureSpellCastersLikeCpp,
            attempt,
            config,
        ))
    }

    fn validate_cast_like_cpp(
        &mut self,
        cast: &PendingCreatureSpellCastLikeCpp,
        config: &LegacyCreatureAggroConfigLikeCpp,
        plan: &mut RuntimePlan,
    ) -> Option<CreatureSpellCastValidationResultLikeCpp> {
        Some(validate_and_append_creature_spell_cast_on_manager_like_cpp(
            self.manager,
            &mut CanonicalCreatureSpellCastersLikeCpp,
            &cast.command,
            cast.difficulty_id,
            cast.turret_ai,
            config,
            plan,
        ))
    }
}

/// What a deferred re-arm found on its caster.
enum CreatureSpellScheduleDrainLikeCpp {
    NotCanonicalOwner,
    StaleEngagement,
    Scheduled,
    RuntimeRngAuthorityRejected,
}

/// Drain the deferred spell actions in the order the selection recorded them:
/// `DoCast` → `ScheduleEvent` per due event, exactly as `CombatAI::UpdateAI`
/// runs them (`CombatAI.cpp:101-106`).
pub(super) fn drain_creature_spell_actions_like_cpp<O>(
    owner: &mut O,
    ownership: &CanonicalCreatureOwnershipLikeCpp,
    pending_actions: Vec<PendingCreatureSpellActionLikeCpp>,
    config: &LegacyCreatureAggroConfigLikeCpp,
    outcome: &mut LegacyCreatureSpellTickOutcomeLikeCpp,
) where
    O: CreatureSpellActionOwnerLikeCpp,
{
    for pending_action in pending_actions {
        let pending_cast = match pending_action {
            PendingCreatureSpellActionLikeCpp::Schedule(schedule) => {
                let drained = owner.with_schedule_caster_like_cpp(&schedule, |creature| {
                    // #1263 F6-8C: the deferred slot re-arm mutates the caster's
                    // canonical runtime schedule, so it needs the canonical owner.
                    if !ownership.decides_like_cpp(
                        schedule.map_id,
                        schedule.instance_id,
                        schedule.caster_guid,
                    ) {
                        return CreatureSpellScheduleDrainLikeCpp::NotCanonicalOwner;
                    }
                    if creature.creature_spell_engagement_epoch_like_cpp()
                        != schedule.engagement_epoch
                    {
                        return CreatureSpellScheduleDrainLikeCpp::StaleEngagement;
                    }
                    let Some(delay) = creature.random_creature_spell_delay_like_cpp(
                        schedule.minimum_ms,
                        schedule.minimum_ms.saturating_mul(2),
                    ) else {
                        return CreatureSpellScheduleDrainLikeCpp::RuntimeRngAuthorityRejected;
                    };
                    creature.schedule_creature_spell_slot_after_like_cpp(schedule.slot, delay);
                    CreatureSpellScheduleDrainLikeCpp::Scheduled
                });
                match drained {
                    Some(CreatureSpellScheduleDrainLikeCpp::NotCanonicalOwner) => {
                        outcome.canonical_incarnation_rejections += 1;
                    }
                    Some(CreatureSpellScheduleDrainLikeCpp::RuntimeRngAuthorityRejected) => {
                        outcome.runtime_rng_authority_rejections += 1;
                    }
                    None
                    | Some(
                        CreatureSpellScheduleDrainLikeCpp::StaleEngagement
                        | CreatureSpellScheduleDrainLikeCpp::Scheduled,
                    ) => {}
                }
                continue;
            }
            PendingCreatureSpellActionLikeCpp::TurretRejectedAttempt(attempt) => {
                if owner.turret_rejected_attempt_like_cpp(&attempt, config) == Some(true) {
                    outcome.turret_rejected_attempt_swings += 1;
                }
                continue;
            }
            PendingCreatureSpellActionLikeCpp::Cast(pending_cast) => pending_cast,
        };
        let Some(validation) =
            owner.validate_cast_like_cpp(&pending_cast, config, &mut outcome.plan)
        else {
            outcome.canonical_cast_missing_target += 1;
            outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            continue;
        };

        match validation {
            CreatureSpellCastValidationResultLikeCpp::Ready(hit_result) => {
                outcome.canonical_cast_preconditions_passed += 1;
                match hit_result {
                    CreatureSpellTargetHitResultLikeCpp::Hit => outcome.spell_hits += 1,
                    CreatureSpellTargetHitResultLikeCpp::Miss => outcome.spell_misses += 1,
                }
            }
            CreatureSpellCastValidationResultLikeCpp::OutOfRange => {
                outcome.spell_range_rejections += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
            CreatureSpellCastValidationResultLikeCpp::LosRejected => {
                outcome.spell_los_rejections += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
            CreatureSpellCastValidationResultLikeCpp::MissingTarget => {
                outcome.canonical_cast_missing_target += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
            CreatureSpellCastValidationResultLikeCpp::TargetRejected => {
                outcome.canonical_cast_target_rejections += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
            CreatureSpellCastValidationResultLikeCpp::CooldownRejected => {
                outcome.canonical_cast_cooldown_rejections += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
            CreatureSpellCastValidationResultLikeCpp::HitResultUnrepresented => {
                outcome.spell_hit_results_unrepresented += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
            CreatureSpellCastValidationResultLikeCpp::CasterIncarnationRejected => {
                outcome.caster_incarnation_rejections += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
            CreatureSpellCastValidationResultLikeCpp::RuntimeRngAuthorityRejected => {
                outcome.runtime_rng_authority_rejections += 1;
                outcome.casts_ready = outcome.casts_ready.saturating_sub(1);
            }
        }
    }
}
