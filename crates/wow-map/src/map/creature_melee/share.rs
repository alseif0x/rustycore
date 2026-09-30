//! Secondary-target side of creature white-swing shared damage.

use super::*;
use crate::manager::MeleeKillCollector;
use wow_core::ObjectGuid;

#[derive(Default)]
pub(super) struct MeleeShareDamageOutcomeLikeCpp {
    pub mutation_events: Vec<MeleeEffect>,
    pub creature_syncs: Vec<(ObjectGuid, CreatureMeleeVictimSyncStateLikeCpp)>,
    pub primary_was_share_target: bool,
    /// Intermediate health values C++ sends when a Player primary victim is
    /// also its own share target. They ride the primary session command so
    /// they remain between AttackerStateUpdate and the final primary health.
    pub primary_player_share_health_updates: Vec<u64>,
}

/// Represent C++ `Unit::DealDamage`'s `SPELL_AURA_SHARE_DAMAGE_PCT` loop
/// (`Unit.cpp:833-856`) for one creature white swing.
///
/// Every active, school-matching aura copies a percentage of the same primary
/// `damageDone`; it neither subtracts from that damage nor recursively shares
/// because the secondary call uses `NODAMAGE`. Canonical Map health remains the
/// sole Player/Creature authority.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_melee_share_damage_like_cpp(
    canonical_manager: &mut MapManager,
    map_id: u16,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    damage_done: u32,
    damage_school_mask: u32,
    attacker_is_player_controlled: bool,
    catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    difficulty_id: u8,
    kills: &mut Option<MeleeKillCollector<'_>>,
) -> MeleeShareDamageOutcomeLikeCpp {
    let mut result = MeleeShareDamageOutcomeLikeCpp::default();
    if damage_done == 0 {
        return result;
    }

    let Some(managed) = canonical_manager.find_map(u32::from(map_id), instance_id) else {
        return result;
    };
    let share_auras = if victim_guid.is_player() {
        managed
            .map()
            .get_typed_player(victim_guid)
            .map_or_else(Vec::new, |victim| {
                catalogs.share_player(
                    victim
                        .unit()
                        .subsystems()
                        .auras
                        .runtime_applications_like_cpp(),
                )
            })
    } else {
        managed
            .map()
            .with_creature_like_cpp(victim_guid, |victim| {
                catalogs.share_creature(
                    &victim.unit().subsystems().auras.applied_auras,
                    difficulty_id,
                )
            })
            .unwrap_or_default()
    };

    for aura in share_auras {
        if !share_aura_still_applied_like_cpp(
            canonical_manager,
            map_id,
            instance_id,
            victim_guid,
            aura.identity,
        ) || aura.school_mask & damage_school_mask == 0
            || aura.amount <= 0
        {
            continue;
        }
        // C++ `CalculatePct(uint32, int32)` performs the multiplication in
        // float and truncates on conversion back to uint32. Unlike split
        // damage, this loop does not clamp the result to the primary damage.
        let share_damage = (damage_done as f32 * aura.amount as f32 / 100.0) as u32;
        if share_damage == 0 {
            continue;
        }
        let Some(secondary) = apply_secondary_share_damage_like_cpp(
            canonical_manager,
            map_id,
            instance_id,
            attacker_guid,
            aura.caster_guid,
            share_damage,
            attacker_is_player_controlled,
            match aura.identity {
                ShareAuraIdentityLikeCpp::Player { spell_id, .. } => spell_id,
                ShareAuraIdentityLikeCpp::Creature { applied, .. } => {
                    i32::try_from(applied.spell_id).unwrap_or(0)
                }
            },
            catalogs,
            difficulty_id,
            kills,
        ) else {
            continue;
        };
        let is_primary_target = aura.caster_guid == victim_guid;
        result.primary_was_share_target |= is_primary_target;
        if is_primary_target && victim_guid.is_player() {
            if let Some(health) = secondary.player_health_after {
                result.primary_player_share_health_updates.push(health);
            }
        } else {
            result.mutation_events.extend(secondary.mutation_events);
        }
        if let Some(sync) = secondary.creature_sync {
            result.creature_syncs.push((aura.caster_guid, sync));
        }
    }

    result
}

