// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented effects that change power, energy and spell charges, and the
//! spell execute log (#1363 slice 9c), from the World
//! `session/spell_effects/effects_power.rs`.

use super::{
    SpellEffectHostLikeCpp, forward_assisting_threat_like_cpp,
    represented_spell_damage_bonus_done_like_cpp,
};
use wow_constants::unit::PowerType;
use wow_core::ObjectGuid;
use wow_entities::MAX_POWERS;
use wow_world_core::session::power_type_from_u8_like_cpp as party_member_power_kind_from_u8_like_cpp;

/// C++ `Spell::EffectEnergize` (`SpellEffects.cpp:1488-1530`) /
/// `Spell::EffectEnergizePct` (`SpellEffects.cpp:1532-1554`).
///
/// The `EffectEnergize` branch applies the level-dependent overrides and the
/// Runic Mana Injector engineering bonus before `Unit::EnergizeBySpell`,
/// which interrupts the regeneration of a `PowerTypeFlags::UseRegenInterrupt`
/// power (`Player::InterruptPowerRegen`, `Unit.cpp:6581-6585`), applies the
/// power and then forwards `damage / 2` assisting threat with
/// `ignoreModifiers = true` (`Unit.cpp:6578-6590`).
///
/// Represented boundary: current canonical player target only. The
/// caster-side level and skill are read from the session Player, so a
/// non-player caster keeps the unmodified effect amount.
pub fn apply_energize_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    damage: i32,
    misc_value: i32,
    target_guid: ObjectGuid,
    percent: bool,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    if target_guid != player_guid
        || host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true)
    {
        return false;
    }
    if misc_value < 0 || misc_value >= MAX_POWERS as i32 {
        return false;
    }
    let Ok(power_id) = u8::try_from(misc_value) else {
        return false;
    };
    let power = party_member_power_kind_from_u8_like_cpp(power_id);
    let damage = if percent {
        damage
    } else {
        energize_caster_scaled_amount_like_cpp(host, spell_id, caster_guid, damage)
    };
    // C++ `Unit::EnergizeBySpell` (`Unit.cpp:6581-6585`) interrupts the
    // target Player's regeneration before `ModifyPower` for a power whose
    // DB2 entry carries `PowerTypeFlags::UseRegenInterrupt`.
    if host
        .hub_ref()
        .catalogs
        .power_type_store_like_cpp()
        .is_some_and(|store| store.uses_regen_interrupt_like_cpp(power as i8))
    {
        interrupt_player_power_regen_like_cpp(host, power, misc_value);
    }

    let outcome = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            let max_power = player.get_max_power(power);
            if max_power <= 0 {
                return None;
            }
            let requested = if percent {
                ((i64::from(max_power) * i64::from(damage)) / 100)
                    .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
            } else {
                damage
            };
            let current = player.get_power(power);
            let after = current.saturating_add(requested).clamp(0, max_power);
            player.unit_mut().set_power(power, after);
            Some((requested, after - current))
        })
        .flatten();
    let Some((requested, applied)) = outcome else {
        return false;
    };
    // C++ `Unit::EnergizeBySpell` (`Unit.cpp:6586`) forwards `damage / 2`
    // assisting threat before the log, with `ignoreModifiers = true`.
    forward_assisting_threat_like_cpp(
        host,
        Some(spell_id),
        caster_guid,
        target_guid,
        requested as f32 / 2.0,
        true,
    );
    // C++ `Unit::EnergizeBySpell` (`Unit.cpp:6578-6590`): `gain` is the
    // delta `ModifyPower` actually applied and `OverEnergize` is what the
    // pool could not take.
    host.send_packet(&wow_packet::packets::combat::SpellEnergizeLog {
        target: target_guid,
        caster: caster_guid,
        spell_id,
        power_type: misc_value,
        amount: applied,
        over_energize: requested.saturating_sub(applied),
    });
    true
}

/// C++ `Player::InterruptPowerRegen` (`Player.cpp:1831-1840`): reset the
/// canonical Player's regen interrupt timestamp and fractional power, then
/// publish `SMSG_INTERRUPT_POWER_REGEN` with the `Powers` value.
fn interrupt_player_power_regen_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    power: PowerType,
    power_type: i32,
) {
    let now_ms = wow_world_core::session::game_time_ms_like_cpp();
    let _ = host.core().mutate_canonical_player_like_cpp(|player| {
        player
            .unit_mut()
            .interrupt_power_regen_like_cpp(power, now_ms);
    });
    host.send_packet(&wow_packet::packets::combat::InterruptPowerRegen { power_type });
}

