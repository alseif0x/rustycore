// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SQLx-free Player full-save lifecycle orchestration.
//!
//! Session snapshots represented Player state; the lifecycle adapter privately
//! owns statement decomposition and single-transaction execution (#286).

use std::sync::Arc;

use tracing::{info, trace, warn};
use wow_persistence::{
    PlayerActionButtonSaveLikeCpp, PlayerActionButtonsSaveLikeCpp,
    PlayerCharacterCommittedGroupsLikeCpp, PlayerCharacterSaveRequestLikeCpp,
    PlayerCharacterSnapshotSaveLikeCpp, PlayerCufProfileSaveLikeCpp,
    PlayerCufProfileSlotSaveLikeCpp, PlayerEquipmentSetSaveLikeCpp, PlayerEquipmentSetStateLikeCpp,
    PlayerEquipmentSetTypeLikeCpp, PlayerFallbackSpellSaveLikeCpp, PlayerGlyphSaveLikeCpp,
    PlayerInstanceLockTimeSaveLikeCpp, PlayerPlayedTimeSaveLikeCpp, PlayerPositionSaveLikeCpp,
    PlayerReputationSaveLikeCpp, PlayerSkillSaveLikeCpp, PlayerSpellChargeSaveLikeCpp,
    PlayerSpellCooldownSaveLikeCpp, PlayerSpellSaveGroupLikeCpp, PlayerSpellSaveLikeCpp,
    PlayerSpellStateLikeCpp, PlayerTalentSaveLikeCpp, PlayerTutorialsSaveLikeCpp,
    PlayerVoidStorageSaveLikeCpp, PlayerVoidStorageSlotSaveLikeCpp,
};

use super::super::{
    AbsolutePlayerMoneyCommitReconciliationLikeCpp, ExclusivePlayerMoneyPersistenceLikeCpp,
    PlayerMoneyCommitCancellationFenceLikeCpp, PlayerSaveToDbSnapshotLikeCpp,
    RepresentedEquipmentSetTypeLikeCpp, RepresentedEquipmentSetUpdateStateLikeCpp,
    RepresentedPlayerSkillStateLikeCpp, RepresentedPlayerSpellStateLikeCpp, WorldSession,
    character_power_snapshot_values_like_cpp, reconcile_absolute_player_money_commit_like_cpp,
    unix_now,
};

