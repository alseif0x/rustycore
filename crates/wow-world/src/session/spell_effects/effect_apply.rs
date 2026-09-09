//! Represented application of individual spell effects.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// C++ `Spell::EffectCreateBattlePet` / `SPELL_EFFECT_UNCAGE_BATTLEPET`.
    /// The Login DB pet/receipt commit precedes the Character DB `DestroyItem`
    /// phase so an interrupted cross-database operation can resume idempotently.
    pub(in crate::session) async fn apply_uncage_battle_pet_effect_like_cpp(
        &mut self,
        spell_id: i32,
        cast_id: ObjectGuid,
        metadata: SpellCastMetadata,
    ) {
        let Some(modifiers) = metadata.cast_item_battle_pet_modifiers else {
            return;
        };

        let Some(request_key) = BattlePetAddRequestKeyLikeCpp::from_source_guid_bytes_like_cpp(
            modifiers.source_item_guid.to_raw_bytes(),
        ) else {
            warn!(
                account = self.account_id,
                species = modifiers.species_id,
                "Durable battle-pet uncage lacks a stable source-item identity"
            );
            return;
        };
        let request_already_committed = match self
            .battle_pet_add_request_committed_like_cpp(request_key)
            .await
        {
            Ok(committed) => committed,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    species = modifiers.species_id,
                    ?error,
                    "Failed to reconcile a durable battle-pet uncage request"
                );
                return;
            }
        };

        if request_already_committed {
            if metadata.cast_item_entry.is_some() {
                let _ = self
                    .destroy_uncaged_battle_pet_item_durable_like_cpp(modifiers.source_item_guid)
                    .await;
            }
            return;
        }

        if let Some(cast_item_entry) = metadata.cast_item_entry
            && self
                .uncage_cast_item_still_matches_like_cpp(cast_item_entry, modifiers)
                .is_none()
        {
            warn!(
                account = self.account_id,
                item_guid = modifiers.source_item_guid.counter(),
                cast_item_entry,
                "Battle-pet uncage source item disappeared or changed before effect execution"
            );
            return;
        }

        let Some(species_entry) = self.battle_pet_species_entry_like_cpp(modifiers.species_id)
        else {
            return;
        };

        let creature_id = u32::try_from(species_entry.creature_id).unwrap_or_default();
        let Some(max_pet_level) = self.battle_pet_max_pet_level_like_cpp() else {
            return;
        };
        if max_pet_level < modifiers.level {
            self.battle_pet_send_error_like_cpp(
                wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::TooHighLevelToUncage,
                creature_id,
            );
            self.send_packet(&wow_packet::packets::spell::CastFailed {
                cast_id,
                spell_id,
                visual: wow_packet::packets::spell::SpellCastVisual::default(),
                reason: SpellCastResult::CantAddBattlePet as i32,
                fail_arg1: 0,
                fail_arg2: 0,
            });
            return;
        }

        let Some(has_max_pet_count) =
            self.battle_pet_has_max_pet_count_like_cpp(modifiers.species_id, self.player_guid())
        else {
            return;
        };
        if has_max_pet_count {
            self.battle_pet_send_error_like_cpp(
                wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::CantHaveMorePetsOfType,
                creature_id,
            );
            self.send_packet(&wow_packet::packets::spell::CastFailed {
                cast_id,
                spell_id,
                visual: wow_packet::packets::spell::SpellCastVisual::default(),
                reason: SpellCastResult::CantAddBattlePet as i32,
                fail_arg1: 0,
                fail_arg2: 0,
            });
            return;
        }

        let breed = (modifiers.breed_data & 0x00FF_FFFF) as u16;
        let quality = ((modifiers.breed_data >> 24) & 0xFF) as u8;
        if let Err(error) = self
            .battle_pet_try_add_pet_durable_like_cpp(
                request_key.as_bytes(),
                modifiers.species_id,
                modifiers.display_id,
                breed,
                quality,
                modifiers.level,
            )
            .await
        {
            warn!(
                account = self.account_id,
                species = modifiers.species_id,
                ?error,
                "Durable battle-pet uncage add was rejected"
            );
            self.battle_pet_send_error_like_cpp(
                wow_packet::packets::misc::BattlePetErrorCodeLikeCpp::CantHaveMorePetsOfType,
                creature_id,
            );
            self.send_packet(&wow_packet::packets::spell::CastFailed {
                cast_id,
                spell_id,
                visual: wow_packet::packets::spell::SpellCastVisual::default(),
                reason: SpellCastResult::CantAddBattlePet as i32,
                fail_arg1: 0,
                fail_arg2: 0,
            });
            return;
        }
        if let (Some(player_guid), Some(player_position)) =
            (self.player_guid(), self.player_position_like_cpp())
        {
            self.send_packet(&wow_packet::packets::spell::PlaySpellVisual::self_target(
                player_guid,
                player_position,
                BATTLE_PET_SPELL_VISUAL_UNCAGE_PET_LIKE_CPP,
            ));
        }
        if metadata.cast_item_entry.is_some() {
            let _ = self
                .destroy_uncaged_battle_pet_item_durable_like_cpp(modifiers.source_item_guid)
                .await;
        }
    }
    pub(in crate::session) async fn apply_effect_teleport_units_like_cpp(
        &mut self,
        effect: &wow_data::SpellEffectInfo,
        target_guid: ObjectGuid,
        target_data: &SpellTargetData,
    ) {
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS {
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if target_guid != player_guid {
            // C++ also supports same-map creature NearTeleportTo here. Rust's
            // represented spell path only owns the player target today; do not
            // fabricate creature movement until Unit target selection is ported.
            return;
        }

        let Some(destination) = target_data.dst_location.as_ref().map(|dst| dst.position) else {
            debug!(
                account = self.account_id,
                effect_index = effect.effect_index,
                "Spell::EffectTeleportUnits represented no-op: missing destination"
            );
            return;
        };

        let target_map = target_data
            .map_id
            .and_then(|map_id| u32::try_from(map_id).ok())
            .unwrap_or_else(|| u32::from(self.player_map_id_like_cpp()));
        let mut destination = destination;
        if destination.orientation == 0.0 {
            if let Some(player_position) = self.player_position_like_cpp() {
                destination.orientation = player_position.orientation;
            }
        }

        self.teleport_to(target_map, destination).await;
    }
    pub(in crate::session) async fn apply_effect_bind_like_cpp(
        &mut self,
        effect: &wow_data::SpellEffectInfo,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        target_data: &SpellTargetData,
    ) {
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND {
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if target_guid != player_guid {
            return;
        }

        let Some((_, current_area_id)) = self.player_zone_area_like_cpp() else {
            return;
        };
        let area_id = Self::bind_area_id_like_cpp(effect.effect_misc_value_1, current_area_id);

        let Some(current_position) = self.player_position_like_cpp() else {
            return;
        };
        let position = target_data
            .dst_location
            .as_ref()
            .map(|dst| dst.position)
            .unwrap_or(current_position);
        let map_id = target_data
            .map_id
            .and_then(|map_id| u32::try_from(map_id).ok())
            .unwrap_or_else(|| u32::from(self.player_map_id_like_cpp()));

        self.set_homebind_like_cpp(
            caster_guid,
            RepresentedHomebindLikeCpp {
                map_id,
                area_id,
                position,
            },
        );
    }
    /// Live bounded C++ `Spell::EffectAddFarsight` consumer.
    ///
    /// Source-of-truth: canonical `wow_map::Map::map_objects`; this helper never
    /// creates fallback/session mirror objects. C++ anchors:
    /// `SpellEffects.cpp:2237-2261`, `SpellInfo.cpp:653-692`,
    /// `SpellInfo.cpp:3894-3910`, `Spell.cpp:133-205`.
    pub(crate) fn apply_effect_add_farsight_like_cpp(
        &mut self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
        spell_visual_id: u32,
        cast_time_ms: u32,
    ) -> Option<wow_map::map::FarsightDynamicObjectCreateOutcomeLikeCpp> {
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_FARSIGHT {
            return None;
        }
        let dest = target_data.dst_location?.position;
        let player_guid = self.player_guid()?;
        let spell_id_u32 = u32::try_from(spell_id).ok()?;
        let spell_visual_id_i32 = i32::try_from(spell_visual_id).ok()?;
        let duration_index = self
            .spell_misc_store
            .as_deref()
            .and_then(|store| store.get_by_spell_id(spell_id_u32))
            .map(|entry| u32::from(entry.duration_index))
            .unwrap_or(0);
        let duration_ms =
            spell_duration_ms_like_cpp(duration_index, self.spell_duration_store.as_deref());
        let radius = spell_effect_radius_like_cpp(
            effect.effect_radius_index_1,
            self.spell_radius_store.as_deref(),
        );
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let map_id = u32::from(self.player_map_id_like_cpp());
        let mut instance_id = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if instance_id.is_none() && managed.map().get_typed_player(player_guid).is_some() {
                instance_id = Some(managed.instance_id());
            }
        });
        let managed = manager.find_map_mut(map_id, instance_id.unwrap_or(0))?;
        Some(managed.map_mut().create_farsight_dynamic_object_like_cpp(
            player_guid,
            spell_id_u32,
            spell_visual_id_i32,
            dest,
            radius,
            duration_ms,
            u64::from(cast_time_ms),
            self.realm_id,
            player_guid.server_id(),
        ))
    }
    /// C++ `Spell::EffectStuck` (`SpellEffects.cpp:3265-3303`).
    pub(in crate::session) async fn apply_stuck_effect_like_cpp(&mut self) {
        const HEARTHSTONE_SPELL_ID_LIKE_CPP: i32 = 8690;

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.represented_cast_unstuck_enabled_like_cpp {
            return;
        }
        if self.resolved_is_in_taxi_flight_like_cpp() != Some(false) {
            return;
        }

        match self.resolved_player_is_alive_like_cpp() {
            None => return,
            Some(false) => {
                let Some(resurrection) = self.player_resurrection_state_snapshot_like_cpp() else {
                    return;
                };
                if !resurrection.death_timer_active {
                    self.set_player_ghost_flag_like_cpp(true);
                    #[cfg(test)]
                    {
                        self.represented_repop_at_graveyard_count =
                            self.represented_repop_at_graveyard_count.saturating_add(1);
                    }
                }
                return;
            }
            Some(true) => {}
        }

        let Some(hearthstone_last_cast) =
            self.spell_last_cast_time_like_cpp(HEARTHSTONE_SPELL_ID_LIKE_CPP)
        else {
            return;
        };
        if hearthstone_last_cast.is_some() {
            self.set_player_alive_like_cpp(false);
            let values_update = self.mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().set_health(0);
                player.values_update(true)
            });
            if let Some(values_update) = values_update {
                self.send_player_values_update_like_cpp(&values_update);
            } else {
                self.send_player_health_values_update_like_cpp(player_guid, 0);
            }
            return;
        }

        if let Some(homebind) = self.represented_homebind_like_cpp() {
            self.teleport_to(homebind.map_id, homebind.position).await;
        }

        let Some(hearthstone_cooldown_ms) = self
            .spell_store
            .as_deref()
            .and_then(|store| store.get(HEARTHSTONE_SPELL_ID_LIKE_CPP))
            .map(|spell_info| spell_info.recovery_time_ms.max(spell_info.cooldown_ms))
        else {
            return;
        };
        if self
            .mutate_cast_execution_like_cpp(|state| {
                state
                    .last_cast_time_per_spell
                    .insert(HEARTHSTONE_SPELL_ID_LIKE_CPP, Instant::now());
            })
            .is_none()
        {
            return;
        }
        self.record_cast_character_spell_cooldown_like_cpp(
            HEARTHSTONE_SPELL_ID_LIKE_CPP,
            hearthstone_cooldown_ms,
        );
        self.send_packet(&wow_packet::packets::spell::CooldownEvent {
            spell_id: HEARTHSTONE_SPELL_ID_LIKE_CPP,
            is_pet: false,
        });
    }
}
