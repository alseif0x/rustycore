//! One complete represented swing shared by Legacy and CanonicalSelected.
//! The caller retains its manager guard throughout the synchronous operation;
//! Legacy additionally retains the established legacy writer guard.
use super::*;
use crate::manager::MeleeKillCollector;
use super::{player_victim::*, creature_victim::*, secondary_targets::*};
use super::source::{CreatureMeleeSource, SourceAdmission};

#[derive(Clone, Copy)]
pub enum CreatureMeleeReadiness {
    NotReady,
    Rejected,
    Ready(PendingCreatureSwingLikeCpp),
}

pub fn creature_melee_readiness(creature: &WorldCreature, map_id: u16, instance_id: u32)
    -> CreatureMeleeReadiness {
    use wow_constants::unit::UnitState;
    use wow_entities::CurrentSpellSlot;
    if !creature.can_swing() { return CreatureMeleeReadiness::NotReady; }
    if creature.creature.lifecycle_metadata().ai_name == "TurretAI"
        || !creature.creature.can_melee_like_cpp() {
        return CreatureMeleeReadiness::Rejected;
    }
    let unit = creature.creature.unit();
    if unit.has_unit_state(UnitState::CHARGING.bits())
        || (unit.has_unit_state(UnitState::CASTING.bits())
            && !unit.current_spell(CurrentSpellSlot::Channeled)
                .is_some_and(|spell| spell.allow_actions_during_channel)) {
        return CreatureMeleeReadiness::Rejected;
    }
    let Some(victim_guid) = creature.creature.ai_ownership().combat_target else {
        return CreatureMeleeReadiness::NotReady;
    };
    if !victim_guid.is_player() && !victim_guid.is_any_type_creature() {
        return CreatureMeleeReadiness::NotReady;
    }
    CreatureMeleeReadiness::Ready(PendingCreatureSwingLikeCpp {
        map_id, instance_id, attacker_guid: creature.guid(),
        attacker_position: creature.position(), attacker_combat_reach: unit.world().combat_reach(),
        attacker_can_state_update: unit.can_attacker_state_update_melee_like_cpp(false), victim_guid,
    })
}

impl MapManager {
    pub fn apply_legacy_creature_melee_swing(&mut self, attacker: Option<&mut WorldCreature>,
        swing: PendingCreatureSwingLikeCpp, catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    ) -> CreatureMeleeSwingOutcome {
        let Some(attacker) = attacker else {
            let mut outcome = CreatureMeleeSwingOutcome::default();
            outcome.melee_precondition_rejections += 1;
            return outcome;
        };
        self.run_creature_melee_swing(CreatureMeleeSource::Legacy(attacker), swing, catalogs, &mut None)
    }

    pub(crate) fn apply_canonical_creature_melee_swing(&mut self, key: crate::MapKey,
        guid: ObjectGuid, witness: crate::map::CreatureActorWitness,
        swing: PendingCreatureSwingLikeCpp, catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    ) -> CreatureMeleeSwingOutcome {
        self.run_creature_melee_swing(
            CreatureMeleeSource::CanonicalSelected { key, guid, witness }, swing, catalogs, &mut None)
    }

    pub(crate) fn apply_canonical_creature_melee_swing_with_kills(&mut self, key: crate::MapKey,
        guid: ObjectGuid, witness: crate::map::CreatureActorWitness,
        swing: PendingCreatureSwingLikeCpp, catalogs: &impl CreatureMeleeCatalogsLikeCpp,
        kills: &mut Option<MeleeKillCollector<'_>>,
    ) -> CreatureMeleeSwingOutcome {
        self.run_creature_melee_swing(
            CreatureMeleeSource::CanonicalSelected { key, guid, witness }, swing, catalogs, kills)
    }

