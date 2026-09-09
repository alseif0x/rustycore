//! Spell metadata and proc rules state definitions, part 2 of 2.
//!
//! Separated from the mod.rs root under #646. Behaviour is preserved.

use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcRowLikeCpp {
    pub spell_id: i32,
    pub school_mask: u8,
    pub spell_family_name: u16,
    pub spell_family_mask: [u32; 4],
    pub proc_flags: [u32; 2],
    pub spell_type_mask: u32,
    pub spell_phase_mask: u32,
    pub hit_mask: u32,
    pub attributes_mask: u32,
    pub disable_effects_mask: u32,
    pub procs_per_minute: f32,
    pub chance: f32,
    pub cooldown_ms: u32,
    pub charges: u8,
}

pub fn can_spell_trigger_proc_on_event_like_cpp(
    proc_entry: &SpellProcEntryLikeCpp,
    event_info: &SpellProcEventInfoLikeCpp,
) -> bool {
    if !proc_flags_intersect_like_cpp(event_info.type_mask, proc_entry.proc_flags) {
        return false;
    }

    if proc_entry.attributes_mask & PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP != 0
        && event_info.actor_is_player
        && event_info.action_target_exists
        && !event_info.action_target_is_honor_or_xp
    {
        return false;
    }

    if proc_entry.attributes_mask & PROC_ATTR_REQ_POWER_COST_LIKE_CPP != 0
        && event_info.proc_spell_has_positive_power_cost != Some(true)
    {
        return false;
    }

    if event_info.type_mask[0]
        & (PROC_FLAG_HEARTBEAT_LIKE_CPP | PROC_FLAG_KILL_LIKE_CPP | PROC_FLAG_DEATH_LIKE_CPP)
        != 0
    {
        return true;
    }

    if proc_entry.school_mask != 0 && event_info.school_mask & proc_entry.school_mask == 0 {
        return false;
    }

    if event_info.type_mask[0] & SPELL_PROC_FLAG_MASK_LIKE_CPP != 0 {
        if let Some(event_spell_info) = event_info.spell_info {
            if !event_spell_info
                .is_affected_like_cpp(proc_entry.spell_family_name, proc_entry.spell_family_mask)
            {
                return false;
            }
        }

        if proc_entry.spell_type_mask != 0
            && event_info.spell_type_mask & proc_entry.spell_type_mask == 0
        {
            return false;
        }
    }

    if event_info.type_mask[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP != 0
        && event_info.spell_phase_mask & proc_entry.spell_phase_mask == 0
    {
        return false;
    }

    if event_info.type_mask[0] & TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
        || (event_info.type_mask[0] & DONE_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
            && event_info.spell_phase_mask & PROC_SPELL_PHASE_CAST_LIKE_CPP == 0)
    {
        let mut hit_mask = proc_entry.hit_mask;
        if hit_mask == 0 {
            hit_mask = PROC_HIT_NORMAL_LIKE_CPP | PROC_HIT_CRITICAL_LIKE_CPP;
            if event_info.type_mask[0] & TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP == 0 {
                hit_mask |= PROC_HIT_ABSORB_LIKE_CPP;
            }
        }

        if event_info.hit_mask & hit_mask == 0 {
            return false;
        }
    }

    true
}

pub(super) fn proc_flags_intersect_like_cpp(lhs: [u32; 2], rhs: [u32; 2]) -> bool {
    lhs[0] & rhs[0] != 0 || lhs[1] & rhs[1] != 0
}

pub fn implicit_proc_aura_info_like_cpp(aura_type: i32) -> Option<ImplicitProcAuraInfoLikeCpp> {
    if !implicit_proc_aura_can_trigger_like_cpp(aura_type) {
        return None;
    }

    Some(ImplicitProcAuraInfoLikeCpp {
        spell_type_mask: implicit_proc_aura_spell_type_mask_like_cpp(aura_type),
        triggered_can_proc: implicit_proc_aura_is_always_triggered_like_cpp(aura_type),
    })
}

pub(super) fn implicit_proc_aura_can_trigger_like_cpp(aura_type: i32) -> bool {
    matches!(
        aura_type,
        aura_types::SPELL_AURA_DUMMY
            | aura_types::SPELL_AURA_PERIODIC_DUMMY
            | aura_types::SPELL_AURA_MOD_CONFUSE
            | aura_types::SPELL_AURA_MOD_THREAT
            | aura_types::SPELL_AURA_MOD_STUN
            | aura_types::SPELL_AURA_MOD_DAMAGE_DONE
            | aura_types::SPELL_AURA_MOD_DAMAGE_TAKEN
            | aura_types::SPELL_AURA_MOD_RESISTANCE
            | aura_types::SPELL_AURA_MOD_STEALTH
            | aura_types::SPELL_AURA_MOD_FEAR
            | aura_types::SPELL_AURA_MOD_ROOT
            | aura_types::SPELL_AURA_TRANSFORM
            | aura_types::SPELL_AURA_REFLECT_SPELLS
            | aura_types::SPELL_AURA_DAMAGE_IMMUNITY
            | aura_types::SPELL_AURA_PROC_TRIGGER_SPELL
            | aura_types::SPELL_AURA_PROC_TRIGGER_DAMAGE
            | aura_types::SPELL_AURA_MOD_CASTING_SPEED_NOT_STACK
            | aura_types::SPELL_AURA_SCHOOL_ABSORB
            | aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL_PCT
            | aura_types::SPELL_AURA_MOD_POWER_COST_SCHOOL
            | aura_types::SPELL_AURA_REFLECT_SPELLS_SCHOOL
            | aura_types::SPELL_AURA_MECHANIC_IMMUNITY
            | aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN
            | aura_types::SPELL_AURA_SPELL_MAGNET
            | aura_types::SPELL_AURA_MOD_ATTACK_POWER
            | aura_types::SPELL_AURA_MOD_POWER_REGEN_PERCENT
            | aura_types::SPELL_AURA_INTERCEPT_MELEE_RANGED_ATTACKS
            | aura_types::SPELL_AURA_OVERRIDE_CLASS_SCRIPTS
            | aura_types::SPELL_AURA_MOD_MECHANIC_RESISTANCE
            | aura_types::SPELL_AURA_RANGED_ATTACK_POWER_ATTACKER_BONUS
            | aura_types::SPELL_AURA_MOD_MELEE_HASTE
            | aura_types::SPELL_AURA_MOD_MELEE_HASTE_3
            | aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE
            | aura_types::SPELL_AURA_PROC_TRIGGER_SPELL_WITH_VALUE
            | aura_types::SPELL_AURA_MOD_SCHOOL_MASK_DAMAGE_FROM_CASTER
            | aura_types::SPELL_AURA_MOD_SPELL_DAMAGE_FROM_CASTER
            | aura_types::SPELL_AURA_MOD_SPELL_CRIT_CHANCE
            | aura_types::SPELL_AURA_ABILITY_IGNORE_AURASTATE
            | aura_types::SPELL_AURA_MOD_INVISIBILITY
            | aura_types::SPELL_AURA_FORCE_REACTION
            | aura_types::SPELL_AURA_MOD_TAUNT
            | aura_types::SPELL_AURA_MOD_DETAUNT
            | aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE
            | aura_types::SPELL_AURA_MOD_ATTACK_POWER_PCT
            | aura_types::SPELL_AURA_MOD_HIT_CHANCE
            | aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT
            | aura_types::SPELL_AURA_MOD_BLOCK_PERCENT
            | aura_types::SPELL_AURA_MOD_ROOT_2
            | aura_types::SPELL_AURA_IGNORE_SPELL_COOLDOWN
    )
}

pub(super) fn implicit_proc_aura_is_always_triggered_like_cpp(aura_type: i32) -> bool {
    matches!(
        aura_type,
        aura_types::SPELL_AURA_OVERRIDE_CLASS_SCRIPTS
            | aura_types::SPELL_AURA_MOD_STEALTH
            | aura_types::SPELL_AURA_MOD_CONFUSE
            | aura_types::SPELL_AURA_MOD_FEAR
            | aura_types::SPELL_AURA_MOD_ROOT
            | aura_types::SPELL_AURA_MOD_STUN
            | aura_types::SPELL_AURA_TRANSFORM
            | aura_types::SPELL_AURA_MOD_INVISIBILITY
            | aura_types::SPELL_AURA_SPELL_MAGNET
            | aura_types::SPELL_AURA_SCHOOL_ABSORB
            | aura_types::SPELL_AURA_MOD_ROOT_2
    )
}

pub(super) fn implicit_proc_aura_spell_type_mask_like_cpp(aura_type: i32) -> u32 {
    match aura_type {
        aura_types::SPELL_AURA_MOD_STEALTH => {
            PROC_SPELL_TYPE_DAMAGE_LIKE_CPP | PROC_SPELL_TYPE_NO_DMG_HEAL_LIKE_CPP
        }
        aura_types::SPELL_AURA_MOD_CONFUSE
        | aura_types::SPELL_AURA_MOD_FEAR
        | aura_types::SPELL_AURA_MOD_ROOT
        | aura_types::SPELL_AURA_MOD_ROOT_2
        | aura_types::SPELL_AURA_MOD_STUN
        | aura_types::SPELL_AURA_TRANSFORM
        | aura_types::SPELL_AURA_MOD_INVISIBILITY => PROC_SPELL_TYPE_DAMAGE_LIKE_CPP,
        _ => PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImplicitSpellProcSourceLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
    pub spell_family_name: u16,
    pub proc_flags: [u32; 2],
    pub proc_chance: f32,
    pub proc_cooldown_ms: u32,
    pub proc_charges: u32,
    pub proc_base_ppm: f32,
    pub attributes3: u32,
    pub effects: Vec<ImplicitSpellProcEffectLikeCpp>,
}

pub fn implicit_spell_proc_entry_like_cpp(
    spell_info: &ImplicitSpellProcSourceLikeCpp,
) -> Option<SpellProcEntryLikeCpp> {
    if spell_info.proc_flags[0] == 0 && spell_info.proc_flags[1] == 0 {
        return None;
    }

    let mut add_trigger_flag = false;
    let mut proc_spell_type_mask = 0;
    let mut non_proc_mask = 0;

    for effect in &spell_info.effects {
        if !effect.is_effect || effect.aura_type == 0 {
            continue;
        }

        let Some(proc_aura_info) = implicit_proc_aura_info_like_cpp(effect.aura_type) else {
            non_proc_mask |= 1_u32.checked_shl(effect.effect_index).unwrap_or(0);
            continue;
        };

        proc_spell_type_mask |= proc_aura_info.spell_type_mask;
        add_trigger_flag |= proc_aura_info.triggered_can_proc;

        if !add_trigger_flag
            && spell_info.proc_flags[0] & TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
            && matches!(
                effect.aura_type,
                aura_types::SPELL_AURA_PROC_TRIGGER_SPELL
                    | aura_types::SPELL_AURA_PROC_TRIGGER_DAMAGE
            )
        {
            add_trigger_flag = true;
        }
    }

    if proc_spell_type_mask == 0 {
        return None;
    }

    let mut proc_entry = SpellProcEntryLikeCpp {
        school_mask: 0,
        spell_family_name: 0,
        spell_family_mask: [0, 0, 0, 0],
        proc_flags: spell_info.proc_flags,
        spell_type_mask: proc_spell_type_mask,
        spell_phase_mask: PROC_SPELL_PHASE_HIT_LIKE_CPP,
        hit_mask: 0,
        attributes_mask: 0,
        disable_effects_mask: non_proc_mask,
        procs_per_minute: 0.0,
        chance: spell_info.proc_chance,
        cooldown_ms: spell_info.proc_cooldown_ms,
        charges: spell_info.proc_charges,
    };

    for effect in &spell_info.effects {
        if effect.is_effect && implicit_proc_aura_info_like_cpp(effect.aura_type).is_some() {
            for (entry_mask, effect_mask) in proc_entry
                .spell_family_mask
                .iter_mut()
                .zip(effect.spell_class_mask.iter())
            {
                *entry_mask |= *effect_mask;
            }
        }
    }

    if proc_entry.spell_family_mask.iter().any(|mask| *mask != 0) {
        proc_entry.spell_family_name = spell_info.spell_family_name;
    }

    if proc_entry.proc_flags[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP == 0
        && proc_entry.proc_flags[1] & PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP != 0
    {
        proc_entry.spell_phase_mask = PROC_SPELL_PHASE_CAST_LIKE_CPP;
    }

    let mut triggers_spell = false;
    for effect in &spell_info.effects {
        if !effect.is_aura {
            continue;
        }

        match effect.aura_type {
            aura_types::SPELL_AURA_REFLECT_SPELLS
            | aura_types::SPELL_AURA_REFLECT_SPELLS_SCHOOL => {
                proc_entry.hit_mask = PROC_HIT_REFLECT_LIKE_CPP;
                break;
            }
            aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT => {
                proc_entry.hit_mask = PROC_HIT_CRITICAL_LIKE_CPP;
                break;
            }
            aura_types::SPELL_AURA_MOD_BLOCK_PERCENT => {
                proc_entry.hit_mask = PROC_HIT_BLOCK_LIKE_CPP;
                break;
            }
            aura_types::SPELL_AURA_MOD_HIT_CHANCE => {
                if effect.calc_value <= -100 {
                    proc_entry.hit_mask = PROC_HIT_MISS_LIKE_CPP;
                }
                break;
            }
            aura_types::SPELL_AURA_PROC_TRIGGER_SPELL
            | aura_types::SPELL_AURA_PROC_TRIGGER_SPELL_WITH_VALUE => {
                triggers_spell = effect.trigger_spell != 0;
                break;
            }
            _ => {}
        }
    }

    if proc_entry.proc_flags[0] & PROC_FLAG_KILL_LIKE_CPP != 0 {
        proc_entry.attributes_mask |= PROC_ATTR_REQ_EXP_OR_HONOR_LIKE_CPP;
    }
    if add_trigger_flag {
        proc_entry.attributes_mask |= PROC_ATTR_TRIGGERED_CAN_PROC_LIKE_CPP;
    }

    if spell_info.attributes3 & attributes::SPELL_ATTR3_CAN_PROC_FROM_PROCS != 0
        && proc_entry.spell_family_mask.iter().all(|mask| *mask == 0)
        && proc_entry.chance >= 100.0
        && spell_info.proc_base_ppm <= 0.0
        && proc_entry.cooldown_ms == 0
        && proc_entry.charges == 0
        && proc_entry.proc_flags[0] & CAN_PROC_FROM_PROCS_UNRESTRICTED_DONE_FLAGS_LIKE_CPP != 0
        && triggers_spell
    {
        return None;
    }

    Some(proc_entry)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpellProcKeyLikeCpp {
    pub spell_id: u32,
    pub difficulty: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellProcLoadErrorKindLikeCpp {
    SpellMissing,
    AllRanksSpellNotRanked,
    AllRanksSpellNotFirstRank,
    DuplicateSpell,
    InvalidSchoolMask,
    NegativeChance,
    NegativeProcsPerMinute,
    MissingProcFlags,
    InvalidSpellTypeMask,
    SpellTypeMaskUnused,
    MissingSpellPhaseMask,
    InvalidSpellPhaseMask,
    SpellPhaseMaskUnused,
    InvalidHitMask,
    HitMaskUnused,
    DisabledEffectIsNotAura,
    ReqSpellmodWithoutSpellmodAura,
    InvalidAttributesMask,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcLoadErrorLikeCpp {
    pub spell_id: u32,
    pub difficulty: Option<u32>,
    pub effect_index: Option<u32>,
    pub kind: SpellProcLoadErrorKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpellProcLoadOutcomeLikeCpp {
    pub store: SpellProcStoreLikeCpp,
    pub loaded_row_count: usize,
    pub generated_entry_count: usize,
    pub errors: Vec<SpellProcLoadErrorLikeCpp>,
}

pub(super) fn apply_spell_proc_defaults_like_cpp(
    entry: &mut SpellProcEntryLikeCpp,
    spell_info: &SpellProcSourceSpellInfoLikeCpp,
) {
    if !entry.proc_flags_any_like_cpp() {
        entry.proc_flags = spell_info.proc_flags;
    }
    if entry.charges == 0 {
        entry.charges = spell_info.proc_charges;
    }
    if entry.chance == 0.0 && entry.procs_per_minute == 0.0 {
        entry.chance = spell_info.proc_chance;
    }
    if entry.cooldown_ms == 0 {
        entry.cooldown_ms = spell_info.proc_cooldown_ms;
    }
}

pub(super) fn validate_spell_proc_entry_like_cpp(
    entry: &mut SpellProcEntryLikeCpp,
    spell_info: &SpellProcSourceSpellInfoLikeCpp,
    errors: &mut Vec<SpellProcLoadErrorLikeCpp>,
) {
    let mut push_error = |kind, effect_index| {
        errors.push(SpellProcLoadErrorLikeCpp {
            spell_id: spell_info.spell_id,
            difficulty: Some(spell_info.difficulty),
            effect_index,
            kind,
        });
    };

    if entry.school_mask & !SPELL_SCHOOL_MASK_ALL_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidSchoolMask, None);
    }
    if entry.chance < 0.0 {
        push_error(SpellProcLoadErrorKindLikeCpp::NegativeChance, None);
        entry.chance = 0.0;
    }
    if entry.procs_per_minute < 0.0 {
        push_error(SpellProcLoadErrorKindLikeCpp::NegativeProcsPerMinute, None);
        entry.procs_per_minute = 0.0;
    }
    if !entry.proc_flags_any_like_cpp() {
        push_error(SpellProcLoadErrorKindLikeCpp::MissingProcFlags, None);
    }
    if entry.spell_type_mask & !PROC_SPELL_TYPE_MASK_ALL_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidSpellTypeMask, None);
    }
    if entry.spell_type_mask != 0 && entry.proc_flags[0] & SPELL_PROC_FLAG_MASK_LIKE_CPP == 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::SpellTypeMaskUnused, None);
    }
    if entry.spell_phase_mask == 0
        && entry.proc_flags[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP != 0
    {
        push_error(SpellProcLoadErrorKindLikeCpp::MissingSpellPhaseMask, None);
    }
    if entry.spell_phase_mask & !PROC_SPELL_PHASE_MASK_ALL_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidSpellPhaseMask, None);
    }
    if entry.spell_phase_mask != 0
        && entry.proc_flags[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP == 0
    {
        push_error(SpellProcLoadErrorKindLikeCpp::SpellPhaseMaskUnused, None);
    }
    if entry.spell_phase_mask == 0
        && entry.proc_flags[0] & REQ_SPELL_PHASE_PROC_FLAG_MASK_LIKE_CPP == 0
        && entry.proc_flags[1] & PROC_FLAG_2_CAST_SUCCESSFUL_LIKE_CPP != 0
    {
        entry.spell_phase_mask = PROC_SPELL_PHASE_CAST_LIKE_CPP;
    }
    if entry.hit_mask & !PROC_HIT_MASK_ALL_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidHitMask, None);
    }
    if entry.hit_mask != 0
        && !(entry.proc_flags[0] & TAKEN_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
            || (entry.proc_flags[0] & DONE_HIT_PROC_FLAG_MASK_LIKE_CPP != 0
                && (entry.spell_phase_mask == 0
                    || entry.spell_phase_mask
                        & (PROC_SPELL_PHASE_HIT_LIKE_CPP | PROC_SPELL_PHASE_FINISH_LIKE_CPP)
                        != 0)))
    {
        push_error(SpellProcLoadErrorKindLikeCpp::HitMaskUnused, None);
    }

    for effect in &spell_info.effects {
        if (entry.disable_effects_mask & (1u32 << effect.effect_index)) != 0
            && !effect.is_aura_like_cpp()
        {
            push_error(
                SpellProcLoadErrorKindLikeCpp::DisabledEffectIsNotAura,
                Some(effect.effect_index),
            );
        }
    }

    if entry.attributes_mask & PROC_ATTR_REQ_SPELLMOD_LIKE_CPP != 0
        && !spell_info.effects.iter().any(|effect| {
            effect.is_aura_like_cpp()
                && matches!(
                    effect.effect_aura,
                    aura_types::SPELL_AURA_ADD_PCT_MODIFIER
                        | aura_types::SPELL_AURA_ADD_FLAT_MODIFIER
                        | aura_types::SPELL_AURA_ADD_PCT_MODIFIER_BY_SPELL_LABEL
                        | aura_types::SPELL_AURA_IGNORE_SPELL_COOLDOWN
                )
        })
    {
        push_error(
            SpellProcLoadErrorKindLikeCpp::ReqSpellmodWithoutSpellmodAura,
            None,
        );
    }

    if entry.attributes_mask & !PROC_ATTR_ALL_ALLOWED_LIKE_CPP != 0 {
        push_error(SpellProcLoadErrorKindLikeCpp::InvalidAttributesMask, None);
        entry.attributes_mask &= PROC_ATTR_ALL_ALLOWED_LIKE_CPP;
    }
}

