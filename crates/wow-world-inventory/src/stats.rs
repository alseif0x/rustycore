// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_data::{PlayerSpellBonusInputLikeCpp, PlayerStatSystemProjectionLikeCpp};
use wow_entities::PlayerEffectiveCombatStatsLikeCpp;
use wow_world_core::session::state::hub_support::{
    RepresentedPlayerGearStatsLikeCpp, SPELL_SCHOOL_MASK_ALL_LIKE_CPP,
    SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP, SPELL_SCHOOL_MASK_SPELL_LIKE_CPP,
};
use wow_world_core::session::{HubMut, HubRef, PlayerStatsAccessLikeCpp};

/// C++ `CombatRating::CR_HIT_MELEE` (`Unit.h:310`).
pub const CR_HIT_MELEE_LIKE_CPP: u8 = 5;

impl crate::InventoryState {
    /// C++ `Player::UpdateSpellDamageAndHealingBonus` producers
    /// (`StatSystem.cpp:171-197`) from `Unit::SpellBaseDamageBonusDone`
    /// (`Unit.cpp:6860-6890`) and `Unit::SpellBaseHealingBonusDone`
    /// (`Unit.cpp:7282-7315`).
    ///
    /// `GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE, mask)` keeps
    /// the full per-school sum (negative amounts included) while
    /// `ModDamageDoneNeg` accumulates only the negative part, so the pure
    /// stat system can reproduce C++'s `Pos = bonus - Neg`.
    pub fn represented_spell_bonus_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> PlayerSpellBonusInputLikeCpp {
        let mut damage_done_flat = [0i32; 7];
        let mut damage_done_neg = [0i32; 7];
        for (misc_value, amount) in access
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
            )
            .unwrap_or_default()
        {
            for (school, flat) in damage_done_flat.iter_mut().enumerate().skip(1) {
                if misc_value & (1_i32 << school) == 0 {
                    continue;
                }
                *flat = flat.saturating_add(amount);
                if amount < 0 {
                    damage_done_neg[school] = damage_done_neg[school].saturating_add(amount);
                }
            }
        }

