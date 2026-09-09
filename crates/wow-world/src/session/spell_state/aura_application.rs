//! Applying, refreshing, stacking and expiring represented auras.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn remove_player_visible_aura_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<AuraApplication> {
        self.mutate_player_aura_subsystem_like_cpp(|auras| {
            auras.remove_runtime_application_like_cpp(slot)
        })
        .flatten()
    }
    pub(crate) fn same_effect_stack_rule_aura_types_like_cpp(
        &self,
        group_id: u32,
    ) -> Option<&BTreeSet<i32>> {
        self.spell_group_stack_rule_store
            .as_ref()
            .and_then(|store| store.same_effect_stack_rule_aura_types_like_cpp(group_id))
    }
    pub(in crate::session) fn remove_indoor_outdoor_auras_for_current_position_represented_like_cpp(
        &mut self,
    ) -> usize {
        if !self.vmap_indoor_check_like_cpp {
            return 0;
        }

        let Some(is_outdoors) = self
            .player_world_local_state_like_cpp()
            .and_then(|state| state.is_outdoors)
        else {
            return 0;
        };

        let attribute = if is_outdoors {
            wow_data::spell::attributes::SPELL_ATTR0_ONLY_INDOORS
        } else {
            wow_data::spell::attributes::SPELL_ATTR0_ONLY_OUTDOORS
        };
        self.remove_represented_auras_with_attribute0_like_cpp(attribute)
    }
    pub(crate) fn remove_represented_auras_due_to_spell_like_cpp(
        &mut self,
        spell_id: i32,
    ) -> usize {
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return 0;
        };
        let slots = visible_auras
            .values()
            .filter_map(|aura| (aura.spell_id == spell_id).then_some(aura.slot))
            .collect::<Vec<_>>();
        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura(slot);
        }
        removed
    }
    /// Apply an aura to the player and send SMSG_AURA_UPDATE.
    pub fn apply_aura(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            0x0000_0001,
        )
    }
    pub(in crate::session) fn apply_aura_with_effect_mask_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            true,
        )
    }
    pub(in crate::session) fn apply_aura_with_effect_mask_without_update_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            false,
        )
    }
    fn apply_aura_with_effect_mask_and_update_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        send_update: bool,
    ) -> Result<(), &'static str> {
        // Find a free slot (0-254) on the canonical Unit owner.
        let slot = self
            .next_player_visible_aura_slot_like_cpp()
            .ok_or("No free aura slots or missing Player aura owner")?;

        // Preserve the represented StatSystem-relevant multiplier on the same
        // AuraApplication. C++ AuraEffect::HandleModTotalPercentStat uses
        // MiscValueB as a per-stat bitmask (zero means all stats), while the
        // generic AuraApplication continues to own the visible slot.
        let (is_ability, total_stat_percentage_effects) = self
            .spell_store()
            .map(|store| {
                let effects = store
                    .get(spell_id)
                    .map(|spell| {
                        spell
                            .effects()
                            .iter()
                            .filter(|effect| {
                                1u32.checked_shl(effect.effect_index)
                                    .is_some_and(|bit| effect_mask & bit != 0)
                                    && effect.effect_aura
                                        == wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                            })
                            .map(|effect| {
                                (
                                    effect.effect_index,
                                    effect.calc_value_no_caster_like_cpp(),
                                    effect.effect_misc_value_1,
                                    effect.effect_misc_value_2,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                (
                    store.has_attribute0_like_cpp(
                        spell_id,
                        wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY,
                    ),
                    effects,
                )
            })
            .unwrap_or_default();
        let modifies_total_stats = !total_stat_percentage_effects.is_empty();
        let preserve_health_pct = is_ability
            && total_stat_percentage_effects
                .iter()
                .any(|(_, _, _, stat_mask)| *stat_mask == 0 || *stat_mask & (1 << 2) != 0);
        let first_total_stat_percentage = total_stat_percentage_effects.first().copied();
        let (
            represented_effect,
            represented_amount,
            represented_misc_value,
            represented_multiplier,
        ) = if let Some((_, amount, _, stat_mask)) = first_total_stat_percentage {
            (
                Some(RepresentedAuraEffectLikeCpp::ModTotalStatPercentage),
                amount,
                Some(stat_mask),
                1.0 + amount as f32 / 100.0,
            )
        } else {
            (None, 0, None, 1.0)
        };
        let represented_effect_amounts: Vec<_> = total_stat_percentage_effects
            .iter()
            .filter_map(|(effect_index, amount, _, _)| {
                u8::try_from(*effect_index).ok().map(|effect_index| {
                    RepresentedAuraEffectAmountLikeCpp {
                        effect_index,
                        amount: *amount,
                    }
                })
            })
            .collect();

        // Create aura
        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.current_map_difficulty_id_like_cpp(),
            caster_guid,
            slot,
            duration_total: duration_ms,
            duration_remaining: duration_ms,
            stack_count: 1,
            aura_flags,
            effect_mask,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect,
            represented_amount,
            represented_effect_amounts: represented_effect_amounts.clone(),
            represented_misc_value,
            represented_multiplier,
            applied_at: Instant::now(),
        };

        if !self.insert_player_visible_aura_like_cpp(aura) {
            return Err("Missing Player aura owner");
        }
        self.sync_canonical_threat_relevant_aura_like_cpp(
            spell_id,
            caster_guid,
            slot,
            effect_mask,
            &represented_effect_amounts,
            true,
        );

        if send_update {
            self.send_aura_update_applied(
                spell_id,
                slot,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
            );
            // C++ applies login/load auras while Player is not yet in world,
            // then folds their modifiers into UpdateAllStats and the initial
            // CreateObject. Do not publish a VALUES delta for a GUID the
            // client has not created yet.
            if modifies_total_stats && self.state == SessionState::LoggedIn {
                self.send_total_stat_percentage_update_like_cpp(preserve_health_pct);
            }
        }
        if spell_id == SPELL_PVP_RULES_ENABLED_LIKE_CPP {
            let _ = self.update_represented_item_level_area_based_scaling_like_cpp();
        }

        Ok(())
    }
    pub(crate) fn apply_login_passive_known_spell_auras_like_cpp(&mut self) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(spell_store) = self.spell_store().cloned() else {
            return 0;
        };

        let mut applied = 0usize;
        for spell_id in self.known_spells_like_cpp().to_vec() {
            if spell_id <= 0
                || !spell_store.is_passive_like_cpp(spell_id)
                || self.player_has_visible_aura_spell_like_cpp(spell_id) != Some(false)
            {
                continue;
            }

            let Some(spell_info) = spell_store.get(spell_id).cloned() else {
                continue;
            };
            let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
            if effect_mask == 0 {
                continue;
            }

            if let Some(equipped) = self
                .spell_equipped_items_store
                .as_ref()
                .and_then(|store| store.entry_for_spell_id_like_cpp(spell_id))
                .filter(|entry| entry.equipped_item_class >= 0)
                .cloned()
            {
                if !self.represented_has_item_fit_to_spell_requirements_like_cpp(&equipped) {
                    continue;
                }
            } else if !self.represented_login_passive_spell_cast_gate_like_cpp(spell_id) {
                continue;
            }

            if self
                .apply_aura_with_effect_mask_like_cpp(
                    spell_id,
                    player_guid,
                    0,
                    AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                    effect_mask,
                )
                .is_ok()
            {
                applied += 1;
            }
        }

        applied
    }
    pub(crate) fn apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(
        &mut self,
        known_spells: &[i32],
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(spell_store) = self.spell_store().cloned() else {
            return 0;
        };

        let mut applied = 0usize;
        for &spell_id in known_spells {
            let Ok(mut previous_spell_id) =
                u32::try_from(spell_id).map(|spell_id| self.prev_spell_in_chain_like_cpp(spell_id))
            else {
                continue;
            };

            while previous_spell_id != 0 {
                let Ok(previous_spell_i32) = i32::try_from(previous_spell_id) else {
                    break;
                };
                if spell_store.is_passive_like_cpp(previous_spell_i32)
                    && self.player_has_visible_aura_spell_like_cpp(previous_spell_i32)
                        == Some(false)
                    && self.represented_login_passive_spell_cast_gate_like_cpp(previous_spell_i32)
                    && let Some(spell_info) = spell_store.get(previous_spell_i32).cloned()
                {
                    let effect_mask = unit_owned_apply_aura_effect_mask_like_cpp(&spell_info);
                    if effect_mask != 0
                        && self
                            .apply_aura_with_effect_mask_like_cpp(
                                previous_spell_i32,
                                player_guid,
                                0,
                                AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100 | 0x0000_0200,
                                effect_mask,
                            )
                            .is_ok()
                    {
                        applied += 1;
                    }
                }

                previous_spell_id = self.prev_spell_in_chain_like_cpp(previous_spell_id);
            }
        }

        applied
    }
    pub(in crate::session) fn apply_represented_provide_spell_focus_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        let slot = self
            .next_player_visible_aura_slot_like_cpp()
            .ok_or("No free aura slots or missing Player aura owner")?;

        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.current_map_difficulty_id_like_cpp(),
            caster_guid,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << effect.effect_index,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::ProvideSpellFocus),
            represented_amount: effect.effect_base_points,
            represented_effect_amounts: represented_aura_effect_amounts_like_cpp(effect),
            represented_misc_value: Some(effect.effect_misc_value_1),
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        };

        if !self.insert_player_visible_aura_like_cpp(aura) {
            return Err("Missing Player aura owner");
        }
        self.send_aura_update_applied(spell_id, slot, caster_guid, 30_000, 0x0000_0001, 0x1);

        Ok(())
    }
    pub(in crate::session) fn apply_represented_aura_modifier_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
        represented_effect: RepresentedAuraEffectLikeCpp,
        duration_ms: u32,
    ) -> Result<(), &'static str> {
        let slot = self
            .next_player_visible_aura_slot_like_cpp()
            .ok_or("No free aura slots or missing Player aura owner")?;

        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.current_map_difficulty_id_like_cpp(),
            caster_guid,
            slot,
            duration_total: duration_ms,
            duration_remaining: duration_ms,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << effect.effect_index,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(represented_effect),
            represented_amount: effect.effect_base_points,
            represented_effect_amounts: represented_aura_effect_amounts_like_cpp(effect),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        };

        if !self.insert_player_visible_aura_like_cpp(aura) {
            return Err("Missing Player aura owner");
        }
        self.send_aura_update_applied(
            spell_id,
            slot,
            caster_guid,
            duration_ms,
            0x0000_0001,
            1u32 << effect.effect_index,
        );

        Ok(())
    }
    /// Remove an aura by slot and send SMSG_AURA_UPDATE.
    pub fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str> {
        let mounted_aura = self
            .resolved_player_visible_auras_like_cpp()
            .and_then(|auras| auras.get(&slot).cloned())
            .is_some_and(|aura| {
                aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted)
            });
        let was_mounted = if mounted_aura {
            self.resolved_player_mounted_like_cpp()
                .ok_or("Missing Player presentation owner")?
        } else {
            false
        };
        let Some(aura) = self.remove_player_visible_aura_like_cpp(slot) else {
            return Err("Aura slot not found");
        };
        if mounted_aura && !self.set_player_mount_presentation_like_cpp(0, false) {
            let _ = self.insert_player_visible_aura_like_cpp(aura);
            return Err("Missing Player presentation owner");
        }
        self.sync_canonical_threat_relevant_aura_like_cpp(
            aura.spell_id,
            aura.caster_guid,
            aura.slot,
            aura.effect_mask,
            &aura.represented_effect_amounts,
            false,
        );

        if aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted) {
            let vehicle_id = self
                .player_mount_vehicle_kit_snapshot_like_cpp()
                .flatten()
                .map(|vehicle| vehicle.vehicle_id())
                .unwrap_or(0);
            #[cfg(test)]
            let vehicle_id = if vehicle_id == 0 {
                self.player_mount_vehicle_id_like_cpp
            } else {
                vehicle_id
            };
            let mount_capability_id = aura.represented_amount;
            let _ = self.mutate_player_mount_vehicle_kit_like_cpp(|kit| {
                if let Some(vehicle_kit) = kit.as_mut() {
                    vehicle_kit.uninstall();
                }
                *kit = None;
            });
            #[cfg(test)]
            {
                self.player_mount_vehicle_id_like_cpp = 0;
                self.player_mount_vehicle_accessories_like_cpp.clear();
                self.player_mount_vehicle_seat_count_like_cpp = 0;
                self.player_mount_vehicle_usable_seat_count_like_cpp = 0;
            }
            if was_mounted {
                if vehicle_id != 0 {
                    #[cfg(test)]
                    {
                        self.mount_vehicle_remove_requests_like_cpp = self
                            .mount_vehicle_remove_requests_like_cpp
                            .saturating_add(1);
                    }
                    self.send_set_vehicle_rec_id_like_cpp(0);
                }
                #[cfg(test)]
                {
                    self.mount_pet_control_enable_requests_like_cpp = self
                        .mount_pet_control_enable_requests_like_cpp
                        .saturating_add(1);
                }
                self.enable_pet_controls_on_dismount_like_cpp();
                #[cfg(test)]
                {
                    self.mount_pet_resummon_requests_like_cpp =
                        self.mount_pet_resummon_requests_like_cpp.saturating_add(1);
                    self.mount_collision_height_update_requests_like_cpp = self
                        .mount_collision_height_update_requests_like_cpp
                        .saturating_add(1);
                }
                self.update_player_collision_height_like_cpp();
                self.send_movement_set_collision_height_like_cpp(
                    wow_packet::packets::movement::UPDATE_COLLISION_HEIGHT_REASON_MOUNT_LIKE_CPP,
                );
            }
            self.send_represented_mount_unit_update_like_cpp(0);
            self.remove_represented_mount_capability_speed_auras_like_cpp(mount_capability_id);
        }
        if matches!(
            aura.represented_effect,
            Some(
                RepresentedAuraEffectLikeCpp::MountedSpeed
                    | RepresentedAuraEffectLikeCpp::Speed
                    | RepresentedAuraEffectLikeCpp::SpeedAlways
                    | RepresentedAuraEffectLikeCpp::SpeedNotStack
                    | RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed
                    | RepresentedAuraEffectLikeCpp::DecreaseSpeed
                    | RepresentedAuraEffectLikeCpp::MinimumSpeed
                    | RepresentedAuraEffectLikeCpp::MinimumSpeedRate
                    | RepresentedAuraEffectLikeCpp::MountedSpeedAlways
                    | RepresentedAuraEffectLikeCpp::MountedSpeedNotStack
            )
        ) {
            self.recompute_represented_run_speed_rate_like_cpp();
        }
        if matches!(
            aura.represented_effect,
            Some(
                RepresentedAuraEffectLikeCpp::MountedFlightSpeed
                    | RepresentedAuraEffectLikeCpp::Fly
                    | RepresentedAuraEffectLikeCpp::FlightSpeed
                    | RepresentedAuraEffectLikeCpp::VehicleFlightSpeed
                    | RepresentedAuraEffectLikeCpp::MountedFlightSpeedAlways
                    | RepresentedAuraEffectLikeCpp::FlightSpeedNotStack
            )
        ) {
            if matches!(
                aura.represented_effect,
                Some(
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeed
                        | RepresentedAuraEffectLikeCpp::Fly
                )
            ) {
                self.update_represented_flight_flags_for_flight_aura_like_cpp(false);
            }
            self.recompute_represented_flight_speed_rate_like_cpp();
        }
        if matches!(
            aura.represented_effect,
            Some(RepresentedAuraEffectLikeCpp::SwimSpeed)
        ) {
            self.recompute_represented_swim_speed_rate_like_cpp();
        }
        if matches!(
            aura.represented_effect,
            Some(
                RepresentedAuraEffectLikeCpp::DecreaseSpeed
                    | RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed
            )
        ) {
            self.recompute_represented_swim_speed_rate_like_cpp();
            self.recompute_represented_flight_speed_rate_like_cpp();
        }
        if aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::DecreaseSpeed) {
            self.recompute_represented_backward_speed_rates_like_cpp();
        }

        // Send SMSG_AURA_UPDATE (removal)
        self.send_aura_update_removed(slot);
        if aura.spell_id == SPELL_PVP_RULES_ENABLED_LIKE_CPP {
            let _ = self.update_represented_item_level_area_based_scaling_like_cpp();
        }
        if self.state == SessionState::LoggedIn
            && self.aura_has_total_stat_percentage_effect_like_cpp(&aura)
        {
            let preserve_health_pct =
                self.total_stat_percentage_aura_preserves_health_pct_like_cpp(&aura);
            self.send_total_stat_percentage_update_like_cpp(preserve_health_pct);
        }

        Ok(())
    }
    pub(crate) fn remove_represented_auras_with_attribute0_like_cpp(
        &mut self,
        attribute: u32,
    ) -> usize {
        let Some(spell_store) = self.spell_store.as_ref() else {
            return 0;
        };
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return 0;
        };

        let slots: Vec<u8> = visible_auras
            .values()
            .filter_map(|aura| {
                spell_store
                    .has_attribute0_like_cpp(aura.spell_id, attribute)
                    .then_some(aura.slot)
            })
            .collect();

        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura(slot);
        }
        removed
    }
    pub(crate) fn remove_moving_or_turning_interrupt_auras_for_far_teleport_like_cpp(
        &mut self,
    ) -> usize {
        let represented_removed = self.remove_auras_with_interrupt_flags_like_cpp(
            SPELL_AURA_INTERRUPT_FLAG_MOVING_OR_TURNING_LIKE_CPP,
            0,
        );
        let canonical_removed = self
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .subsystems_mut()
                    .auras
                    .remove_interruptible_auras(
                        SPELL_AURA_INTERRUPT_FLAG_MOVING_OR_TURNING_LIKE_CPP,
                        0,
                    )
                    .len()
            })
            .unwrap_or(0);

        represented_removed + canonical_removed
    }
    pub(crate) fn remove_represented_growth_auras_cancelable_like_cpp(&mut self) -> usize {
        self.remove_represented_cancelable_auras_by_effect_like_cpp(
            RepresentedAuraEffectLikeCpp::ModScale,
        )
    }
    pub(crate) fn remove_represented_mod_speed_no_control_auras_cancelable_like_cpp(
        &mut self,
    ) -> usize {
        self.remove_represented_cancelable_auras_by_effect_like_cpp(
            RepresentedAuraEffectLikeCpp::ModSpeedNoControl,
        )
    }
    pub(in crate::session) fn remove_represented_cancelable_auras_by_effect_like_cpp(
        &mut self,
        represented_effect: RepresentedAuraEffectLikeCpp,
    ) -> usize {
        let no_aura_cancel = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return 0;
        };
        let slots: Vec<u8> = visible_auras
            .values()
            .filter_map(|aura| {
                // C++ removes SPELL_AURA_MOUNTED only when its SpellInfo is
                // cancelable, positive, and non-passive; the same predicate is
                // used for SPELL_AURA_MOD_SCALE in CancelGrowthAura. These
                // represented effects model positive player-cancelable paths;
                // SpellMisc attributes preserve the C++ no-player-cancel gate.
                if self.spell_store.as_ref().is_some_and(|store| {
                    store.has_attribute0_like_cpp(aura.spell_id, no_aura_cancel)
                }) {
                    return None;
                }
                (aura.represented_effect == Some(represented_effect)).then_some(aura.slot)
            })
            .collect();

        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura(slot);
        }
        removed
    }
    pub(crate) fn remove_represented_cancelable_owned_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
    ) -> usize {
        let Some(spell_store) = self.spell_store.as_ref() else {
            return 0;
        };
        if spell_store.get(spell_id).is_none()
            || spell_store.has_attribute0_like_cpp(
                spell_id,
                wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL,
            )
            || spell_store.is_channeled_like_cpp(spell_id)
            || spell_store.is_passive_like_cpp(spell_id)
        {
            return 0;
        }

        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return 0;
        };
        let slots: Vec<u8> = visible_auras
            .values()
            .filter_map(|aura| {
                if aura.spell_id != spell_id {
                    return None;
                }
                if !caster_guid.is_empty() && aura.caster_guid != caster_guid {
                    return None;
                }
                // C++ checks SpellInfo before RemoveOwnedAura: no
                // SPELL_ATTR0_NO_AURA_CANCEL, positive, and non-passive.
                // Full SpellInfo::IsPositive is not represented yet; allow
                // the locally materialized positive/cancelable aura shapes,
                // including the single-effect generic represented aura.
                (aura.represented_effect.is_none()
                    || matches!(
                        aura.represented_effect,
                        Some(
                            RepresentedAuraEffectLikeCpp::Mounted
                                | RepresentedAuraEffectLikeCpp::ModScale
                                | RepresentedAuraEffectLikeCpp::ModSpeedNoControl
                        )
                    ))
                .then_some(aura.slot)
            })
            .collect();

        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura(slot);
        }
        removed
    }
    pub(crate) fn remove_auras_with_interrupt_flags_like_cpp(
        &mut self,
        flags: u32,
        flags2: u32,
    ) -> usize {
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return 0;
        };
        let slots: Vec<u8> = visible_auras
            .values()
            .filter(|aura| {
                (flags != 0 && aura.aura_interrupt_flags & flags != 0)
                    || (flags2 != 0 && aura.aura_interrupt_flags2 & flags2 != 0)
            })
            .map(|aura| aura.slot)
            .collect();

        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura(slot);
        }
        removed
    }
    /// Check all active auras for expiry and remove those whose duration has elapsed.
    /// Called from the synchronous tick loop (~every 200ms via creature_tick).
    pub(crate) fn tick_auras(&mut self) {
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return;
        };
        if visible_auras.is_empty() {
            return;
        }

        // Collect expired slots (avoid borrow conflict)
        let expired: Vec<u8> = visible_auras
            .values()
            .filter(|a| {
                // Permanent auras (duration_total == 0) never expire
                a.duration_total > 0
                    && a.applied_at.elapsed().as_millis() as u32 >= a.duration_total
            })
            .map(|a| a.slot)
            .collect();

        for slot in expired {
            let spell_id = visible_auras.get(&slot).map(|a| a.spell_id).unwrap_or(0);
            let _ = self.remove_aura(slot);
            debug!(
                account = self.account_id,
                slot = slot,
                spell_id = spell_id,
                "Aura expired"
            );
        }
    }
    fn send_aura_update_removed(&self, slot: u8) {
        let Some(target_guid) = self.player_guid() else {
            return;
        };
        self.send_packet(&wow_packet::packets::misc::AuraUpdate {
            unit_guid: target_guid,
            update_all: false,
            auras: vec![wow_packet::packets::misc::AuraInfoLikeCpp {
                slot,
                aura_data: None,
            }],
        });
    }
    pub(in crate::session) fn remove_represented_stealth_or_invisibility_auras_by_type_like_cpp(
        &mut self,
    ) -> Option<usize> {
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;
        let slots: Vec<u8> = visible_auras
            .iter()
            .filter_map(|(slot, aura)| {
                matches!(
                    aura.represented_effect,
                    Some(RepresentedAuraEffectLikeCpp::Stealth)
                        | Some(RepresentedAuraEffectLikeCpp::Invisibility)
                )
                .then_some(*slot)
            })
            .collect();
        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura(slot);
        }
        Some(removed)
    }
}
