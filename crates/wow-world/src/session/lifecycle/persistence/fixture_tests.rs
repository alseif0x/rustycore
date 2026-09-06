//! Frozen pre-C1 save projection/ack oracle for existing fixture and equivalence tests.
//! Not compiled into the server. Production uses one owner capture and a consumed receipt.
#![cfg(test)]

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn current_player_character_save_request_like_cpp(
        &mut self,
        snapshot: &PlayerSaveToDbSnapshotLikeCpp,
        now_unix_secs: i64,
    ) -> Option<PlayerCharacterSaveRequestLikeCpp> {
        // A cancelled/ambiguous transaction can contain money plus other
        // absolute replacements (for example a talent reset). Building a
        // partial full-save plan from the pre-COMMIT runtime snapshot would
        // overwrite those non-money rows even if the earlier COMMIT succeeded.
        if self
            .durable_loot_money_persistence_like_cpp
            .is_indeterminate_like_cpp()
        {
            return None;
        }

        let guid_counter = snapshot.guid.counter() as u64;
        let powers = character_power_snapshot_values_like_cpp(&snapshot.powers);
        if powers.is_none() && std::env::var_os("RUSTYCORE_SPELL_POWER_TRACE").is_some() {
            info!(
                guid = ?snapshot.guid,
                "RUST_PLAYER_POWER_SAVE skipped: no authoritative canonical power snapshot"
            );
        }
        let (dungeon_difficulty, raid_difficulty, legacy_raid_difficulty) =
            self.player_difficulty_preferences_snapshot_like_cpp()?;
        let character = PlayerCharacterSnapshotSaveLikeCpp {
            position: PlayerPositionSaveLikeCpp {
                x: snapshot.position.x,
                y: snapshot.position.y,
                z: snapshot.position.z,
                orientation: snapshot.position.orientation,
                map_id: snapshot.map_id,
                instance_id: snapshot.instance_id,
                zone_id: self.player_zone_area_like_cpp()?.0 as u16,
            },
            level: snapshot.level,
            xp: snapshot.xp,
            money: snapshot.money,
            rest_state: self.resolved_xp_rest_state_like_cpp()?,
            player_flags: self.resolved_player_flags_for_rest_state_save_like_cpp()?,
            rest_bonus: self.resolved_xp_rest_bonus_like_cpp()?,
            logout_time: now_unix_secs.max(0) as u64,
            is_logout_resting: self.resolved_visible_resting_like_cpp()?,
            health: snapshot.health,
            powers,
            talent_reset_cost: self.represented_talent_reset_cost_like_cpp()?,
            talent_reset_time: self.represented_talent_reset_time_secs_like_cpp()?,
            explored_zones: self.represented_explored_zones_db_string_like_cpp()?,
            dungeon_difficulty,
            raid_difficulty,
            legacy_raid_difficulty,
        };

        let spell_runtime = self.player_spell_runtime_snapshot_like_cpp();
        let spells = if let Some(spells) = self.complete_represented_player_spell_rows_like_cpp() {
            Some(PlayerSpellSaveGroupLikeCpp::Complete {
                rows: spells
                    .values()
                    .map(|spell| PlayerSpellSaveLikeCpp {
                        spell_id: spell.spell_id,
                        active: spell.active,
                        disabled: spell.disabled,
                        dependent: spell.dependent,
                        favorite: spell.favorite,
                        state: match spell.state {
                            RepresentedPlayerSpellStateLikeCpp::Unchanged => {
                                PlayerSpellStateLikeCpp::Unchanged
                            }
                            RepresentedPlayerSpellStateLikeCpp::Changed => {
                                PlayerSpellStateLikeCpp::Changed
                            }
                            RepresentedPlayerSpellStateLikeCpp::New => PlayerSpellStateLikeCpp::New,
                            RepresentedPlayerSpellStateLikeCpp::Removed => {
                                PlayerSpellStateLikeCpp::Removed
                            }
                            RepresentedPlayerSpellStateLikeCpp::Temporary => {
                                PlayerSpellStateLikeCpp::Temporary
                            }
                        },
                    })
                    .collect(),
                fallback_rows_were_present: spell_runtime
                    .as_ref()
                    .is_some_and(|runtime| !runtime.fallback_rows.is_empty()),
            })
        } else if spell_runtime
            .as_ref()
            .is_some_and(|runtime| !runtime.fallback_rows.is_empty())
        {
            Some(PlayerSpellSaveGroupLikeCpp::Fallback {
                rows: spell_runtime
                    .as_ref()
                    .expect("non-empty fallback spell runtime")
                    .fallback_rows
                    .values()
                    .map(|spell| PlayerFallbackSpellSaveLikeCpp {
                        spell_id: spell.spell_id,
                        active: spell.active,
                        dependent: spell.dependent,
                    })
                    .collect(),
            })
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented player spell save because PlayerSpellMap was not loaded coherently"
            );
            None
        };

        let skills = if self.has_complete_player_skill_save_authority_like_cpp() {
            self.resolved_player_skill_non_durable_tombstones_like_cpp()
                .zip(self.resolved_player_skill_records_like_cpp())
                .map(|(tombstones, records)| {
                    records
                        .values()
                        .filter(|skill| {
                            skill.state != RepresentedPlayerSkillStateLikeCpp::Deleted
                                && !tombstones.contains(&skill.skill_id)
                        })
                        .map(|skill| PlayerSkillSaveLikeCpp {
                            skill_id: skill.skill_id,
                            value: skill.value,
                            max: skill.max,
                            profession_slot: skill.profession_slot,
                        })
                        .collect()
                })
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented player skill save because complete character_skills slot authority is unavailable"
            );
            None
        };

        let talent_runtime = self.player_talent_runtime_snapshot_like_cpp();
        let glyphs = if talent_runtime
            .as_ref()
            .is_some_and(|runtime| runtime.glyphs_loaded)
        {
            Some(
                talent_runtime
                    .as_ref()
                    .expect("checked canonical glyph authority")
                    .glyph_groups
                    .iter()
                    .enumerate()
                    .flat_map(|(talent_group, glyphs)| {
                        glyphs
                            .iter()
                            .copied()
                            .enumerate()
                            .map(move |(glyph_slot, glyph_id)| PlayerGlyphSaveLikeCpp {
                                talent_group: talent_group as u8,
                                glyph_slot: glyph_slot as u8,
                                glyph_id,
                            })
                    })
                    .collect(),
            )
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented player glyph save because character_glyphs was not loaded coherently"
            );
            None
        };

        let talents = if talent_runtime
            .as_ref()
            .is_some_and(|runtime| runtime.talents_loaded)
        {
            let mut rows = Vec::new();
            for (talent_group, talents) in talent_runtime
                .as_ref()
                .expect("checked canonical talent authority")
                .talent_groups
                .iter()
                .enumerate()
            {
                for (talent_id, rank) in talents {
                    if self
                        .represented_talent_info_like_cpp(*talent_id, *rank)
                        .is_some()
                    {
                        rows.push(PlayerTalentSaveLikeCpp {
                            talent_id: *talent_id,
                            rank: *rank,
                            talent_group: talent_group as u8,
                        });
                    }
                }
            }
            Some(rows)
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented player talent save because character_talent was not loaded coherently"
            );
            None
        };

        let spell_history = self.player_spell_history_snapshot_like_cpp();
        let spell_cooldowns = if spell_history
            .as_ref()
            .is_some_and(|history| history.cooldowns_loaded)
        {
            Some(
                spell_history
                    .as_ref()
                    .expect("loaded history resolved above")
                    .cooldowns
                    .values()
                    .map(|cooldown| PlayerSpellCooldownSaveLikeCpp {
                        spell_id: cooldown.spell_id,
                        item_id: cooldown.item_id,
                        cooldown_end_unix_secs: (cooldown.cooldown_end_ms / 1_000)
                            .min(i64::MAX as u64)
                            as i64,
                        category_id: cooldown.category_id,
                        category_end_unix_secs: (cooldown.category_end_ms / 1_000)
                            .min(i64::MAX as u64)
                            as i64,
                    })
                    .collect(),
            )
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented player spell cooldown save because character_spell_cooldown was not loaded coherently"
            );
            None
        };

        let spell_charges = if spell_history
            .as_ref()
            .is_some_and(|history| history.charges_loaded)
        {
            Some(
                spell_history
                    .as_ref()
                    .expect("loaded history resolved above")
                    .charges
                    .iter()
                    .flat_map(|(&category_id, charges)| {
                        charges
                            .iter()
                            .map(move |charge| PlayerSpellChargeSaveLikeCpp {
                                category_id,
                                recharge_start_unix_secs: (charge.recharge_start_ms / 1_000)
                                    .min(i64::MAX as u64)
                                    as i64,
                                recharge_end_unix_secs: (charge.recharge_end_ms / 1_000)
                                    .min(i64::MAX as u64)
                                    as i64,
                            })
                    })
                    .collect(),
            )
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented player spell charge save because character_spell_charges was not loaded coherently"
            );
            None
        };

        let action_buttons = if let Some(action_buttons) =
            self.loaded_action_buttons_snapshot_like_cpp()
        {
            let (spec, trait_config_id) = self.represented_action_button_db_context_like_cpp()?;
            Some(PlayerActionButtonsSaveLikeCpp {
                spec,
                trait_config_id,
                rows: action_buttons
                    .iter()
                    .copied()
                    .enumerate()
                    .filter_map(|(button, packed_action)| {
                        if packed_action == 0 {
                            return None;
                        }
                        Some(PlayerActionButtonSaveLikeCpp {
                            button: u8::try_from(button).ok()?,
                            packed_action,
                        })
                    })
                    .collect(),
            })
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented player action-button save because character_action was not loaded coherently"
            );
            None
        };

        let equipment_sets = match self
            .with_owned_equipment_sets_like_cpp(|sets, loaded| (sets.clone(), loaded))
        {
            Some((sets, true)) => Some(
                sets.values()
                    .map(|equipment_set| PlayerEquipmentSetSaveLikeCpp {
                        set_guid: equipment_set.guid,
                        set_id: equipment_set.set_id,
                        set_type: match equipment_set.set_type {
                            RepresentedEquipmentSetTypeLikeCpp::Equipment => {
                                PlayerEquipmentSetTypeLikeCpp::Equipment
                            }
                            RepresentedEquipmentSetTypeLikeCpp::Transmog => {
                                PlayerEquipmentSetTypeLikeCpp::Transmog
                            }
                        },
                        state: match equipment_set.state {
                            RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged => {
                                PlayerEquipmentSetStateLikeCpp::Unchanged
                            }
                            RepresentedEquipmentSetUpdateStateLikeCpp::Changed => {
                                PlayerEquipmentSetStateLikeCpp::Changed
                            }
                            RepresentedEquipmentSetUpdateStateLikeCpp::New => {
                                PlayerEquipmentSetStateLikeCpp::New
                            }
                            RepresentedEquipmentSetUpdateStateLikeCpp::Deleted => {
                                PlayerEquipmentSetStateLikeCpp::Deleted
                            }
                        },
                        name: equipment_set.set_name.clone(),
                        icon: equipment_set.set_icon.clone(),
                        ignore_mask: equipment_set.ignore_mask,
                        assigned_spec_index: equipment_set.assigned_spec_index,
                        pieces: equipment_set
                            .pieces
                            .iter()
                            .map(|guid| guid.counter() as u64)
                            .collect(),
                        appearances: equipment_set.appearances.to_vec(),
                        enchants: equipment_set.enchants,
                    })
                    .collect(),
            ),
            _ => {
                warn!(
                    account = self.account_id,
                    player_guid = ?self.player_guid(),
                    "Skipping represented equipment-set save because canonical Player authority was unavailable or character_equipmentsets/character_transmog_outfits were not loaded coherently"
                );
                None
            }
        };

        let void_storage = match self
            .with_owned_void_storage_like_cpp(|items, loaded| (items.to_vec(), loaded))
        {
            Some((items, true)) => Some(
                items
                    .iter()
                    .enumerate()
                    .map(|(slot, item)| PlayerVoidStorageSlotSaveLikeCpp {
                        slot: u8::try_from(slot).expect("void-storage slot fits u8"),
                        item: item.as_ref().map(|item| PlayerVoidStorageSaveLikeCpp {
                            item_id: item.item_id,
                            item_entry: item.item_entry,
                            creator_guid: item.creator_guid.counter() as u64,
                            fixed_scaling_level: item.fixed_scaling_level,
                            random_properties_id: item.random_properties_id,
                            random_properties_seed: item.random_properties_seed,
                            context: item.context,
                        }),
                    })
                    .collect(),
            ),
            _ => {
                warn!(
                    account = self.account_id,
                    player_guid = ?self.player_guid(),
                    "Skipping represented void-storage save because canonical Player authority was unavailable or character_void_storage was not loaded coherently"
                );
                None
            }
        };

        // C++ `_SaveQuestStatus` only consumes entries present in `m_QuestStatusSave`; it does
        // not rewrite every loaded quest during Player::SaveToDB. Rust's quest mutation paths
        // already persist their changed quest directly, but there is no coherent dirty-set seam
        // yet. Rewriting every active quest here can delete objective rows that were not mapped
        // into represented state, so preserve them until that dirty tracking exists.

        let tutorials = if self.tutorials_changed_like_cpp {
            if self.tutorials_loaded_coherently_like_cpp {
                Some(PlayerTutorialsSaveLikeCpp {
                    tutorials: self.tutorials_like_cpp,
                    already_persisted: self.tutorials_loaded_from_db_like_cpp,
                })
            } else {
                warn!(
                    account = self.account_id,
                    "Skipping SaveTutorialsData because tutorial data was not loaded coherently"
                );
                None
            }
        } else {
            None
        };

        let instance_lock_times = self
            .represented_instance_reset_times_like_cpp
            .iter()
            .map(
                |(&instance_id, &release_time)| PlayerInstanceLockTimeSaveLikeCpp {
                    instance_id,
                    release_time,
                },
            )
            .collect();
        let (total_time, level_time) = self.current_played_time_values_like_cpp();
        let played_time = PlayerPlayedTimeSaveLikeCpp {
            total_time,
            level_time,
        };

        let reputations = self
            .with_reputation_mgr_like_cpp(|mgr| mgr.pending_save_rows_like_cpp())?
            .into_iter()
            .map(
                |(faction_id, standing, flags)| PlayerReputationSaveLikeCpp {
                    faction_id,
                    standing,
                    flags,
                },
            )
            .collect();

        let cuf_profiles = match self.owned_player_cuf_profiles_like_cpp() {
            Some((profiles, true)) => Some(
                (0..wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP)
                    .map(|id| PlayerCufProfileSlotSaveLikeCpp {
                        profile_id: id as u8,
                        profile: profiles.get(id).and_then(Option::as_ref).map(|profile| {
                            PlayerCufProfileSaveLikeCpp {
                                profile_name: profile.profile_name.clone(),
                                frame_height: profile.frame_height,
                                frame_width: profile.frame_width,
                                sort_by: profile.sort_by,
                                health_text: profile.health_text,
                                bool_options: profile.bool_options,
                                top_point: profile.top_point,
                                bottom_point: profile.bottom_point,
                                left_point: profile.left_point,
                                top_offset: profile.top_offset,
                                bottom_offset: profile.bottom_offset,
                                left_offset: profile.left_offset,
                            }
                        }),
                    })
                    .collect(),
            ),
            _ => {
                warn!(
                    account = self.account_id,
                    player_guid = ?self.player_guid(),
                    "Skipping represented CUF profile save because canonical character_cuf_profiles authority was unavailable or not loaded coherently"
                );
                None
            }
        };

        Some(PlayerCharacterSaveRequestLikeCpp {
            player_guid: guid_counter,
            account_id: self.account_id,
            wall_clock_unix_secs: now_unix_secs,
            character,
            spells,
            skills,
            glyphs,
            talents,
            spell_cooldowns,
            spell_charges,
            action_buttons,
            equipment_sets,
            void_storage,
            tutorials,
            instance_lock_times,
            played_time,
            reputations,
            cuf_profiles,
        })
    }

    pub(in crate::session) fn mark_player_spells_saved_like_cpp(&mut self) {
        if self
            .mutate_player_spell_runtime_like_cpp(
                wow_entities::PlayerSpellRuntimeState::mark_spell_rows_saved_like_cpp,
            )
            .is_none()
        {
            return;
        }
        self.sync_player_registry_state_like_cpp();
    }

    #[cfg(test)]
    pub(in crate::session) fn fixture_mark_player_spells_saved_like_cpp(&mut self) {
        if self
            .mutate_player_spell_runtime_like_cpp(|runtime| {
                runtime.rows.retain(|_, spell| {
                    if spell.state == wow_entities::PlayerSpellLoadState::Removed {
                        return false;
                    }
                    if spell.state != wow_entities::PlayerSpellLoadState::Temporary {
                        spell.state = wow_entities::PlayerSpellLoadState::Unchanged;
                    }
                    true
                });
                runtime.removed_known_spells.clear();
                runtime
                    .trait_definition_ids
                    .retain(|spell_id, _| runtime.rows.contains_key(spell_id));
                runtime.dependent_known_spells = runtime
                    .rows
                    .values()
                    .filter(|spell| spell.dependent)
                    .map(|spell| spell.spell_id)
                    .collect();
                runtime.favorite_known_spells = runtime
                    .rows
                    .values()
                    .filter(|spell| spell.favorite)
                    .map(|spell| spell.spell_id)
                    .collect();
                runtime.known_spells = runtime
                    .rows
                    .values()
                    .filter(|spell| !spell.disabled)
                    .map(|spell| spell.spell_id)
                    .collect();
            })
            .is_none()
        {
            return;
        }
        self.sync_player_registry_state_like_cpp();
    }

    fn mark_player_skills_saved_like_cpp(&mut self) {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.fixture_mark_player_skills_saved_like_cpp();
            return;
        }
        if self
            .with_owned_player_mut_like_cpp(wow_entities::Player::mark_skill_records_saved_like_cpp)
            .is_none()
        {
            return;
        }
        self.sync_player_registry_state_like_cpp();
    }

    #[cfg(test)]
    pub(in crate::session) fn mark_current_player_save_to_db_committed_like_cpp(
        &mut self,
        committed: &PlayerCharacterCommittedGroupsLikeCpp,
    ) {
        if committed.player_spells {
            self.mark_player_spells_saved_like_cpp();
        }
        if committed.fallback_player_spells {
            let _ = self.mutate_player_spell_runtime_like_cpp(|runtime| {
                runtime.fallback_rows.clear();
            });
        }
        if committed.player_skills {
            self.mark_player_skills_saved_like_cpp();
        }
        if committed.equipment_sets {
            self.mark_equipment_sets_saved_like_cpp();
        }
        if committed.tutorials_insert {
            self.tutorials_loaded_from_db_like_cpp = true;
        }
        if committed.tutorials_changed {
            self.tutorials_changed_like_cpp = false;
        }
        if committed.reputation {
            let _ = self.mutate_reputation_mgr_like_cpp(|mgr| {
                mgr.mark_pending_save_to_db_committed_like_cpp();
            });
        }
    }
}