    fn run_creature_melee_swing(&mut self, mut source: CreatureMeleeSource<'_>,
        mut swing: PendingCreatureSwingLikeCpp, catalogs: &impl CreatureMeleeCatalogsLikeCpp,
        kills: &mut Option<MeleeKillCollector<'_>>,
    ) -> CreatureMeleeSwingOutcome {
        let mut outcome = CreatureMeleeSwingOutcome::default();
        let mut canonical_manager = self;
        if let Err(error) = source.prepare_swing(&mut canonical_manager, &mut swing) {
            outcome.melee_precondition_rejections += 1;
            if matches!(error, SourceAdmission::Incarnation) {
                outcome.attacker_incarnation_rejections += 1;
            }
            return outcome;
        }
        let primary_threat_plan = canonical_manager
            .find_map(u32::from(swing.map_id), swing.instance_id)
            .map(|managed| {
                super::threat::plan_creature_damage_threat_like_cpp(managed.map(), swing.attacker_guid, None, catalogs, managed.difficulty())
            })
            .unwrap_or_default();

        let apply = |canonical_manager: &mut MapManager,
                     kills: &mut Option<MeleeKillCollector<'_>>,
                     swing: &PendingCreatureSwingLikeCpp,
                     damage,
                     presentation: Option<(MeleePresentation, i32)>,
                     absorbed: u32,
                     wire_health_before: Option<u64>,
                     represented_damage_done: Option<u32>| {
            if swing.victim_guid.is_player() {
                super::commit::apply_creature_melee_damage_to_canonical_player_on_map_like_cpp(
                    canonical_manager,
                    u32::from(swing.map_id),
                    swing.instance_id,
                    swing.attacker_guid,
                    swing.attacker_position,
                    swing.attacker_combat_reach,
                    swing.attacker_can_state_update,
                    swing.victim_guid,
                    damage,
                    wire_health_before,
                )
            } else {
                super::commit::apply_creature_melee_damage_to_canonical_creature_on_map_like_cpp(canonical_manager, u32::from(swing.map_id), swing.instance_id, swing.attacker_guid, swing.attacker_position, swing.attacker_combat_reach, swing.attacker_can_state_update, swing.victim_guid, damage, presentation, absorbed, wire_health_before, represented_damage_done, primary_threat_plan, catalogs, kills)
            }
        };

        match apply(&mut canonical_manager, kills, &swing, None, None, 0, None, None) {
            CreatureMeleeApplyResultLikeCpp::Ready => {}
            CreatureMeleeApplyResultLikeCpp::Hit { .. } => {
                unreachable!("melee precondition validation must not mutate canonical health")
            }
            CreatureMeleeApplyResultLikeCpp::OutOfRange => {
                outcome.melee_range_rejections += 1;
                source.retry(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::BadFacing => {
                outcome.melee_facing_rejections += 1;
                source.retry(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerStateRejected => {
                outcome.attacker_state_rejections += 1;
                source.rearm(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::LosRejected => {
                outcome.melee_los_rejections += 1;
                source.rearm(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::VictimNotAlive => {
                // `DoMeleeAttackIfReady` resets BASE_ATTACK after
                // `AttackerStateUpdate` returns early for a dead victim.
                outcome.melee_precondition_rejections += 1;
                source.rearm(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerUnavailable => {
                outcome.melee_precondition_rejections += 1;
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::MissingVictim => {
                outcome.melee_precondition_rejections += 1;
                return outcome;
            }
        }

        if !swing.attacker_can_state_update {
            outcome.attacker_state_rejections += 1;
            source.rearm(&mut canonical_manager);
            return outcome;
        }
        let Some(damage) = source.roll_damage(&mut canonical_manager) else {
            outcome.melee_precondition_rejections += 1;
            // `DoMeleeAttackIfReady` rearms BASE_ATTACK after
            // `AttackerStateUpdate` even if damage calculation cannot produce a
            // represented result.
            source.rearm(&mut canonical_manager);
            return outcome;
        };
        // The compatibility bridge preserves the pre-existing damage and wire
        // behavior, but it does not model RollMeleeOutcomeAgainst or later
        // proc/daze draws. Keep gameplay running while preventing a later
        // creature spell from claiming an exact shared-RNG position.
        source.invalidate_rng(&mut canonical_manager);
        let damage = damage.max(1);

        // C++ `CalculateMeleeDamage` rolls the attack table after mitigation and
        // before the outcome switch (`Unit.cpp:1341-1443`). A player victim
        // resolves its represented outcome, block, armour and taken terms
        // here. Every term needs the spell store, so without one the
        // pre-table always-hit bridge is preserved.
        let mut state = MeleeSwingStateLikeCpp::new_like_cpp(damage);
        let damage = if swing.victim_guid.is_player() {
            player_victim_damage_like_cpp(
                &mut canonical_manager,
                &source,
                &swing,
                catalogs,
                damage,
                &mut state,
            )
        } else {
            creature_victim_damage_like_cpp(
                &mut canonical_manager,
                &source,
                &swing,
                catalogs,
                damage,
                &mut state,
            )
        };
        let MeleeSecondaryTargetsOutcomeLikeCpp {
            damage,
            primary_wire_health_before,
            represented_damage_done,
            split_mutation_events,
            split_combat_log_packets,
            share_mutation_events,
            primary_was_share_target,
            primary_player_share_health_updates,
        } = apply_secondary_targets_damage_like_cpp(
            &mut canonical_manager,
            &source,
            &swing,
            catalogs,
            damage,
            &mut state,
            &mut outcome.syncs,
            kills,
        );
        if !state.outcome_represented {
            outcome.melee_outcomes_unrepresented += 1;
        }

        // C++ `CalculateMeleeDamage` returns before `DealMeleeDamage` for an
        // avoided swing (`Unit.cpp:1345-1355`, `1395-1407`): no health write, no
        // death check and no proc. The command carries the victim's unchanged
        // canonical tuple so the session's revision gate stays exact.
        if let Some(_avoided) = state.avoided_outcome {
            let victim = canonical_manager
                .find_map(u32::from(swing.map_id), swing.instance_id)
                .and_then(|managed| managed.map().get_typed_player(swing.victim_guid))
                .map(|victim| {
                    (
                        victim.unit().data().health,
                        victim.unit().health_state_revision_like_cpp(),
                        victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8,
                    )
                });
            let Some((victim_health_after, victim_health_state_revision_after, target_level)) =
                victim
            else {
                outcome.melee_precondition_rejections += 1;
                return outcome;
            };
            // C++ `Unit::AttackerStateUpdate` removes the attacking-interrupt
            // auras before `CalculateMeleeDamage`, so an avoided swing removes
            // them too (`Unit.cpp:2172-2173`).
            outcome.attacking_interrupt_auras_removed += source.finish_swing(&mut canonical_manager);
            outcome.commands.push(
                CreatureMeleePlayerHit {
                    swing,
                    damage: 0,
                    over_damage: -1,
                    target_level,
                    victim_health_after,
                    victim_health_state_revision_after,
                    presentation: state.hit_info,
                    
                    original_damage: state.original_damage,
                    absorbed: 0,
                    mana_spent: 0,
                    absorb_consumptions: Vec::new(),
                    split_combat_log_packets: Vec::new(),
                    self_share_health_updates: Vec::new(),
                },
            );
            return outcome;
        }

        let (
            victim_applied_damage,
            victim_threat,
            victim_health_before,
            victim_health_after,
            victim_health_state_revision_before,
            victim_health_state_revision_after,
            victim_creature_sync_identity,
            over_damage,
            target_level,
            events,
        ) = match apply(
            &mut canonical_manager,
            kills,
            &swing,
            Some(damage),
            state.creature_victim_presentation,
            state.absorbed_damage,
            primary_was_share_target
                .then_some(primary_wire_health_before)
                .flatten(),
            (!swing.victim_guid.is_player()).then_some(represented_damage_done),
        ) {
            CreatureMeleeApplyResultLikeCpp::Hit {
                victim_applied_damage,
                victim_threat,
                victim_health_before,
                victim_health_after,
                victim_health_state_revision_before,
                victim_health_state_revision_after,
                victim_creature_sync_identity,
                over_damage,
                target_level,
                events,
            } => (
                victim_applied_damage,
                victim_threat,
                victim_health_before,
                victim_health_after,
                victim_health_state_revision_before,
                victim_health_state_revision_after,
                victim_creature_sync_identity,
                over_damage,
                target_level,
                events,
            ),
            CreatureMeleeApplyResultLikeCpp::Ready => unreachable!(
                "melee apply with represented damage must not return validation readiness"
            ),
            CreatureMeleeApplyResultLikeCpp::OutOfRange => {
                outcome.melee_range_rejections += 1;
                source.retry(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::BadFacing => {
                outcome.melee_facing_rejections += 1;
                source.retry(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerStateRejected => {
                outcome.attacker_state_rejections += 1;
                source.rearm(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::LosRejected => {
                outcome.melee_los_rejections += 1;
                source.rearm(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::VictimNotAlive => {
                outcome.melee_precondition_rejections += 1;
                source.rearm(&mut canonical_manager);
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::AttackerUnavailable => {
                outcome.melee_precondition_rejections += 1;
                return outcome;
            }
            CreatureMeleeApplyResultLikeCpp::MissingVictim => {
                outcome.melee_precondition_rejections += 1;
                return outcome;
            }
        };
        outcome.attacking_interrupt_auras_removed += source.finish_swing(&mut canonical_manager);
        outcome.canonical_hits += 1;
        if swing.victim_guid.is_player() {
            outcome.commands.push(
                CreatureMeleePlayerHit {
                    swing,
                    damage,
                    over_damage,
                    target_level,
                    victim_health_after,
                    victim_health_state_revision_after,
                    presentation: state.hit_info,
                    
                    original_damage: state.original_damage,
                    absorbed: state.absorbed_damage,
                    mana_spent: state.mana_spent,
                    absorb_consumptions: state.absorb_consumptions,
                    split_combat_log_packets: split_combat_log_packets,
                    self_share_health_updates: primary_player_share_health_updates,
                },
            );
            outcome.events.extend(split_mutation_events);
            outcome.events.extend(share_mutation_events);
        } else {
            if !state.creature_victim_avoided {
                outcome.canonical_creature_hits += 1;
            }
            outcome
                .events
                .extend(state.creature_victim_absorb_events);
            outcome.events.extend(split_mutation_events);
            outcome.events.extend(split_combat_log_packets);
            let mut primary_events = events.into_iter();
            if let Some(attacker_state) = primary_events.next() {
                outcome.events.push(attacker_state);
            }
            outcome.events.extend(share_mutation_events);
            outcome.events.extend(primary_events);
            if victim_health_state_revision_after != victim_health_state_revision_before
                || victim_threat.is_some()
            {
                outcome.syncs.push(CreatureVictimCompatibilitySyncLikeCpp {
                    swing,
                    state: CreatureMeleeVictimSyncStateLikeCpp {
                        applied_damage: victim_applied_damage,
                        threat: victim_threat,
                        victim_health_before,
                        victim_health_after,
                        victim_health_state_revision_before,
                        victim_health_state_revision_after,
                        identity: victim_creature_sync_identity
                            .expect("creature victim commits carry incarnation authority"),
                    },
                });
            }
        }
        outcome
    }
}
