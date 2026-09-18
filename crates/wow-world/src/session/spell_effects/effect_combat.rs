//! Represented damage, heal and taunt effect application.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.

use super::*;

impl WorldSession {
    pub(in crate::session) fn apply_effect_environmental_damage_like_cpp(
        &mut self,
        effect: &wow_data::SpellEffectInfo,
        target_guid: ObjectGuid,
    ) {
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE {
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if target_guid != player_guid || self.resolved_player_is_alive_like_cpp() != Some(true) {
            return;
        }
        if self
            .resolved_player_damage_control_like_cpp()
            .is_none_or(|state| state.environmental_damage_immune)
        {
            return;
        }

        let damage = u32::try_from(effect.effect_base_points.max(0)).unwrap_or(0);
        let Some((original_health, health_after, _, _, _)) =
            self.apply_owned_player_damage_like_cpp(damage, wow_constants::DeathState::JustDied)
        else {
            return;
        };
        self.sync_player_registry_state_like_cpp();
        if health_after != original_health {
            self.send_player_health_update_like_cpp(player_guid, u64::from(health_after));
        }
        self.send_environmental_damage_log_like_cpp(
            player_guid,
            DAMAGE_FIRE_LIKE_CPP,
            damage,
            0,
            0,
        );
    }
    /// Helper: apply heal to target (self or creature).
    pub(in crate::session) async fn apply_heal(
        &mut self,
        spell_id: Option<i32>,
        target_guid: ObjectGuid,
        heal_amount: u32,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        self.apply_heal_from_caster_like_cpp(spell_id, player_guid, target_guid, heal_amount)
            .await
    }
    pub(in crate::session) async fn apply_heal_from_caster_like_cpp(
        &mut self,
        spell_id: Option<i32>,
        healer_guid: ObjectGuid,
        target_guid: ObjectGuid,
        heal_amount: u32,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        let heal_amount = if spell_id.is_some() {
            self.represented_spell_healing_bonus_taken_like_cpp(target_guid, heal_amount)
        } else {
            heal_amount
        };
        // Si target es el mismo jugador
        if target_guid == player_guid {
            let Some((current, healed, _, effective_heal)) =
                self.apply_owned_player_heal_like_cpp(heal_amount)
            else {
                return Err("Target player owner not available");
            };
            if effective_heal == 0 && self.resolved_player_is_alive_like_cpp() != Some(true) {
                debug!(
                    account = self.account_id,
                    heal = heal_amount,
                    "Skipping self heal because C++ EffectHeal requires alive target"
                );
                return Ok(());
            }
            // C++ `Unit::HealBySpell` publishes `SMSG_SPELL_HEAL_LOG` after
            // `DealHeal` (`Unit.cpp:6557-6564`).
            self.publish_heal_spell_log_like_cpp(
                spell_id,
                healer_guid,
                target_guid,
                heal_amount,
                effective_heal,
            );
            info!(account = self.account_id, heal = heal_amount, "Healed self");
            if healed != current {
                self.sync_player_registry_state_like_cpp();
                self.send_player_health_values_update_like_cpp(player_guid, u64::from(healed));
            }
            self.forward_heal_threat_like_cpp(spell_id, healer_guid, target_guid, effective_heal);
            return Ok(());
        }

        let account_id = self.account_id;
        let heal_outcome = self
            .mutate_world_creature(target_guid, |creature| {
                if !creature.is_alive() {
                    debug!(
                        account = account_id,
                        creature = ?target_guid,
                        heal = heal_amount,
                        "Skipping creature heal because C++ EffectHeal requires alive target"
                    );
                    return None;
                }
                info!(
                    account = account_id,
                    creature = ?target_guid,
                    heal = heal_amount,
                    "Healed creature"
                );

                let effective_heal = {
                    let unit = creature.creature.unit_mut();
                    let current = unit.data().health;
                    let max = unit.data().max_health;
                    let healed = current.saturating_add(u64::from(heal_amount)).min(max);
                    unit.set_health(healed);
                    healed.saturating_sub(current).min(u64::from(u32::MAX)) as u32
                };

                Some((creature.creature.unit().values_update(), effective_heal))
            })
            .ok_or("Target not found")?;

        let Some((values_update, effective_heal)) = heal_outcome else {
            return Ok(());
        };
        self.forward_heal_threat_like_cpp(spell_id, healer_guid, target_guid, effective_heal);
        self.publish_heal_spell_log_like_cpp(
            spell_id,
            healer_guid,
            target_guid,
            heal_amount,
            effective_heal,
        );

        if self.client_visible_guids_like_cpp.contains(&target_guid)
            && let Some(update) = self.represented_unit_values_update_to_update_object_like_cpp(
                target_guid,
                self.player_map_id_like_cpp(),
                &values_update,
            )
        {
            self.send_packet(&update);
        }

        Ok(())
    }
    /// C++ `Unit::SendHealSpellLog` (`Unit.cpp:6538-6555`): the heal combat log
    /// the client shows for a spell heal. A heal without a represented spell has
    /// no `HealInfo` spell to log, so it stays silent; heal absorb and critical
    /// heals are not represented, so `Absorbed` is zero and `Crit` false.
    pub(in crate::session) fn publish_heal_spell_log_like_cpp(
        &self,
        spell_id: Option<i32>,
        healer_guid: ObjectGuid,
        target_guid: ObjectGuid,
        heal_amount: u32,
        effective_heal: u32,
    ) {
        let Some(spell_id) = spell_id else {
            return;
        };
        let health = i32::try_from(heal_amount).unwrap_or(i32::MAX);
        self.send_packet(&wow_packet::packets::combat::SpellHealLog {
            target: target_guid,
            caster: healer_guid,
            spell_id,
            health,
            original_heal: health,
            over_heal: i32::try_from(heal_amount.saturating_sub(effective_heal))
                .unwrap_or(i32::MAX),
            absorbed: 0,
            crit: false,
        });
    }
    pub(in crate::session) async fn apply_heal_max_health_like_cpp(
        &mut self,
        spell_id: i32,
        damage: i32,
        healer_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;

        let player_vitals = self
            .resolved_player_vitals_like_cpp()
            .ok_or("Target player owner not available")?;
        let target_missing_health = if target_guid == player_guid {
            if !player_vitals.2 {
                return Ok(());
            }
            player_vitals.1.saturating_sub(player_vitals.0)
        } else {
            let Some(target_missing_health) = self
                .mutate_world_creature(target_guid, |creature| {
                    creature
                        .is_alive()
                        .then(|| creature.max_hp().saturating_sub(creature.current_hp()))
                })
                .flatten()
            else {
                return Ok(());
            };
            target_missing_health
        };

        let heal_amount = if damage == 0 {
            player_vitals.1
        } else {
            target_missing_health
        };

        if heal_amount == 0 {
            return Ok(());
        }
        self.apply_heal_from_caster_like_cpp(Some(spell_id), healer_guid, target_guid, heal_amount)
            .await
    }
    pub(in crate::session) async fn apply_heal_pct_like_cpp(
        &mut self,
        spell_id: i32,
        damage: i32,
        healer_guid: ObjectGuid,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        if damage < 0 {
            debug!(
                account = self.account_id,
                heal_pct = damage,
                "Skipping SPELL_EFFECT_HEAL_PCT because C++ EffectHealPct returns when damage < 0"
            );
            return Ok(());
        }

        let player_guid = self.player_guid().ok_or("No player GUID")?;
        let target_max_health = if target_guid == player_guid {
            let Some((_, max_health, is_alive)) = self.resolved_player_vitals_like_cpp() else {
                return Err("Target player owner not available");
            };
            if !is_alive {
                return Ok(());
            }
            max_health
        } else {
            let Some(max_health) = self
                .mutate_world_creature(target_guid, |creature| {
                    creature.is_alive().then(|| creature.max_hp())
                })
                .flatten()
            else {
                return Ok(());
            };
            max_health
        };

        let heal_amount = ((target_max_health as f32) * (damage as f32) / 100.0) as u32;
        if heal_amount == 0 {
            return Ok(());
        }
        self.apply_heal_from_caster_like_cpp(Some(spell_id), healer_guid, target_guid, heal_amount)
            .await
    }
    pub(in crate::session) async fn apply_health_leech_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        spell_id: i32,
        damage: i32,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        if damage < 0 {
            debug!(
                account = self.account_id,
                leech_damage = damage,
                "Skipping SPELL_EFFECT_HEALTH_LEECH because C++ EffectHealthLeech returns when damage < 0"
            );
            return Ok(());
        }

        let player_guid = self.player_guid().ok_or("No player GUID")?;
        let damage_amount = damage as u32;
        let Some(effective_damage) = self
            .mutate_world_creature(target_guid, |creature| {
                creature
                    .is_alive()
                    .then(|| damage_amount.min(creature.current_hp()))
            })
            .flatten()
        else {
            return Ok(());
        };

        if damage_amount > 0 {
            self.apply_damage_with_generator_like_cpp(
                item_guid_generator,
                Some(spell_id),
                target_guid,
                damage_amount,
            )
            .await?;
        }
        if effective_damage > 0 && self.resolved_player_is_alive_like_cpp() == Some(true) {
            self.apply_heal(Some(spell_id), player_guid, effective_damage)
                .await?;
        }
        Ok(())
    }
    /// C++ `Spell::EffectTaunt` / `SPELL_EFFECT_ATTACK_ME`.
    pub(in crate::session) fn apply_taunt_effect_like_cpp(
        &mut self,
        spell_id: i32,
        target_guid: ObjectGuid,
        enforce_effect_taunt_current_victim_gate: bool,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        let difficulty = self.current_map_difficulty_id_like_cpp();
        let taunt_effect_mask = self.spell_store().and_then(|store| {
            store
                .effects_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    self.difficulty_store().map(AsRef::as_ref),
                )
                .and_then(|effects| {
                    effects.iter().find_map(|effect| {
                        (effect.effect
                            == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                            && effect.effect_aura
                                == wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT)
                            .then(|| 1u32.checked_shl(effect.effect_index))
                            .flatten()
                    })
                })
        });
        let duration_ms = {
            let duration_index = self
                .spell_catalogs
                .spell_misc_store
                .as_deref()
                .and_then(|store| {
                    store.entry_for_spell_difficulty_like_cpp(
                        u32::try_from(spell_id).unwrap_or(0),
                        self.current_map_difficulty_id_like_cpp(),
                    )
                })
                .map(|entry| u32::from(entry.duration_index))
                .unwrap_or(0);
            spell_duration_ms_like_cpp(
                duration_index,
                self.spell_catalogs.spell_duration_store.as_deref(),
            )
        };

