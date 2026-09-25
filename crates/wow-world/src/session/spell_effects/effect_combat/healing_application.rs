use super::*;

impl WorldSession {
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
            // C++ `Unit::HealBySpell` runs `CalcHealAbsorb` before `DealHeal`
            // (`Unit.cpp:6557-6562`, `2020-2084`), so the heal the target
            // receives is what the heal-absorb shields left.
            let (heal_amount, absorbed) =
                self.apply_owned_player_heal_absorb_like_cpp(spell_id, healer_guid, heal_amount);
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
                absorbed,
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
            0,
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
    /// no `HealInfo` spell to log, so it stays silent; critical heals are not
    /// represented, so `Crit` stays false and no crit-roll float follows.
    ///
    /// `heal_amount` is `HealInfo::GetHeal()` after the heal-absorb shields;
    /// `HealInfo::GetOriginalHeal()` is that amount plus `absorbed`, exactly
    /// like C++ `HealInfo::AbsorbHeal`.
    pub(in crate::session) fn publish_heal_spell_log_like_cpp(
        &self,
        spell_id: Option<i32>,
        healer_guid: ObjectGuid,
        target_guid: ObjectGuid,
        heal_amount: u32,
        effective_heal: u32,
        absorbed: u32,
    ) {
        let Some(spell_id) = spell_id else {
            return;
        };
        self.send_packet(&wow_packet::packets::combat::SpellHealLog {
            target: target_guid,
            caster: healer_guid,
            spell_id,
            health: i32::try_from(heal_amount).unwrap_or(i32::MAX),
            original_heal: i32::try_from(heal_amount.saturating_add(absorbed)).unwrap_or(i32::MAX),
            over_heal: i32::try_from(heal_amount.saturating_sub(effective_heal))
                .unwrap_or(i32::MAX),
            absorbed: i32::try_from(absorbed).unwrap_or(i32::MAX),
            crit: false,
        });
    }

    /// C++ `Unit::CalcHealAbsorb` (`Unit.cpp:2020-2084`) for the session's own
    /// player target: every `SPELL_AURA_SCHOOL_HEAL_ABSORB` whose `MiscValue`
    /// covers the heal's school spends its amount before the heal lands,
    /// publishes one `SMSG_SPELL_HEAL_ABSORB_LOG` per consuming shield and is
    /// removed once spent. Returns `(remaining heal, absorbed)`.
    ///
    /// Boundary: a creature heal target keeps its auras with no represented
    /// mutable amount, so its heal absorb stays open, exactly like the damage
    /// absorb for creature victims.
    pub(in crate::session) fn apply_owned_player_heal_absorb_like_cpp(
        &mut self,
        spell_id: Option<i32>,
        healer_guid: ObjectGuid,
        heal_amount: u32,
    ) -> (u32, u32) {
        let Some(spell_id) = spell_id else {
            return (heal_amount, 0);
        };
        let Ok(spell_id_key) = u32::try_from(spell_id) else {
            return (heal_amount, 0);
        };
        let Some(spell_store) = self.spell_store().cloned() else {
            return (heal_amount, 0);
        };
        let school_mask = self.spell_school_mask_for_difficulty_like_cpp(
            spell_id_key,
            self.current_map_difficulty_id_like_cpp(),
        );
        let Some(auras) = self.canonical_player_snapshot_like_cpp(|player| {
            player
                .unit()
                .subsystems()
                .auras
                .runtime_applications_like_cpp()
                .clone()
        }) else {
            return (heal_amount, 0);
        };
        let shields = crate::session_rules::player_heal_absorb_shields_like_cpp(
            &auras,
            spell_store.as_ref(),
            school_mask,
        );
        let absorb = crate::session_rules::represented_heal_absorb_like_cpp(&shields, heal_amount);
        if absorb.absorbed == 0 {
            return (absorb.heal, 0);
        }
        let Some(victim_guid) = self.player_guid() else {
            return (absorb.heal, absorb.absorbed);
        };
        let original_heal = i32::try_from(heal_amount).unwrap_or(i32::MAX);
        for consumption in &absorb.consumed {
            let shield = self
                .canonical_player_snapshot_like_cpp(|player| {
                    player
                        .unit()
                        .subsystems()
                        .auras
                        .runtime_application_like_cpp(consumption.slot)
                        .map(|aura| (aura.caster_guid, aura.spell_id))
                })
                .flatten();
            if let Some((absorb_caster, absorb_spell_id)) = shield
                && consumption.consumed > 0
            {
                // C++ `healInfo.GetTarget()->SendMessageToSet(absorbLog.Write(), true)`
                // (`Unit.cpp:2070-2083`); the represented rail is session-local.
                self.send_packet(&wow_packet::packets::combat::SpellHealAbsorbLog {
                    target: victim_guid,
                    absorb_caster,
                    healer: healer_guid,
                    absorb_spell_id,
                    absorbed_spell_id: spell_id,
                    absorbed: consumption.consumed,
                    original_heal,
                });
            }
            let _ = self.with_owned_player_mut_like_cpp(|player| {
                crate::session::combat::write_absorbed_shield_amount_like_cpp(
                    player,
                    consumption.slot,
                    consumption.effect_index,
                    consumption.remaining,
                );
            });
            if consumption.removed {
                let _ = self.remove_aura(consumption.slot);
            }
        }
        (absorb.heal, absorb.absorbed)
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
}
