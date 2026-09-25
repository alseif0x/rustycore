// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature spell publication: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp;
use super::creature_melee_spell_miss_threshold_3_3_5_like_cpp;
use super::{CreatureSpellCastPlanLikeCpp, CreatureSpellTargetHitResultLikeCpp, ObjectGuid};
use super::{Position, PowerType, RecipientRule, RuntimeEvent, RuntimePlan, SpellTargetData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum CreatureSpellHitProfileLikeCpp {
    NoAttackMissAfterRequiredRoll,
    BaseMeleeMiss {
        miss_threshold_per_ten_thousand: u32,
    },
}

pub(in crate::session) fn represented_creature_spell_hit_profile_like_cpp(
    metadata: &wow_data::spell::SpellHitMetadataLikeCpp,
    active_effect_indices: &[u32],
    attributes: [u32; 15],
) -> Option<CreatureSpellHitProfileLikeCpp> {
    const SPELL_DAMAGE_CLASS_MELEE_LIKE_CPP: i8 = 2;
    const SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP: u8 = 0x01;
    const SPELL_ATTR0_IS_ABILITY_LIKE_CPP: u32 = 0x0000_0010;
    const SPELL_ATTR3_NO_AVOIDANCE_LIKE_CPP: u32 = 0x0000_0040;
    const SPELL_ATTR3_ALWAYS_HIT_LIKE_CPP: u32 = 0x0004_0000;
    const SPELL_ATTR7_ALLOW_SPELL_REFLECTION_LIKE_CPP: u32 = 0x0000_0001;
    const SPELL_ATTR7_NO_ATTACK_MISS_LIKE_CPP: u32 = 0x0200_0000;

    if metadata.defense_type != SPELL_DAMAGE_CLASS_MELEE_LIKE_CPP
        || metadata.school_mask != SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP
        || metadata.spell_mechanic != 0
        || active_effect_indices.is_empty()
        || active_effect_indices
            .iter()
            .any(|effect_index| metadata.effect_mechanics.get(effect_index).copied() != Some(0))
        || attributes[0] & SPELL_ATTR0_IS_ABILITY_LIKE_CPP == 0
        || attributes[3] & (SPELL_ATTR3_NO_AVOIDANCE_LIKE_CPP | SPELL_ATTR3_ALWAYS_HIT_LIKE_CPP)
            != 0
        || attributes[7] & SPELL_ATTR7_ALLOW_SPELL_REFLECTION_LIKE_CPP != 0
    {
        return None;
    }

    Some(
        if attributes[7] & SPELL_ATTR7_NO_ATTACK_MISS_LIKE_CPP != 0 {
            CreatureSpellHitProfileLikeCpp::NoAttackMissAfterRequiredRoll
        } else {
            CreatureSpellHitProfileLikeCpp::BaseMeleeMiss {
                miss_threshold_per_ten_thousand: creature_melee_spell_miss_threshold_3_3_5_like_cpp(
                ),
            }
        },
    )
}

pub(in crate::session) fn resolve_creature_spell_hit_profile_like_cpp(
    profile: CreatureSpellHitProfileLikeCpp,
    roll: Option<u32>,
) -> Option<CreatureSpellTargetHitResultLikeCpp> {
    // C++ `Unit::MeleeSpellHitResult` draws `urand(0, 9999)` before applying
    // NO_ATTACK_MISS to the miss-chance bucket. Both currently represented
    // profiles therefore require exactly one authoritative draw.
    let roll = roll.filter(|roll| *roll <= 9_999)?;
    match profile {
        CreatureSpellHitProfileLikeCpp::NoAttackMissAfterRequiredRoll => {
            Some(CreatureSpellTargetHitResultLikeCpp::Hit)
        }
        CreatureSpellHitProfileLikeCpp::BaseMeleeMiss {
            miss_threshold_per_ten_thousand,
        } => Some(if roll < miss_threshold_per_ten_thousand {
            CreatureSpellTargetHitResultLikeCpp::Miss
        } else {
            CreatureSpellTargetHitResultLikeCpp::Hit
        }),
    }
}

pub(in crate::session) fn append_committed_creature_spell_packets_like_cpp(
    plan: &mut RuntimePlan,
    command: &CreatureSpellCastPlanLikeCpp,
    cast_id: ObjectGuid,
    hit_result: CreatureSpellTargetHitResultLikeCpp,
    source_position: Position,
    visibility_range: f32,
    full_log_data: &wow_packet::packets::spell::SpellCastLogData,
) {
    use wow_packet::ServerPacket;
    use wow_packet::packets::spell::{
        SpellCastVisual, SpellGoPkt, SpellMissReason, SpellMissTarget, SpellStartPkt,
        SpellTargetData,
    };

    let visual = SpellCastVisual {
        spell_visual_id: command.spell_x_spell_visual_id,
        script_visual_id: 0,
    };
    let target = SpellTargetData {
        flags: 0x2,
        unit: command.target_guid,
        item: ObjectGuid::EMPTY,
        ..Default::default()
    };
    let start = SpellStartPkt {
        cast_data: Default::default(),
        caster: command.caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: command.spell_id,
        visual: visual.clone(),
        cast_flags: 0x0000_0002,
        cast_flags_ex: 0,
        cast_time_ms: command.cast_time_ms,
        target: target.clone(),
    };
    let (hit_targets, miss_targets) = match hit_result {
        CreatureSpellTargetHitResultLikeCpp::Hit => (vec![command.target_guid], Vec::new()),
        CreatureSpellTargetHitResultLikeCpp::Miss => (
            Vec::new(),
            vec![SpellMissTarget::new(
                command.target_guid,
                SpellMissReason::Miss,
            )],
        ),
    };
    let go = SpellGoPkt {
        cast_data: Default::default(),
        caster: command.caster_guid,
        cast_id,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: command.spell_id,
        visual,
        cast_flags: command.spell_go_cast_flags,
        cast_flags_ex: 0,
        cast_time_ms: crate::session::game_time_ms_like_cpp(),
        target,
        hit_targets,
        miss_targets,
    };
    plan.events.push(RuntimeEvent {
        source_guid: command.caster_guid,
        recipients: RecipientRule::NearbyVisibleDurableSpellCast {
            source_guid: command.caster_guid,
            map_id: command.map_id,
            instance_id: command.instance_id,
            source_position,
            range: visibility_range,
            required_3d: false,
            basic_go_packet_bytes: go.to_bytes(),
            full_go_packet_bytes: go.to_full_log_bytes_like_cpp(full_log_data),
        },
        packet_bytes: start.to_bytes(),
    });
}

pub(in crate::session) fn creature_spell_cast_log_data_like_cpp(
    caster: &wow_entities::Creature,
    spell: &wow_data::SpellInfo,
    difficulty_id: u8,
) -> Option<wow_packet::packets::spell::SpellCastLogData> {
    use wow_packet::packets::spell::{SpellCastLogData, SpellLogPowerData};

    // The enclosing M2.6 cast path admits only base-difficulty spells and zero
    // effective costs. Keep the cost/aura portions independently fail-closed
    // so later callers cannot fabricate a complete log snapshot.
    if difficulty_id != 0
        || creature_ai_spell_has_unrepresented_nonzero_power_cost_like_cpp(spell)
        || !caster
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_cast_log_aura_authority_like_cpp()
    {
        return None;
    }

    // C++ `SpellInfo::CalcPowerCost` retains one row per power type even when
    // its effective amount is zero. That includes signed enum sentinels such
    // as `POWER_ALL=127`: the unknown-power rejection is reached only by
    // cost-bearing branches (for example percentage or use-all-power), all of
    // which M2.6 rejects before this helper. Preserve the remaining zero rows
    // in DB2 order and deduplicate them by their raw signed type.
    let mut power_data = Vec::new();
    for power in &spell.power_costs {
        let power_type = i32::from(power.power_type);
        if power_data
            .iter()
            .any(|known: &SpellLogPowerData| known.power_type == power_type)
        {
            continue;
        }
        // C++ retains the signed raw enum value in the wire row. Values that
        // cannot address its Unit power array keep an amount of zero without
        // changing or dropping the row (including HEALTH=-2 and POWER_ALL).
        let amount = <PowerType as num_traits::FromPrimitive>::from_i8(power.power_type)
            .map(|represented_power| caster.unit().get_power(represented_power))
            .unwrap_or(0);
        power_data.push(SpellLogPowerData {
            power_type,
            amount,
            cost: 0,
        });
    }

    let primary_power = caster.power_type();
    let primary_power_type = primary_power as i32;
    if !power_data
        .iter()
        .any(|power| power.power_type == primary_power_type)
    {
        power_data.insert(
            0,
            SpellLogPowerData {
                power_type: primary_power_type,
                amount: caster.unit().get_power(primary_power),
                cost: 0,
            },
        );
    }

    let stats = caster.combat_log_stats_like_cpp();
    Some(SpellCastLogData {
        // Mirrors the C++ `uint64 GetHealth()` assignment to the signed wire
        // field. DB-backed creature health originates in a u32 and always fits.
        health: caster.unit().data().health as i64,
        attack_power: caster.combat_log_attack_power_like_cpp(),
        spell_power: stats.spell_power,
        armor: stats.armor,
        power_data,
    })
}
