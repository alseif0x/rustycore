//! Represented damage, heal and taunt effect application.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.
//! Application methods live in private damage/combat and healing children;
//! shared damage, healing and target projections remain in this parent module.

use super::*;

#[path = "effect_combat/damage_and_combat_application.rs"]
mod damage_and_combat_application;
#[path = "effect_combat/healing_application.rs"]
mod healing_application;

impl WorldSession {
    /// C++ `Unit::SpellDamageBonusDone` (`Unit.cpp:6623-6680`) for the
    /// represented player-caster `SPELL_DIRECT_DAMAGE`:
    /// `int32(max((pdamage + int32(SpellBaseDamageBonusDone(schoolMask) *
    /// BonusCoefficient) + int32(BonusCoefficientFromAP * AP)) * DoneTotalMod,
    /// 0))`.
    ///
    /// Boundaries: the family-scripted damage terms are not modelled yet; the
    /// represented model stores one `BonusCoefficient` per spell rather than per
    /// `SpellEffectInfo`; creature casters keep the raw value. A spell whose
    /// `SpellMisc.SchoolMask` is unavailable also keeps the raw value. The
    /// `effect_index` carries the C++ `SpellEffectInfo` whose mechanic feeds the
    /// `MOD_DAMAGE_DONE_FOR_MECHANIC` term, and `coefficient_from_ap` its
    /// `BonusCoefficientFromAP` table value.
    pub(in crate::session) fn represented_spell_damage_bonus_done_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        coefficient: f32,
        coefficient_from_ap: f32,
        base_damage: u32,
    ) -> u32 {
        if caster_guid != self.player_guid().unwrap_or(ObjectGuid::EMPTY) {
            return base_damage;
        }
        // C++ `SpellDamageBonusDone` (`Unit.cpp:6607-6612`) returns before both
        // the flat advertised benefit and the percentage chain.
        if self.represented_spell_has_attribute_like_cpp(
            spell_id,
            3,
            wow_data::spell::attributes::SPELL_ATTR3_IGNORE_CASTER_MODIFIERS,
        ) {
            return base_damage;
        }
        let Some(school_mask) = self.represented_spell_school_mask_like_cpp(spell_id) else {
            return base_damage;
        };
        let Some(benefit) = self.represented_spell_base_damage_bonus_done_like_cpp(school_mask)
        else {
            return base_damage;
        };
        let Some(done_total_mod) = self.represented_spell_damage_pct_done_like_cpp(
            spell_id,
            effect_index,
            school_mask,
            target_guid,
        ) else {
            return base_damage;
        };
        let done_total = (benefit as f32 * coefficient) as i32
            + self.represented_spell_bonus_coefficient_from_ap_like_cpp(coefficient_from_ap);
        let damage = (base_damage as f32 + done_total as f32) * done_total_mod;
        u32::try_from(damage.max(0.0).min(u32::MAX as f32) as u32).unwrap_or(u32::MAX)
    }

    /// C++ `SpellDamageBonusDone`/`SpellHealingBonusDone` "Check for table
    /// values" (`Unit.cpp:6633-6649`, `7132-7140`): a positive
    /// `BonusCoefficientFromAP` adds
    /// `int32(stack * BonusCoefficientFromAP * APbonus)` to the flat done
    /// benefit.
    ///
    /// Boundaries: `stack` is always one in the represented model; the
    /// `SpellModOp::BonusCoefficient` adjustment is not represented; and the
    /// attack type is always `BASE_ATTACK` because the represented `SpellInfo`
    /// carries no `SpellFamilyName`/`EquippedItemSubClassMask`, so the C++
    /// `RANGED_ATTACK`/`OFF_ATTACK` selection and the victim's
    /// `SPELL_AURA_*_ATTACK_POWER_ATTACKER_BONUS` term remain unrepresented.
    fn represented_spell_bonus_coefficient_from_ap_like_cpp(
        &self,
        coefficient_from_ap: f32,
    ) -> i32 {
        if !(coefficient_from_ap > 0.0) {
            return 0;
        }
        let Some(attack_power) = self.canonical_player_total_attack_power_like_cpp() else {
            return 0;
        };
        (coefficient_from_ap * attack_power) as i32
    }

    /// C++ `SpellInfo::HasAttribute` for the represented spell, resolved through
    /// the current map difficulty and its `FallbackDifficultyID` chain. `false`
    /// when the represented store is unavailable, so callers keep their
    /// un-gated behaviour instead of failing closed on missing metadata.
    pub(in crate::session) fn represented_spell_has_attribute_like_cpp(
        &self,
        spell_id: i32,
        attribute_word: usize,
        attribute: u32,
    ) -> bool {
        let Some(spell_store) = self.spell_store() else {
            return false;
        };
        spell_store.has_attribute_for_difficulty_like_cpp(
            spell_id,
            self.current_map_difficulty_id_like_cpp(),
            self.difficulty_store().map(AsRef::as_ref),
            attribute_word,
            attribute,
        )
    }

    /// C++ `Unit::SpellDamagePctDone` (`Unit.cpp:6683-6772`) player branch: the
    /// `maxModDamagePercentSchool` term (the highest published
    /// `ActivePlayerData::ModDamageDonePercent` among the spell's schools) times
    /// the `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS` (168) multiplier for the victim's
    /// creature type, the `SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE` (303)
    /// multiplier for every active victim aura state, the
    /// `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC` (249)
    /// multiplier for every victim aura mechanic, and the additive
    /// `SPELL_AURA_MOD_DAMAGE_DONE_FOR_MECHANIC` (276) percentage for the cast
    /// effect's mechanic, then the Mage Ice Lance (`*3` on a frozen victim) and
    /// Warlock Drain Soul (`*2` while the caster is wounded) scripted terms.
    /// `SPELL_ATTR3_IGNORE_CASTER_MODIFIERS` and
    /// `SPELL_ATTR6_IGNORE_CASTER_DAMAGE_MODIFIERS` return `1.0f` before any term
    /// is read.
    ///
    /// Boundary: the Warlock Shadow Bite per-DoT term, the
    /// `SPELL_AURA_ABILITY_IGNORE_AURASTATE` shortcut of `Unit::HasAuraState` and
    /// the `SpellFamilyName` switch guard (the id is family-unique instead)
    /// remain unrepresented, and a target whose creature type, aura state or
    /// mechanics cannot be resolved keeps only the multipliers that did resolve.
    fn represented_spell_damage_pct_done_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
        school_mask: u8,
        target_guid: ObjectGuid,
    ) -> Option<f32> {
        // C++ `SpellDamagePctDone` early-outs (`Unit.cpp:6690-6698`).
        if self.represented_spell_has_attribute_like_cpp(
            spell_id,
            3,
            wow_data::spell::attributes::SPELL_ATTR3_IGNORE_CASTER_MODIFIERS,
        ) || self.represented_spell_has_attribute_like_cpp(
            spell_id,
            6,
            wow_data::spell::attributes::SPELL_ATTR6_IGNORE_CASTER_DAMAGE_MODIFIERS,
        ) {
            return Some(1.0);
        }
        let snapshot = self.canonical_player_effective_combat_stats_like_cpp()?;
        let mask = u32::from(school_mask);
        let mut max_mod = 0.0_f32;
        for (school, percent) in snapshot.mod_damage_done_percent.iter().enumerate() {
            if mask & (1_u32 << school) != 0 {
                max_mod = max_mod.max(*percent);
            }
        }
        let creature_type_mask = self.represented_target_creature_type_mask_like_cpp(target_guid);
        if creature_type_mask != 0 {
            for (misc_value, amount) in self
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
                )
                .unwrap_or_default()
            {
                if misc_value & creature_type_mask as i32 != 0 {
                    max_mod *= 1.0 + amount as f32 / 100.0;
                }
            }
        }
        let aura_state_mask = self.represented_target_aura_state_mask_like_cpp(target_guid);
        if aura_state_mask != 0 {
            for (misc_value, amount) in self
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
                )
                .unwrap_or_default()
            {
                if represented_aura_state_bit_like_cpp(misc_value)
                    .is_some_and(|bit| aura_state_mask & bit != 0)
                {
                    max_mod *= 1.0 + amount as f32 / 100.0;
                }
            }
        }
        let target_mechanic_mask = self.represented_target_mechanic_mask_like_cpp(target_guid);
        if target_mechanic_mask != 0 {
            for (misc_value, amount) in self
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::
                        SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
                )
                .unwrap_or_default()
            {
                if represented_mechanic_bit_like_cpp(misc_value)
                    .is_some_and(|bit| target_mechanic_mask & bit != 0)
                {
                    max_mod *= 1.0 + amount as f32 / 100.0;
                }
            }
        }
        if let Some(mechanic) =
            self.represented_spell_damage_mechanic_like_cpp(spell_id, effect_index)
        {
            let pct = self
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_FOR_MECHANIC,
                )
                .unwrap_or_default()
                .into_iter()
                .filter(|(misc_value, _)| *misc_value == mechanic)
                .map(|(_, amount)| amount)
                .sum::<i32>();
            if pct != 0 {
                max_mod *= 1.0 + pct as f32 / 100.0;
            }
        }
        // Custom scripted damage (`Unit.cpp:6748-6770`). The represented
        // `SpellInfo` has no `SpellFamilyName`, so the family switch is keyed by
        // the globally unique spell id and the `SPELLFAMILY_MAGE` /
        // `SPELLFAMILY_WARLOCK` guard is implied rather than read.
        const ICE_LANCE_LIKE_CPP: i32 = 228598;
        const DRAIN_SOUL_LIKE_CPP: i32 = 198590;
        if spell_id == ICE_LANCE_LIKE_CPP {
            // C++ `victim->HasAuraState(AURA_STATE_FROZEN, spellProto, this)`.
            // Boundary: the `SPELL_AURA_ABILITY_IGNORE_AURASTATE` caster
            // shortcut in `Unit::HasAuraState` is not represented.
            let frozen = 1_u32 << (wow_entities::AURA_STATE_FROZEN - 1);
            if self.represented_target_aura_state_mask_like_cpp(target_guid) & frozen != 0 {
                max_mod *= 3.0;
            }
        } else if spell_id == DRAIN_SOUL_LIKE_CPP {
            // C++ `HasAuraState(AURA_STATE_WOUNDED_20_PERCENT)` reads the caster.
            let wounded = 1_u32 << (wow_entities::AURA_STATE_WOUNDED_20_PERCENT - 1);
            if self
                .represented_player_aura_state_mask_like_cpp()
                .unwrap_or(0)
                & wounded
                != 0
            {
                max_mod *= 2.0;
            }
        }
        Some(max_mod)
    }

    /// C++ `SpellEffectInfo::Mechanic` with `SpellInfo::Mechanic` as the
    /// fallback, selected through the current map difficulty and its
    /// `FallbackDifficultyID` chain. `None` when the caster's spell metadata is
    /// unavailable or the spell carries no mechanic.
    fn represented_spell_damage_mechanic_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
    ) -> Option<i32> {
        let spell_store = self.spell_store()?;
        let metadata = spell_store.hit_metadata_for_difficulty_like_cpp(
            spell_id,
            self.current_map_difficulty_id_like_cpp(),
            self.difficulty_store().map(AsRef::as_ref),
        )?;
        let effect_mechanic = metadata
            .effect_mechanics
            .get(&effect_index)
            .copied()
            .unwrap_or(0);
        let mechanic = if effect_mechanic != 0 {
            effect_mechanic
        } else {
            i32::from(metadata.spell_mechanic)
        };
        (mechanic != 0).then_some(mechanic)
    }

    /// C++ `Unit::HasAuraWithMechanic` (`Unit.cpp:4714-4729`) for the
    /// represented target: the union of every applied aura's
    /// `SpellInfo::Mechanic` and the mechanics of its applied effects, `0` when
    /// the target cannot be resolved.
    pub(in crate::session) fn represented_target_mechanic_mask_like_cpp(
        &self,
        target_guid: ObjectGuid,
    ) -> u64 {
        let Some(spell_store) = self.spell_store() else {
            return 0;
        };
        let difficulty_store = self.difficulty_store();
        let difficulty_store = difficulty_store.map(AsRef::as_ref);
        if Some(target_guid) == self.player_guid() {
            let Some(auras) = self.resolved_player_visible_auras_like_cpp() else {
                return 0;
            };
            return crate::session_rules::aura_application_mechanic_mask_like_cpp(
                &auras,
                spell_store,
                difficulty_store,
            );
        }
        let Some(manager) = self.map_manager.as_ref() else {
            return 0;
        };
        let difficulty_id = self.current_map_difficulty_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let applied_auras = {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(creature) =
                manager.find_creature(self.player_map_id_like_cpp(), instance_id, target_guid)
            else {
                return 0;
            };
            creature
                .creature
                .unit()
                .subsystems()
                .auras
                .applied_auras
                .clone()
        };
        crate::session_rules::applied_aura_mechanic_mask_like_cpp(
            &applied_auras,
            spell_store,
            difficulty_id,
            difficulty_store,
        )
    }

    /// C++ `Unit::HasAuraState` for the represented target: the target's
    /// `m_unitData->AuraState`, i.e. its aura-driven bits plus the alive-health
    /// bits `Unit::Update` maintains. `0` when the target cannot be resolved.
    pub(in crate::session) fn represented_target_aura_state_mask_like_cpp(
        &self,
        target_guid: ObjectGuid,
    ) -> u32 {
        self.represented_unit_aura_state_mask_like_cpp(target_guid)
    }

    /// The represented target's current health percentage: the session player's
    /// canonical vitals or a world creature's runtime health. `None` when the
    /// target cannot be resolved.
    fn represented_target_health_pct_like_cpp(&self, target_guid: ObjectGuid) -> Option<f32> {
        if Some(target_guid) == self.player_guid() {
            let (health, max_health, _) = self.resolved_player_vitals_like_cpp()?;
            return Some(100.0 * health as f32 / max_health.max(1) as f32);
        }
        let manager = self.map_manager.as_ref()?;
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let manager = manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let creature =
            manager.find_creature(self.player_map_id_like_cpp(), instance_id, target_guid)?;
        let max_health = creature.max_hp();
        (max_health > 0).then(|| 100.0 * creature.current_hp() as f32 / max_health as f32)
    }

    /// C++ `Unit::GetCreatureTypeMask` (`Unit.cpp:8796-8800`): the bit of the
    /// victim creature's template type, `0` for players or when the template is
    /// unavailable.
    pub(in crate::session) fn represented_target_creature_type_mask_like_cpp(
        &self,
        target_guid: ObjectGuid,
    ) -> u32 {
        let Some(manager) = self.map_manager.as_ref() else {
            return 0;
        };
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let entry = {
            let manager = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(creature) =
                manager.find_creature(self.player_map_id_like_cpp(), instance_id, target_guid)
            else {
                return 0;
            };
            creature.create_data.entry
        };
        self.creature_template_lifecycle_store_like_cpp()
            .and_then(|store| store.get(entry))
            .map(|template| {
                if template.creature_type >= 1 {
                    1_u32 << (template.creature_type - 1)
                } else {
                    0
                }
            })
            .unwrap_or(0)
    }

    /// C++ `Unit::SpellHealingBonusDone` (`Unit.cpp:7100-7183`) for the
    /// represented player-caster `SPELL_DIRECT_DAMAGE`-style direct heal:
    /// `int32(max(float(healamount + int32(SpellBaseHealingBonusDone(schoolMask)
    /// * BonusCoefficient) + int32(BonusCoefficientFromAP * AP)) * DoneTotalMod,
    /// 0.0f))`.
    ///
    /// Boundaries: the victim `SPELL_AURA_MOD_HEALING` term is only applied
    /// when the victim is the session player, because creature auras are not
    /// represented; the `SPELLFAMILY_POTION` early-out (no represented family
    /// name), the periodic-leech suppression, the spell-mod coefficient
    /// adjustment and the scripted handlers are not modelled either. Creature
    /// casters keep the raw value, and a missing `SpellMisc` row or canonical
    /// snapshot fails closed.
    pub(in crate::session) fn represented_spell_healing_bonus_done_like_cpp(
        &self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        coefficient: f32,
        coefficient_from_ap: f32,
        base_heal: u32,
    ) -> u32 {
        let Some(player_guid) = self.player_guid() else {
            return base_heal;
        };
        if caster_guid != player_guid {
            return base_heal;
        }
        let Some(school_mask) = self.represented_spell_school_mask_like_cpp(spell_id) else {
            return base_heal;
        };
        let Some(mut benefit) =
            self.represented_spell_base_healing_bonus_done_like_cpp(school_mask)
        else {
            return base_heal;
        };
        if target_guid == player_guid {
            // C++ `DoneAdvertisedBenefit += victim->
            // GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_HEALING,
            // spellProto->GetSchoolMask())`.
            benefit = benefit.saturating_add(
                self.resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING,
                )
                .unwrap_or_default()
                .into_iter()
                .filter(|(misc_value, _)| misc_value & i32::from(school_mask) != 0)
                .map(|(_, amount)| amount)
                .sum::<i32>(),
            );
        }
        let Some(snapshot) = self.canonical_player_effective_combat_stats_like_cpp() else {
            return base_heal;
        };
        let done_total = (benefit as f32 * coefficient) as i32
            + self.represented_spell_bonus_coefficient_from_ap_like_cpp(coefficient_from_ap);
        // C++ `Unit::SpellHealingPctDone` (`Unit.cpp:7185-7229`): the two
        // attribute early-outs return `1.0f`, otherwise the healing done
        // percentage times the versus-aurastate multiplier plus the
        // missing-health scaling auras. The aura's `IsAffectingSpell`
        // family/flag gate and the `SPELLFAMILY_POTION` early-out are not
        // represented, so both terms apply to any represented heal the aura
        // owner casts.
        let done_total_mod = if self.represented_healing_pct_done_gated_like_cpp(spell_id) {
            1.0
        } else {
            let mut modifier = snapshot.mod_healing_done_percent;
            let aura_state_mask = self.represented_target_aura_state_mask_like_cpp(target_guid);
            if aura_state_mask != 0 {
                for (misc_value, amount) in self
                    .resolved_aura_effects_by_spell_aura_type_like_cpp(
                        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
                    )
                    .unwrap_or_default()
                {
                    if represented_aura_state_bit_like_cpp(misc_value)
                        .is_some_and(|bit| aura_state_mask & bit != 0)
                    {
                        modifier *= 1.0 + amount as f32 / 100.0;
                    }
                }
            }
            let effects = self
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH,
                )
                .unwrap_or_default();
            if !effects.is_empty()
                && let Some(health_pct) = self.represented_target_health_pct_like_cpp(target_guid)
            {
                let health_pct_diff = (100.0 - health_pct).max(0.0);
                for (_, amount) in effects {
                    modifier *= 1.0 + (amount as f32 * health_pct_diff / 100.0) / 100.0;
                }
            }
            modifier
        };
        let heal = (base_heal as f32 + done_total as f32) * done_total_mod;
        u32::try_from(heal.max(0.0).min(u32::MAX as f32) as u32).unwrap_or(u32::MAX)
    }

    /// C++ `Unit::SpellHealingPctDone` (`Unit.cpp:7189-7198`) early-outs:
    /// `SPELL_ATTR3_IGNORE_CASTER_MODIFIERS` and
    /// `SPELL_ATTR6_IGNORE_HEALING_MODIFIERS` gate the whole healing percentage
    /// chain while `SpellBaseHealingBonusDone` still contributes the flat
    /// benefit.
    fn represented_healing_pct_done_gated_like_cpp(&self, spell_id: i32) -> bool {
        self.represented_spell_has_attribute_like_cpp(
            spell_id,
            3,
            wow_data::spell::attributes::SPELL_ATTR3_IGNORE_CASTER_MODIFIERS,
        ) || self.represented_spell_has_attribute_like_cpp(
            spell_id,
            6,
            wow_data::spell::attributes::SPELL_ATTR6_IGNORE_HEALING_MODIFIERS,
        )
    }

    /// C++ `Unit::SpellBaseHealingBonusDone` (`Unit.cpp:7282-7315`): the
    /// `SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` short circuit, otherwise the
    /// `SPELL_AURA_MOD_HEALING_DONE` sum whose misc is zero or intersects the
    /// school mask, plus `GetBaseSpellPowerBonus()`, the mana-class intellect
    /// term and the `SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT` percentages.
    fn represented_spell_base_healing_bonus_done_like_cpp(&self, school_mask: u8) -> Option<i32> {
        let snapshot = self.canonical_player_effective_combat_stats_like_cpp()?;
        let mask = i32::from(school_mask);
        if snapshot.override_spell_power_by_ap_percent > 0.0 {
            let total_attack_power = snapshot
                .attack_power
                .saturating_add(snapshot.attack_power_mod_pos)
                .max(0) as f32
                * (1.0 + snapshot.attack_power_multiplier);
            return Some(
                (total_attack_power * snapshot.override_spell_power_by_ap_percent / 100.0 + 0.5)
                    as i32,
            );
        }
        let mut benefit = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE,
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|(misc_value, _)| *misc_value == 0 || misc_value & mask != 0)
            .map(|(_, amount)| amount)
            .sum::<i32>()
            .saturating_add(snapshot.spell_power);
        if snapshot.base_mana > 0 {
            // C++ `GetPowerIndex(POWER_MANA) != MAX_POWERS` adds the intellect
            // term; the class base-mana row represents that mana slot.
            benefit = benefit.saturating_add(snapshot.stats[3].max(0));
        }
        for (stat_index, _, amount) in self
            .resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT,
            )
            .unwrap_or_default()
        {
            if let Some(stat) = usize::try_from(stat_index)
                .ok()
                .and_then(|index| snapshot.stats.get(index))
            {
                benefit = benefit.saturating_add((*stat as f32 * amount as f32 / 100.0) as i32);
            }
        }
        Some(benefit)
    }

    /// C++ `Unit::SpellHealingBonusTaken` (`Unit.cpp:7231-7239`): the most
    /// positive and most negative active `SPELL_AURA_MOD_HEALING_PCT` (118)
    /// amounts, each applied with `AddPct`, to healing the unit receives.
    ///
    /// Boundary: only the session player's auras are represented, so creature
    /// targets keep the raw amount; the Nourish druid case is not modelled.
    fn represented_spell_healing_bonus_taken_like_cpp(
        &self,
        target_guid: ObjectGuid,
        heal_amount: u32,
    ) -> u32 {
        if Some(target_guid) != self.player_guid() {
            return heal_amount;
        }
        let amounts = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_PCT,
            )
            .unwrap_or_default()
            .into_iter()
            .map(|(_, amount)| amount);
        let mut taken_total_mod = 1.0_f32;
        let mut min_negative = 0i32;
        let mut max_positive = 0i32;
        for amount in amounts {
            min_negative = min_negative.min(amount);
            max_positive = max_positive.max(amount);
        }
        if min_negative != 0 {
            taken_total_mod *= 1.0 + min_negative as f32 / 100.0;
        }
        if max_positive != 0 {
            taken_total_mod *= 1.0 + max_positive as f32 / 100.0;
        }
        let taken = heal_amount as f32 * taken_total_mod;
        u32::try_from(taken.max(0.0).min(u32::MAX as f32) as u32).unwrap_or(u32::MAX)
    }

    /// C++ `SpellInfo::GetSchoolMask()` as loaded from the spell's
    /// `SpellMisc.SchoolMask`; `None` when the store or row is absent.
    fn represented_spell_school_mask_like_cpp(&self, spell_id: i32) -> Option<u8> {
        let spell_id = u32::try_from(spell_id).ok()?;
        let entry = self
            .spell_catalogs
            .spell_misc_store()?
            .get_by_spell_id(spell_id)?;
        (entry.school_mask != 0).then_some(entry.school_mask)
    }

    /// C++ `Unit::SpellBaseDamageBonusDone` (`Unit.cpp:6860-6890`): the
    /// `SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` short circuit, otherwise
    /// `GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE, schoolMask)`
    /// plus `GetBaseSpellPowerBonus()` plus the
    /// `SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT` terms.
    fn represented_spell_base_damage_bonus_done_like_cpp(&self, school_mask: u8) -> Option<i32> {
        let snapshot = self.canonical_player_effective_combat_stats_like_cpp()?;
        let mask = i32::from(school_mask);
        if snapshot.override_spell_power_by_ap_percent > 0.0 {
            let total_attack_power = snapshot
                .attack_power
                .saturating_add(snapshot.attack_power_mod_pos)
                .max(0) as f32
                * (1.0 + snapshot.attack_power_multiplier);
            return Some(
                (total_attack_power * snapshot.override_spell_power_by_ap_percent / 100.0 + 0.5)
                    as i32,
            );
        }
        let mut benefit = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|(misc_value, _)| misc_value & mask != 0)
            .map(|(_, amount)| amount)
            .sum::<i32>()
            .saturating_add(snapshot.spell_power);
        for (aura_mask, stat_index, amount) in self
            .resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT,
            )
            .unwrap_or_default()
        {
            if aura_mask & mask == 0 {
                continue;
            }
            if let Some(stat) = usize::try_from(stat_index)
                .ok()
                .and_then(|index| snapshot.stats.get(index))
            {
                benefit = benefit.saturating_add((*stat as f32 * amount as f32 / 100.0) as i32);
            }
        }
        Some(benefit)
    }
}

/// C++ `UI64LIT(1) << mechanic`: the bit a positive mechanic occupies in a
/// `Unit::HasAuraWithMechanic` mask. `None` for the unset or out-of-range
/// mechanic values the represented runtime must fail closed on.
fn represented_mechanic_bit_like_cpp(mechanic: i32) -> Option<u64> {
    (1..64).contains(&mechanic).then(|| 1_u64 << mechanic)
}

/// C++ `1 << (flag - 1)` for a positive `AuraStateType`. `None` for the unset
/// or out-of-range state a malformed aura row could carry, so a consumer fails
/// closed instead of shifting out of range.
fn represented_aura_state_bit_like_cpp(aura_state: i32) -> Option<u32> {
    u32::try_from(aura_state)
        .ok()
        .and_then(|state| state.checked_sub(1))
        .and_then(|bit| 1_u32.checked_shl(bit))
}