pub(super) fn infer_same_effect_stack_aura_types_like_cpp<SpellInfoById>(
    spell_ids: &BTreeSet<u32>,
    spell_info_by_id: &mut SpellInfoById,
) -> BTreeSet<i32>
where
    SpellInfoById: FnMut(u32) -> Option<SpellInfo>,
{
    let mut frequency = BTreeMap::<i32, usize>::new();
    let mut aura_order = Vec::<i32>::new();

    for spell_id in spell_ids {
        if let Some(spell_info) = spell_info_by_id(*spell_id) {
            for effect in spell_info.effects() {
                if !effect.is_aura_like_cpp() {
                    continue;
                }

                let aura_type = normalize_same_effect_subgroup_aura_like_cpp(effect.effect_aura);
                if !frequency.contains_key(&aura_type) {
                    aura_order.push(aura_type);
                }
                *frequency.entry(aura_type).or_default() += 1;
            }
        }
    }

    let mut selected_aura_type = 0;
    let mut selected_count = 0;
    for aura_type in aura_order {
        let current_count = frequency.get(&aura_type).copied().unwrap_or(0);
        if current_count > selected_count {
            selected_aura_type = aura_type;
            selected_count = current_count;
        }
    }

    if selected_aura_type == aura_types::SPELL_AURA_MOD_MELEE_HASTE {
        BTreeSet::from([
            aura_types::SPELL_AURA_MOD_MELEE_HASTE,
            aura_types::SPELL_AURA_MOD_MELEE_RANGED_HASTE,
            aura_types::SPELL_AURA_MOD_RANGED_HASTE,
        ])
    } else {
        BTreeSet::from([selected_aura_type])
    }
}