fn share_aura_still_applied_like_cpp(
    canonical_manager: &MapManager,
    map_id: u16,
    instance_id: u32,
    victim_guid: ObjectGuid,
    identity: ShareAuraIdentityLikeCpp,
) -> bool {
    let Some(managed) = canonical_manager.find_map(u32::from(map_id), instance_id) else {
        return false;
    };
    match identity {
        ShareAuraIdentityLikeCpp::Player {
            slot,
            spell_id,
            caster_guid,
            effect_index,
        } => managed
            .map()
            .get_typed_player(victim_guid)
            .and_then(|victim| {
                victim
                    .unit()
                    .subsystems()
                    .auras
                    .runtime_applications_like_cpp()
                    .get(&slot)
            })
            .is_some_and(|aura| {
                aura.spell_id == spell_id
                    && aura.caster_guid == caster_guid
                    && 1_u32
                        .checked_shl(effect_index)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
            }),
        ShareAuraIdentityLikeCpp::Creature {
            applied,
            effect_index,
        } => managed
            .map()
            .with_creature_like_cpp(victim_guid, |victim| {
                victim
                    .unit()
                    .subsystems()
                    .auras
                    .applied_auras
                    .contains(&applied)
                    && 1_u32
                        .checked_shl(effect_index)
                        .is_some_and(|bit| applied.effect_mask & bit != 0)
            })
            .unwrap_or(false),
    }
}

struct SecondaryShareDamageOutcomeLikeCpp {
    mutation_events: Vec<MeleeEffect>,
    creature_sync: Option<CreatureMeleeVictimSyncStateLikeCpp>,
    player_health_after: Option<u64>,
}

