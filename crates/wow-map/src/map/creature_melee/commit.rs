use super::*;
use crate::manager::MeleeKillCollector;
use wow_entities::{LineOfSightOptions, WorldObject, WorldObjectEnvironment};

pub fn is_creature_melee_los_clear_like_cpp(
    attacker: &WorldObject,
    victim: &WorldObject,
    environment: &impl WorldObjectEnvironment,
) -> bool {
    attacker.is_within_los_in_map(victim, environment, LineOfSightOptions::default())
}

pub(super) fn apply_creature_melee_damage_to_canonical_player_on_map_like_cpp(
    canonical_map_manager: &mut MapManager,
    map_id: u32,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    attacker_position: Position,
    attacker_combat_reach: f32,
    attacker_can_state_update: bool,
    victim_guid: ObjectGuid,
    damage: Option<u32>,
    wire_health_before: Option<u64>,
) -> CreatureMeleeApplyResultLikeCpp {
    let Some(managed) = canonical_map_manager.find_map_mut(map_id, instance_id) else {
        return CreatureMeleeApplyResultLikeCpp::MissingVictim;
    };

    let (health_before, health_state_revision_before, target_level) = {
        let map = managed.map();
        let Some(victim) = map.get_typed_player(victim_guid) else {
            return CreatureMeleeApplyResultLikeCpp::MissingVictim;
        };

        let victim_position = victim.unit().world().position();
        let victim_data = victim.unit().data();
        let victim_combat_reach = victim_data.combat_reach;
        if !is_within_melee_range_like_cpp(
            attacker_position,
            attacker_combat_reach,
            victim_position,
            victim_combat_reach,
        ) {
            return CreatureMeleeApplyResultLikeCpp::OutOfRange;
        }
        if !is_within_target_boundary_radius_like_cpp(
            attacker_position,
            attacker_combat_reach,
            victim_position,
            victim_combat_reach,
            victim_data.bounding_radius,
        ) && !is_unit_facing_target_for_melee_like_cpp(attacker_position, victim_position)
        {
            return CreatureMeleeApplyResultLikeCpp::BadFacing;
        }
        if (!victim.unit().is_alive() || victim.unit().data().health == 0)
            && wire_health_before.is_none()
        {
            return CreatureMeleeApplyResultLikeCpp::VictimNotAlive;
        }
        if !attacker_can_state_update {
            return CreatureMeleeApplyResultLikeCpp::AttackerStateRejected;
        }
        // C++ `Unit::AttackerStateUpdate` requires a real attacker and checks
        // LOS before removing attacking-interrupt auras or calculating damage.
        let Some(attacker_validation) = map.with_creature_like_cpp(attacker_guid, |attacker| {
            if !attacker.is_alive() {
                return Err(CreatureMeleeApplyResultLikeCpp::AttackerUnavailable);
            }
            if !is_creature_melee_los_clear_like_cpp(
                attacker.unit().world(),
                victim.unit().world(),
                map,
            ) {
                return Err(CreatureMeleeApplyResultLikeCpp::LosRejected);
            }
            Ok(())
        }) else {
            return CreatureMeleeApplyResultLikeCpp::AttackerUnavailable;
        };
        if let Err(result) = attacker_validation {
            return result;
        }

        (
            victim.unit().data().health,
            victim.unit().health_state_revision_like_cpp(),
            victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8,
        )
    };

    let Some(damage) = damage else {
        return CreatureMeleeApplyResultLikeCpp::Ready;
    };

    let Some(victim) = managed.map_mut().get_typed_player_mut(victim_guid) else {
        return CreatureMeleeApplyResultLikeCpp::MissingVictim;
    };
    let health_after = health_before.saturating_sub(u64::from(damage));
    victim.unit_mut().set_health(health_after);
    if health_after == 0 {
        // The map owner commits the authoritative lethal transition. Session
        // delivery is presentation-only and may legitimately be skipped after
        // logout or a map transfer.
        victim
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
        victim.unit_mut().set_health(0);
    }
    let health_state_revision_after = victim.unit().health_state_revision_like_cpp();
    let wire_health_before = wire_health_before.unwrap_or(health_before);
    let over_damage = if u64::from(damage) >= wire_health_before {
        u64::from(damage)
            .saturating_sub(wire_health_before)
            .min(i32::MAX as u64) as i32
    } else {
        -1
    };
    CreatureMeleeApplyResultLikeCpp::Hit {
        victim_applied_damage: damage,
        victim_threat: None,
        victim_health_before: health_before,
        victim_health_after: health_after,
        victim_health_state_revision_before: health_state_revision_before,
        victim_health_state_revision_after: health_state_revision_after,
        victim_creature_sync_identity: None,
        over_damage,
        target_level,
        events: Vec::new(),
    }
}
pub(super) fn apply_creature_melee_damage_to_canonical_creature_on_map_like_cpp(
    canonical_map_manager: &mut MapManager,
    map_id: u32,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    attacker_position: Position,
    attacker_combat_reach: f32,
    attacker_can_state_update: bool,
    victim_guid: ObjectGuid,
    damage: Option<u32>,
    // C++ `CalcDamageInfo`'s `(HitInfo, TargetState, Blocked)` when the caller
    // already rolled the attack table; `None` keeps this bridge's normal-hit
    // presentation.
    outcome_presentation: Option<(MeleePresentation, i32)>,
    absorbed: u32,
    wire_health_before: Option<u64>,
    represented_damage_done: Option<u32>,
    threat_plan: CreatureDamageThreatPlanLikeCpp,
    catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    kills: &mut Option<MeleeKillCollector<'_>>,
) -> CreatureMeleeApplyResultLikeCpp {
    let Some(managed) = canonical_map_manager.find_map_mut(map_id, instance_id) else {
        return CreatureMeleeApplyResultLikeCpp::MissingVictim;
    };

    let (
        health_before,
        health_state_revision_before,
        victim_incarnation_authority,
        victim_health_state_revision_authority,
        victim_spawn_id,
        victim_loot_lifecycle_revision_before,
        victim_death_state_before,
        victim_ai_state_before,
        target_level,
        damage,
        damage_done,
        applied_damage,
        hit_info,
    ) = match {
        let map = managed.map();
        map.with_creature_like_cpp(victim_guid, |victim| {
            let victim_position = victim.unit().world().position();
            let victim_data = victim.unit().data();
            let victim_combat_reach = victim_data.combat_reach;
            if !is_within_melee_range_like_cpp(
                attacker_position,
                attacker_combat_reach,
                victim_position,
                victim_combat_reach,
            ) {
                return Err(CreatureMeleeApplyResultLikeCpp::OutOfRange);
            }
            if !is_within_target_boundary_radius_like_cpp(
                attacker_position,
                attacker_combat_reach,
                victim_position,
                victim_combat_reach,
                victim_data.bounding_radius,
            ) && !is_unit_facing_target_for_melee_like_cpp(attacker_position, victim_position)
            {
                return Err(CreatureMeleeApplyResultLikeCpp::BadFacing);
            }
            if !victim.is_alive() && wire_health_before.is_none() {
                return Err(CreatureMeleeApplyResultLikeCpp::VictimNotAlive);
            }
            if !attacker_can_state_update {
                return Err(CreatureMeleeApplyResultLikeCpp::AttackerStateRejected);
            }
            let Some(attacker_validation) = map.with_creature_like_cpp(attacker_guid, |attacker| {
                if !attacker.is_alive() {
                    return Err(CreatureMeleeApplyResultLikeCpp::AttackerUnavailable);
                }
                if !is_creature_melee_los_clear_like_cpp(
                    attacker.unit().world(),
                    victim.unit().world(),
                    map,
                ) {
                    return Err(CreatureMeleeApplyResultLikeCpp::LosRejected);
                }
                Ok(attacker.is_charmed_owned_by_player_or_player_like_cpp())
            }) else {
                return Err(CreatureMeleeApplyResultLikeCpp::AttackerUnavailable);
            };
            let attacker_is_player_controlled = attacker_validation?;

            let Some(damage) = damage else {
                return Err(CreatureMeleeApplyResultLikeCpp::Ready);
            };
            let damage_done = represented_damage_done.unwrap_or_else(|| {
                victim.calculate_damage_for_sparring_like_cpp(
                    true,
                    attacker_is_player_controlled,
                    damage,
                )
            });
            // `AttackerStateUpdate` already carries the raw damage. C++ applies
            // the unkillable Creature clamp later inside `DealDamage`, after
            // sparring and the share loop, so only the health transition uses
            // this reduced amount.
            let applied_damage = victim
                .damage_after_unkillable_gate_like_cpp(attacker_guid == victim_guid, damage_done);
            let mut hit_info =
                outcome_presentation.map_or(MeleePresentation::default(), |(info, _)| info);
            if victim.should_fake_damage_from_like_cpp(true, attacker_is_player_controlled) {
                hit_info.fake_damage = true;
            }

            Ok((
                victim.unit().data().health,
                victim.unit().health_state_revision_like_cpp(),
                victim.loot_authority_like_cpp().clone(),
                victim.unit().health_state_revision_authority_like_cpp(),
                victim.spawn_id(),
                victim.loot_lifecycle_revision_like_cpp(),
                victim.unit().death_state(),
                victim.ai_ownership().state,
                victim.unit().data().level.clamp(0, i32::from(u8::MAX)) as u8,
                damage,
                damage_done,
                applied_damage,
                hit_info,
            ))
        })
    } {
        Some(Ok(prepared)) => prepared,
        Some(Err(result)) => return result,
        None => return CreatureMeleeApplyResultLikeCpp::MissingVictim,
    };

    let Some(victim) = managed.map_mut().get_typed_creature_mut(victim_guid) else {
        return CreatureMeleeApplyResultLikeCpp::MissingVictim;
    };
    let killed = victim.apply_ai_damage_before_death_state_at_game_time_like_cpp(
        applied_damage,
        catalogs.game_time_ms(),
        wow_entities::game_time_secs_like_cpp(),
    );
    if killed {
        victim.set_death_state_runtime(
            wow_constants::DeathState::JustDied,
            wow_entities::game_time_secs_like_cpp(),
        );
        victim.unit_mut().set_health(0);
    }
    let health_after = victim.unit().data().health;
    let health_state_revision_after = victim.unit().health_state_revision_like_cpp();
    if killed {
        if let Some(collector) = kills.as_mut() {
            collector.capture(
                managed.map(),
                crate::MapKey::new(map_id, instance_id),
                MeleeKillCollector::PRIMARY,
                victim_guid,
            );
        }
    }
    // C++ returns before `AddThreat` when sparring reduced `damageDone` to
    // zero. `UNKILLABLE` clamps only the later `damageTaken`, so that path
    // still reaches `AddThreat(0)` and establishes combat references.
    let victim_threat = if killed || damage_done == 0 {
        None
    } else {
        super::threat::apply_creature_damage_threat_on_map_like_cpp(
            managed.map_mut(),
            victim_guid,
            attacker_guid,
            applied_damage,
            threat_plan,
        )
    };
    let victim_creature_sync_identity = managed
        .map()
        .with_creature_like_cpp(victim_guid, |victim| {
            CreatureMeleeVictimSyncIdentityLikeCpp {
                authority: victim_incarnation_authority,
                health_state_revision_authority: victim_health_state_revision_authority,
                spawn_id: victim_spawn_id,
                loot_lifecycle_revision_before: victim_loot_lifecycle_revision_before,
                loot_lifecycle_revision_after: victim.loot_lifecycle_revision_like_cpp(),
                death_state_before: victim_death_state_before,
                death_state_after: victim.unit().death_state(),
                ai_state_before: victim_ai_state_before,
                ai_state_after: victim.ai_ownership().state,
            }
        })
        .expect("the just-mutated creature remains in the canonical map");
    // C++ serializes AttackerStateUpdate before DealMeleeDamage applies the
    // creature sparring clamp. Its overkill field therefore uses the raw wire
    // damage against pre-hit health, not the post-sparring applied damage.
    let wire_health_before = wire_health_before.unwrap_or(health_before);
    let over_damage = if u64::from(damage) >= wire_health_before {
        u64::from(damage)
            .saturating_sub(wire_health_before)
            .min(i32::MAX as u64) as i32
    } else {
        -1
    };
    let values_update = managed
        .map()
        .with_creature_like_cpp(victim_guid, |victim| victim.unit().values_update())
        .expect("the just-mutated creature remains in the canonical map");

    let mut events = Vec::new();
    events.push(MeleeEffect::AttackState {
        attacker: attacker_guid,
        victim: victim_guid,
        presentation: hit_info,
        damage: damage.min(i32::MAX as u32) as i32,
        original_damage: damage.min(i32::MAX as u32) as i32,
        over_damage,
        blocked: outcome_presentation.map_or(0, |(_, blocked)| blocked.max(0)),
        absorbed: absorbed.min(i32::MAX as u32) as i32,
        target_level,
    });
    events.push(MeleeEffect::Values {
        guid: victim_guid,
        update: values_update,
    });

    CreatureMeleeApplyResultLikeCpp::Hit {
        victim_applied_damage: applied_damage,
        victim_threat,
        victim_health_before: health_before,
        victim_health_after: health_after,
        victim_health_state_revision_before: health_state_revision_before,
        victim_health_state_revision_after: health_state_revision_after,
        victim_creature_sync_identity: Some(victim_creature_sync_identity),
        over_damage,
        target_level,
        events,
    }
}