        let Some((threat_value, taunt_slot)) = self
            .mutate_world_creature(target_guid, |creature| {
                let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
                if !combat.owner_can_have_threat_list
                    || (enforce_effect_taunt_current_victim_gate
                        && combat.current_victim_guid == Some(player_guid))
                {
                    return None;
                }
                let threat = (!combat.is_threat_list_empty(false))
                    .then(|| combat.match_unit_threat_to_highest_threat_like_cpp(player_guid))
                    .flatten();
                if taunt_effect_mask.is_none() {
                    combat.set_threat_taunt_state(
                        player_guid,
                        wow_entities::ThreatTauntState::Taunt(1),
                    );
                    combat.current_victim_guid = Some(player_guid);
                }
                let taunt_slot =
                    taunt_effect_mask
                        .filter(|_| duration_ms != 0)
                        .and_then(|effect_mask| {
                            creature.apply_taunt_aura_like_cpp(
                                player_guid,
                                u32::try_from(spell_id).ok()?,
                                effect_mask,
                                duration_ms,
                            )
                        });
                Some((threat, taunt_slot))
            })
            .flatten()
        else {
            return Ok(());
        };

        if let Some(threat_value) = threat_value {
            self.sync_represented_creature_threat_to_canonical_like_cpp(
                target_guid,
                player_guid,
                threat_value,
            );
        }
        if let (Some(slot), Some(effect_mask)) = (taunt_slot, taunt_effect_mask) {
            use wow_packet::ServerPacket;

            let visible_duration_ms = u32::try_from(duration_ms).unwrap_or(u32::MAX);
            let aura = AuraApplication {
                spell_id,
                difficulty_id: self.current_map_difficulty_id_like_cpp(),
                caster_guid: player_guid,
                slot,
                duration_total: visible_duration_ms,
                duration_remaining: visible_duration_ms,
                stack_count: 1,
                aura_flags: 0,
                effect_mask,
                aura_interrupt_flags: 0,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: 0,
                represented_effect_amounts: Vec::new(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            };
            let packet = wow_packet::packets::misc::AuraUpdate {
                unit_guid: target_guid,
                update_all: false,
                auras: vec![crate::session_rules::player_aura_info_like_cpp(
                    &aura,
                    self.player_level_like_cpp(),
                    self.player_map_id_like_cpp(),
                )],
            };
            self.send_packet(&packet);
            self.broadcast_creature_packet_to_visible_set_like_cpp(target_guid, packet.to_bytes());
        }

        // C++ special-cases Hand of Reckoning (62124) by casting 67485 on
        // non-player targets outside the threat-list path. That triggered spell
        // requires broader spell/aura runtime and is intentionally outside this
        // represented threat-list slice.
        if spell_id == 62124 {
            debug!(
                account = self.account_id,
                target = ?target_guid,
                "represented EffectTaunt does not yet cast Hand of Reckoning damage spell 67485"
            );
        }

        Ok(())
    }
    /// Helper: apply damage to target creature.
    pub(in crate::session) async fn apply_damage_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        spell_id: Option<i32>,
        target_guid: ObjectGuid,
        damage_amount: u32,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        self.apply_damage_from_caster_like_cpp(
            item_guid_generator,
            spell_id,
            player_guid,
            target_guid,
            damage_amount,
            ObjectGuid::create_null(),
            0,
        )
        .await
    }
    #[cfg(test)]
    pub(in crate::session) async fn apply_damage(
        &mut self,
        spell_id: Option<i32>,
        target_guid: ObjectGuid,
        damage_amount: u32,
    ) -> Result<(), &'static str> {
        let generators = self.id_generators_for_test_like_cpp();
        self.apply_damage_with_generator_like_cpp(
            generators.item.as_ref(),
            spell_id,
            target_guid,
            damage_amount,
        )
        .await
    }
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

    pub(in crate::session) async fn apply_damage_from_caster_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        spell_id: Option<i32>,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        damage_amount: u32,
        // C++ `SpellNonMeleeDamage::castId`/`SpellVisual`: the combat log needs
        // the cast identity the client correlates with `SMSG_SPELL_GO`. Callers
        // without a represented cast context pass an empty GUID and no visual.
        cast_id: ObjectGuid,
        spell_visual_id: u32,
    ) -> Result<(), &'static str> {
        use wow_packet::ServerPacket;
        use wow_packet::packets::movement::MonsterMoveStop;

        let player_guid = self.player_guid().ok_or("No player GUID")?;
        let account_id = self.account_id;
        let caster_is_session_player = caster_guid == player_guid;
        let controlling_player_guid = if caster_guid.is_player() {
            Some(caster_guid)
        } else {
            self.mutate_world_creature(caster_guid, |creature| {
                creature
                    .creature
                    .unit()
                    .subsystems()
                    .control
                    .charmer_or_owner_guid()
                    .filter(|guid| guid.is_player())
            })
            .flatten()
        };
        let caster_rewards_session_player = controlling_player_guid == Some(player_guid);
        let tap_group_guids = caster_rewards_session_player
            .then(|| self.current_group_member_guids_for_tap_like_cpp(player_guid))
            .unwrap_or_default();
        let difficulty = self.current_map_difficulty_id_like_cpp();
        let difficulty_store = self.difficulty_store().cloned();
        let suppress_harmful_threat = spell_id.is_some_and(|spell_id| {
            self.spell_store().is_some_and(|store| {
                store.has_attribute_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store.as_deref(),
                    1,
                    wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT,
                ) || store.has_attribute_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store.as_deref(),
                    4,
                    wow_data::spell::attributes::SPELL_ATTR4_NO_HARMFUL_THREAT,
                )
            })
        });
        let no_initial_threat = spell_id.is_some_and(|spell_id| {
            self.spell_store().is_some_and(|store| {
                store.has_attribute_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store.as_deref(),
                    2,
                    wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT,
                )
            })
        });
        let spell_threat_entry = spell_id
            .and_then(|spell_id| u32::try_from(spell_id).ok())
            .and_then(|spell_id| self.spell_threat_entry_like_cpp(spell_id))
            .copied();
        let spell_threat_pct_mod = spell_threat_entry.map_or(1.0, |entry| entry.pct_mod);
        let spell_school_mask = spell_id
            .and_then(|spell_id| u32::try_from(spell_id).ok())
            .map_or(1, |spell_id| {
                self.spell_school_mask_for_difficulty_like_cpp(
                    spell_id,
                    self.current_map_difficulty_id_like_cpp(),
                )
            });
        let caster_school_threat_mod = if caster_is_session_player {
            self.hydrate_canonical_threat_relevant_auras_like_cpp();
            self.mutate_canonical_player_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .total_aura_multiplier_by_misc_mask_like_cpp(
                        wow_data::spell::aura_types::SPELL_AURA_MOD_THREAT,
                        spell_school_mask,
                    )
            })
            .unwrap_or(1.0)
        } else {
            1.0
        };

        // Si target es otra criatura — mutate canonical shared map state.
        let damage_outcome = self
            .mutate_world_creature(target_guid, |creature| {
                if !creature.is_alive() {
                    debug!(
                        account = account_id,
                        creature = ?target_guid,
                        damage = damage_amount,
                        "Skipping spell damage because C++ EffectSchoolDMG requires alive target"
                    );
                    return None;
                }
                info!(
                    account = account_id,
                    creature = ?target_guid,
                    damage = damage_amount,
                    "Dealt damage to creature"
                );

                if caster_rewards_session_player {
                    creature
                        .creature
                        .set_tapped_by_player(player_guid, &tap_group_guids);
                }
                // C++ `SpellNonMeleeDamage::preHitHealth`, read before
                // `DealDamage` so the log can report the overkill.
                let pre_hit_health = creature.current_hp();
                let died = creature.take_damage_before_death_state_like_cpp(damage_amount);
                let newly_engaged = !died
                    && damage_amount > 0
                    && !suppress_harmful_threat
                    && !(no_initial_threat && !creature.creature.is_in_combat())
                    && creature.creature.ai_ownership().combat_target.is_none();
                let threat_value = if !died
                    && damage_amount > 0
                    && !suppress_harmful_threat
                    && !(no_initial_threat && !creature.creature.is_in_combat())
                {
                    // C++ `Spell::DoAllEffectOnTarget` calls `Unit::AtTargetAttacked`, then
                    // `Unit::DealDamage` adds threat for non-player hostile victims.
                    if creature.creature.ai_ownership().combat_target.is_none() {
                        creature.enter_combat(caster_guid);
                    }
                    creature
                        .creature
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .add_threat(
                            caster_guid,
                            damage_amount as f32 * spell_threat_pct_mod * caster_school_threat_mod,
                        );
                    creature
                        .creature
                        .unit()
                        .subsystems()
                        .combat
                        .threat_value(caster_guid)
                } else {
                    None
                };
                let kill_info = if died {
                    info!(
                        "Creature {} (entry={}) killed",
                        target_guid,
                        creature.entry()
                    );
                    let move_stop = creature.stop_move_spline_like_cpp().map(|stop| {
                        MonsterMoveStop {
                            mover_guid: target_guid,
                            current_pos: stop.position,
                            spline_id: stop.spline_id,
                        }
                        .to_bytes()
                    });
                    Some((creature.entry(), target_guid, move_stop))
                } else {
                    None
                };
                Some((
                    kill_info,
                    creature.creature.unit().values_update(),
                    threat_value,
                    newly_engaged,
                    pre_hit_health,
                ))
            })
            .ok_or("Target creature not found")?;
        let Some((kill_info, mut values_update, threat_value, newly_engaged, pre_hit_health)) =
            damage_outcome
        else {
            return Ok(());
        };

        // C++ `Unit::DealSpellDamage` sends the combat log for the hit before
        // the kill cascade (`Unit.cpp:1250-1260`, `Unit::SendSpellNonMeleeDamageLog`
        // `Unit.cpp:5353-5380`). The represented hit has no spell absorb, resist
        // or block stage for a creature target yet, and no spell critical
        // representation, so those fields and `HitInfo` stay zero.
        if let Some(spell_id) = spell_id {
            let damage = damage_amount.min(i32::MAX as u32) as i32;
            self.send_packet(&wow_packet::packets::combat::SpellNonMeleeDamageLog {
                target: target_guid,
                caster: caster_guid,
                cast_id,
                spell_id,
                visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                damage,
                original_damage: damage,
                overkill: if damage_amount > pre_hit_health {
                    i32::try_from(damage_amount - pre_hit_health).unwrap_or(i32::MAX)
                } else {
                    -1
                },
                school_mask: spell_school_mask.min(u32::from(u8::MAX)) as u8,
                absorbed: 0,
                resisted: 0,
                shield_block: 0,
                periodic: false,
                flags: 0,
            });
        }
        if let Some(threat_value) = threat_value {
            self.sync_represented_creature_threat_to_canonical_like_cpp(
                target_guid,
                caster_guid,
                threat_value,
            );
        }
        if newly_engaged {
            self.publish_spell_pull_attack_start_like_cpp(target_guid, caster_guid);
        }

        // Process creature death outside the mutable borrow
        if let Some((entry, guid, move_stop)) = kill_info {
            if let Some(bytes) = move_stop {
                let _ = self.send_tx().send(bytes);
            }
            self.ensure_represented_creature_kill_loot_like_cpp(guid)
                .await;
            // Give XP for the kill
            let (mob_level, can_give_experience) = self
                .mutate_world_creature(guid, |creature| {
                    (
                        creature.level(),
                        creature.creature.can_give_experience_like_cpp(),
                    )
                })
                .unwrap_or((1, false));
            let xp = can_give_experience
                .then(|| self.creature_kill_xp(mob_level))
                .unwrap_or(0);
            if caster_rewards_session_player && xp > 0 {
                // This direct spell-damage kill path has no represented
                // KillRewarder group fanout yet; do not advertise a group
                // rate until the XP amount is scaled per member like C++.
                self.give_xp(xp, guid, 1.0).await;
            }
            if caster_rewards_session_player {
                let reputation_rate =
                    self.represented_creature_kill_reputation_rate_like_cpp(player_guid, guid);
                self.reward_reputation_from_creature_kill_like_cpp(
                    entry,
                    guid,
                    mob_level,
                    reputation_rate,
                );
                self.on_creature_killed_with_generator_like_cpp(item_guid_generator, entry, guid)
                    .await;
                #[cfg(test)]
                self.record_represented_creature_kill_hooks_like_cpp(player_guid, guid);
            }
            if let Some(death_values_update) = self
                .complete_represented_creature_death_state_after_kill_hooks_like_cpp(
                    caster_guid,
                    guid,
                )
            {
                values_update = death_values_update;
            }
        }

        if self.client_visible_guids_like_cpp.contains(&target_guid)
            && let Some(update) = self.represented_unit_values_update_to_update_object_like_cpp(
                target_guid,
                self.player_map_id_like_cpp(),
                &values_update,
            )
        {
            self.send_packet(&update);
        }

        Ok(())
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