        let mut damage_of_stat_percent = [[0i32; 5]; 7];
        for (school_mask, stat_index, amount) in access
            .resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT,
            )
            .unwrap_or_default()
        {
            let Ok(stat) = usize::try_from(stat_index) else {
                continue;
            };
            for (school, per_school) in damage_of_stat_percent.iter_mut().enumerate().skip(1) {
                if school_mask & (1_i32 << school) == 0 {
                    continue;
                }
                if let Some(value) = per_school.get_mut(stat) {
                    *value = value.saturating_add(amount);
                }
            }
        }

        let healing_done_flat = access
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE,
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|(misc_value, _)| {
                *misc_value == 0 || (*misc_value & SPELL_SCHOOL_MASK_ALL_LIKE_CPP) != 0
            })
            .map(|(_, amount)| amount)
            .sum::<i32>();

        let mut healing_of_stat_percent = [0i32; 5];
        for (stat_index, _, amount) in access
            .resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT,
            )
            .unwrap_or_default()
        {
            if let Ok(stat) = usize::try_from(stat_index)
                && let Some(value) = healing_of_stat_percent.get_mut(stat)
            {
                *value = value.saturating_add(amount);
            }
        }

        // C++ `AuraEffect::HandleModDamagePercentDone`
        // (`SpellAuraEffects.cpp:4525-4548`) sets `ModDamageDonePercent[i]` to
        // `GetTotalAuraMultiplierByMiscMask(SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
        // 1 << i)` for every school the handling effect intersects; a school
        // with no such effect keeps the 1.0 create value.
        let damage_percent_effects = access
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            )
            .unwrap_or_default();
        let mut damage_done_percent = [1.0_f32; 7];
        for (school, percent) in damage_done_percent.iter_mut().enumerate() {
            let mask = 1_i32 << school;
            if !damage_percent_effects
                .iter()
                .any(|(misc_value, _)| misc_value & mask != 0)
            {
                continue;
            }
            *percent = damage_percent_effects
                .iter()
                .filter(|(misc_value, _)| misc_value & mask != 0)
                .fold(1.0_f32, |acc, (_, amount)| {
                    acc * (1.0 + *amount as f32 / 100.0)
                });
        }

        // C++ `Player::UpdateHealingDonePercentMod` (`StatSystem.cpp:588-599`)
        // multiplies `1 + amount/100` over every active
        // `SPELL_AURA_MOD_HEALING_DONE_PERCENT` (136) effect and clamps the
        // published `ModHealingDonePercent` at zero.
        let healing_done_percent = access
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE_PERCENT,
            )
            .unwrap_or_default()
            .into_iter()
            .fold(1.0_f32, |acc, (_, amount)| {
                acc * (1.0 + amount as f32 / 100.0)
            })
            .max(0.0);

        // C++ `AuraEffect::HandleModVersatilityByPct`
        // (`SpellAuraEffects.cpp:3797-3808`) sums every active
        // `SPELL_AURA_MOD_VERSATILITY` (471) amount into `VersatilityBonus`
        // through `SetUpdateFieldStatValue` (clamped at zero).
        let versatility_bonus_aura = access
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_VERSATILITY,
            )
            .unwrap_or_default()
            .into_iter()
            .map(|(_, amount)| amount)
            .sum::<i32>();

        // C++ `AuraEffect::HandleModTargetResistance`
        // (`SpellAuraEffects.cpp:3507-3530`) adds an effect covering
        // `SPELL_SCHOOL_MASK_NORMAL` to `ModTargetPhysicalResistance` and one
        // covering the whole `SPELL_SCHOOL_MASK_SPELL` to `ModTargetResistance`;
        // `Player::ApplySpellPenetrationBonus` (`StatSystem.cpp:231-235`)
        // subtracts the item/enchant `ITEM_MOD_SPELL_PENETRATION` there.
        let mut target_resistance_aura = 0i32;
        let mut target_physical_resistance_aura = 0i32;
        for (misc_value, amount) in access
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_TARGET_RESISTANCE,
            )
            .unwrap_or_default()
        {
            if misc_value & SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP != 0 {
                target_physical_resistance_aura =
                    target_physical_resistance_aura.saturating_add(amount);
            }
            if misc_value & SPELL_SCHOOL_MASK_SPELL_LIKE_CPP == SPELL_SCHOOL_MASK_SPELL_LIKE_CPP {
                target_resistance_aura = target_resistance_aura.saturating_add(amount);
            }
        }

        let override_effects = access
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT,
            )
            .unwrap_or_default();
        // C++ `Player::ApplySpellPowerBonus` returns early while the override
        // aura is present, so the item accumulator never reaches
        // `m_baseSpellPower`.
        let base_spell_power = if override_effects.is_empty() {
            gear.spell_power
        } else {
            0
        };
        let override_spell_power_by_ap_pct = override_effects
            .into_iter()
            .map(|(_, amount)| amount as f32)
            .sum::<f32>();

        PlayerSpellBonusInputLikeCpp {
            base_spell_power,
            damage_done_flat,
            damage_done_neg,
            damage_of_stat_percent,
            healing_done_flat,
            healing_of_stat_percent,
            override_spell_power_by_ap_pct,
            damage_done_percent,
            healing_done_percent,
            versatility_bonus_aura,
            target_resistance_aura,
            item_spell_penetration: gear.spell_penetration_bonus,
            target_physical_resistance_aura,
            weapon_damage_pct: self.represented_weapon_damage_pct_like_cpp(access),
            weapon_damage_flat: self.represented_weapon_damage_flat_like_cpp(access),
        }
    }

    /// Publish one complete `UpdateAllStats` projection on the canonical
    /// Player. Packet adapters may format this value, while combat systems can
    /// consume it without reaching through Session or rebuilding item input.
    /// The snapshot is derived and is intentionally not a persistence record.
    pub fn publish_player_effective_combat_stats_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
        level: u8,
        projection: PlayerStatSystemProjectionLikeCpp,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) {
        let mut resistances = gear.resistances;
        // Physical resistance is the final armor value after agility and flat
        // armor; the six magic schools come from `Unit::UpdateResistances`
        // (item `BASE_VALUE` plus the resistance aura producers).
        resistances[0] = projection.armor;
        for (index, value) in access
            .represented_school_resistances_like_cpp(gear)
            .into_iter()
            .enumerate()
        {
            resistances[index + 1] = value;
        }
        let weapon_damage = wow_data::player::effective_weapon_damage_ranges_like_cpp(
            projection,
            gear.weapon_damage,
            gear.base_attack_time,
            access.represented_shapeshift_combat_round_time_like_cpp(),
        );
        // C++ `Player::UpdateExpertise` (`StatSystem.cpp:759-786`) truncates the
        // combat-rating bonus to `int32`, adds the `SPELL_AURA_MOD_EXPERTISE`
        // sum whose spell is fit for that attack's weapon, clamps at zero and
        // writes `MainhandExpertise`/`OffhandExpertise` per attack.
        // `ActivePlayerData::RangedExpertise`/`CombatRatingExpertise` are never
        // written by C++ (`UpdateExpertise` returns early for `RANGED_ATTACK`)
        // and stay at their zero create value.
        let rating_expertise = (gear.combat_ratings[23] as f32
            * access.combat_rating_multiplier_like_cpp(level, 23))
        .trunc();
        let mainhand_expertise = (rating_expertise
            + self.represented_expertise_aura_modifier_like_cpp(
                access,
                wow_constants::WeaponAttackType::BaseAttack,
            ) as f32)
            .max(0.0);
        let offhand_expertise = (rating_expertise
            + self.represented_expertise_aura_modifier_like_cpp(
                access,
                wow_constants::WeaponAttackType::OffAttack,
            ) as f32)
            .max(0.0);
        // C++ `Unit::CalcArmorReducedDamage` reads the live
        // `GetRatingBonusValue(CR_ARMOR_PENETRATION)` and clamps it to 100.
        let armor_penetration_rating = crate::CR_ARMOR_PENETRATION_LIKE_CPP;
        let armor_penetration_pct = (gear.combat_ratings[usize::from(armor_penetration_rating)]
            as f32
            * access.combat_rating_multiplier_like_cpp(
                level,
                u32::from(armor_penetration_rating),
            ))
        .clamp(0.0, 100.0);
        // C++ `Player::UpdateMeleeHitChances` (`StatSystem.cpp:743-746`).
        let melee_hit_chance_pct = 7.5
            + gear.combat_ratings[usize::from(crate::CR_HIT_MELEE_LIKE_CPP)] as f32
                * access.combat_rating_multiplier_like_cpp(
                    level,
                    u32::from(crate::CR_HIT_MELEE_LIKE_CPP),
                );
        let mana_regen_mp5 = gear.mana_regen_bonus as f32 / 5.0
            + access.mana_regen_mp5_from_auras_like_cpp(projection.stats);
        let mana_regen_from_spirit = access.mana_regen_from_stats_like_cpp(
            level,
            access.player_class_like_cpp(),
            projection.stats,
        ) * access.mana_regen_aura_multiplier_like_cpp();
        let mana_regen_combat =
            mana_regen_mp5 + mana_regen_from_spirit * access.mana_regen_interrupt_modifier_like_cpp();
        let stats = PlayerEffectiveCombatStatsLikeCpp {
            stats: projection.stats,
            stat_pos_buff: projection.stat_pos_buff,
            stat_neg_buff: projection.stat_neg_buff,
            base_health: projection.create_health,
            max_health: projection.max_health,
            base_mana: projection.base_mana,
            max_mana: projection.max_mana,
            armor: projection.armor,
            resistances,
            attack_power: projection.attack_power,
            attack_power_mod_pos: projection.attack_power_mod_pos,
            attack_power_multiplier: projection.attack_power_multiplier,
            ranged_attack_power: projection.ranged_attack_power,
            ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
            ranged_attack_power_multiplier: projection.ranged_attack_power_multiplier,
            min_damage: weapon_damage[0][0],
            max_damage: weapon_damage[0][1],
            weapon_damage,
            min_ranged_damage: weapon_damage[2][0],
            max_ranged_damage: weapon_damage[2][1],
            combat_ratings: gear.combat_ratings,
            spell_power: gear.spell_power,
            mod_damage_done_pos: projection.mod_damage_done_pos,
            mod_damage_done_neg: projection.mod_damage_done_neg,
            mod_healing_done_pos: projection.mod_healing_done_pos,
            mod_damage_done_percent: projection.mod_damage_done_percent,
            mod_healing_done_percent: projection.mod_healing_done_percent,
            armor_penetration_pct,
            melee_hit_chance_pct,
            mod_target_resistance: projection.mod_target_resistance,
            mod_target_physical_resistance: projection.mod_target_physical_resistance,
            weapon_damage_pct: projection.weapon_damage_pct,
            weapon_damage_flat: projection.weapon_damage_flat,
            versatility_bonus: projection.versatility_bonus,
            override_spell_power_by_ap_percent: projection.override_spell_power_by_ap_percent,
            override_ap_by_spell_power_percent: projection.override_ap_by_spell_power_percent,
            mana_regen: mana_regen_from_spirit + mana_regen_mp5,
            mana_regen_combat,
            health_regen: gear.health_regen_bonus,
            spell_penetration: gear.spell_penetration_bonus,
            mainhand_expertise,
            offhand_expertise,
            ranged_expertise: 0.0,
            combat_rating_expertise: 0.0,
            shield_block: i32::try_from(gear.shield_block_value)
                .unwrap_or(i32::MAX)
                .saturating_add(gear.shield_block_base_mod),
            block_pct: projection.block_pct,
            dodge_pct: projection.dodge_pct,
            dodge_from_attr: projection.dodge_from_attr,
            parry_pct: projection.parry_pct,
            parry_from_attr: projection.parry_from_attr,
            crit_pct: projection.crit_pct,
            ranged_crit_pct: projection.ranged_crit_pct,
            offhand_crit_pct: projection.offhand_crit_pct,
            spell_crit_pct: projection.spell_crit_pct,
            ..PlayerEffectiveCombatStatsLikeCpp::default()
        };
        access.replace_player_effective_combat_stats_like_cpp(stats);
    }

    pub fn publish_effective_stats_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
        level: u8,
        _include_represented_item_bonuses: bool,
        projection: PlayerStatSystemProjectionLikeCpp,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) {
        // The canonical Player accumulator is always the source for this
        // projection. The boolean remains at the adapter boundary for
        // compatibility with callers that already name the C++ option, but a
        // second inventory-derived path is deliberately impossible here.
        self.publish_player_effective_combat_stats_like_cpp(access, level, projection, gear);
    }
}

