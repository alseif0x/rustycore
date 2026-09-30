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
    /// The represented player's flat and percentage damage-bonus stages from
    /// C++ `Unit::SpellDamageBonusDone` (`Unit.cpp:6592-6681`):
    /// `int32(max((pdamage + int32(SpellBaseDamageBonusDone(schoolMask) *
    /// BonusCoefficient) + int32(BonusCoefficientFromAP * AP)) * DoneTotalMod,
    /// 0))`.
    ///
    /// Boundaries: this existing represented Session caller applies the stages
    /// to its `SPELL_DIRECT_DAMAGE` input, while C++ returns from that damage
    /// type before these stages (`Unit.cpp:6608-6612`); this extraction retains
    /// the Rust caller behavior. Ice Lance and Drain Soul factors remain in the
    /// world adapter, while other family-specific damage terms are unmodeled.
    /// The represented model stores one `BonusCoefficient` per spell rather
    /// than per `SpellEffectInfo`; creature casters and spells without a
    /// `SpellMisc.SchoolMask` keep the raw value. `effect_index` selects the
    /// C++ effect mechanic and `coefficient_from_ap` supplies its table value.
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
        // C++ `SpellDamageBonusDone` (`Unit.cpp:6608-6612`) returns before both
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
        let coefficient_benefit =
            wow_combat::spell_advertised_coefficient_benefit_like_cpp(benefit, coefficient);
        let attack_power_benefit =
            self.represented_spell_bonus_coefficient_from_ap_like_cpp(coefficient_from_ap);
        let done_total = wow_combat::spell_done_flat_benefit_add_ap_like_cpp(
            coefficient_benefit,
            attack_power_benefit,
        );
        wow_combat::spell_damage_bonus_done_like_cpp(base_damage, done_total, done_total_mod)
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
        wow_combat::spell_bonus_coefficient_from_ap_like_cpp(coefficient_from_ap, attack_power)
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

    /// C++ `Unit::SpellDamagePctDone` (`Unit.cpp:6683-6773`) player branch: the
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
        // C++ `SpellDamagePctDone` attribute early-outs (`Unit.cpp:6688-6694`).
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
        let creature_type_mask = self.represented_target_creature_type_mask_like_cpp(target_guid);
        let damage_done_versus = if creature_type_mask != 0 {
            self.resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS,
            )
            .unwrap_or_default()
        } else {
            Vec::new()
        };
        let aura_state_mask = self.represented_target_aura_state_mask_like_cpp(target_guid);
        let damage_done_versus_aura_state = if aura_state_mask != 0 {
            self.resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
            )
            .unwrap_or_default()
        } else {
            Vec::new()
        };
        let target_mechanic_mask = self.represented_target_mechanic_mask_like_cpp(target_guid);
        let damage_percent_done_by_target_aura_mechanic = if target_mechanic_mask != 0 {
            self.resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::
                    SPELL_AURA_MOD_DAMAGE_PERCENT_DONE_BY_TARGET_AURA_MECHANIC,
            )
            .unwrap_or_default()
        } else {
            Vec::new()
        };
        let spell_mechanic =
            self.represented_spell_damage_mechanic_like_cpp(spell_id, effect_index);
        let damage_done_for_mechanic = spell_mechanic.map(|_| {
            self.resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_FOR_MECHANIC,
            )
            .unwrap_or_default()
        });
        // Custom scripted damage (`Unit.cpp:6750-6770`). The represented
        // `SpellInfo` has no `SpellFamilyName`, so the family switch is keyed by
        // the globally unique spell id and the `SPELLFAMILY_MAGE` /
        // `SPELLFAMILY_WARLOCK` guard is implied rather than read.
        const ICE_LANCE_LIKE_CPP: i32 = 228598;
        const DRAIN_SOUL_LIKE_CPP: i32 = 198590;
        let scripted_factor = if spell_id == ICE_LANCE_LIKE_CPP {
            // C++ `victim->HasAuraState(AURA_STATE_FROZEN, spellProto, this)`.
            // Boundary: the `SPELL_AURA_ABILITY_IGNORE_AURASTATE` caster
            // shortcut in `Unit::HasAuraState` is not represented.
            let frozen = 1_u32 << (wow_entities::AURA_STATE_FROZEN - 1);
            if self.represented_target_aura_state_mask_like_cpp(target_guid) & frozen != 0 {
                3.0
            } else {
                1.0
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
                2.0
            } else {
                1.0
            }
        } else {
            1.0
        };
        Some(wow_combat::spell_damage_pct_done_like_cpp(
            wow_combat::SpellDamagePctDoneInputsLikeCpp {
                school_mask,
                school_percentages: &snapshot.mod_damage_done_percent,
                creature_type_mask,
                damage_done_versus: &damage_done_versus,
                target_aura_state_mask: aura_state_mask,
                damage_done_versus_aura_state: &damage_done_versus_aura_state,
                target_mechanic_mask,
                damage_percent_done_by_target_aura_mechanic:
                    &damage_percent_done_by_target_aura_mechanic,
                spell_mechanic,
                damage_done_for_mechanic: damage_done_for_mechanic.as_deref(),
                scripted_factor,
            },
        ))
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

    /// C++ `Unit::SpellHealingBonusDone` (`Unit.cpp:7089-7183`) for the
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
            let victim_effects = self
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING,
                )
                .unwrap_or_default();
            benefit = wow_combat::spell_healing_bonus_from_victim_aura_effects_like_cpp(
                benefit,
                school_mask,
                &victim_effects,
            );
        }
        let Some(snapshot) = self.canonical_player_effective_combat_stats_like_cpp() else {
            return base_heal;
        };
        let coefficient_benefit =
            wow_combat::spell_advertised_coefficient_benefit_like_cpp(benefit, coefficient);
        let attack_power_benefit =
            self.represented_spell_bonus_coefficient_from_ap_like_cpp(coefficient_from_ap);
        let done_total = wow_combat::spell_done_flat_benefit_add_ap_like_cpp(
            coefficient_benefit,
            attack_power_benefit,
        );
        // C++ `Unit::SpellHealingPctDone` (`Unit.cpp:7185-7227`): the two
        // attribute early-outs return `1.0f`, otherwise the healing done
        // percentage times the versus-aurastate multiplier plus the
        // missing-health scaling auras. The aura's `IsAffectingSpell`
        // family/flag gate and the `SPELLFAMILY_POTION` early-out are not
        // represented, so both terms apply to any represented heal the aura
        // owner casts.
        let done_total_mod = if self.represented_healing_pct_done_gated_like_cpp(spell_id) {
            wow_combat::spell_healing_pct_done_like_cpp(
                wow_combat::SpellHealingPctDoneInputsLikeCpp {
                    gated: true,
                    healing_done_percent: snapshot.mod_healing_done_percent,
                    target_aura_state_mask: 0,
                    damage_done_versus_aura_state: &[],
                    healing_done_pct_versus_target_health: &[],
                    target_health_pct: None,
                },
            )
        } else {
            let aura_state_mask = self.represented_target_aura_state_mask_like_cpp(target_guid);
            let aura_state_effects = if aura_state_mask != 0 {
                self.resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE_VERSUS_AURASTATE,
                )
                .unwrap_or_default()
            } else {
                Vec::new()
            };
            let healing_pct_versus_target_health = self
                .resolved_aura_effects_by_spell_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE_PCT_VERSUS_TARGET_HEALTH,
                )
                .unwrap_or_default();
            let healing_pct_amounts = healing_pct_versus_target_health
                .iter()
                .map(|(_, amount)| *amount)
                .collect::<Vec<_>>();
            let target_health_pct = if healing_pct_versus_target_health.is_empty() {
                None
            } else {
                self.represented_target_health_pct_like_cpp(target_guid)
            };
            wow_combat::spell_healing_pct_done_like_cpp(
                wow_combat::SpellHealingPctDoneInputsLikeCpp {
                    gated: false,
                    healing_done_percent: snapshot.mod_healing_done_percent,
                    target_aura_state_mask: aura_state_mask,
                    damage_done_versus_aura_state: &aura_state_effects,
                    healing_done_pct_versus_target_health: &healing_pct_amounts,
                    target_health_pct,
                },
            )
        };
        wow_combat::spell_healing_bonus_done_like_cpp(base_heal, done_total, done_total_mod)
    }

    /// C++ `Unit::SpellHealingPctDone` (`Unit.cpp:7185-7227`) early-outs:
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

    /// C++ `Unit::SpellBaseHealingBonusDone` (`Unit.cpp:7282-7318`): the
    /// `SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` short circuit, otherwise the
    /// `SPELL_AURA_MOD_HEALING_DONE` sum whose misc is zero or intersects the
    /// school mask, plus `GetBaseSpellPowerBonus()`, the mana-class intellect
    /// term and the `SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT` percentages.
    fn represented_spell_base_healing_bonus_done_like_cpp(&self, school_mask: u8) -> Option<i32> {
        let snapshot = self.canonical_player_effective_combat_stats_like_cpp()?;
        if snapshot.override_spell_power_by_ap_percent > 0.0 {
            return Some(wow_combat::spell_power_override_from_ap_like_cpp(
                snapshot.attack_power,
                snapshot.attack_power_mod_pos,
                snapshot.attack_power_multiplier,
                snapshot.override_spell_power_by_ap_percent,
            ));
        }
        let healing_done_effects = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_HEALING_DONE,
            )
            .unwrap_or_default();
        let healing_stat_effects = self
            .resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT,
            )
            .unwrap_or_default();
        // C++ `GetPowerIndex(POWER_MANA) != MAX_POWERS` adds intellect only
        // for the mana class represented by a positive base-mana row.
        Some(wow_combat::spell_base_healing_bonus_fallback_like_cpp(
            school_mask,
            snapshot.spell_power,
            snapshot.base_mana,
            &snapshot.stats,
            &healing_done_effects,
            &healing_stat_effects,
        ))
    }

    /// C++ `Unit::SpellHealingBonusTaken` (`Unit.cpp:7229-7280`): the most
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
            .map(|(_, amount)| amount)
            .collect::<Vec<_>>();
        wow_combat::spell_healing_bonus_taken_like_cpp(heal_amount, &amounts)
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

    /// C++ `Unit::SpellBaseDamageBonusDone` (`Unit.cpp:6860-6891`): the
    /// `SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` short circuit, otherwise
    /// `GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE, schoolMask)`
    /// plus `GetBaseSpellPowerBonus()` plus the
    /// `SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT` terms.
    fn represented_spell_base_damage_bonus_done_like_cpp(&self, school_mask: u8) -> Option<i32> {
        let snapshot = self.canonical_player_effective_combat_stats_like_cpp()?;
        if snapshot.override_spell_power_by_ap_percent > 0.0 {
            return Some(wow_combat::spell_power_override_from_ap_like_cpp(
                snapshot.attack_power,
                snapshot.attack_power_mod_pos,
                snapshot.attack_power_multiplier,
                snapshot.override_spell_power_by_ap_percent,
            ));
        }
        let damage_done_effects = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
            )
            .unwrap_or_default();
        let damage_stat_effects = self
            .resolved_aura_effects_with_misc_values_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT,
            )
            .unwrap_or_default();
        Some(wow_combat::spell_base_damage_bonus_fallback_like_cpp(
            school_mask,
            snapshot.spell_power,
            &snapshot.stats,
            &damage_done_effects,
            &damage_stat_effects,
        ))
    }
}
