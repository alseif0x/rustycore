//! Represented aura state owned at the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;
pub(crate) use wow_world_spell::RepresentedShapeshiftMutationLikeCpp;
use wow_world_spell::shapeshift_form_of_spell_like_cpp;

fn represented_aura_visual_without_caster_like_cpp(
    spell_id: u32,
    difficulty_id: u8,
    config: &LegacyCreatureAggroConfigLikeCpp,
) -> u32 {
    let Some(store) = config.spell_x_spell_visual_store.as_ref() else {
        return 0;
    };
    for difficulty_id in creature_ai_spell_difficulty_chain_like_cpp(difficulty_id, config) {
        let mut rows = store
            .entries_like_cpp()
            .filter(|row| row.spell_id == spell_id && row.difficulty_id == difficulty_id)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            continue;
        }
        // `SpellMgr` inserts DB2 rows at `lower_bound` ordered by descending
        // CasterPlayerConditionID. Because the DB2 store is walked in ID
        // order, equal-condition rows are inserted before earlier rows and
        // therefore end up in descending DB2-ID order.
        rows.sort_by_key(|row| std::cmp::Reverse((row.caster_player_condition_id, row.id)));
        return rows
            .into_iter()
            .find(|row| row.caster_player_condition_id == 0 && row.caster_unit_condition_id == 0)
            .map_or(0, |row| row.id);
    }
    0
}

#[path = "aura/effect_queries.rs"]
mod effect_queries;
#[path = "aura/spell_hit_authority.rs"]
mod spell_hit_authority;

impl WorldSession {
    pub(in crate::session) fn represented_player_aura_state_mask_like_cpp(&self) -> Option<u32> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_player_aura_state_mask_like_cpp(hub)
    }

    /// C++ `Unit::m_unitData->AuraState` for any represented unit: the canonical
    /// session player or a world creature. `0` when the unit cannot be resolved.
    pub(in crate::session) fn represented_unit_aura_state_mask_like_cpp(
        &self,
        unit_guid: ObjectGuid,
    ) -> u32 {
        if Some(unit_guid) == self.player_guid() {
            return self
                .represented_player_aura_state_mask_like_cpp()
                .unwrap_or(0);
        }
        let Some(manager) = self.core.map_manager.as_ref() else {
            return 0;
        };
        let instance_id = self
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let manager = manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        manager
            .find_creature(self.core.player_map_id_like_cpp(), instance_id, unit_guid)
            .map(|creature| {
                creature.creature.unit().subsystems().auras.aura_state_mask
                    | crate::map_manager::WorldCreature::health_aura_state_like_cpp(
                        u64::from(creature.current_hp()),
                        u64::from(creature.max_hp()),
                        creature.is_alive(),
                    )
            })
            .unwrap_or(0)
    }

    pub(crate) fn represented_spell_has_mod_shapeshift_effect_like_cpp(
        &self,
        spell_id: i32,
    ) -> bool {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_spell_has_mod_shapeshift_effect_like_cpp(hub, spell_id)
    }


    pub(in crate::session) fn represented_cast_speed_multiplier_like_cpp(&self) -> f32 {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_cast_speed_multiplier_like_cpp(hub)
    }


    pub(in crate::session) fn calculate_represented_mounted_aura_amount_like_cpp(
        &self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
    ) -> i32 {
        let mut mount_type_id = u16::try_from(effect.effect_misc_value_2).unwrap_or_default();
        if let Some(mount_entry) = self.catalogs.mount_store.as_ref().and_then(|store| {
            u32::try_from(spell_id)
                .ok()
                .and_then(|spell_id| store.get_by_source_spell_id_like_cpp(spell_id))
        }) {
            mount_type_id = mount_entry.mount_type_id;
        }

        if mount_type_id != 0 {
            let Some(riding_skill) = crate::session::hub_ref(self)
                .resolved_player_skill_value_like_cpp(SKILL_RIDING_LIKE_CPP)
            else {
                return effect.effect_base_points;
            };
            if let Some((is_submerged, is_in_water)) =
                crate::session::hub_ref(self).represented_player_mount_liquid_state_like_cpp()
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
        let difficulty_store = self.catalogs.difficulty_store().cloned();
        let aura_options_store = self
            .catalogs
            .spell_catalogs
            .spell_aura_options_store
            .clone();
        let spell_misc_store = self.catalogs.spell_catalogs.spell_misc_store().cloned();

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
            // C++ `Player::_LoadAuras` allocates a fresh `HighGuid::Cast` for
            // every restored Aura base. With no live caster pointer,
            // `Aura::Aura` resolves `GetSpellXSpellVisualId()` by skipping
            // caster-conditioned rows and taking the first unconditional row.
            let Some(cast_id) = self.next_represented_spell_cast_guid_like_cpp(spell_id) else {
                break;
            };
            let spell_visual_id = represented_aura_visual_without_caster_like_cpp(
                row.spell_id,
                row.difficulty,
                &self.config.legacy_creature_aggro_config_like_cpp,
            );
            let provenance = wow_entities::AuraCastProvenanceLikeCpp {
                cast_id,
                spell_visual_id: i32::try_from(spell_visual_id).unwrap_or(i32::MAX),
            };
            let aura_flags = AFLAG_NOCASTER_LIKE_CPP | 0x0000_0100;
            let canonical_snapshot = self.canonical_threat_aura_snapshot_for_difficulty_like_cpp(
                spell_id,
                row.difficulty,
                row.effect_mask,
                &represented_effect_amounts,
            );
            let aura = AuraApplication {
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
            };
            let _fallback_snapshot = canonical_snapshot.clone();
            let _fallback_aura = aura.clone();
            let _canonical = self
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    player.install_player_threat_aura_like_cpp(slot, canonical_snapshot, aura);
                    player
                        .unit_mut()
                        .subsystems_mut()
                        .auras
                        .set_aura_cast_provenance_like_cpp(slot, provenance);
                })
                .is_some();
            #[cfg(test)]
            let installed = if _canonical {
                Some(())
            } else if self.core.player_handle_like_cpp.is_none() {
                self.mutate_player_aura_subsystem_like_cpp(|auras| {
                    auras.insert_threat_snapshot_like_cpp(slot, _fallback_snapshot);
                    auras.insert_runtime_application_like_cpp(_fallback_aura);
                    auras.set_aura_cast_provenance_like_cpp(slot, provenance);
                })
            } else {
                None
            };
            #[cfg(not(test))]
            let installed = _canonical.then_some(());
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
            .catalogs
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
            .catalogs
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
            .choose(&mut self.core.driver.represented_runtime_rng_like_cpp)
            .copied()
    }

    pub(crate) fn represented_player_is_polymorphed_like_cpp(&self) -> Option<bool> {
        let (state, hub) = crate::session::split_spell_state_ref(self);
        state.represented_player_is_polymorphed_like_cpp(hub)
    }
}


#[cfg(test)]
#[path = "../../../unit_tests/session/spell_state/aura/f3_shims.rs"]
mod f3_shims;