impl WorldSession {
    /// Await a typed adapter transaction while the cancellation fence and the
    /// Session-owned money exclusion remain active. The adapter observes the
    /// durable money marker; Session owns reconciliation and quarantine.
    pub(crate) async fn await_exclusive_player_money_transaction_outcome_like_cpp<F>(
        &mut self,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        outcome_future: F,
        money_before: u64,
        money_after: u64,
        operation: &'static str,
    ) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp>
    where
        F: std::future::Future<Output = wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp>,
    {
        let mut cancellation_fence = PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(
            &self.durable_loot_money_persistence_like_cpp,
        ));
        match outcome_future.await {
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::Committed => {
                cancellation_fence.disarm_like_cpp();
                Some(money_persistence)
            }
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
                reason,
            } => {
                cancellation_fence.disarm_like_cpp();
                warn!(error = %reason, operation, "player-money transaction definitely rolled back");
                None
            }
            wow_persistence::PlayerMoneyTransactionOutcomeLikeCpp::CommitOutcomeUnknown {
                reason,
                observed_money,
            } => match reconcile_absolute_player_money_commit_like_cpp(
                money_before,
                money_after,
                observed_money,
            ) {
                AbsolutePlayerMoneyCommitReconciliationLikeCpp::Committed => {
                    cancellation_fence.disarm_like_cpp();
                    warn!(
                        error = %reason,
                        operation,
                        money_before,
                        money_after,
                        "player-money COMMIT reply was lost but durable money proves the transaction committed"
                    );
                    Some(money_persistence)
                }
                AbsolutePlayerMoneyCommitReconciliationLikeCpp::RolledBack => {
                    cancellation_fence.disarm_like_cpp();
                    warn!(
                        error = %reason,
                        operation,
                        money_before,
                        money_after,
                        "player-money COMMIT reply was lost but durable money proves the transaction rolled back"
                    );
                    None
                }
                AbsolutePlayerMoneyCommitReconciliationLikeCpp::Indeterminate => {
                    self.durable_loot_money_persistence_like_cpp
                        .mark_indeterminate_like_cpp();
                    cancellation_fence.disarm_like_cpp();
                    self.kick(
                        "player-money COMMIT outcome is unknown; relog required before another money mutation",
                    );
                    warn!(
                        error = %reason,
                        operation,
                        money_before,
                        money_after,
                        ?observed_money,
                        "player-money COMMIT outcome remains indeterminate; quarantined the session"
                    );
                    None
                }
            },
        }
    }

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
        self.sync_represented_explored_zones_from_canonical_like_cpp();
        let character = PlayerCharacterSnapshotSaveLikeCpp {
            position: PlayerPositionSaveLikeCpp {
                x: snapshot.position.x,
                y: snapshot.position.y,
                z: snapshot.position.z,
                orientation: snapshot.position.orientation,
                map_id: snapshot.map_id,
                instance_id: snapshot.instance_id,
                zone_id: self.player_zone_id_like_cpp as u16,
            },
            level: self.player_level_like_cpp(),
            xp: self.resolved_player_xp_like_cpp()?,
            money: self.resolved_player_money_like_cpp()?,
            rest_state: self.resolved_xp_rest_state_like_cpp()?,
            player_flags: self.resolved_player_flags_for_rest_state_save_like_cpp()?,
            rest_bonus: self.resolved_xp_rest_bonus_like_cpp()?,
            logout_time: now_unix_secs.max(0) as u64,
            is_logout_resting: self.resolved_visible_resting_like_cpp()?,
            health: snapshot.health,
            powers,
            talent_reset_cost: self.represented_talent_reset_cost_like_cpp,
            talent_reset_time: self.represented_talent_reset_time_secs_like_cpp,
            explored_zones: self.represented_explored_zones_db_string_like_cpp(),
            dungeon_difficulty: self.resolved_dungeon_difficulty_id_like_cpp()?,
            raid_difficulty: self.resolved_raid_difficulty_id_like_cpp()?,
            legacy_raid_difficulty: self.resolved_legacy_raid_difficulty_id_like_cpp()?,
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
            let tombstones = self.player_skill_non_durable_tombstones_like_cpp();
            Some(
                self.player_skill_records_like_cpp()
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
                    .collect(),
            )
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
            let (spec, trait_config_id) = self.represented_action_button_db_context_like_cpp();
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

        let equipment_sets = if self.represented_equipment_sets_loaded_like_cpp {
            Some(
                self.represented_equipment_sets_like_cpp
                    .values()
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
            )
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented equipment-set save because character_equipmentsets/character_transmog_outfits were not loaded coherently"
            );
            None
        };

        let void_storage = if self.represented_void_storage_loaded_like_cpp {
            Some(
                self.represented_void_storage_items_like_cpp
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
            )
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented void-storage save because character_void_storage was not loaded coherently"
            );
            None
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
            .reputation_mgr_like_cpp()
            .pending_save_rows_like_cpp()
            .into_iter()
            .map(
                |(faction_id, standing, flags)| PlayerReputationSaveLikeCpp {
                    faction_id,
                    standing,
                    flags,
                },
            )
            .collect();

        let cuf_profiles = if self.cuf_profiles_loaded_like_cpp {
            Some(
                (0..wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP)
                    .map(|id| PlayerCufProfileSlotSaveLikeCpp {
                        profile_id: id as u8,
                        profile: self
                            .cuf_profiles_like_cpp
                            .get(id)
                            .and_then(Option::as_ref)
                            .map(|profile| PlayerCufProfileSaveLikeCpp {
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
                            }),
                    })
                    .collect(),
            )
        } else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping represented CUF profile save because character_cuf_profiles was not loaded coherently"
            );
            None
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
        let Some(mut runtime) = self.player_spell_runtime_snapshot_like_cpp() else {
            return;
        };
        runtime.rows.retain(|_, spell| {
            if spell.state == RepresentedPlayerSpellStateLikeCpp::Removed {
                return false;
            }
            if spell.state != RepresentedPlayerSpellStateLikeCpp::Temporary {
                spell.state = RepresentedPlayerSpellStateLikeCpp::Unchanged;
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
        let _ = self.replace_player_spell_runtime_like_cpp(runtime);
        self.sync_player_registry_state_like_cpp();
    }

    fn mark_player_skills_saved_like_cpp(&mut self) {
        let mut records = self.player_skill_records_like_cpp();
        let mut tombstones = self.player_skill_non_durable_tombstones_like_cpp();
        for skill in records.values_mut() {
            if skill.state == RepresentedPlayerSkillStateLikeCpp::Deleted {
                tombstones.insert(skill.skill_id);
            }
            skill.state = RepresentedPlayerSkillStateLikeCpp::Unchanged;
        }
        let occupied = self.complete_player_skill_occupied_slots_like_cpp();
        let _ = self.replace_player_skill_runtime_exact_like_cpp(
            records,
            true,
            occupied.is_some(),
            occupied,
            tombstones,
        );
        self.sync_player_registry_state_like_cpp();
    }

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
            self.reputation_mgr_like_cpp_mut()
                .mark_pending_save_to_db_committed_like_cpp();
        }
    }

    pub(crate) async fn save_current_player_to_db_like_cpp(&mut self) {
        // C++ `Player::SaveToDB` delays the next autosave for manual, code, and
        // autosave callers before it appends statements.
        self.reset_player_save_timer_like_cpp();

        let money_tracker = Arc::clone(&self.durable_loot_money_persistence_like_cpp);
        let money_save_fence = money_tracker.close_admission_for_save_like_cpp();
        trace!(fence = "player.save.mutations_closed", "persistence fence");
        self.wait_for_durable_item_loot_persistence_like_cpp().await;
        self.apply_pending_durable_item_loot_completions_with_objective_drain_like_cpp(false)
            .await;
        let money_state_is_determinate = self
            .reconcile_durable_loot_money_before_save_like_cpp()
            .await;
        if !money_state_is_determinate {
            // The same unknown transaction may also have committed talents,
            // reset metadata, inventory, or other absolute state. Do not let a
            // disconnect/autosave restore any pre-COMMIT runtime snapshot.
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_like_cpp()
                .await;
            return;
        }
        trace!(
            fence = "player.save.pending_durable_work_drained",
            "persistence fence"
        );
        let money_mutation_lock = money_tracker.lock_money_mutation_like_cpp().await;
        if money_tracker.is_indeterminate_like_cpp() {
            self.kick(
                "player persistence became indeterminate while waiting for the full-save money lock; aborting the entire save",
            );
            drop(money_mutation_lock);
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_like_cpp()
                .await;
            return;
        }

        let Some(snapshot) = self.sync_session_from_save_to_db_snapshot_like_cpp() else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                has_session_position = self.player_position_like_cpp().is_some(),
                has_canonical_map_manager = self.canonical_map_manager.is_some(),
                "Skipping Player::SaveToDB represented save because no coherent player snapshot is available"
            );
            drop(money_mutation_lock);
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_like_cpp()
                .await;
            return;
        };
        let Some(player_lifecycle_port) = self.player_lifecycle_port_like_cpp().map(Arc::clone)
        else {
            warn!(
                account = self.account_id,
                player_guid = ?self.player_guid(),
                "Skipping Player::SaveToDB represented save because lifecycle persistence is unavailable"
            );
            drop(money_mutation_lock);
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_like_cpp()
                .await;
            return;
        };

        let Some(request) =
            self.current_player_character_save_request_like_cpp(&snapshot, unix_now())
        else {
            self.kick(
                "player persistence became indeterminate before the full-save semantic snapshot; aborting the entire save",
            );
            drop(money_mutation_lock);
            drop(money_save_fence);
            self.drain_represented_quest_objective_progress_like_cpp()
                .await;
            return;
        };
        let mut cancellation_fence =
            PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(&money_tracker));
        let result = player_lifecycle_port.save_character_like_cpp(request).await;
        match result.outcome {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { rows } => {
                cancellation_fence.disarm_like_cpp();
                self.mark_current_player_save_to_db_committed_like_cpp(&result.committed);
                trace!(
                    publication = "player.save.dirty_state_clean",
                    "persistence publication"
                );
                info!(
                    guid = snapshot.guid.counter(),
                    statement_count = rows,
                    "Player::SaveToDB represented save committed in one CharacterDatabase transaction"
                );
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => {
                cancellation_fence.disarm_like_cpp();
                warn!(
                    guid = snapshot.guid.counter(),
                    "Failed to commit Player::SaveToDB represented transaction: {reason}"
                );
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                // The full save includes many absolute replacements. A money
                // row alone cannot establish whether that whole transaction
                // committed, so preserve dirty flags and force a reload before
                // any further money mutation can race an unknown durable base.
                money_tracker.mark_indeterminate_like_cpp();
                trace!(fence = "player.save.relogin_required", "persistence fence");
                cancellation_fence.disarm_like_cpp();
                self.kick(
                    "Player::SaveToDB COMMIT outcome is unknown; relog required before another money mutation",
                );
                warn!(
                    guid = snapshot.guid.counter(),
                    "Player::SaveToDB represented transaction COMMIT outcome is unknown: {reason}"
                );
            }
        }
        drop(money_mutation_lock);
        drop(money_save_fence);
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;
    }
}