pub(super) fn normalize_same_effect_subgroup_aura_like_cpp(aura_type: i32) -> i32 {
    if matches!(
        aura_type,
        aura_types::SPELL_AURA_MOD_MELEE_HASTE
            | aura_types::SPELL_AURA_MOD_MELEE_RANGED_HASTE
            | aura_types::SPELL_AURA_MOD_RANGED_HASTE
    ) {
        aura_types::SPELL_AURA_MOD_MELEE_HASTE
    } else {
        aura_type
    }
}

pub(super) fn spell_rank_chain_has_any_aura_like_cpp<SpellInfoById, NextRankSpell>(
    spell_id: u32,
    aura_types: &BTreeSet<i32>,
    spell_info_by_id: &mut SpellInfoById,
    next_rank_spell: &mut NextRankSpell,
) -> bool
where
    SpellInfoById: FnMut(u32) -> Option<SpellInfo>,
    NextRankSpell: FnMut(u32) -> Option<u32>,
{
    let mut current_spell_id = Some(spell_id);
    let mut seen = BTreeSet::new();

    while let Some(spell_id) = current_spell_id {
        if !seen.insert(spell_id) {
            break;
        }

        let Some(spell_info) = spell_info_by_id(spell_id) else {
            return false;
        };

        if aura_types
            .iter()
            .any(|aura_type| spell_info.has_aura_like_cpp(*aura_type))
        {
            return true;
        }

        current_spell_id = next_rank_spell(spell_id);
    }

    false
}

