//! Applying, refreshing, stacking and expiring represented auras.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn player_aura_application_cx_like_cpp(
        &mut self,
    ) -> wow_world_application::PlayerAuraApplicationCxLikeCpp<'_> {
        let presentation = self.core.player_aura_removal_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::AuraRemovalFixtureRefsLikeCpp::new(
                &mut self.fixtures.auras.player_aura_authority_complete_like_cpp,
                &mut self
                    .fixtures
                    .auras
                    .player_spell_hit_aura_authority_tombstoned_like_cpp,
                &mut self.fixtures.auras.visible_auras,
                &mut self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                &mut self.fixtures.vehicles.player_mount_display_id_like_cpp,
                &mut self.fixtures.vehicles.player_mounted_like_cpp,
                &mut self.fixtures.presentation.player_unit_flags_like_cpp,
                &self.fixtures.presentation.player_object_scale_like_cpp,
            ),
        );
        let control = self.core.aura_mount_control_access_like_cpp(
            self.catalogs.creatures.display_info_store.as_deref(),
            self.catalogs.creatures.model_data_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::AuraMountControlFixtureRefsLikeCpp::new(
                &mut self.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &mut self.fixtures.movement.movement_counter_like_cpp,
                &mut self.fixtures.movement.player_collision_height_like_cpp,
                &self.fixtures.movement.player_position,
                &mut self.fixtures.movement.player_movement_flags_like_cpp,
                &self.fixtures.movement.player_movement_time_like_cpp,
                &mut self
                    .fixtures
                    .movement
                    .represented_can_swim_to_fly_transition_like_cpp,
                &self.fixtures.identity.player_scale_duration_like_cpp,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_gender,
                wow_world_core::session::AuraDismountPetFixtureRefsLikeCpp::new(
                    &self.fixtures.pets.represented_pet_guid_like_cpp,
                    &mut self.fixtures.pets.represented_pet_react_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_command_state_like_cpp,
                    &mut self.fixtures.pets.represented_pet_stable_like_cpp,
                    &mut self
                        .fixtures
                        .pets
                        .represented_character_pet_rows_empty_authority_complete_like_cpp,
                    &mut self
                        .fixtures
                        .pets
                        .represented_temporary_unsummoned_pet_number_like_cpp,
                    &mut self.fixtures.pets.represented_old_pet_spell_like_cpp,
                    &mut self.fixtures.pets.temporary_mount_pet_react_state_like_cpp,
                ),
                &mut self.fixtures.movement.movement_speed_rates_like_cpp,
                &mut self.fixtures.movement.forced_speed_changes_like_cpp,
                &self.fixtures.pets.represented_pet_guid_like_cpp,
                &mut self
                    .fixtures
                    .pets
                    .represented_pet_movement_speed_rates_like_cpp,
                &mut self
                    .fixtures
                    .pets
                    .represented_pet_speed_propagations_like_cpp,
                &self.fixtures.combat.in_combat,
                &mut self.fixtures.movement.last_fall_time_like_cpp,
                &mut self.fixtures.movement.last_fall_z_like_cpp,
            ),
        );
        let stats = self.core.aura_stats_access_builder_like_cpp(
            &self.catalogs,
            &self.config,
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::StatsCombatFixtureRefs::new_like_cpp(
                &mut self.fixtures.combat.player_health_like_cpp,
                &mut self.fixtures.combat.player_max_health_like_cpp,
                &mut self.fixtures.combat.player_alive_like_cpp,
                &mut self.fixtures.combat.represented_player_powers_like_cpp[0],
                &mut self.fixtures.combat.represented_player_max_powers_like_cpp[0],
                &mut self.fixtures.combat.represented_player_base_mana_like_cpp,
            ),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
        );
        let item_sets = self.core.owned_item_set_access_like_cpp(
            self.catalogs.items.set_store.as_deref(),
            self.catalogs.spell_catalogs.item_set_spell_store.as_deref(),
            self.catalogs.spell_catalogs.spell_store.as_deref(),
            self.catalogs.heirloom_store.as_deref(),
            self.catalogs.items.stats_store.as_deref(),
            self.catalogs.curve_store.as_deref(),
            self.catalogs.curve_point_store.as_deref(),
            self.catalogs.content_tuning_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .represented_primary_specialization_id_like_cpp,
        );
        let catalogs = wow_world_application::AuraApplicationCatalogsLikeCpp::new(
            self.catalogs.mount_capability_store.as_deref(),
            self.catalogs.spell_catalogs.spell_store.as_ref(),
            self.catalogs.chr.classes_store.as_deref(),
            self.catalogs.difficulty_store().map(AsRef::as_ref),
            self.catalogs.items.store.as_ref(),
            self.catalogs.items.stats_store.as_ref(),
            self.catalogs.items.effect_store.as_ref(),
            self.catalogs
                .spell_catalogs
                .spell_shapeshift_form_store
                .as_deref(),
            self.catalogs.map_store().map(AsRef::as_ref),
            wow_world_inventory::ItemModsCatalogsViewLikeCpp::new(
                self.catalogs.items.store.as_ref(),
                self.catalogs.items.stats_store.as_ref(),
                self.catalogs.scaling_stat_distribution_store.as_ref(),
                self.catalogs.scaling_stat_values_store.as_ref(),
                self.catalogs.shield_block_regular_game_table.as_ref(),
                self.catalogs.spell_catalogs.spell_shapeshift_form_store(),
            ),
        );
        wow_world_application::PlayerAuraApplicationCxLikeCpp::new(
            &mut self.spell_state,
            &mut self.inventory,
            presentation,
            control,
            stats,
            item_sets,
            catalogs,
            &self.loot,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_application::AuraApplicationFixtureRefsLikeCpp::new(
                &mut self.fixtures.auras.represented_shapeshift_form_like_cpp,
                &self.fixtures.identity.player_class,
                &self.fixtures.identity.player_level,
                &self.quest_state,
                &self.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                &self.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                &self.fixtures.vehicles.player_transport_login_state_like_cpp,
                &mut self.fixtures.vehicles.player_mount_vehicle_id_like_cpp,
                &mut self
                    .fixtures
                    .vehicles
                    .player_mount_vehicle_accessories_like_cpp,
                &mut self
                    .fixtures
                    .vehicles
                    .player_mount_vehicle_seat_count_like_cpp,
                &mut self
                    .fixtures
                    .vehicles
                    .player_mount_vehicle_usable_seat_count_like_cpp,
                &mut self
                    .fixtures
                    .vehicles
                    .mount_vehicle_remove_requests_like_cpp,
                &mut self
                    .fixtures
                    .pets
                    .mount_pet_control_enable_requests_like_cpp,
                &mut self.fixtures.pets.mount_pet_resummon_requests_like_cpp,
                &mut self
                    .fixtures
                    .vehicles
                    .mount_collision_height_update_requests_like_cpp,
            ),
        )
    }

    pub(in crate::session) fn remove_player_visible_aura_like_cpp(
        &mut self,
        slot: u8,
    ) -> Option<AuraApplication> {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.remove_player_visible_aura_like_cpp(&mut hub, slot)
    }
    pub(in crate::session) fn remove_indoor_outdoor_auras_for_current_position_represented_like_cpp(
        &mut self,
    ) -> usize {
        if !self.config.vmap_indoor_check_like_cpp {
            return 0;
        }

        let Some(is_outdoors) = crate::session::hub_ref(self)
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
        let Some(visible_auras) =
            crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()
        else {
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
        // The admitted application aura owner holds this transition; the World
        // shell only builds the borrowed context (#1263 F6).
        self.player_aura_application_cx_like_cpp()
            .apply_aura_with_effect_mask_like_cpp(
                spell_id,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
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
        self.player_aura_application_cx_like_cpp()
            .apply_aura_with_effect_mask_provenance_and_update_like_cpp(
                spell_id,
                caster_guid,
                duration_ms,
                aura_flags,
                effect_mask,
                provenance,
                send_update,
            )
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
                || crate::session::hub_ref(self).player_has_visible_aura_spell_like_cpp(spell_id)
                    != Some(false)
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
                .catalogs
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
            let Ok(mut previous_spell_id) = u32::try_from(spell_id)
                .map(|spell_id| self.catalogs.prev_spell_in_chain_like_cpp(spell_id))
            else {
                continue;
            };

            while previous_spell_id != 0 {
                let Ok(previous_spell_i32) = i32::try_from(previous_spell_id) else {
                    break;
                };
                if spell_store.is_passive_like_cpp(previous_spell_i32)
                    && crate::session::hub_ref(self)
                        .player_has_visible_aura_spell_like_cpp(previous_spell_i32)
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

                previous_spell_id = self
                    .catalogs
                    .prev_spell_in_chain_like_cpp(previous_spell_id);
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
            difficulty_id: self.core.current_map_difficulty_id_like_cpp(),
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
            difficulty_id: self.core.current_map_difficulty_id_like_cpp(),
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
        self.player_aura_application_cx_like_cpp()
            .remove_aura_like_cpp(slot)
    }
    pub(crate) fn remove_represented_auras_with_attribute0_like_cpp(
        &mut self,
        attribute: u32,
    ) -> usize {
        let Some(spell_store) = self.catalogs.spell_catalogs.spell_store.as_ref() else {
            return 0;
        };
        let Some(visible_auras) =
            crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()
        else {
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
            .core
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
        let Some(visible_auras) =
            crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()
        else {
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
                if self
                    .catalogs
                    .spell_catalogs
                    .spell_store
                    .as_ref()
                    .is_some_and(|store| {
                        store.has_attribute0_like_cpp(aura.spell_id, no_aura_cancel)
                    })
                {
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
        let Some(spell_store) = self.catalogs.spell_catalogs.spell_store.as_ref() else {
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

        let Some(visible_auras) =
            crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()
        else {
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
        let Some(visible_auras) =
            crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()
        else {
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
        // C++ `Creature::Update` owns creature aura lifetimes; this session
        // expires the creature auras it applied through the same wall-clock
        // sweep it uses for its own auras.
        self.tick_represented_creature_auras_like_cpp();
        let Some(visible_auras) =
            crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()
        else {
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
                account = self.core.account_id,
                slot = slot,
                spell_id = spell_id,
                "Aura expired"
            );
        }
    }
    pub(in crate::session) fn remove_represented_stealth_or_invisibility_auras_by_type_like_cpp(
        &mut self,
    ) -> Option<usize> {
        let visible_auras =
            crate::session::hub_ref(self).resolved_player_visible_auras_like_cpp()?;
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

#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura_application/f3_shims.rs"]
mod f3_shims;