/// C++ `Spell::EffectEnergize`'s caster-scaled amount
/// (`SpellEffects.cpp:1507-1527`).
///
/// The switch runs before `Unit::EnergizeBySpell`, so it uses the caster's
/// level and, for the Runic Mana Injector, the caster Player's engineering
/// skill. `AddPct(damage, 25)` is `damage += int32(damage * 25 / 100.0f)`.
fn energize_caster_scaled_amount_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &H,
    spell_id: i32,
    caster_guid: ObjectGuid,
    damage: i32,
) -> i32 {
    let caster_is_player = host.player_guid() == Some(caster_guid);
    let caster_level = if caster_is_player {
        host.hub_ref().player_level_like_cpp()
    } else {
        0
    };
    let mut damage = match spell_id {
        // Blood Fury: `damage -= 10 * max(0, min(30, level - 60))`.
        24_571 => damage - 10 * i32::from(caster_level.saturating_sub(60).min(30)),
        // Burst of Energy: `damage -= 4 * max(0, min(15, level - 60))`.
        24_532 => damage - 4 * i32::from(caster_level.saturating_sub(60).min(15)),
        _ => damage,
    };
    // Runic Mana Injector: engineers gain 25% more.
    if spell_id == 67_490
        && caster_is_player
        && host
            .hub_ref()
            .resolved_player_skill_value_like_cpp(wow_entities::SKILL_ENGINEERING_LIKE_CPP)
            .is_some_and(|value| value != 0)
    {
        damage += (damage as f32 * 25.0 / 100.0) as i32;
    }
    damage
}

/// C++ `Spell::GetExecuteLogEffect` (`Spell.cpp:5062-5074`): the cast's log
/// entry for one `SpellEffectName`, created on first use.
fn represented_spell_execute_log_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    effect: i32,
) -> &mut wow_packet::packets::combat::SpellLogEffect {
    host.split_spell_state_mut()
        .0
        .represented_spell_execute_log_effect_like_cpp(effect)
}

/// C++ `Spell::ExecuteLogEffectTakeTargetPower` (`Spell.cpp:5076-5086`).
pub fn record_spell_execute_log_take_target_power_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    effect: i32,
    victim: ObjectGuid,
    points: u32,
    power_type: u32,
    amplitude: f32,
) {
    represented_spell_execute_log_effect_like_cpp(host, effect)
        .power_drain_targets
        .push(
            wow_packet::packets::combat::SpellLogEffectPowerDrainParams {
                victim,
                points,
                power_type,
                amplitude,
            },
        );
}

/// C++ `Spell::ExecuteLogEffectExtraAttacks` (`Spell.cpp:5088-5095`).
pub fn record_spell_execute_log_extra_attacks_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    effect: i32,
    victim: ObjectGuid,
    num_attacks: u32,
) {
    represented_spell_execute_log_effect_like_cpp(host, effect)
        .extra_attacks_targets
        .push(
            wow_packet::packets::combat::SpellLogEffectExtraAttacksParams {
                victim,
                num_attacks,
            },
        );
}

/// C++ `Spell::ExecuteLogEffectDurabilityDamage` (`Spell.cpp:5108-5116`).
pub fn record_spell_execute_log_durability_damage_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    effect: i32,
    victim: ObjectGuid,
    item_id: i32,
    amount: i32,
) {
    represented_spell_execute_log_effect_like_cpp(host, effect)
        .durability_damage_targets
        .push(
            wow_packet::packets::combat::SpellLogEffectDurabilityDamageParams {
                victim,
                item_id,
                amount,
            },
        );
}

/// C++ `Spell::SendSpellExecuteLog` (`Spell.cpp:5048-5060`), called from
/// `FinishTargetProcessing` (`Spell.cpp:8493-8496`) once every effect has
/// resolved. The cast's accumulator is taken, so a later cast starts clean.
pub fn send_spell_execute_log_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    caster_guid: ObjectGuid,
) {
    if !host
        .spell_state()
        .has_represented_spell_execute_log_effects_like_cpp()
    {
        return;
    }
    let effects = host
        .split_spell_state_mut()
        .0
        .take_represented_spell_execute_log_effects_like_cpp();
    host.send_packet(&wow_packet::packets::combat::SpellExecuteLog {
        caster: caster_guid,
        spell_id,
        effects,
    });
}