pub(super) fn calculate_pct_i32_like_cpp(base: i32, pct: f32) -> i32 {
    ((base as f32) * pct / 100.0) as i32
}

pub(super) const fn spell_effect_accepts_implicit_target_conditions_like_cpp(effect: u32) -> bool {
    use spell_effect_types::*;
    matches!(
        effect,
        SPELL_EFFECT_PERSISTENT_AREA_AURA
            | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
            | SPELL_EFFECT_APPLY_AREA_AURA_RAID
            | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
            | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
            | SPELL_EFFECT_APPLY_AREA_AURA_PET
            | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
            | SPELL_EFFECT_APPLY_AURA_ON_PET
            | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
            | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
    )
}

pub(super) const fn implicit_target_category_accepts_conditions_like_cpp(target: u32) -> bool {
    matches!(
        target,
        2 | 3
            | 4
            | 7
            | 8
            | 15
            | 16
            | 20
            | 24
            | 30
            | 31
            | 33
            | 34
            | 37
            | 38
            | 40
            | 46
            | 51
            | 52
            | 54
            | 56
            | 58
            | 59
            | 60
            | 61
            | 89
            | 93
            | 104
            | 105
            | 107
            | 108
            | 109
            | 110
            | 115
            | 116
            | 118
            | 119
            | 120
            | 122
            | 123
            | 128
            | 129
            | 130
            | 133
            | 134
            | 135
            | 136
            | 142
            | 151
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SpellHitCategoriesRowLikeCpp {
    pub(super) record_id: u32,
    pub(super) category_id: u32,
    pub(super) charge_category_id: u32,
    pub(super) defense_type: i8,
    pub(super) spell_mechanic: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SpellHitMiscRowLikeCpp {
    pub(super) record_id: u32,
    pub(super) school_mask: u8,
}
