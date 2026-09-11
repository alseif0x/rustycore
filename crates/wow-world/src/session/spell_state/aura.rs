//! Represented aura state owned at the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn player_aura_subsystem_snapshot_like_cpp(
        &self,
    ) -> Option<wow_entities::AuraSubsystem> {
        let canonical =
            self.with_owned_player_like_cpp(|player| player.unit().subsystems().auras.clone());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            let mut auras = wow_entities::AuraSubsystem::default();
            auras.set_persisted_player_aura_authority_complete_like_cpp(
                self.player_aura_authority_complete_like_cpp,
            );
            if self.player_spell_hit_aura_authority_tombstoned_like_cpp {
                auras.tombstone_spell_hit_aura_authority_like_cpp();
            }
            for aura in self.visible_auras.values().cloned() {
                auras.insert_runtime_application_like_cpp(aura);
            }
            for (&slot, snapshot) in &self.canonical_threat_aura_snapshots_like_cpp {
                auras.insert_threat_snapshot_like_cpp(slot, snapshot.clone());
            }
            return Some(auras);
        }
        canonical
    }
    pub(in crate::session) fn mutate_player_aura_subsystem_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::AuraSubsystem) -> R,
    ) -> Option<R> {
        let mut mutate = Some(mutate);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            let mut auras = self.player_aura_subsystem_snapshot_like_cpp()?;
            let result =
                mutate
                    .take()
                    .expect("test Player aura mutation executes once")(&mut auras);
            self.player_aura_authority_complete_like_cpp =
                auras.persisted_player_aura_authority_complete_like_cpp();
            self.player_spell_hit_aura_authority_tombstoned_like_cpp =
                auras.spell_hit_aura_authority_tombstoned_like_cpp();
            self.visible_auras = auras.runtime_applications_like_cpp().clone();
            self.canonical_threat_aura_snapshots_like_cpp.clear();
            for slot in 0..=u8::MAX {
                if let Some(snapshot) = auras.threat_snapshot_like_cpp(slot) {
                    self.canonical_threat_aura_snapshots_like_cpp
                        .insert(slot, snapshot.clone());
                }
            }
            return Some(result);
        }
        self.with_owned_player_mut_like_cpp(|player| {
            mutate.take().expect("Player aura mutation executes once")(
                &mut player.unit_mut().subsystems_mut().auras,
            )
        })
    }
    pub(crate) fn set_player_aura_authority_complete_like_cpp(&mut self, complete: bool) -> bool {
        self.mutate_player_aura_subsystem_like_cpp(|auras| {
            auras.set_persisted_player_aura_authority_complete_like_cpp(complete);
        })
        .is_some()
    }
    #[cfg(test)]
    pub(crate) fn player_aura_authority_complete_like_cpp(&self) -> bool {
        self.player_aura_authority_complete_like_cpp
    }
    pub(crate) fn resolved_player_aura_authority_complete_like_cpp(&self) -> Option<bool> {
        self.player_aura_subsystem_snapshot_like_cpp()
            .map(|auras| auras.persisted_player_aura_authority_complete_like_cpp())
    }
    pub(crate) fn tombstone_player_spell_hit_aura_authority_like_cpp(&mut self) {
        let _ = self.mutate_player_aura_subsystem_like_cpp(|auras| {
            auras.tombstone_spell_hit_aura_authority_like_cpp();
        });
    }
    fn represented_active_glyph_aura_source_is_empty_like_cpp(&self) -> bool {
        self.player_talent_runtime_snapshot_like_cpp()
            .filter(|runtime| runtime.glyphs_loaded_like_cpp())
            .map(|runtime| {
                let active_group = runtime.active_group_like_cpp();
                (0..wow_entities::PLAYER_MAX_GLYPH_SLOTS_LIKE_CPP as u8)
                    .all(|slot| runtime.glyph_like_cpp(active_group, slot) == Some(0))
            })
            .unwrap_or(false)
    }
    /// C++ `_LoadTraits` creates missing configs for specialization indexes
    /// `0..MAX_SPECIALIZATIONS - 1`, and `CreateTraitConfig` can attach granted
    /// entries. This narrow proof is therefore limited to a complete set of
    /// persisted, active-for-spec combat configs whose global entry query was
    /// empty. `ValidateConfig` accepts those empty configs, so C++ neither
    /// replaces them with granted entries nor creates a missing config.
    fn represented_trait_config_aura_source_is_empty_like_cpp(&self) -> bool {
        let Some(runtime) = self.player_spell_runtime_snapshot_like_cpp() else {
            return false;
        };
        if !runtime.trait_config_rows_complete
            || !runtime.trait_entry_rows_complete
            || !runtime.trait_entry_rows_empty
        {
            return false;
        }

        let Some(specializations) = self.chr.specialization_store.as_ref() else {
            return false;
        };
        let mut expected_specs = BTreeSet::new();
        for index in 0..(MAX_SPECIALIZATIONS_LIKE_CPP - 1) {
            let Some(spec) = u8::try_from(index).ok().and_then(|index| {
                specializations.get_by_class_and_index_like_cpp(self.player_class_like_cpp(), index)
            }) else {
                return false;
            };
            let Ok(spec_id) = i32::try_from(spec.id) else {
                return false;
            };
            if !expected_specs.insert(spec_id) {
                return false;
            }
        }

        if runtime.trait_config_rows.len() != expected_specs.len() {
            return false;
        }
        for config in runtime.trait_config_rows.values() {
            let (config_type, specialization_id, combat_flags) = config.header;
            const TRAIT_CONFIG_TYPE_COMBAT_LIKE_CPP: i32 = 1;
            const TRAIT_COMBAT_CONFIG_ACTIVE_FOR_SPEC_LIKE_CPP: i32 = 0x1;
            if config_type != TRAIT_CONFIG_TYPE_COMBAT_LIKE_CPP
                || combat_flags & TRAIT_COMBAT_CONFIG_ACTIVE_FOR_SPEC_LIKE_CPP == 0
                || !expected_specs.remove(&specialization_id)
            {
                return false;
            }
        }
        expected_specs.is_empty()
    }
    /// C++ `Map::AddPlayerToMap` can dispatch `InstanceScript::OnPlayerEnter`,
    /// Scenario, and Battleground hooks before the login authority is
    /// published. Those hooks are not represented, so only an exact ordinary
    /// world-map DB2 row excludes them.
    fn represented_add_player_to_map_aura_source_is_empty_like_cpp(&self) -> bool {
        self.maps
            .store
            .as_ref()
            .and_then(|store| store.get(u32::from(self.player_map_id_like_cpp())))
            .is_some_and(|map| map.instance_type == wow_data::map::MAP_COMMON)
    }
    pub(in crate::session) fn can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(
        &self,
        difficulty_id: u8,
    ) -> bool {
        let Some(aura_subsystem) = self.player_aura_subsystem_snapshot_like_cpp() else {
            return false;
        };
        if !self.player_spell_hit_source_identity_complete_like_cpp()
            || aura_subsystem.spell_hit_aura_authority_tombstoned_like_cpp()
            || !aura_subsystem.persisted_player_aura_authority_complete_like_cpp()
            || !self.player_equipment_inventory_authority_complete_like_cpp()
            || self
                .resolved_represented_guild_id_like_cpp()
                .is_none_or(|guild_id| guild_id != 0)
            || self.complete_player_skill_records_like_cpp().is_none()
            || self
                .complete_represented_player_spell_rows_like_cpp()
                .is_none()
            || self
                .complete_represented_spell_trait_definition_ids_like_cpp()
                .is_none_or(|traits| !traits.is_empty())
            || !self.represented_trait_config_aura_source_is_empty_like_cpp()
            || !self.represented_active_glyph_aura_source_is_empty_like_cpp()
            || !self.represented_battle_pet_login_spell_source_is_empty_like_cpp()
            || !self.represented_add_player_to_map_aura_source_is_empty_like_cpp()
            || !self.represented_auto_push_quest_aura_source_is_empty_like_cpp()
            || !self.represented_character_pet_aura_source_is_empty_like_cpp()
            || !self.represented_quest_login_aura_sources_are_hit_inert_like_cpp(difficulty_id)
            || !self.represented_spell_area_autocast_source_is_empty_like_cpp()
            || !self.represented_war_mode_update_zone_aura_source_is_empty_like_cpp()
            || !self.represented_update_area_pvp_rule_aura_source_is_empty_like_cpp()
            || !self.represented_update_zone_script_aura_source_is_hit_inert_like_cpp()
            || !aura_subsystem
                .runtime_applications_like_cpp()
                .values()
                .all(|aura| self.player_visible_aura_is_spell_hit_inert_like_cpp(aura))
            || self
                .resolved_inventory_items_like_cpp()
                .is_none_or(|items| !items.is_empty())
            || self
                .resolved_buyback_items_like_cpp()
                .is_none_or(|items| !items.is_empty())
            || self
                .resolved_inventory_item_objects_like_cpp()
                .is_none_or(|items| !items.is_empty())
        {
            return false;
        }

        let known_spells = self.known_spells_like_cpp();
        if known_spells.is_empty() {
            return true;
        }
        let Some(spell_store) = self.spell_catalogs.spell_store.as_ref() else {
            return false;
        };
        known_spells.iter().copied().all(|spell_id| {
            spell_id > 0
                && spell_store
                    .misc_attributes_for_difficulty_like_cpp(
                        spell_id,
                        difficulty_id,
                        self.difficulty_store.as_deref(),
                    )
                    .is_some_and(|attributes| {
                        attributes[0] & wow_data::spell::attributes::SPELL_ATTR0_PASSIVE == 0
                            || u32::try_from(spell_id).ok().is_some_and(|spell_id| {
                                self.player_target_spell_is_hit_inert_like_cpp(
                                    spell_id,
                                    difficulty_id,
                                )
                            })
                    })
        })
    }
    pub(crate) fn can_authorize_empty_player_spell_hit_aura_source_like_cpp(&self) -> bool {
        self.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(
            self.current_map_difficulty_id_like_cpp(),
        )
    }
    /// Publish the combined session-source proof to the canonical Player.
    /// Positive publication is reserved for explicit login/snapshot boundaries;
    /// individual source mutations call the invalidation helper instead.
    pub(crate) fn sync_player_spell_hit_aura_authority_to_canonical_like_cpp(
        &mut self,
    ) -> Option<bool> {
        let complete = self.can_authorize_empty_player_spell_hit_aura_source_like_cpp();
        self.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_spell_hit_aura_authority_inert_like_cpp(complete);
            complete
        })
    }
    pub(in crate::session) fn invalidate_canonical_player_spell_hit_aura_authority_like_cpp(
        &mut self,
    ) {
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .invalidate_spell_hit_aura_authority_like_cpp();
        });
    }
    pub(crate) fn spell_area_for_aura_map_bounds_like_cpp(
        &self,
        spell_id: u32,
    ) -> Vec<&SpellAreaLikeCpp> {
        self.spell_catalogs
            .spell_area_store
            .as_ref()
            .map(|store| store.spell_area_for_aura_map_bounds_like_cpp(spell_id))
            .unwrap_or_default()
    }
    pub(in crate::session) fn represented_has_aura_state_like_cpp(&self, _aura_state: u32) -> bool {
        false
    }
    pub(in crate::session) fn calculate_represented_mounted_aura_amount_like_cpp(
        &self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
    ) -> i32 {
        let mut mount_type_id = u16::try_from(effect.effect_misc_value_2).unwrap_or_default();
        if let Some(mount_entry) = self.mount_store.as_ref().and_then(|store| {
            u32::try_from(spell_id)
                .ok()
                .and_then(|spell_id| store.get_by_source_spell_id_like_cpp(spell_id))
        }) {
            mount_type_id = mount_entry.mount_type_id;
        }

        if mount_type_id != 0 {
            let Some(riding_skill) =
                self.resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)
            else {
                return effect.effect_base_points;
            };
            if let Some((is_submerged, is_in_water)) =
                self.represented_player_mount_liquid_state_like_cpp()
                && let Ok(capability) = self
                    .represented_mount_capability_selection_for_type_like_cpp(
                        mount_type_id,
                        u32::from(riding_skill),
                        None,
                        is_submerged,
                        is_in_water,
                    )
            {
                return i32::try_from(capability.id).unwrap_or(effect.effect_base_points);
            }
        }

        effect.effect_base_points
    }
    pub(crate) fn load_represented_character_auras_like_cpp(
        &mut self,
        aura_rows: impl IntoIterator<Item = CharacterAuraRowLikeCpp>,
        effect_rows: impl IntoIterator<Item = CharacterAuraEffectRowLikeCpp>,
        timediff_secs: u32,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let spell_store = self.spell_store().cloned();
        let difficulty_store = self.difficulty_store().cloned();
        let aura_options_store = self.spell_catalogs.spell_aura_options_store.clone();
        let spell_misc_store = self.spell_catalogs.spell_misc_store().cloned();

        let effects: Vec<_> = effect_rows
            .into_iter()
            .filter(|row| {
                row.spell_id != 0
                    && u32::from(row.effect_index)
                        < wow_data::conditions::MAX_SPELL_EFFECTS_LIKE_CPP
            })
            .collect();

        let mut loaded = 0usize;
        for mut row in aura_rows {
            if row.spell_id == 0
                || spell_store
                    .as_ref()
                    .is_some_and(|store| store.get(row.spell_id as i32).is_none())
                || (row.difficulty != 0
                    && difficulty_store
                        .as_ref()
                        .is_some_and(|store| !store.contains(u32::from(row.difficulty))))
            {
                continue;
            }

            let aura_expires_offline = spell_misc_store
                .as_ref()
                .and_then(|store| store.get_by_spell_id(row.spell_id))
                .is_some_and(|misc| {
                    (misc.attributes[4] as u32
                        & wow_data::spell::attributes::SPELL_ATTR4_AURA_EXPIRES_OFFLINE)
                        != 0
                });
            let Some(remain_time_ms) = adjusted_represented_pet_aura_remain_time_like_cpp(
                row.remain_time_ms,
                timediff_secs,
                true,
                aura_expires_offline,
            ) else {
                continue;
            };
            row.remain_time_ms = remain_time_ms;

            if let Some(store) = aura_options_store.as_ref() {
                let proc_charges = store.proc_charges_like_cpp(row.spell_id, row.difficulty);
                row.remain_charges = if proc_charges == 0 {
                    0
                } else if row.remain_charges == 0 {
                    proc_charges
                } else {
                    row.remain_charges
                };
            }

            let Some(slot) = self.next_player_visible_aura_slot_like_cpp() else {
                break;
            };

            let caster_guid = if row.caster_guid.is_empty() {
                player_guid
            } else {
                row.caster_guid
            };
            let represented_effect_amounts: Vec<_> = effects
                .iter()
                .filter(|effect| {
                    let effect_caster_guid = if effect.caster_guid.is_empty() {
                        player_guid
                    } else {
                        effect.caster_guid
                    };
                    effect_caster_guid == caster_guid
                        && effect.spell_id == row.spell_id
                        && effect.effect_mask == row.effect_mask
                })
                .map(|effect| RepresentedAuraEffectAmountLikeCpp {
                    effect_index: effect.effect_index,
                    amount: effect.amount,
                })
                .collect();
            let duration_total = u32::try_from(row.max_duration_ms).unwrap_or(0);
            let duration_remaining = u32::try_from(row.remain_time_ms).unwrap_or(0);
            let spell_id = i32::try_from(row.spell_id).unwrap_or(i32::MAX);
            let aura_flags = AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100;
            let canonical_snapshot = self.canonical_threat_aura_snapshot_for_difficulty_like_cpp(
                spell_id,
                row.difficulty,
                row.effect_mask,
                &represented_effect_amounts,
            );
            let installed = self.mutate_player_aura_subsystem_like_cpp(|auras| {
                auras.insert_threat_snapshot_like_cpp(slot, canonical_snapshot);
                auras.insert_runtime_application_like_cpp(AuraApplication {
                    spell_id,
                    difficulty_id: row.difficulty,
                    caster_guid,
                    slot,
                    duration_total,
                    duration_remaining,
                    stack_count: row.stack_count.max(1),
                    aura_flags,
                    effect_mask: row.effect_mask,
                    aura_interrupt_flags: 0,
                    aura_interrupt_flags2: 0,
                    represented_effect: None,
                    represented_amount: 0,
                    represented_effect_amounts,
                    represented_misc_value: None,
                    represented_multiplier: 1.0,
                    applied_at: Instant::now(),
                });
            });
            if installed.is_none() {
                break;
            }
            self.send_aura_update_applied(
                spell_id,
                slot,
                caster_guid,
                duration_total,
                aura_flags,
                row.effect_mask,
            );
            loaded += 1;
        }
        loaded
    }
    #[allow(dead_code)]
    pub(crate) fn represented_mount_aura_display_candidates_like_cpp(
        &self,
        spell_id: u32,
    ) -> Vec<i32> {
        let Some(mount) = self
            .mount_store
            .as_ref()
            .and_then(|store| store.get_by_source_spell_id_like_cpp(spell_id))
        else {
            return Vec::new();
        };

        if mount.flags & wow_data::MOUNT_FLAG_SELF_MOUNT != 0 {
            return vec![wow_data::DISPLAYID_HIDDEN_MOUNT];
        }

        let Some(displays) = self
            .mount_x_display_store
            .as_ref()
            .and_then(|store| store.displays_for_mount_like_cpp(mount.id))
        else {
            return Vec::new();
        };

        displays
            .iter()
            .filter(|display| {
                display.player_condition_id == 0
                    || self.represented_mount_x_display_usable_like_cpp(display.player_condition_id)
            })
            .map(|display| display.creature_display_info_id)
            .collect()
    }
    #[allow(dead_code)]
    pub(crate) fn select_represented_mount_aura_display_like_cpp(
        &mut self,
        spell_id: u32,
    ) -> Option<i32> {
        let candidates = self.represented_mount_aura_display_candidates_like_cpp(spell_id);
        candidates
            .choose(&mut self.represented_runtime_rng_like_cpp)
            .copied()
    }
    pub(in crate::session) fn resolved_has_represented_aura_effect_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<bool> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .any(|aura| aura.represented_effect == Some(effect))
        })
    }
    #[cfg(test)]
    fn has_represented_aura_effect_like_cpp(&self, effect: RepresentedAuraEffectLikeCpp) -> bool {
        self.resolved_has_represented_aura_effect_like_cpp(effect)
            .expect("test Player aura owner must resolve")
    }
    pub(in crate::session) fn resolved_has_represented_aura_effect_with_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<bool> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras.values().any(|aura| {
                aura.represented_effect == Some(effect)
                    && aura.represented_misc_value == Some(misc_value)
            })
        })
    }
    #[cfg(test)]
    pub(in crate::session) fn has_represented_aura_effect_with_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> bool {
        self.resolved_has_represented_aura_effect_with_misc_value_like_cpp(effect, misc_value)
            .expect("test Player aura owner must resolve")
    }
    pub(in crate::session) fn resolved_total_represented_aura_modifier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .sum()
        })
    }
    #[cfg(test)]
    pub(in crate::session) fn total_represented_aura_modifier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> i32 {
        self.resolved_total_represented_aura_modifier_like_cpp(effect)
            .expect("test Player aura owner must resolve")
    }
    pub(in crate::session) fn resolved_total_represented_aura_modifier_by_misc_value_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
        misc_value: i32,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| {
                    aura.represented_effect == Some(effect)
                        && aura.represented_misc_value == Some(misc_value)
                })
                .map(|aura| aura.represented_amount)
                .sum()
        })
    }
    pub(in crate::session) fn resolved_total_represented_aura_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .fold(1.0, |acc, aura| acc * aura.represented_multiplier)
        })
    }
    #[cfg(test)]
    pub(in crate::session) fn total_represented_aura_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> f32 {
        self.resolved_total_represented_aura_multiplier_like_cpp(effect)
            .expect("test Player aura owner must resolve")
    }
    pub(in crate::session) fn aura_has_total_stat_percentage_effect_like_cpp(
        &self,
        aura: &AuraApplication,
    ) -> bool {
        self.spell_store().is_some_and(|store| {
            store.get(aura.spell_id).is_some_and(|spell| {
                spell.effects().iter().any(|effect| {
                    1u32.checked_shl(effect.effect_index)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
                        && effect.effect_aura
                            == wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                })
            })
        })
    }
    pub(in crate::session) fn total_stat_percentage_aura_preserves_health_pct_like_cpp(
        &self,
        aura: &AuraApplication,
    ) -> bool {
        self.spell_store().is_some_and(|store| {
            store.has_attribute0_like_cpp(
                aura.spell_id,
                wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY,
            ) && store.get(aura.spell_id).is_some_and(|spell| {
                spell.effects().iter().any(|effect| {
                    1u32.checked_shl(effect.effect_index)
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
                        && effect.effect_aura
                            == wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE
                        && (effect.effect_misc_value_2 == 0
                            || effect.effect_misc_value_2 & (1 << 2) != 0)
                })
            })
        })
    }
    pub(in crate::session) fn max_represented_aura_amount_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .filter(|amount| *amount > 0)
                .max()
                .unwrap_or(0)
        })
    }
    pub(in crate::session) fn max_negative_represented_aura_amount_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .filter(|amount| *amount < 0)
                .min()
                .unwrap_or(0)
        })
    }
    pub(in crate::session) fn total_represented_aura_amount_multiplier_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<f32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .fold(1.0, |multiplier, aura| {
                    multiplier * (1.0 + aura.represented_amount.max(0) as f32 / 100.0)
                })
        })
    }
    pub(in crate::session) fn total_represented_aura_amount_like_cpp(
        &self,
        effect: RepresentedAuraEffectLikeCpp,
    ) -> Option<i32> {
        self.resolved_player_visible_auras_like_cpp().map(|auras| {
            auras
                .values()
                .filter(|aura| aura.represented_effect == Some(effect))
                .map(|aura| aura.represented_amount)
                .sum()
        })
    }
}
