//! Represented mount auras and their capability speed effects.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn apply_represented_mounted_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        let mounted_amount =
            self.calculate_represented_mounted_aura_amount_like_cpp(spell_id, effect);
        let selected_display_id = u32::try_from(spell_id)
            .ok()
            .and_then(|spell_id| self.select_represented_mount_aura_display_like_cpp(spell_id))
            .unwrap_or(0);
        let creature_entry = u32::try_from(effect.effect_misc_value_1).unwrap_or(0);
        let creature_template_mount =
            self.represented_mount_creature_template_fallback_like_cpp(creature_entry);
        let display_id = if selected_display_id != 0 {
            selected_display_id
        } else {
            creature_template_mount
                .map(|(display_id, _)| display_id)
                .unwrap_or(0)
        };
        let vehicle_id = creature_template_mount
            .map(|(_, vehicle_id)| vehicle_id)
            .unwrap_or(0);

        let slot = self
            .next_player_visible_aura_slot_like_cpp()
            .ok_or("No free aura slots or missing Player aura owner")?;

        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.current_map_difficulty_id_like_cpp(),
            caster_guid,
            slot,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << effect.effect_index,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::Mounted),
            represented_amount: mounted_amount,
            represented_effect_amounts: represented_aura_effect_amounts_like_cpp(effect),
            represented_misc_value: Some(effect.effect_misc_value_1),
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        };

        if !self.set_player_mount_presentation_like_cpp(display_id, true) {
            return Err("Missing Player presentation owner");
        }
        if !self.insert_player_visible_aura_like_cpp(aura) {
            return Err("Missing Player aura owner");
        }
        if self.create_player_mount_vehicle_kit_like_cpp(vehicle_id, creature_entry) {
            #[cfg(test)]
            {
                self.mount_vehicle_create_requests_like_cpp = self
                    .mount_vehicle_create_requests_like_cpp
                    .saturating_add(1);
            }
            self.send_set_vehicle_rec_id_like_cpp(vehicle_id);
            self.send_on_cancel_expected_vehicle_ride_aura_like_cpp();
        }
        #[cfg(test)]
        {
            self.mount_pet_control_disable_requests_like_cpp = self
                .mount_pet_control_disable_requests_like_cpp
                .saturating_add(1);
        }
        self.disable_pet_controls_on_mount_like_cpp(
            wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP,
            wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
        );
        #[cfg(test)]
        {
            self.mount_collision_height_update_requests_like_cpp = self
                .mount_collision_height_update_requests_like_cpp
                .saturating_add(1);
        }
        self.update_player_collision_height_like_cpp();
        self.send_movement_set_collision_height_like_cpp(
            wow_packet::packets::movement::UPDATE_COLLISION_HEIGHT_REASON_MOUNT_LIKE_CPP,
        );
        self.apply_represented_mount_capability_speed_aura_like_cpp(mounted_amount, caster_guid);

        self.send_aura_update_applied(
            spell_id,
            slot,
            caster_guid,
            0,
            0x0000_0001,
            1u32 << effect.effect_index,
        );
        self.send_represented_mount_unit_update_like_cpp(display_id);

        Ok(())
    }
    fn apply_represented_mount_capability_speed_aura_like_cpp(
        &mut self,
        mount_capability_id: i32,
        caster_guid: ObjectGuid,
    ) {
        let Some(mod_spell_aura_id) =
            self.represented_mount_capability_mod_spell_like_cpp(mount_capability_id)
        else {
            return;
        };

        let Some(spell_info) = self
            .spell_store()
            .and_then(|store| store.get(mod_spell_aura_id))
            .cloned()
        else {
            return;
        };

        for effect in spell_info.effects().iter().filter(|effect| {
            effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
        }) {
            let represented_effect = match effect.effect_aura {
                aura if aura == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED => {
                    RepresentedAuraEffectLikeCpp::MountedSpeed
                }
                aura if aura == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS => {
                    RepresentedAuraEffectLikeCpp::MountedSpeedAlways
                }
                aura if aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK =>
                {
                    RepresentedAuraEffectLikeCpp::MountedSpeedNotStack
                }
                aura if aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED =>
                {
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeed
                }
                aura if aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS =>
                {
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeedAlways
                }
                aura if aura == wow_data::spell::aura_types::SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK => {
                    RepresentedAuraEffectLikeCpp::FlightSpeedNotStack
                }
                _ => continue,
            };

            if self
                .apply_represented_aura_modifier_like_cpp(
                    mod_spell_aura_id,
                    caster_guid,
                    effect,
                    represented_effect,
                    0,
                )
                .is_ok()
            {
                self.recompute_represented_mounted_speed_rates_like_cpp();
            }
        }
    }
    pub(in crate::session) fn remove_represented_mount_capability_speed_auras_like_cpp(
        &mut self,
        mount_capability_id: i32,
    ) {
        let Some(mod_spell_aura_id) =
            self.represented_mount_capability_mod_spell_like_cpp(mount_capability_id)
        else {
            return;
        };
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return;
        };
        let slots: Vec<u8> = visible_auras
            .values()
            .filter_map(|aura| (aura.spell_id == mod_spell_aura_id).then_some(aura.slot))
            .collect();
        for slot in slots {
            let _ = self.remove_aura(slot);
        }
    }
    #[cfg(test)]
    pub(crate) fn apply_represented_mounted_aura_for_test_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        self.apply_represented_mounted_aura_like_cpp(spell_id, caster_guid, effect)
    }
    pub(crate) fn remove_represented_mount_auras_cancelable_like_cpp(&mut self) -> usize {
        self.remove_represented_cancelable_auras_by_effect_like_cpp(
            RepresentedAuraEffectLikeCpp::Mounted,
        )
    }
    pub(in crate::session) fn remove_represented_mounted_auras_by_type_like_cpp(&mut self) -> bool {
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return false;
        };
        let mounted_slots: Vec<u8> = visible_auras
            .iter()
            .filter_map(|(slot, aura)| {
                (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted))
                    .then_some(*slot)
            })
            .collect();
        for slot in mounted_slots {
            let _ = self.remove_aura(slot);
        }

        let Some(mounted) = self.resolved_player_mounted_like_cpp() else {
            return false;
        };
        if mounted {
            if !self.set_player_mount_presentation_like_cpp(0, false) {
                return false;
            }
            let _ = self.mutate_player_mount_vehicle_kit_like_cpp(|kit| *kit = None);
            #[cfg(test)]
            {
                self.player_mount_vehicle_id_like_cpp = 0;
                self.player_mount_vehicle_accessories_like_cpp.clear();
                self.player_mount_vehicle_seat_count_like_cpp = 0;
                self.player_mount_vehicle_usable_seat_count_like_cpp = 0;
            }
        }
        true
    }
}