/// C++ `Spell::EffectPowerDrain`/`EffectPowerBurn` pre-scale the effect
/// amount with `Unit::SpellDamageBonusDone(..., SPELL_DIRECT_DAMAGE, ...)`
/// (`SpellEffects.cpp:1082-1088`) before draining or burning. The
/// `damage < 0` gate runs before that call, so a negative base keeps its
/// value for the effect's own refusal.
///
/// C++ passes `SPELL_DIRECT_DAMAGE` to `SpellDamageBonusTaken`, whose first
/// guard returns the amount unchanged for direct damage
/// (`Unit.cpp:6775-6777`). The taken-side aura terms used by periodic spell
/// damage therefore do not apply to power drain or power burn.
pub fn power_drain_pre_scaled_damage_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    effect_index: u32,
    caster_guid: ObjectGuid,
    target_guid: ObjectGuid,
    coefficient: f32,
    coefficient_from_ap: f32,
    base_damage: i32,
) -> i32 {
    if base_damage < 0 {
        return base_damage;
    }
    i32::try_from(represented_spell_damage_bonus_done_like_cpp(
        host,
        spell_id,
        effect_index,
        caster_guid,
        target_guid,
        coefficient,
        coefficient_from_ap,
        u32::try_from(base_damage).unwrap_or(0),
    ))
    .unwrap_or(i32::MAX)
}

