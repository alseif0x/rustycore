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
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.remove_player_visible_aura_like_cpp(slot)
            })
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none() && self.gossip_handleless_fixture() {
            return self
                .mutate_player_aura_subsystem_like_cpp(|auras| {
                    auras.remove_runtime_application_like_cpp(slot)
                })
                .flatten();
        }
        canonical
    }
    pub(crate) fn same_effect_stack_rule_aura_types_like_cpp(
        &self,
        group_id: u32,
    ) -> Option<&BTreeSet<i32>> {
        self.spell_catalogs
            .spell_group_stack_rule_store
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
            .and_then(|state| state.is_outdoors_like_cpp())
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
        let slots = wow_entities::AuraSubsystem::runtime_slots_for_spell(&visible_auras, spell_id);
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
        self.apply_aura_with_effect_mask_provenance_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
            true,
        )
    }
    pub(in crate::session) fn apply_aura_with_effect_mask_and_provenance_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_provenance_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            provenance,
            true,
        )
    }
    /// Test-only entry point for a represented multi-effect application, needed
    /// by acceptance cases (for example the transform/`IsPolymorphed` path)
    /// that must mark more than the first effect active.
    #[cfg(test)]
    pub(crate) fn apply_aura_with_effect_mask_for_test_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        effect_mask: u32,
    ) -> Result<(), &'static str> {
        self.apply_aura_with_effect_mask_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            0x0000_0001,
            effect_mask,
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
        self.apply_aura_with_effect_mask_provenance_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_ms,
            aura_flags,
            effect_mask,
            wow_entities::AuraCastProvenanceLikeCpp::default(),
            false,
        )
    }
    fn apply_aura_with_effect_mask_provenance_and_update_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        duration_ms: u32,
        aura_flags: u32,
        effect_mask: u32,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
        send_update: bool,
    ) -> Result<(), &'static str> {
        // Find a free slot (0-254) on the canonical Unit owner.
        let slot = self
            .next_player_visible_aura_slot_like_cpp()
            .ok_or("No free aura slots or missing Player aura owner")?;

        // Catalog selection and calculation precede attribute selection and
        // runtime construction; installation remains a separate phase.
        let spell_store = self.spell_store();
        let (aura, represented_effect_amounts, modifies_total_stats, preserve_health_pct) =
            wow_entities::AuraSubsystem::build_stat_runtime_application(
                spell_id, caster_guid, slot, duration_ms, aura_flags, effect_mask,
                |id| spell_store.and_then(|store| store.get(id)).map(|spell| spell.effects()),
                |effect| (
                    effect.effect_index, effect.effect_aura,
                    effect.effect_misc_value_1, effect.effect_misc_value_2,
                ),
                wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
                |id| spell_store.is_some_and(|store| store.has_attribute0_like_cpp(
                    id, wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY,
                )),
                || self.current_map_difficulty_id_like_cpp(),
                Instant::now,
            );

        if !self.insert_player_visible_aura_with_provenance_like_cpp(aura, provenance) {
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
        // C++ `AuraEffect::HandleModAttackSpeed`/`HandleModMeleeSpeedPct`/
        // `HandleModCombatSpeedPct`/`HandleAuraModRangedHaste`
        // (`SpellAuraEffects.cpp:4353-4393`) reinstall the attack-time
        // multipliers on every apply.
        self.sync_represented_attack_speed_like_cpp();
        // C++ `AuraEffect::HandleAuraModShapeshift` -> `Player::InitDataForForm`
        // (`Player.cpp:22076-22098`) owns the form and recalcs its attack times
        // and damage.
        if let Some(mutation) = self.sync_represented_shapeshift_form_ownership_like_cpp(spell_id) {
            self.sync_represented_shapeshift_form_like_cpp(mutation);
        }
        // C++ `AuraEffect::HandleAuraModPowerDisplay` (`SpellAuraEffects.cpp:4027-4039`).
        if self.represented_spell_has_power_display_effect_like_cpp(spell_id) {
            self.sync_represented_display_power_like_cpp();
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
                .spell_catalogs
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

        let aura = wow_entities::AuraSubsystem::build_focus_runtime_application(
            spell_id, caster_guid, slot, effect, 
            |effect| (effect.effect_index, effect.effect_base_points, effect.effect_misc_value_1),
            || self.current_map_difficulty_id_like_cpp(),
            Instant::now,
        );

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

        let aura = wow_entities::AuraSubsystem::build_modifier_runtime_application(
            spell_id, caster_guid, slot, effect, represented_effect, duration_ms, 
            |effect| (effect.effect_index, effect.effect_base_points, effect.effect_misc_value_1),
            || self.current_map_difficulty_id_like_cpp(),
            Instant::now,
        );

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
        // C++ `AuraEffect::HandleAuraTransform` remove path
        // (`SpellAuraEffects.cpp:2129-2131`): the application is already gone,
        // so the aura that owns the transform spell clears it.
        let _ = self.remove_represented_transform_aura_like_cpp(&aura);
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
            let _ = self.remove_player_mount_vehicle_kit_like_cpp();
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
        // C++ removes the aura's attack-time multiplier through the same
        // `ApplyAttackTimePercentMod` handlers the apply path used.
        self.sync_represented_attack_speed_like_cpp();
        if let Some(mutation) =
            self.sync_represented_shapeshift_form_ownership_like_cpp(aura.spell_id)
        {
            self.sync_represented_shapeshift_form_like_cpp(mutation);
        }
        // C++ `AuraEffect::HandleAuraModPowerDisplay` (`SpellAuraEffects.cpp:4027-4039`).
        if self.represented_spell_has_power_display_effect_like_cpp(aura.spell_id) {
            self.sync_represented_display_power_like_cpp();
        }

        Ok(())
    }
    pub(crate) fn remove_represented_auras_with_attribute0_like_cpp(
        &mut self,
        attribute: u32,
    ) -> usize {
        let Some(spell_store) = self.spell_catalogs.spell_store.as_ref() else {
            return 0;
        };
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return 0;
        };

        let slots = wow_entities::AuraSubsystem::runtime_slots_with_attribute(
            &visible_auras, attribute,
            |spell_id, attribute| spell_store.has_attribute0_like_cpp(spell_id, attribute),
        );

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
        let slots = wow_entities::AuraSubsystem::runtime_cancelable_slots_for_effect(
            &visible_auras, represented_effect,
            |spell_id| self.spell_catalogs.spell_store.as_ref().is_some_and(|store| {
                store.has_attribute0_like_cpp(spell_id, no_aura_cancel)
            }),
        );

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
        let Some(spell_store) = self.spell_catalogs.spell_store.as_ref() else {
            return 0;
        };
        if !wow_entities::AuraSubsystem::owned_spell_is_cancelable(
            spell_id,
            |id| spell_store.get(id).is_some(),
            |id| spell_store.has_attribute0_like_cpp(
                id, wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL,
            ),
            |id| spell_store.is_channeled_like_cpp(id),
            |id| spell_store.is_passive_like_cpp(id),
        )
        {
            return 0;
        }

        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return 0;
        };
        let slots = wow_entities::AuraSubsystem::runtime_cancelable_owned_slots(
            &visible_auras, spell_id, caster_guid,
        );

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
        let slots = wow_entities::AuraSubsystem::runtime_slots_with_interrupt_flags(
            &visible_auras, flags, flags2,
        );

        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura(slot);
        }
        removed
    }
    /// Check all active auras for expiry and remove those whose duration has elapsed.
    /// Called from the synchronous tick loop (~every 200ms via creature_tick).
    pub(crate) fn tick_auras(&mut self) {
        // C++ `Creature::Update` owns creature aura lifetimes; this session
        // expires the creature auras it applied through the same wall-clock
        // sweep it uses for its own auras.
        self.tick_represented_creature_auras_like_cpp();
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return;
        };
        if visible_auras.is_empty() {
            return;
        }

        // Collect expired slots (avoid borrow conflict)
        let expired = wow_entities::AuraSubsystem::expired_runtime_slots(
            &visible_auras, |applied_at| applied_at.elapsed().as_millis(),
        );

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
        let slots = wow_entities::AuraSubsystem::runtime_stealth_or_invisibility_slots(
            &visible_auras,
        );
        let removed = slots.len();
        for slot in slots {
            let _ = self.remove_aura(slot);
        }
        Some(removed)
    }
}

#[cfg(test)]
#[path = "aura_application_tests.rs"]
mod tests;