impl crate::InventoryState {
    pub fn represented_player_gear_stats_like_cpp(
        &self,
        hub: HubRef<'_>,
        _include_represented_item_bonuses: bool,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        // C++ keeps the result of `_ApplyItemBonuses` on Player and uses that
        // accumulator for every subsequent stat calculation. Reading the
        // inventory here as well would count an item once through its sparse
        // row and again through the canonical Player modifier state after an
        // equip/swap. Login seeds the same accumulator before this projection,
        // so this is the single contribution path for every lifecycle.
        let bonuses = self.resolved_item_bonus_state_like_cpp(hub)?;
        Some(represented_player_gear_stats_from_bonuses_like_cpp(bonuses))
    }

    pub fn represented_player_gear_stats_with_access_like_cpp(
        &self,
        access: &PlayerStatsAccessLikeCpp<'_>,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        let canonical = access.item_modifier_runtime_snapshot_like_cpp();
        #[cfg(any(test, feature = "test-fixtures"))]
        let canonical = if canonical.is_none() && access.owner_handle_absent_like_cpp() {
            Some(
                self.player_item_test_fixture_like_cpp
                    .represented_item_modifier_runtime_like_cpp
                    .clone(),
            )
        } else {
            canonical
        };
        let bonuses = canonical?.bonuses_snapshot_like_cpp();
        Some(represented_player_gear_stats_from_bonuses_like_cpp(bonuses))
    }
}

