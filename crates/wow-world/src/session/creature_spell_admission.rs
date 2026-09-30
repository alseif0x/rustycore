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
    super::canonical_runtime::spell::catalogs::with_policies(config, |policies|
        wow_map::map_manager::target_is_valid(caster, victim, spell_attributes, policies))
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
    let action = wow_map::map_manager::SpellAction::TurretRejectedAttempt(wow_map::map_manager::SpellRejectedAttempt {
        caster_guid: attempt.caster_guid, target_guid: attempt.target_guid, map_id: attempt.map_id,
        instance_id: attempt.instance_id, engagement_epoch: attempt.engagement_epoch,
        spell_id: attempt.spell_id, difficulty_id: attempt.difficulty_id,
    });
    super::canonical_runtime::spell::legacy::consume_action(canonical_map_manager, legacy_map_manager,
        action, config, &mut super::RuntimePlan::default())
        .is_some_and(|outcome| outcome.turret_rejected_attempt_swings != 0)
}
