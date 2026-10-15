use super::*;

impl WorldSession {
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
        let account_id = self.core.account_id;
        let caster_is_session_player = caster_guid == player_guid;
        let controlling_player_guid = if caster_guid.is_player() {
            Some(caster_guid)
        } else {
            self.core
                .mutate_world_creature(caster_guid, |creature| {
                    creature
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
        let difficulty = self.core.current_map_difficulty_id_like_cpp();
        let difficulty_store = self.catalogs.difficulty_store().cloned();
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
            .and_then(|spell_id| self.catalogs.spell_threat_entry_like_cpp(spell_id))
            .copied();
        let spell_threat_pct_mod = spell_threat_entry.map_or(1.0, |entry| entry.pct_mod);
        let spell_school_mask = spell_id
            .and_then(|spell_id| u32::try_from(spell_id).ok())
            .map_or(1, |spell_id| {
                self.catalogs.spell_school_mask_for_difficulty_like_cpp(
                    spell_id,
                    self.core.current_map_difficulty_id_like_cpp(),
                )
            });
        let caster_school_threat_mod = if caster_is_session_player {
            self.hydrate_canonical_threat_relevant_auras_like_cpp();
            self.core
                .mutate_canonical_player_like_cpp(|player| {
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
            .core
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
                    creature.set_tapped_by_player(player_guid, &tap_group_guids);
                }
                // C++ `SpellNonMeleeDamage::preHitHealth`, read before
                // `DealDamage` so the log can report the overkill.
                let pre_hit_health = creature.current_hp();
                let died = creature.take_damage_before_death_state_like_cpp(damage_amount);
                let newly_engaged = !died
                    && damage_amount > 0
                    && !suppress_harmful_threat
                    && !(no_initial_threat && !creature.is_in_combat())
                    && creature.ai_ownership().combat_target.is_none();
                let threat_value = if !died
                    && damage_amount > 0
                    && !suppress_harmful_threat
                    && !(no_initial_threat && !creature.is_in_combat())
                {
                    // C++ `Spell::DoAllEffectOnTarget` calls `Unit::AtTargetAttacked`, then
                    // `Unit::DealDamage` adds threat for non-player hostile victims.
                    if creature.ai_ownership().combat_target.is_none() {
                        creature.enter_combat(caster_guid);
                    }
                    creature.unit_mut().subsystems_mut().combat.add_threat(
                        caster_guid,
                        damage_amount as f32 * spell_threat_pct_mod * caster_school_threat_mod,
                    );
                    creature
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
                    let move_stop =
                        crate::creature_movement::CreatureMovementLikeCpp::canonical_like_cpp(
                            creature,
                        )
                        .stop_move_spline_like_cpp()
                        .map(|stop| {
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
                    creature.unit().values_update(),
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
            wow_world_spell::sync_represented_creature_threat_to_canonical_like_cpp(
                self,
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
            wow_world_loot_ops::ensure_represented_creature_kill_loot_like_cpp(self, guid).await;
            // Give XP for the kill
            let (mob_level, can_give_experience) = self
                .core
                .mutate_world_creature(guid, |creature| {
                    (creature.level(), creature.can_give_experience_like_cpp())
                })
                .unwrap_or((1, false));
            let xp = can_give_experience
                .then(|| {
                    let (s, h) = crate::session::split_world_entities_ref(self);
                    s.creature_kill_xp(h, mob_level)
                })
                .unwrap_or(0);
            if caster_rewards_session_player && xp > 0 {
                // This direct spell-damage kill path has no represented
                // KillRewarder group fanout yet; do not advertise a group
                // rate until the XP amount is scaled per member like C++.
                self.give_xp(xp, guid, 1.0).await;
            }
            if caster_rewards_session_player {
                let reputation_rate =
                    wow_world_entities::represented_creature_kill_reputation_rate_like_cpp(
                        self,
                        player_guid,
                        guid,
                    );
                wow_world_entities::reward_reputation_from_creature_kill_like_cpp(
                    self,
                    entry,
                    guid,
                    mob_level,
                    reputation_rate,
                );
                self.on_creature_killed_with_generator_like_cpp(item_guid_generator, entry, guid)
                    .await;
                #[cfg(test)]
                wow_world_entities::record_represented_creature_kill_hooks_like_cpp(
                    self,
                    player_guid,
                    guid,
                );
            }
            if let Some(death_values_update) =
                wow_world_entities::complete_represented_creature_death_state_after_kill_hooks_like_cpp(
                    self,
                    caster_guid,
                    guid,
                )
            {
                values_update = death_values_update;
            }
        }

        if self
            .core
            .client_visible_guids_like_cpp
            .contains(&target_guid)
            && let Some(update) = self.represented_unit_values_update_to_update_object_like_cpp(
                target_guid,
                self.core.player_map_id_like_cpp(),
                &values_update,
            )
        {
            self.send_packet(&update);
        }

        Ok(())
    }
}