fn represented_player_gear_stats_from_bonuses_like_cpp(
    bonuses: wow_entities::PlayerItemBonusStateLikeCpp,
) -> RepresentedPlayerGearStatsLikeCpp {
    let mut gear = RepresentedPlayerGearStatsLikeCpp::default();
    for (target, amount) in gear.stats.iter_mut().zip(bonuses.stats_base) {
        *target = target.saturating_add(amount);
    }
    gear.attack_power = gear.attack_power.saturating_add(bonuses.attack_power_total);
    gear.ranged_attack_power = gear
        .ranged_attack_power
        .saturating_add(bonuses.ranged_attack_power_total);
    gear.health = gear.health.saturating_add(bonuses.health_base);
    gear.mana = gear.mana.saturating_add(bonuses.mana_base);
    for (target, amount) in gear.combat_ratings.iter_mut().zip(bonuses.combat_ratings) {
        *target = target.saturating_add(amount);
    }
    gear.spell_power = gear.spell_power.saturating_add(bonuses.spell_power_bonus);
    gear.armor = gear
        .armor
        .saturating_add(bonuses.armor_base)
        .saturating_add(bonuses.armor_total)
        .saturating_add(bonuses.resistances_base[0]);
    for (target, amount) in gear.resistances.iter_mut().zip(bonuses.resistances_base) {
        *target = target.saturating_add(amount);
    }
    gear.mana_regen_bonus = bonuses.mana_regen_bonus;
    gear.health_regen_bonus = bonuses.health_regen_bonus;
    gear.spell_penetration_bonus = bonuses.spell_penetration_bonus;
    gear.shield_block_base_mod = bonuses.shield_block_base_mod;
    gear.shield_block_value = bonuses.shield_block_value;
    gear.weapon_damage = bonuses.weapon_damage;
    gear.base_attack_time = bonuses.base_attack_time;

    gear
}