/// C++ `Spell::EffectPowerDrain` / `Spell::EffectPowerBurn`.
///
/// A creature target drains the pool of the representation it was admitted
/// through and restores `drained * CalcValueMultiplier` to the caster
/// through the represented `EnergizeBySpell` (`SpellEffects.cpp:1094-1100`);
/// the canonical player target keeps C++'s host-drain rule, where no gain is
/// restored. `EffectPowerBurn` applies the drained amount scaled by
/// `SpellEffectInfo::CalcValueMultiplier` (`SpellEffects.cpp:1157-1164`),
/// and both effects publish `ExecuteLogEffectTakeTargetPower`
/// (`SpellEffects.cpp:1101`, `1160`).
///
/// The creature branch drains through the gated owner root
/// (`mutate_world_creature`), and the burn damage that follows is applied by
/// the same gated root, so both write the one admitted representation: C++
/// accumulates the scaled damage into the spell's damage pipeline
/// (`SpellEffects.cpp:1157-1164`) and the represented chain applies it to the
/// creature target, which would otherwise replace the canonical entity with
/// the legacy snapshot and discard the drained pool. A refused drain still
/// publishes nothing, and the drained pool is published as a unit data field.
///
/// Represented boundary: a creature power change reaches observers only as
/// the values update sent here, since the represented creature update-object
/// path owns that publication.
pub async fn apply_power_drain_effect_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawns: &wow_world_entities::CreatureSpawnCatalogsLikeCpp,
    spell_id: i32,
    effect: u32,
    damage: i32,
    misc_value: i32,
    target_guid: ObjectGuid,
    burn_damage: bool,
    value_multiplier: f32,
    cast_id: ObjectGuid,
    spell_visual_id: u32,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    if damage < 0 || (target_guid != player_guid && !target_guid.is_creature()) {
        return false;
    }
    if target_guid == player_guid
        && host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true)
    {
        return false;
    }
    if misc_value < 0 || misc_value >= MAX_POWERS as i32 {
        return false;
    }
    let Ok(power_id) = u8::try_from(misc_value) else {
        return false;
    };
    let power = party_member_power_kind_from_u8_like_cpp(power_id);

    // C++ accepts any living target whose `GetPowerType()` matches the
    // effect (`SpellEffects.cpp:1078`, `1151`). A creature target drains and
    // burns through the gated owner root, so the drained pool and the damage
    // that follows are applied to the same admitted representation instead of
    // the canonical snapshot replacement discarding the drain.
    if target_guid.is_creature() {
        let drained = host
            .core_mut()
            .mutate_world_creature(target_guid, |world_creature| {
                let creature = &mut *world_creature;
                if !creature.is_alive()
                    || party_member_power_kind_from_u8_like_cpp(
                        creature.unit().data().display_power,
                    ) != power
                {
                    return None;
                }
                let current = creature.unit().get_power(power).max(0);
                let drain = current.min(damage);
                creature.unit_mut().set_power(power, current - drain);
                Some((drain, creature.unit().values_update()))
            })
            .flatten();
        let Some((drained, values_update)) = drained else {
            return false;
        };
        // The drained pool is a unit data field, published the same way the
        // represented creature heal publishes its health change.
        if drained > 0
            && host
                .core()
                .client_visible_guids_like_cpp
                .contains(&target_guid)
            && let Some(update) = host.represented_unit_values_update_to_update_object_like_cpp(
                target_guid,
                host.core().player_map_id_like_cpp(),
                &values_update,
            )
        {
            host.send_packet(&update);
        }
        record_spell_execute_log_take_target_power_like_cpp(
            host,
            i32::try_from(effect).unwrap_or(0),
            target_guid,
            u32::try_from(drained).unwrap_or(u32::MAX),
            u32::try_from(misc_value).unwrap_or(0),
            value_multiplier,
        );
        if burn_damage {
            // C++ `newDamage = int32(newDamage * dmgMultiplier)` and
            // `m_damage += newDamage` (`SpellEffects.cpp:1157-1164`); the
            // represented creature damage path applies that amount with the
            // cast identity the combat log needs.
            let burned = (drained as f32 * value_multiplier) as i32;
            if burned > 0 {
                let _ = host
                    .apply_damage_from_caster_like_cpp(
                        item_guid_generator,
                        Some(spell_id),
                        player_guid,
                        target_guid,
                        u32::try_from(burned).unwrap_or(u32::MAX),
                        cast_id,
                        spell_visual_id,
                    )
                    .await;
            }
            return true;
        }
        // C++ `unitCaster->EnergizeBySpell(unitCaster, m_spellInfo, gain,
        // powerType)` restores the caster's share unconditionally when the
        // caster is not the target (`SpellEffects.cpp:1094-1100`), so a
        // zero drain still publishes its zeroed energize log; the
        // represented energize path owns that log, the assisting threat and
        // the regen interrupt.
        let gain = (drained as f32 * value_multiplier) as i32;
        apply_energize_effect_like_cpp(
            host,
            spell_id,
            player_guid,
            gain,
            misc_value,
            player_guid,
            false,
        );
        return true;
    }

    let drained = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            if party_member_power_kind_from_u8_like_cpp(player.unit().data().display_power) != power
            {
                return None;
            }
            let current = player.get_power(power).max(0);
            let drain = current.min(damage);
            player.unit_mut().set_power(power, current - drain);
            Some(drain)
        })
        .flatten();
    let Some(drained) = drained else {
        return false;
    };

    // C++ logs the drained power before the burn multiplier
    // (`SpellEffects.cpp:1160`), and with `gainMultiplier` for drain; an
    // empty pool logs `points 0` rather than skipping the row.
    record_spell_execute_log_take_target_power_like_cpp(
        host,
        i32::try_from(effect).unwrap_or(0),
        target_guid,
        u32::try_from(drained).unwrap_or(u32::MAX),
        u32::try_from(misc_value).unwrap_or(0),
        if burn_damage { 0.0 } else { value_multiplier },
    );

    if burn_damage {
        // C++ `newDamage = int32(newDamage * dmgMultiplier)`
        // (`SpellEffects.cpp:1162`); the represented target is the player,
        // so this stays the owned-player damage path.
        let burned = (drained as f32 * value_multiplier) as i32;
        if burned > 0 {
            if crate::SpellClickHostLikeCpp::apply_native_player_damage_like_cpp(
                host,
                u32::try_from(burned).unwrap_or(u32::MAX),
                wow_entities::PlayerDeathOperationContext::new(player_guid, true),
                creature_spawns,
                item_guid_generator,
            )
            .await
            .is_err()
            {
                return false;
            }
            host.sync_player_registry_state_like_cpp();
        }
    }

    // The represented caster is always the victim here, so C++'s
    // `unitCaster != unitTarget` gain (`EnergizeBySpell`,
    // `SpellEffects.cpp:1094-1100`) has no producer yet. `spell_id` keeps
    // the call site's cast identity for that future branch.
    let _ = spell_id;

    drained > 0
}
