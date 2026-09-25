// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature spell admission: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{CreatureSpellTargetHitResultLikeCpp, LegacyCreatureAggroConfigLikeCpp, ObjectGuid};
use super::{PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP, SharedCanonicalMapManager, UnitFlags, UnitState};

pub(in crate::session) enum CreatureSpellCastValidationResultLikeCpp {
    Ready(CreatureSpellTargetHitResultLikeCpp),
    OutOfRange,
    LosRejected,
    MissingTarget,
    TargetRejected,
    CooldownRejected,
    HitResultUnrepresented,
    RuntimeRngAuthorityRejected,
    CasterIncarnationRejected,
}

#[cfg(test)]
pub(in crate::session) fn creature_spell_target_accepts_npc_attack_like_cpp(
    target_flags: UnitFlags,
    spell_attributes: &[u32; 15],
) -> bool {
    const SPELL_ATTR6_CAN_TARGET_UNTARGETABLE_LIKE_CPP: u32 = 0x0100_0000;

    !target_flags.intersects(
        UnitFlags::NON_ATTACKABLE
            | UnitFlags::UNINTERACTIBLE
            | UnitFlags::ON_TAXI
            | UnitFlags::NOT_ATTACKABLE_1
            | UnitFlags::IMMUNE_TO_NPC,
    ) && (!target_flags.contains(UnitFlags::NON_ATTACKABLE_2)
        || spell_attributes[6] & SPELL_ATTR6_CAN_TARGET_UNTARGETABLE_LIKE_CPP != 0)
}

pub(in crate::session) fn creature_spell_target_is_valid_attack_target_like_cpp(
    caster: &wow_entities::Creature,
    victim: &wow_entities::Player,
    spell_attributes: &[u32; 15],
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    if !caster.unit().world().object().is_in_world()
        || !victim.unit().world().object().is_in_world()
        || !caster.unit().is_alive()
        || !victim.unit().is_alive()
        || victim.is_game_master_like_cpp()
        || victim.unit().unit_state() & (UnitState::DIED | UnitState::IN_FLIGHT).bits() != 0
        || !caster
            .unit()
            .can_see_or_detect_unit_like_cpp(victim.unit(), false, false, false)
    {
        return false;
    }

    let Some(faction_templates) = config.faction_template_store.as_deref() else {
        return false;
    };
    let Ok(caster_faction_template_id) = u32::try_from(caster.unit().data().faction_template)
    else {
        return false;
    };
    let Ok(victim_faction_template_id) = u32::try_from(victim.unit().data().faction_template)
    else {
        return false;
    };
    let (Some(caster_faction_template), Some(victim_faction_template)) = (
        faction_templates.get(caster_faction_template_id),
        faction_templates.get(victim_faction_template_id),
    ) else {
        return false;
    };

    let mut victim_flags = victim.unit().unit_flags_like_cpp();
    if spell_attributes[6] & 0x0100_0000 != 0 {
        victim_flags.remove(UnitFlags::NON_ATTACKABLE_2);
    }
    let mut context = wow_entities::UnitAttackContextLikeCpp {
        victim_is_game_master_player: victim.is_game_master_like_cpp(),
        visibility_represented: true,
        attacker_can_see_or_detect_target: true,
        victim_unit_state: victim.unit().unit_state(),
        attacker_unit_flags: caster.unit().unit_flags_like_cpp().bits(),
        victim_unit_flags: victim_flags.bits(),
        relation_represented: true,
        attacker_is_hostile_to_victim: caster_faction_template
            .is_hostile_to_like_cpp(victim_faction_template),
        victim_is_hostile_to_attacker: victim_faction_template
            .is_hostile_to_like_cpp(caster_faction_template),
        attacker_is_friendly_to_victim: caster_faction_template
            .is_friendly_to_like_cpp(victim_faction_template),
        victim_is_friendly_to_attacker: victim_faction_template
            .is_friendly_to_like_cpp(caster_faction_template),
        victim_has_affecting_player: true,
        ..Default::default()
    };

    let creature_faction_id = u32::from(caster_faction_template.faction);
    if creature_faction_id != 0 {
        if victim.has_forced_reputation_rank_like_cpp(creature_faction_id) {
            // The canonical player currently retains only the presence of a
            // forced reaction, not its rank. Its exact reaction is therefore
            // unrepresented at this cast-time boundary.
            return false;
        }
        let Some(factions) = config.faction_store.as_deref() else {
            return false;
        };
        let Some(creature_faction) = factions.get(creature_faction_id) else {
            return false;
        };
        if creature_faction.can_have_reputation_like_cpp()
            && victim.has_reputation_state_like_cpp(creature_faction_id)
        {
            context.player_creature_reputation_represented = true;
            context.creature_is_contested_guard =
                caster_faction_template.is_contested_guard_faction_like_cpp();
            context.player_has_contested_pvp_flag =
                victim.has_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
            context.player_at_war_with_creature_faction =
                victim.is_at_war_with_faction_like_cpp(creature_faction_id);
        }
    }

    wow_entities::Unit::is_valid_attack_target_represented_like_cpp(&context)
}