/// Apply the already-sized `NODAMAGE` recursive call after its represented
/// `DealDamageMods` target gates. Damage immunity is deliberately absent: the
/// target 3.4.3 share loop does not call `IsImmunedToDamage` here.
#[allow(clippy::too_many_arguments)]
fn apply_secondary_share_damage_like_cpp(
    canonical_manager: &mut MapManager,
    map_id: u16,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    caster_guid: ObjectGuid,
    share_damage: u32,
    attacker_is_player_controlled: bool,
    spell_id: i32,
    catalogs: &impl CreatureMeleeCatalogsLikeCpp,
    difficulty_id: u8,
    kills: &mut Option<MeleeKillCollector<'_>>,
) -> Option<SecondaryShareDamageOutcomeLikeCpp> {
    let managed = canonical_manager.find_map(u32::from(map_id), instance_id)?;
    let rejected_by_damage_mods = if caster_guid.is_player() {
        managed.map().get_typed_player(caster_guid).map(|caster| {
            !caster.unit().world().object().is_in_world()
                || !caster.unit().is_alive()
                || caster.unit().unit_state() & wow_constants::unit::UnitState::IN_FLIGHT.bits()
                    != 0
        })
    } else {
        managed.map().with_creature_like_cpp(caster_guid, |caster| {
            !caster.unit().world().object().is_in_world()
                || !caster.is_alive()
                || caster.unit().unit_state() & wow_constants::unit::UnitState::IN_FLIGHT.bits()
                    != 0
                || caster.is_evading_attacks_like_cpp()
        })
    }?;
    if rejected_by_damage_mods {
        return Some(SecondaryShareDamageOutcomeLikeCpp {
            mutation_events: Vec::new(),
            creature_sync: None,
            player_health_after: None,
        });
    }

    let threat_plan = super::threat::plan_creature_damage_threat_like_cpp(
        managed.map(),
        attacker_guid,
        Some(spell_id),
        catalogs,
        difficulty_id,
    );
    let mut mutation_events = Vec::new();
    let mut creature_sync = None;
    let mut player_health_after = None;
    if caster_guid.is_player() {
        let caster = canonical_manager
            .find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_player_mut(caster_guid)?;
        let health_before = caster.unit().data().health;
        let health_after = health_before.saturating_sub(u64::from(share_damage));
        caster.unit_mut().set_health(health_after);
        if health_after == 0 {
            caster
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
            caster.unit_mut().set_health(0);
        }
        mutation_events.push(MeleeEffect::PlayerHealth {
            guid: caster_guid,
            health: health_after.min(i64::MAX as u64) as i64,
        });
        player_health_after = Some(health_after);
    } else {
        let managed = canonical_manager.find_map_mut(u32::from(map_id), instance_id)?;
        let map = managed.map_mut();
        let caster = map.get_typed_creature_mut(caster_guid)?;
        let health_before = caster.unit().data().health;
        let revision_before = caster.unit().health_state_revision_like_cpp();
        let identity_authority = caster.loot_authority_like_cpp().clone();
        let health_authority = caster.unit().health_state_revision_authority_like_cpp();
        let spawn_id = caster.spawn_id();
        let loot_revision_before = caster.loot_lifecycle_revision_like_cpp();
        let death_state_before = caster.unit().death_state();
        let ai_state_before = caster.ai_ownership().state;
        let applied_damage = caster.calculate_damage_for_sparring_like_cpp(
            true,
            attacker_is_player_controlled,
            share_damage,
        );
        let applied_damage = caster
            .damage_after_unkillable_gate_like_cpp(attacker_guid == caster_guid, applied_damage);
        let killed = caster.apply_ai_damage_before_death_state_at_game_time_like_cpp(
            applied_damage,
            catalogs.game_time_ms(),
            wow_entities::game_time_secs_like_cpp(),
        );
        if killed {
            caster.set_death_state_runtime(
                wow_constants::DeathState::JustDied,
                wow_entities::game_time_secs_like_cpp(),
            );
            caster.unit_mut().set_health(0);
        }
        if killed {
            if let Some(collector) = kills.as_mut() {
                collector.capture(
                    map,
                    crate::MapKey::new(u32::from(map_id), instance_id),
                    MeleeKillCollector::SHARE,
                    caster_guid,
                );
            }
        }
        let threat = if killed {
            None
        } else {
            super::threat::apply_creature_damage_threat_on_map_like_cpp(
                canonical_manager
                    .find_map_mut(u32::from(map_id), instance_id)?
                    .map_mut(),
                caster_guid,
                attacker_guid,
                applied_damage,
                threat_plan,
            )
        };
        let caster = canonical_manager
            .find_map_mut(u32::from(map_id), instance_id)?
            .map_mut()
            .get_typed_creature_mut(caster_guid)?;
        let health_after = caster.unit().data().health;
        creature_sync = Some(CreatureMeleeVictimSyncStateLikeCpp {
            applied_damage,
            threat,
            victim_health_before: health_before,
            victim_health_after: health_after,
            victim_health_state_revision_before: revision_before,
            victim_health_state_revision_after: caster.unit().health_state_revision_like_cpp(),
            identity: CreatureMeleeVictimSyncIdentityLikeCpp {
                authority: identity_authority,
                health_state_revision_authority: health_authority,
                spawn_id,
                loot_lifecycle_revision_before: loot_revision_before,
                loot_lifecycle_revision_after: caster.loot_lifecycle_revision_like_cpp(),
                death_state_before,
                death_state_after: caster.unit().death_state(),
                ai_state_before,
                ai_state_after: caster.ai_ownership().state,
            },
        });
        mutation_events.push(MeleeEffect::Values {
            guid: caster_guid,
            update: caster.unit().values_update(),
        });
    }

    Some(SecondaryShareDamageOutcomeLikeCpp {
        mutation_events,
        creature_sync,
        player_health_after,
    })
}
