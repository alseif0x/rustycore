//! Represented cast-state helpers outside the #589 request lifecycle.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) async fn execute_represented_spell_click_clickee_caster_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        cast: &RepresentedSpellClickCastLikeCpp,
        player_guid: ObjectGuid,
        clickee_owner_guid: Option<ObjectGuid>,
    ) -> RepresentedSpellClickClickeeCasterOutcomeLikeCpp {
        use wow_packet::packets::spell::{SpellCastVisual, SpellGoPkt, SpellTargetData};

        if cast.caster != RepresentedSpellClickUnitRefLikeCpp::Clickee {
            return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedCaster;
        }
        let target_guid = match cast.target {
            RepresentedSpellClickUnitRefLikeCpp::Clicker => player_guid,
            RepresentedSpellClickUnitRefLikeCpp::Clickee => creature_guid,
            RepresentedSpellClickUnitRefLikeCpp::Owner => {
                return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedTarget;
            }
        };
        match cast.original_caster {
            RepresentedSpellClickUnitRefLikeCpp::Clicker => {}
            RepresentedSpellClickUnitRefLikeCpp::Owner
                if clickee_owner_guid == Some(player_guid) => {}
            _ => {
                return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedOriginalCaster;
            }
        }

        let Ok(spell_id) = i32::try_from(cast.spell_id) else {
            return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Failed;
        };
        let Some(spell_info) = self
            .spell_store()
            .and_then(|store| store.get(spell_id))
            .cloned()
        else {
            return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Failed;
        };
        let Some(damage_amount) =
            represented_spell_click_school_damage_amount_like_cpp(&spell_info)
        else {
            return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedCaster;
        };

        let Some(cast_id) = self.next_represented_spell_cast_guid_like_cpp(spell_id) else {
            return RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Failed;
        };
        self.send_packet(&SpellGoPkt {
            cast_data: Default::default(),
            caster: creature_guid,
            cast_id,
            original_cast_id: cast_id,
            spell_id,
            visual: SpellCastVisual::default(),
            cast_flags: 0,
            cast_flags_ex: 0,
            cast_time_ms: crate::session_rules::game_time_ms_like_cpp(),
            target: SpellTargetData {
                flags: 0x2,
                unit: target_guid,
                item: ObjectGuid::EMPTY,
                ..SpellTargetData::default()
            },
            hit_targets: vec![target_guid],
            miss_targets: Vec::new(),
        });

        let result = if target_guid == player_guid {
            self.apply_represented_spell_click_creature_damage_to_clicker_like_cpp(
                spell_id,
                player_guid,
                damage_amount,
            )
            .await
        } else {
            self.apply_represented_spell_click_creature_damage_to_clickee_like_cpp(
                spell_id,
                creature_guid,
                damage_amount,
            )
            .await
        };
        match result {
            Ok(()) => RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Executed,
            Err(_) => RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Failed,
        }
    }
    pub(in crate::session) fn record_cast_character_spell_cooldown_like_cpp(
        &mut self,
        spell_id: i32,
        cooldown_ms: u32,
    ) {
        if !self
            .player_spell_history_snapshot_like_cpp()
            .is_some_and(|history| history.cooldowns_loaded)
            || cooldown_ms == 0
        {
            return;
        }
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return;
        };
        let cooldown_secs = i64::from(cooldown_ms.saturating_add(999) / 1_000);
        if cooldown_secs == 0 {
            return;
        }
        self.record_loaded_character_spell_cooldown_like_cpp(
            spell_id,
            0,
            unix_now().saturating_add(cooldown_secs),
            0,
            0,
        );
    }
    #[cfg(test)]
    pub fn set_player_create_cast_spell_store_like_cpp(
        &mut self,
        store: Arc<PlayerCreateInfoCastSpellStoreLikeCpp>,
    ) {
        self.player_create_cast_spell_store_like_cpp = Some(store);
    }
    /// Conservative C++ `SpellArea::IsFitToRequirements` projection used only
    /// to decide whether an unrepresented AUTOCAST aura could exist. A proven
    /// mismatch returns false; battleground and spell-specific conditions are
    /// deliberately left as "could fit" because Rust does not own them here.
    fn represented_spell_area_could_autocast_for_player_like_cpp(
        &self,
        spell_area: &SpellAreaLikeCpp,
    ) -> bool {
        if spell_area.gender != wow_data::GENDER_NONE_LIKE_CPP
            && spell_area.gender != self.player_gender_like_cpp()
        {
            return false;
        }

        if spell_area.race_mask != 0 {
            let race_mask =
                wow_data::skill::race_mask_for_race_like_cpp(self.player_race_like_cpp());
            if race_mask != 0 && spell_area.race_mask & race_mask as u64 == 0 {
                return false;
            }
        }

        if spell_area.area_id != 0 {
            let Some(world_local) = self.player_world_local_state_like_cpp() else {
                return false;
            };
            if !world_local.has_zone_area_authority_like_cpp() {
                return true;
            }
            let (zone_id, area_id) = world_local.zone_area_like_cpp();
            if spell_area.area_id != zone_id && spell_area.area_id != area_id {
                return false;
            }
        }

        if spell_area.quest_start != 0 {
            let Some(status) =
                self.represented_spell_area_quest_status_like_cpp(spell_area.quest_start)
            else {
                return true;
            };
            let Some(status_mask) = 1_u32.checked_shl(u32::from(status)) else {
                return true;
            };
            if status_mask & spell_area.quest_start_status == 0 {
                return false;
            }
        }

        if spell_area.quest_end != 0 {
            let Some(status) =
                self.represented_spell_area_quest_status_like_cpp(spell_area.quest_end)
            else {
                return true;
            };
            let Some(status_mask) = 1_u32.checked_shl(u32::from(status)) else {
                return true;
            };
            if status_mask & spell_area.quest_end_status == 0 {
                return false;
            }
        }

        if spell_area.aura_spell != 0 {
            let Some(required_spell_id) = spell_area.aura_spell.checked_abs() else {
                return true;
            };
            let Some(has_aura) = self.player_has_visible_aura_spell_like_cpp(required_spell_id)
            else {
                return false;
            };
            if (spell_area.aura_spell > 0 && !has_aura) || (spell_area.aura_spell < 0 && has_aura) {
                return false;
            }
        }

        true
    }
    pub(in crate::session) fn represented_spell_area_autocast_source_is_empty_like_cpp(
        &self,
    ) -> bool {
        self.spell_catalogs
            .spell_area_store
            .as_ref()
            .is_some_and(|store| {
                store.areas_like_cpp().iter().all(|spell_area| {
                    spell_area.flags & SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP == 0
                        || !self
                            .represented_spell_area_could_autocast_for_player_like_cpp(spell_area)
                })
            })
    }
    /// Represented C++ first-login `PlayerInfo::castSpells[GetCreateMode()]`.
    ///
    /// C++ executes these casts after clearing `AT_LOGIN_FIRST` and before
    /// `CONFIG_START_ALL_EXPLORED` / `CONFIG_START_ALL_REP`. This slice uses the
    /// represented spell executor; full triggered-cast semantics still depend on
    /// the remaining Spell runtime port.
    pub(crate) async fn apply_represented_first_login_cast_spells_with_catalogs_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(create_mode) = self.player_create_mode_like_cpp() else {
            return 0;
        };
        let spells = player_bootstrap
            .cast_spells
            .cast_spells_like_cpp(
                self.player_race_like_cpp(),
                self.player_class_like_cpp(),
                create_mode,
            )
            .to_vec();

        let mut cast_count = 0usize;
        for spell_id in spells {
            // C++ `CharacterHandler.cpp` casts the create-mode spells with
            // `CastSpell(pCurrChar, spellId, true)`, i.e. TRIGGERED_FULL_MASK:
            // the cast is triggered and ignores the global cooldown.
            if self
                .execute_server_triggered_spell_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    spell_id as i32,
                    player_guid,
                    SpellCastMetadata {
                        cast_flags: CAST_FLAG_PENDING_LIKE_CPP,
                        triggered_ignores_global_cooldown_like_cpp: true,
                        ..SpellCastMetadata::default()
                    },
                )
                .await
                .is_ok()
            {
                cast_count += 1;
            }
        }

        cast_count
    }
    #[cfg(test)]
    pub(crate) async fn apply_represented_first_login_cast_spells_like_cpp(&mut self) -> usize {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        self.apply_represented_first_login_cast_spells_with_catalogs_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            &player_bootstrap,
        )
        .await
    }
    pub(crate) fn broadcast_to_movement_set_like_cpp(&self, bytes: Vec<u8>, _include_self: bool) {
        self.broadcast_to_movement_set_in_range_like_cpp(
            bytes,
            crate::map_manager::VISIBILITY_RADIUS,
        );
    }
    pub(crate) fn broadcast_to_movement_set_realm_like_cpp(
        &self,
        bytes: Vec<u8>,
        _include_self: bool,
    ) {
        self.broadcast_to_movement_set_in_range_and_connection_like_cpp(
            bytes,
            crate::map_manager::VISIBILITY_RADIUS,
            true,
        );
    }
    pub(crate) fn broadcast_to_movement_set_in_range_like_cpp(&self, bytes: Vec<u8>, range: f32) {
        self.broadcast_to_movement_set_in_range_and_connection_like_cpp(bytes, range, false);
    }
    fn broadcast_to_movement_set_in_range_and_connection_like_cpp(
        &self,
        bytes: Vec<u8>,
        range: f32,
        realm_connection: bool,
    ) {
        let (Some(guid), Some(registry)) = (self.player_guid(), self.player_registry()) else {
            return;
        };
        let Some(source_position) = self.player_position_like_cpp() else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        for registration in registry.movement_recipients_within_range(
            guid,
            map_id,
            instance_id,
            source_position,
            range,
        ) {
            let command = SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid: guid,
                map_id,
                instance_id,
                packet_bytes: bytes.clone(),
            };
            let command = if realm_connection {
                SessionCommand::SendRealmIfVisibleLikeCpp(command)
            } else {
                SessionCommand::SendIfVisibleLikeCpp(command)
            };
            let _ = registry.try_send_current_command(registration, command);
        }
    }
    pub(in crate::session) fn represented_login_passive_spell_cast_gate_like_cpp(
        &self,
        spell_id: i32,
    ) -> bool {
        let Some(spell_store) = self.spell_catalogs.spell_store.as_ref() else {
            return false;
        };
        let (stances, _) = spell_store.shapeshift_masks_like_cpp(spell_id);
        let Some(form) = self.represented_shapeshift_form_like_cpp() else {
            return false;
        };
        let stance_mask = form
            .checked_sub(1)
            .and_then(|shift| 1u64.checked_shl(shift))
            .unwrap_or(0);
        let need_cast = stances == 0
            || (form != 0 && (stances & stance_mask) != 0)
            || (form == 0
                && spell_store.has_attribute2_like_cpp(
                    spell_id,
                    wow_data::spell::attributes::SPELL_ATTR2_ALLOW_WHILE_NOT_SHAPESHIFTED_CASTER_FORM,
                ));

        if !need_cast {
            return false;
        }

        let Ok(spell_id_u32) = u32::try_from(spell_id) else {
            return false;
        };
        let caster_aura_state = self
            .spell_catalogs
            .spell_aura_restrictions_store
            .as_ref()
            .and_then(|store| {
                store
                    .entries_for_spell_id_like_cpp(spell_id_u32)
                    .find(|entry| entry.difficulty_id == 0 || entry.difficulty_id == u8::MAX)
                    .or_else(|| store.entries_for_spell_id_like_cpp(spell_id_u32).next())
            })
            .map(|entry| entry.caster_aura_state)
            .unwrap_or(0);

        caster_aura_state == 0
            || self.represented_has_aura_state_like_cpp(u32::from(caster_aura_state))
    }
    pub(crate) fn represented_cast_spell_info_like_cpp(
        &self,
        spell_info: &wow_data::SpellInfo,
    ) -> wow_data::SpellInfo {
        let override_spells = self.represented_override_spells_like_cpp();
        if let Some(overrides) = override_spells.get(&spell_info.spell_id) {
            if let Some(store) = self.spell_store() {
                for new_spell_id in overrides {
                    if let Some(new_info) = store.get(*new_spell_id) {
                        return new_info.clone();
                    }
                }
            }
        }

        spell_info.clone()
    }
    #[cfg(test)]
    pub(crate) fn represented_can_duel_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedCanDuelSpellCastLikeCpp] {
        &self.represented_can_duel_spell_casts_like_cpp
    }
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_talent_respec_visual_spell_cast_like_cpp(
        &mut self,
        cast: RepresentedTalentRespecVisualSpellCastLikeCpp,
    ) {
        #[cfg(test)]
        self.represented_talent_respec_visual_spell_casts_like_cpp
            .push(cast);
    }
    #[cfg(test)]
    pub(crate) fn represented_talent_respec_visual_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedTalentRespecVisualSpellCastLikeCpp] {
        &self.represented_talent_respec_visual_spell_casts_like_cpp
    }
}