/// Resolve the effective `SpellRange` row C++ `Spell::GetMinMaxRange` reads.
///
/// C++ walks the difficulty-specific `SpellMisc.RangeIndex` into
/// `sSpellRangeStore`, whose rows already carry official/custom SQL overlays
/// and the final `hotfix_data` removals. Both the cast-time range gate and the
/// TurretAI attempt gate must read that same authority or a hotfixed range
/// silently applies to one of them only.
pub(in crate::session) fn creature_ai_effective_spell_range_like_cpp<'a>(
    spell_id: u32,
    difficulty_id: u8,
    config: &'a LegacyCreatureAggroConfigLikeCpp,
) -> Option<&'a wow_data::SpellRangeEntry> {
    let misc = config
        .spell_misc_store
        .as_ref()?
        .entry_for_spell_difficulty_with_fallback_like_cpp(
            spell_id,
            difficulty_id,
            config.difficulty_store.as_deref(),
        )?;
    config
        .spell_range_store
        .as_ref()?
        .get(u32::from(misc.range_index))
}

/// A TurretAI cast attempt that a represented `Spell::CheckCast` rejection
/// stopped before any plan was built.
pub(in crate::session) struct TurretRejectedCastAttemptLikeCpp {
    pub(in crate::session) caster_guid: ObjectGuid,
    pub(in crate::session) target_guid: ObjectGuid,
    pub(in crate::session) map_id: u16,
    pub(in crate::session) instance_id: u32,
    pub(in crate::session) engagement_epoch: u64,
    pub(in crate::session) spell_id: u32,
    pub(in crate::session) difficulty_id: u8,
}

/// Consume the BASE_ATTACK swing of a TurretAI attempt C++ would have made.
///
/// C++ `UnitAI::DoSpellAttackIfReady` runs the strict
/// `IsWithinCombatRange(GetMaxRange(false))` gate, then calls `CastSpell` and
/// `resetAttackTimer` inside that branch. The reset therefore happens even when
/// `Spell::CheckCast` rejects the cast — for a `disables` row, for instance —
/// but never when the victim is out of that raw range or exactly on its bound.
/// Returns whether the swing was consumed.
pub(in crate::session) fn apply_turret_rejected_cast_attempt_like_cpp(
    canonical_map_manager: &SharedCanonicalMapManager,
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    attempt: &TurretRejectedCastAttemptLikeCpp,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> bool {
    let Ok(manager) = canonical_map_manager.lock() else {
        return false;
    };
    // Same canonical -> legacy lock order the cast validation path uses.
    let mut legacy_guard = legacy_map_manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(legacy_caster) =
        legacy_guard.find_creature(attempt.map_id, attempt.instance_id, attempt.caster_guid)
    else {
        return false;
    };
    if !legacy_caster.is_alive()
        || legacy_caster.state() != wow_entities::CreatureAiState::InCombat
        || legacy_caster.creature.ai_ownership().combat_target != Some(attempt.target_guid)
        || legacy_caster.creature_spell_engagement_epoch_like_cpp() != attempt.engagement_epoch
    {
        return false;
    }
    let Some(range) =
        creature_ai_effective_spell_range_like_cpp(attempt.spell_id, attempt.difficulty_id, config)
    else {
        return false;
    };
    let Some(managed) = manager.find_map(u32::from(attempt.map_id), attempt.instance_id) else {
        return false;
    };
    let within_raw_combat_range = {
        let map = managed.map();
        let Some(caster) = map.creature_transform_vitals_snapshot_like_cpp(attempt.caster_guid)
        else {
            return false;
        };
        let Some(victim) = map.get_typed_player(attempt.target_guid) else {
            return false;
        };
        if !caster.is_alive || !victim.unit().is_alive() {
            return false;
        }
        let reach_sum =
            caster.combat_reach.max(0.0) + victim.unit().world().combat_reach().max(0.0);
        let turret_combat_maximum = range.range_max[0].max(0.0) + reach_sum;
        let distance_sq = caster
            .position
            .distance_sq(&victim.unit().world().position());
        distance_sq < turret_combat_maximum * turret_combat_maximum
    };
    if !within_raw_combat_range {
        return false;
    }
    let Some(creature) =
        legacy_guard.find_creature_mut(attempt.map_id, attempt.instance_id, attempt.caster_guid)
    else {
        return false;
    };
    creature.record_swing();
    true
}