impl crate::InventoryState {
    /// C++ `Player::InitDataForForm` (`Player.cpp:22076-22098`) base attack
    /// times: a form with `CombatRoundTime` drives both melee attacks and
    /// leaves the ranged attack at `BASE_ATTACK_TIME`; otherwise the equipped
    /// weapon delays (`SetRegularAttackTime`) apply.
    pub fn apply_represented_shapeshift_base_attack_time_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> bool {
        let regular = self
            .represented_player_gear_stats_like_cpp(hub.shared(), true)
            .map(|gear| gear.base_attack_time);
        let combat_round_time = hub
            .shared()
            .represented_shapeshift_combat_round_time_like_cpp();
        hub.core
            .mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit_mut();
                let (base, offhand, ranged) = match combat_round_time {
                    Some(round_time) => (round_time as u32, round_time as u32, 2_000),
                    None => {
                        let Some(regular) = regular else {
                            return;
                        };
                        // C++ `Player::SetRegularAttackTime` only writes an attack
                        // whose equipped weapon declares a delay; every other attack
                        // keeps its current time.
                        let current = unit.base_attack_speed();
                        (
                            if regular[0] > 0 {
                                regular[0]
                            } else {
                                current[0]
                            },
                            if regular[1] > 0 {
                                regular[1]
                            } else {
                                current[1]
                            },
                            if regular[2] > 0 {
                                regular[2]
                            } else {
                                current[2]
                            },
                        )
                    }
                };
                unit.set_base_attack_time_like_cpp(
                    wow_constants::WeaponAttackType::BaseAttack,
                    base,
                );
                unit.set_base_attack_time_like_cpp(
                    wow_constants::WeaponAttackType::OffAttack,
                    offhand,
                );
                unit.set_base_attack_time_like_cpp(
                    wow_constants::WeaponAttackType::RangedAttack,
                    ranged,
                );
            })
            .is_some()
    }
}
