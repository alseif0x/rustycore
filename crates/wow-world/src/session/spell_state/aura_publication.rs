//! Aura slot updates and packets published to the client.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn send_initial_player_auras_like_cpp(&self) {
        let Some(visible_auras) = self.resolved_player_visible_auras_like_cpp() else {
            return;
        };
        if visible_auras.is_empty() {
            return;
        }
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let mut visible: Vec<_> = visible_auras.values().collect();
        visible.sort_by_key(|aura| aura.slot);
        let auras = visible
            .into_iter()
            .map(|aura| {
                Self::player_aura_info_like_cpp(
                    aura,
                    self.player_level_like_cpp(),
                    self.player_map_id_like_cpp(),
                )
            })
            .collect();
        self.send_packet(&wow_packet::packets::misc::AuraUpdate::full_for(
            player_guid,
            auras,
        ));
    }
    /// Validate the exact active effect mask retained by one represented
    /// `AuraApplication`. Whole-spell inertness covers cast-time side effects;
    /// the mask check additionally proves that every live aura bit maps to a
    /// represented, allowlisted `APPLY_AURA` effect.
    pub(in crate::session) fn player_visible_aura_is_spell_hit_inert_like_cpp(
        &self,
        aura: &AuraApplication,
    ) -> bool {
        use wow_data::spell::spell_effect_types;

        let Ok(spell_id) = u32::try_from(aura.spell_id) else {
            return false;
        };
        if aura.effect_mask == 0
            || !self.player_target_spell_is_hit_inert_like_cpp(spell_id, aura.difficulty_id)
        {
            return false;
        }
        let Some(effects) = self.spell_store.as_ref().and_then(|store| {
            store.effects_for_difficulty_like_cpp(
                aura.spell_id,
                aura.difficulty_id,
                self.difficulty_store.as_deref(),
            )
        }) else {
            return false;
        };

        let mut represented_mask = 0_u32;
        for effect in effects {
            let Some(bit) = 1_u32.checked_shl(effect.effect_index) else {
                return false;
            };
            if aura.effect_mask & bit == 0 {
                continue;
            }
            if effect.effect != spell_effect_types::SPELL_EFFECT_APPLY_AURA
                || !Self::player_target_spell_effect_is_hit_inert_like_cpp(effect)
            {
                return false;
            }
            represented_mask |= bit;
        }
        represented_mask == aura.effect_mask
    }
    pub(crate) fn resolved_player_visible_auras_like_cpp(
        &self,
    ) -> Option<HashMap<u8, AuraApplication>> {
        self.player_aura_subsystem_snapshot_like_cpp()
            .map(|auras| auras.runtime_applications_like_cpp().clone())
    }
    pub(crate) fn insert_player_visible_aura_like_cpp(&mut self, aura: AuraApplication) -> bool {
        self.mutate_player_aura_subsystem_like_cpp(|auras| {
            auras.insert_runtime_application_like_cpp(aura);
        })
        .is_some()
    }
    pub(in crate::session) fn player_has_visible_aura_spell_like_cpp(
        &self,
        spell_id: i32,
    ) -> Option<bool> {
        self.player_aura_subsystem_snapshot_like_cpp().map(|auras| {
            auras
                .runtime_applications_like_cpp()
                .values()
                .any(|aura| aura.spell_id == spell_id)
        })
    }
    pub(in crate::session) fn next_player_visible_aura_slot_like_cpp(&self) -> Option<u8> {
        let auras = self.player_aura_subsystem_snapshot_like_cpp()?;
        (0..u8::MAX).find(|slot| !auras.runtime_applications_like_cpp().contains_key(slot))
    }
    pub(in crate::session) fn represented_war_mode_update_zone_aura_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        self.represented_player_flags_value_like_cpp().is_some()
            && !self.represented_player_has_flag_like_cpp(PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP)
    }
    /// C++ `Player::UpdateZone` dispatches OutdoorPvP and Battlefield handlers
    /// after `SpellArea`. Their live control state is not represented here, so
    /// registered zones that can add hit-relevant auras remain fail-closed.
    /// Nagrand and Terokkar Forest are narrow exceptions: their exact source
    /// spells are admitted only when the effective spell projection and
    /// runtime-hook authority prove them hit-inert.
    pub(in crate::session) fn represented_update_zone_script_aura_source_is_hit_inert_like_cpp(
        &self,
    ) -> bool {
        let Some(world_local) = self.player_world_local_state_like_cpp() else {
            return false;
        };
        if !world_local.zone_area_authority_complete {
            return false;
        }
        let zone_id = world_local.zone_id;
        let audited_source_spell_id = match zone_id {
            // OutdoorPvPNA::NA_CAPTURE_BUFF.
            3_518 => Some(33_795),
            // OutdoorPvPTF::TF_CAPTURE_BUFF. The C++ handler is instantiated
            // for map 530 and is therefore reachable here for Terokkar Forest.
            3_519 => Some(33_377),
            _ => None,
        };
        if let Some(spell_id) = audited_source_spell_id {
            return self.player_target_spell_is_hit_inert_like_cpp(
                spell_id,
                self.current_map_difficulty_id_like_cpp(),
            );
        }
        !matches!(
            zone_id,
            // OutdoorPvPSI, the four OutdoorPvPTF dungeon zone IDs,
            // OutdoorPvPZM, OutdoorPvPHP, and BattlefieldWG respectively.
            // OutdoorPvPNA and OutdoorPvPTF on map 530 are audited above; the
            // dungeon IDs remain conservative because this authority does not
            // model C++'s `(Map*, zone)` OutdoorPvP registration key.
            1_377
                | 3_428
                | 3_429
                | 3_791
                | 3_789
                | 3_792
                | 3_790
                | 3_521
                | 3_607
                | 3_717
                | 3_715
                | 3_716
                | 3_483
                | 3_563
                | 3_562
                | 3_713
                | 3_714
                | 3_836
                | 4_197
        )
    }
    /// C++ `Player::UpdateArea` calls `EnablePvpRules` when the current area
    /// or any AreaTable ancestor has `FreeForAllPvP`; that path casts 208682
    /// and 134735. The hierarchy must be complete, non-cyclic, and terminate
    /// at parent zero before Rust can prove those casts absent.
    pub(in crate::session) fn represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        let Some(world_local) = self.player_world_local_state_like_cpp() else {
            return false;
        };
        if !world_local.zone_area_authority_complete {
            return false;
        }
        let Some(areas) = self.area_table_store.as_ref() else {
            return false;
        };
        let mut area_id = world_local.area_id;
        if area_id == 0 {
            return false;
        }

        let mut visited = HashSet::new();
        loop {
            if !visited.insert(area_id) {
                return false;
            }
            let Some(area) = areas.get(area_id) else {
                return false;
            };
            if area.flags & AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP != 0 {
                return false;
            }
            area_id = u32::from(area.parent_area_id);
            if area_id == 0 {
                return true;
            }
        }
    }
    pub(in crate::session) fn send_on_cancel_expected_vehicle_ride_aura_like_cpp(&mut self) {
        #[cfg(test)]
        {
            self.mount_cancel_expected_vehicle_aura_packets_like_cpp = self
                .mount_cancel_expected_vehicle_aura_packets_like_cpp
                .saturating_add(1);
        }
        self.send_packet(&wow_packet::packets::vehicle::OnCancelExpectedRideVehicleAura);
    }
    pub(in crate::session) fn send_aura_update_applied(
        &self,
        spell_id: i32,
        slot: u8,
        caster: ObjectGuid,
        duration: u32,
        flags: u32,
        effect_mask: u32,
    ) {
        let Some(target_guid) = self.player_guid() else {
            return;
        };
        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.current_map_difficulty_id_like_cpp(),
            caster_guid: caster,
            slot,
            duration_total: duration,
            duration_remaining: duration,
            stack_count: 1,
            aura_flags: flags,
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
        self.send_packet(&wow_packet::packets::misc::AuraUpdate {
            unit_guid: target_guid,
            update_all: false,
            auras: vec![Self::player_aura_info_like_cpp(
                &aura,
                self.player_level_like_cpp(),
                self.player_map_id_like_cpp(),
            )],
        });
    }
    #[cfg(test)]
    pub(crate) fn visible_aura_slot_for_spell_like_cpp(&self, spell_id: i32) -> Option<u8> {
        self.resolved_player_visible_auras_like_cpp()
            .expect("test Player aura owner must resolve")
            .values()
            .find_map(|aura| (aura.spell_id == spell_id).then_some(aura.slot))
    }
    pub(in crate::session) fn update_represented_flight_flags_for_flight_aura_like_cpp(
        &mut self,
        apply: bool,
    ) {
        let should_enable = if apply {
            true
        } else {
            let Some(has_fly) = self
                .resolved_has_represented_aura_effect_like_cpp(RepresentedAuraEffectLikeCpp::Fly)
            else {
                return;
            };
            let Some(has_mounted_flight_speed) = self
                .resolved_has_represented_aura_effect_like_cpp(
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
                )
            else {
                return;
            };
            has_fly || has_mounted_flight_speed
        };
        self.set_represented_can_swim_to_fly_transition_like_cpp(should_enable);
        let can_fly_changed = self.set_represented_can_fly_like_cpp(should_enable);
        if !should_enable && can_fly_changed {
            self.move_represented_player_fall_like_cpp();
        }
    }
}
