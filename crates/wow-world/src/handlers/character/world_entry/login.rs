// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Full post-instance player login sequence.

use super::super::spell_rules::{
    apply_skill_rewarded_spell_changes_to_login_like_cpp, favorite_known_spells_for_send_like_cpp,
    sync_loaded_fist_weapons_with_unarmed_like_cpp,
};
use super::super::vendor::rules::{LoadedItemRefundDecision, loaded_item_refund_decision};
use super::*;

mod action_buttons;
mod admission;
mod aura_loading;
mod cuf_profiles;
mod currency_loading;
mod default_skills;
mod glyph_loading;
mod group_loading;
mod mail_loading;
mod pet_loading;
mod reputation_loading;
mod skill_loading;
mod spell_loading;
mod spell_map_finalization;
mod talent_loading;
mod transport_restore;

use admission::LoginAdmissionDataLikeCpp;

impl WorldSession {
    /// Continue the player login after the instance socket is connected.
    ///
    /// Called when the `instance_link_rx` oneshot delivers the new channels.
    /// Sends ResumeComms and the full login sequence after the instance socket is connected.
    pub async fn handle_continue_player_login_with_module_registry_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        modules: &wow_module_api::ModuleRegistry,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
        player_rest_rates: &crate::session::PlayerRestRatePolicyLikeCpp,
        progression: &crate::session::ProgressionCatalogsLikeCpp,
        feature_policy: &SupportFeaturePolicyLikeCpp,
        player_grid_loader: &crate::session::PlayerGridLoadResolverLikeCpp,
    ) {
        let guid: ObjectGuid = match self.player_loading() {
            Some(g) => g,
            None => {
                warn!("handle_continue_player_login called but no player_loading set");
                return;
            }
        };
        self.set_player_loading(None);
        self.set_connect_to_key(None);
        self.set_connect_to_serial(None);

        // Send ResumeComms only when using ConnectTo flow.
        // In direct login (no session_mgr), the client didn't go through ConnectTo
        // and doesn't expect ResumeComms — sending it causes a disconnect.
        if self.session_mgr().is_some() {
            self.send_packet(&ResumeComms);
        }

        let Some(player_lifecycle_port) = self.player_lifecycle_port_like_cpp().map(Arc::clone)
        else {
            warn!("No player lifecycle persistence port for continue login");
            self.release_character_login_claim_like_cpp();
            return;
        };
        let base_row = match player_lifecycle_port
            .load_character_base_like_cpp(wow_persistence::PlayerCharacterBaseLoadRequestLikeCpp {
                player_guid: guid.counter() as u64,
            })
            .await
        {
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(Some(row)) => row,
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Loaded(None) => {
                warn!("Character {:?} not found in database", guid);
                self.release_character_login_claim_like_cpp();
                return;
            }
            wow_persistence::PlayerCharacterBaseLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load character {:?}: {reason}", guid);
                self.release_character_login_claim_like_cpp();
                return;
            }
        };

        let name = base_row.name.clone();
        // Store character name for chat messages.
        self.set_loaded_player_name_like_cpp(name.clone());
        let race = base_row.race;
        let class = base_row.class;
        let gender = base_row.gender;
        let level = base_row.level;
        // C++ CHAR_SEL_CHARACTER column order:
        // 7=xp, 8=money, 14..18=position/map/orientation, 21=createMode, 23..24=played time,
        // 28=resettalents_cost, 29=resettalents_time, 39=at_login, 40=zone.
        let mut zone: i32 = base_row.zone_id.unwrap_or(0) as i32; // smallint unsigned
        let at_login_flags = base_row.at_login_flags.unwrap_or(0);
        let create_mode = base_row.create_mode.unwrap_or(0);
        let mut map_id: i32 = base_row.map_id.unwrap_or(0) as i32; // smallint unsigned
        let saved_map_id_for_transport = map_id as u16;
        let saved_transport_guid_low = base_row.transport_guid_low.unwrap_or(0);
        let saved_transport_position = Position::new(
            base_row.transport_x.unwrap_or(0.0),
            base_row.transport_y.unwrap_or(0.0),
            base_row.transport_z.unwrap_or(0.0),
            base_row.transport_orientation.unwrap_or(0.0),
        );
        let pos_x = base_row.position_x.unwrap_or(0.0);
        let pos_y = base_row.position_y.unwrap_or(0.0);
        let pos_z = base_row.position_z.unwrap_or(0.0);
        let orientation = base_row.orientation.unwrap_or(0.0);

        let mut position = Position::new(pos_x, pos_y, pos_z, orientation);
        let display_id = default_display_id(race, gender);
        let Some(LoginAdmissionDataLikeCpp {
            saved_character_map_is_battleground,
            battleground_login_data,
            homebind: loaded_login_homebind,
            guild_id: loaded_guild_id_like_cpp,
        }) = self
            .load_login_admission_data_like_cpp(&player_lifecycle_port, guid, map_id)
            .await
        else {
            return;
        };
        let valid_login_homebind = loaded_login_homebind.filter(|homebind| {
            usable_character_homebind_like_cpp(
                *homebind,
                self.map_store().map(Arc::as_ref),
                self.expansion,
            )
        });
        let first_login = at_login_flags & 0x020 != 0;
        let Some(player_create_info) = player_bootstrap.create_info.get(race, class).copied()
        else {
            warn!(
                player_guid = guid.counter(),
                race,
                class,
                "C++ Player::_LoadHomeBind rejected missing/invalid playercreateinfo; aborting login"
            );
            self.kick("WorldSession::HandlePlayerLogin Player::_LoadHomeBind player info failed");
            return;
        };
        if loaded_login_homebind.is_some() && valid_login_homebind.is_none() {
            warn!(
                player_guid = guid.counter(),
                "repairing invalid, instanceable, or expansion-inaccessible character homebind like C++ Player::_LoadHomeBind"
            );
            self.delete_invalid_character_homebind_like_cpp(guid).await;
        }
        let repaired_or_valid_homebind = if let Some(homebind) = valid_login_homebind {
            Some(homebind)
        } else {
            self.repair_character_homebind_like_cpp(
                guid,
                race,
                player_create_info,
                create_mode,
                first_login,
            )
            .await
        };
        let Some(login_homebind) = repaired_or_valid_homebind else {
            warn!(
                player_guid = guid.counter(),
                race,
                class,
                create_mode,
                "C++ Player::_LoadHomeBind could not establish a valid homebind; aborting login"
            );
            self.kick("WorldSession::HandlePlayerLogin Player::_LoadHomeBind failed");
            return;
        };

        // C++ constructs the selected `Player` before hydrating its character
        // rows. Establish the generation-checked owner now so every following
        // load step writes directly into that one value instead of requiring a
        // Session-side bootstrap mirror.
        let attached_controller = self.ensure_login_player_controller_like_cpp(
            guid,
            name.clone(),
            position,
            map_id as u16,
            race,
            class,
            level,
            gender,
        );

        if !self
            .load_character_mail_for_login_like_cpp(&player_lifecycle_port, guid)
            .await
        {
            return;
        }

        // Load played time + money/xp from DB using C++ CHAR_SEL_CHARACTER order.
        self.total_played_time = base_row.total_played_time.unwrap_or(0);
        self.level_played_time = base_row.level_played_time.unwrap_or(0);
        if !self.set_player_gold_like_cpp(base_row.money.unwrap_or(0)) {
            self.kick("WorldSession::HandlePlayerLogin canonical Player money hydration failed");
            return;
        }
        if !self.set_player_inventory_slot_count_like_cpp(
            loaded_inventory_slot_count_with_legacy_rust_compat(
                base_row.inventory_slots.unwrap_or(INVENTORY_DEFAULT_SIZE),
            ),
        ) || !self.set_player_bank_bag_slot_count_like_cpp(base_row.bank_slots.unwrap_or(0))
        {
            self.kick("WorldSession::HandlePlayerLogin canonical Player inventory capacity hydration failed");
            return;
        }
        self.set_player_xp_like_cpp(base_row.xp.unwrap_or(0));
        if !self.set_represented_talent_reset_state_like_cpp(
            base_row.talent_reset_cost.unwrap_or(0),
            base_row.talent_reset_time_secs.unwrap_or(0),
        ) || !self
            .set_represented_active_talent_group_like_cpp(base_row.active_talent_group.unwrap_or(0))
            || !self.set_represented_bonus_talent_groups_like_cpp(
                base_row.bonus_talent_groups.unwrap_or(0),
            )
        {
            self.kick(
                "WorldSession::HandlePlayerLogin canonical Player specialization hydration failed",
            );
            return;
        }
        self.set_player_create_mode_like_cpp(create_mode);
        if !self.set_represented_at_login_flags_like_cpp(at_login_flags) {
            self.kick(
                "canonical Player persistent-capability owner unavailable during login hydration",
            );
            return;
        }
        let saved_rest_state = base_row.rest_state.unwrap_or(REST_STATE_NORMAL_LIKE_CPP);
        let saved_rest_bonus = base_row.rest_bonus.unwrap_or(0.0);
        let saved_logout_time_secs = base_row.logout_time_secs.unwrap_or(0);
        let saved_logout_was_resting = base_row.logout_was_resting.unwrap_or(0) != 0;
        self.load_represented_explored_zones_like_cpp(&base_row.explored_zones);
        self.set_player_guid(Some(guid));
        self.set_loaded_player_flags_like_cpp(base_row.player_flags.unwrap_or(0));
        self.set_loaded_player_flags_ex_like_cpp(base_row.player_flags_ex.unwrap_or(0));
        self.set_loaded_player_identity_like_cpp(map_id as u16, race, class, level, gender);
        // C++ recalculates zone/area from terrain after AddToMap
        // (`Player::SendInitialPacketsAfterAddToMap`). Seed from DB until
        // that post-add terrain pass runs.
        self.set_player_zone_area_like_cpp(zone as u32, zone as u32);
        if !self.set_represented_homebind_like_cpp(RepresentedHomebindLikeCpp {
            map_id: login_homebind.map_id,
            area_id: login_homebind
                .bind_area_id
                .expect("validated character homebind must have an area ID"),
            position: login_homebind.position,
        }) {
            self.kick("canonical Player homebind owner unavailable during login hydration");
            return;
        }
        if let Some(guild_id) = loaded_guild_id_like_cpp {
            if !self.set_represented_guild_id_like_cpp(guild_id) {
                self.kick("canonical Player guild owner unavailable during login hydration");
                return;
            }
        }
        self.load_represented_player_difficulties_like_cpp(
            base_row.dungeon_difficulty.unwrap_or(0),
            base_row.raid_difficulty.unwrap_or(0),
            base_row.legacy_raid_difficulty.unwrap_or(0),
        );
        self.load_represented_login_pet_state_like_cpp(
            &player_lifecycle_port,
            guid,
            base_row.summoned_pet_number.unwrap_or(0),
        )
        .await;
        self.load_group_membership_for_login_like_cpp(&player_lifecycle_port, guid)
            .await;
        self.refresh_next_level_xp_with_catalogs_like_cpp(progression);
        self.clamp_loaded_player_xp_to_next_level_like_cpp();
        if saved_character_map_is_battleground {
            // Rust does not yet have a live BattlegroundMgr roster/status
            // authority, so it cannot prove C++'s `currentBg &&
            // IsPlayerInBattleground && status != WAIT_LEAVE` resume branch.
            // Follow C++'s BG-unavailable branch instead of fabricating or
            // joining a canonical map from stale DB data.
            let fallback = battleground_login_fallback_location_like_cpp(
                battleground_login_data,
                Some(login_homebind),
                self.map_store().map(Arc::as_ref),
            );
            if let Some(fallback) = fallback {
                let fallback_map_id = u16::try_from(fallback.map_id)
                    .expect("validated battleground login fallback map ID");
                map_id = i32::from(fallback_map_id);
                position = fallback.position;
                self.seed_login_location_zone_area_like_cpp(&mut zone, fallback);
                self.set_player_map_position_like_cpp(fallback_map_id, fallback.position);
                let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
                info!(
                    player_guid = guid.counter(),
                    map_id,
                    "battleground runtime unavailable; relocated to entry point/homebind like C++ Player::LoadFromDB"
                );
            } else {
                warn!(
                    player_guid = guid.counter(),
                    saved_map_id = map_id,
                    "battleground unavailable and no valid entry point/homebind was loaded"
                );
            }
        } else if attached_controller {
            let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        }
        if self.retry_login_at_homebind_like_cpp(
            &mut map_id,
            &mut zone,
            &mut position,
            login_homebind,
        ) {
            info!(
                player_guid = guid.counter(),
                map_id,
                "initial canonical map selection failed; relocated to homebind like C++ Player::LoadFromDB"
            );
        }
        if attached_controller {
            #[cfg(test)]
            self.apply_loaded_player_flags_to_canonical_like_cpp();
            let _ = self.apply_represented_group_leader_flag_like_cpp();
        }
        self.load_represented_xp_rest_bonus_like_cpp(saved_rest_state, saved_rest_bonus);
        let applied_rest_bonus = self.apply_offline_xp_rest_bonus_with_policy_like_cpp(
            player_rest_rates,
            saved_logout_time_secs,
            wow_core::GameTime::now().as_secs(),
            saved_logout_was_resting,
        );
        if std::env::var_os("RUSTYCORE_REST_TRACE").is_some() {
            info!(
                player_guid = guid.counter(),
                saved_rest_state,
                saved_rest_bonus,
                saved_logout_time_secs,
                saved_logout_was_resting,
                applied_rest_bonus,
                rest_bonus = self.resolved_xp_rest_bonus_like_cpp(),
                rest_state = self.resolved_xp_rest_state_like_cpp(),
                "RUST_PLAYER_REST_LOAD"
            );
        }
        self.load_represented_character_titles_like_cpp(
            &base_row.known_titles.clone().unwrap_or_default(),
            base_row.chosen_title.unwrap_or(0),
        );

        self.load_account_toys_like_cpp().await;
        self.load_account_heirlooms_like_cpp().await;
        self.load_account_item_appearances_like_cpp().await;
        self.load_account_transmog_illusions_like_cpp().await;
        let account_mount_rows_complete_like_cpp = self.load_account_mounts_like_cpp().await;

        let realm_id = self.realm_id();
        let inventory = self
            .load_inventory_for_login_like_cpp(&player_lifecycle_port, guid, realm_id)
            .await;
        let Some(inventory) = inventory else {
            return;
        };
        let visible_items = inventory.visible_items;
        let inv_slots = inventory.inv_slots;
        let item_creates = inventory.item_creates;
        let loaded_equipped_item_guids = inventory.loaded_equipped_item_guids;
        let loaded_item_time_updates = inventory.loaded_item_time_updates;
        let loaded_non_equipped_enchantment_updates =
            inventory.loaded_non_equipped_enchantment_updates;
        self.load_cuf_profiles_for_login_like_cpp(&player_lifecycle_port, guid)
            .await;

        if !self
            .load_character_currencies_for_login_like_cpp(&player_lifecycle_port, guid)
            .await
        {
            return;
        }

        // ── Load known spells from character_spell ──
        let spell_loading::LoginSpellRowsLikeCpp {
            player_spell_rows: loaded_player_spell_rows,
            side_effect_spells: mut loaded_spell_side_effect_spells,
            mut known_spells,
            complete: loaded_player_spell_rows_complete_like_cpp,
        } = self
            .load_character_spell_rows_for_login_like_cpp(&player_lifecycle_port, guid)
            .await;
        let (favorite_spell_rows, favorite_spell_rows_complete_like_cpp) = self
            .load_character_favorite_spells_for_login_like_cpp(&player_lifecycle_port, guid)
            .await;
        let mut skill_rewarded_dependent_spells = HashSet::new();
        let mut skill_rewarded_removed_spells = HashSet::new();

        // ── C++ Player::_LoadSkills ──
        let (mut skill_records, loaded_skill_records_like_cpp) = self
            .load_character_skill_rows_for_login_like_cpp(&player_lifecycle_port, guid)
            .await;
        let mut skill_info_by_id = BTreeMap::<u16, wow_data::SkillInfoEntry>::new();

        // C++ `_LoadSkills` rejects forbidden race/class rows, fixes language,
        // mono and level ranges, then `UpdateSkillsForLevel` applies
        // ALWAYS_MAX_VALUE before learning the skill-rewarded spells.
        if let (Some(skill_store), Some(skill_line_store), Some(skill_tiers_store)) = (
            self.skill_store().cloned(),
            self.skill_line_store().cloned(),
            self.skill_tiers_store().cloned(),
        ) {
            let mut normalized_records = HashMap::new();
            let mut persisted_records: Vec<_> = skill_records.into_values().collect();
            persisted_records.sort_by_key(|skill| skill.skill_id);
            for mut skill_record in persisted_records {
                let Some(entry) = skill_store.loaded_skill_info_like_cpp(
                    skill_record.skill_id,
                    race,
                    class,
                    level,
                    skill_record.value,
                    skill_record.max,
                    skill_line_store.as_ref(),
                    skill_tiers_store.as_ref(),
                ) else {
                    warn!(
                        player_guid = guid.counter(),
                        race,
                        class,
                        skill_id = skill_record.skill_id,
                        "Skipping forbidden persisted skill like C++ Player::_LoadSkills"
                    );
                    continue;
                };
                skill_record.step = entry.step;
                skill_record.value = entry.rank;
                skill_record.max = entry.max_rank;
                // Pinned 3.4.3 C++ `_LoadSkills` also inserts a status and
                // initial update-field slot when the persisted value is zero.
                // `HasSkill` then remains false, allowing the later
                // `LearnDefaultSkills` pass to reactivate a default skill.
                normalized_records.insert(skill_record.skill_id, skill_record);
                skill_info_by_id.insert(entry.skill_id, entry);
            }
            skill_records = normalized_records;
            sync_loaded_fist_weapons_with_unarmed_like_cpp(
                &mut skill_records,
                &mut skill_info_by_id,
                level,
            );
        }

        if loaded_skill_records_like_cpp
            && !self.replace_player_skill_records_like_cpp(skill_records.clone(), true, false)
        {
            self.kick("canonical Player skill owner unavailable while loading skills");
            return;
        }
        for entry in skill_info_by_id.values() {
            let mut changes = self.skill_rewarded_spell_changes_for_login_like_cpp(
                entry.skill_id,
                entry.rank,
                race,
                class,
                level,
            );
            // C++ `_LoadSkills` runs before `_LoadSpells`, so its RemoveSpell
            // branch cannot remove a character_spell row that has not been
            // loaded yet. The later LearnDefaultSkills pass below runs after
            // `_LoadSpells` and does apply removals.
            changes.remove.clear();
            apply_skill_rewarded_spell_changes_to_login_like_cpp(
                &mut known_spells,
                &mut loaded_spell_side_effect_spells,
                &mut skill_rewarded_dependent_spells,
                &mut skill_rewarded_removed_spells,
                changes,
            );
        }

        let talent_rows_complete_like_cpp = self
            .load_character_talents_for_login_like_cpp(
                &player_lifecycle_port,
                player_bootstrap,
                guid,
                &mut known_spells,
                &mut skill_rewarded_dependent_spells,
            )
            .await;

        let custom_spell_count = self.apply_represented_start_all_spells_with_catalogs_like_cpp(
            player_bootstrap,
            &mut known_spells,
        );
        if custom_spell_count > 0 {
            info!(
                player_guid = guid.counter(),
                custom_spell_count,
                "Applied represented C++ Player::LearnCustomSpells / CONFIG_START_ALL_SPELLS"
            );
        }
        let mut loaded_dependency_roots = loaded_spell_side_effect_spells.clone();
        loaded_dependency_roots.extend(known_spells.iter().copied());
        loaded_dependency_roots.sort_unstable();
        loaded_dependency_roots.dedup();
        let dependent_spell_count = self.apply_loaded_spell_dependencies_from_roots_like_cpp(
            &loaded_dependency_roots,
            &mut known_spells,
        );
        if dependent_spell_count > 0 {
            info!(
                player_guid = guid.counter(),
                dependent_spell_count,
                "Applied represented C++ Player::_LoadSpells/AddSpell spell_learn_spell dependencies"
            );
        }
        for &spell_id in &known_spells {
            if !loaded_spell_side_effect_spells.contains(&spell_id) {
                loaded_spell_side_effect_spells.push(spell_id);
            }
        }
        let login_proficiencies =
            self.apply_login_known_spell_proficiencies_like_cpp(&loaded_spell_side_effect_spells);
        if login_proficiencies > 0 {
            info!(
                player_guid = guid.counter(),
                login_proficiencies,
                "Applied represented login spell proficiencies like C++ Player::_LoadSpells/AddSpell"
            );
        }
        let login_combat_capabilities = self
            .apply_login_known_spell_combat_capabilities_like_cpp(&loaded_spell_side_effect_spells);
        if login_combat_capabilities > 0 {
            info!(
                player_guid = guid.counter(),
                login_combat_capabilities,
                "Applied represented login parry/block capabilities like C++ Player::_LoadSpells/AddSpell"
            );
        }
        let inactive_lower_rank_count =
            self.deactivate_lower_rank_known_spells_for_send_like_cpp(&mut known_spells);
        if inactive_lower_rank_count > 0 {
            info!(
                player_guid = guid.counter(),
                inactive_lower_rank_count,
                "Deactivated represented lower-rank known spells like C++ Player::AddSpell"
            );
        }

        // Store final known_spells in session for later use (ShowTradeSkill, etc.)
        self.set_known_spells_like_cpp(known_spells.clone());
        self.set_represented_favorite_known_spells_like_cpp(favorite_spell_rows.clone());
        let login_passive_auras = self.apply_login_passive_known_spell_auras_like_cpp();
        if login_passive_auras > 0 {
            info!(
                player_guid = guid.counter(),
                login_passive_auras,
                "Applied represented login passive spell auras like C++ Player::_LoadSpells/AddSpell"
            );
        }
        let prev_rank_passive_auras =
            self.apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(&known_spells);
        if prev_rank_passive_auras > 0 {
            info!(
                player_guid = guid.counter(),
                prev_rank_passive_auras,
                "Applied represented C++ Player::_LoadSpells/AddSpell previous-rank passive auras"
            );
        }
        let promoted_character_mounts =
            self.promote_loaded_character_mount_spells_like_cpp(&known_spells);
        if promoted_character_mounts > 0 {
            info!(
                player_guid = guid.counter(),
                promoted_character_mounts,
                "Promoted loaded character mount spells into the represented account mount collection like C++ Player::_LoadSpells -> AddMount"
            );
        }

        self.load_character_glyphs_for_login_like_cpp(
            &player_lifecycle_port,
            player_bootstrap,
            guid,
        )
        .await;

        let Some(action_buttons) = self
            .load_action_buttons_for_login_like_cpp(&player_lifecycle_port, guid)
            .await
        else {
            return;
        };

        // Store current map and character info for VALUES updates + stat recalculation
        self.set_loaded_player_identity_like_cpp(map_id as u16, race, class, level, gender);
        let validated_persisted_transport_login = self
            .restore_persisted_transport_for_login_like_cpp(
                guid,
                saved_transport_guid_low,
                saved_map_id_for_transport,
                saved_transport_position,
                saved_character_map_is_battleground,
                login_homebind,
                attached_controller,
                &mut map_id,
                &mut zone,
                &mut position,
            )
            .await;
        self.refresh_next_level_xp_with_catalogs_like_cpp(progression);
        // NOTE: known_spells is stored below after DBC merge (see "Merge DBC auto-learned spells")

        let reputation_rows_complete_like_cpp = self
            .load_character_reputation_for_login_like_cpp(&player_lifecycle_port, guid)
            .await;

        // C++ `Player::LoadFromDB` restores `fields.health` after `UpdateAllStats`,
        // clamping it to the recalculated max and preserving zero as corpse state.
        let saved_health = base_row.health;
        let loaded_powers = std::array::from_fn(|index| {
            base_row.powers[index].unwrap_or(0).min(i32::MAX as u32) as i32
        });
        self.set_loaded_player_powers_like_cpp(loaded_powers);
        let saved_power0 = loaded_powers[0];

        // Load active quests from characters DB
        self.load_player_quests().await;

        let persisted_skill_count = skill_info_by_id.len();
        let Some(default_skill_entries) = self.apply_default_skills_for_login_like_cpp(
            race,
            class,
            level,
            &mut skill_records,
            &mut skill_info_by_id,
        ) else {
            return;
        };

        for entry in &default_skill_entries {
            let changes = self.skill_rewarded_spell_changes_for_login_like_cpp(
                entry.skill_id,
                entry.rank,
                race,
                class,
                level,
            );
            apply_skill_rewarded_spell_changes_to_login_like_cpp(
                &mut known_spells,
                &mut loaded_spell_side_effect_spells,
                &mut skill_rewarded_dependent_spells,
                &mut skill_rewarded_removed_spells,
                changes,
            );
        }

        // Default skill spells run through C++ AddSpell just like DB-loaded
        // spells. Re-run the idempotent represented side effects so newly
        // learned dependencies, proficiencies, capabilities and passives are
        // present before the initial player CreateObject.
        let (default_dependent_spell_count, loaded_spell_skills_complete_like_cpp) = self
            .apply_loaded_spell_dependency_skills_like_cpp(
                &mut known_spells,
                &mut loaded_spell_side_effect_spells,
            );
        if loaded_skill_records_like_cpp && loaded_spell_skills_complete_like_cpp {
            let Some(canonical_skill_records) = self.resolved_player_skill_records_like_cpp()
            else {
                self.kick("canonical Player skill owner unavailable during login finalization");
                return;
            };
            skill_records = canonical_skill_records;
            let occupied_slots = u16::try_from(skill_records.len()).unwrap_or(u16::MAX);
            if !self
                .set_complete_player_skill_records_like_cpp(skill_records.clone(), occupied_slots)
            {
                warn!(
                    player_guid = guid.counter(),
                    occupied_slots, "Could not authorize represented post-login player skill slots"
                );
            }
        }
        let default_inactive_lower_rank_count =
            self.deactivate_lower_rank_known_spells_for_send_like_cpp(&mut known_spells);
        self.set_known_spells_like_cpp(known_spells.clone());
        self.apply_login_known_spell_proficiencies_like_cpp(&loaded_spell_side_effect_spells);
        self.apply_login_known_spell_combat_capabilities_like_cpp(&loaded_spell_side_effect_spells);
        self.apply_login_passive_known_spell_auras_like_cpp();
        self.apply_loaded_known_spell_previous_rank_passive_auras_like_cpp(&known_spells);
        self.promote_loaded_character_mount_spells_like_cpp(&known_spells);

        let login_spell_map_authority_complete_like_cpp = loaded_player_spell_rows_complete_like_cpp
            && favorite_spell_rows_complete_like_cpp
            && talent_rows_complete_like_cpp
            && account_mount_rows_complete_like_cpp
            && reputation_rows_complete_like_cpp;
        self.finalize_player_spell_map_for_login_like_cpp(
            guid,
            loaded_player_spell_rows,
            &favorite_spell_rows,
            &skill_rewarded_dependent_spells,
            &skill_rewarded_removed_spells,
            login_spell_map_authority_complete_like_cpp,
        );

        info!(
            player_guid = guid.counter(),
            loaded_skill_count = persisted_skill_count,
            default_skill_count = default_skill_entries.len(),
            default_dependent_spell_count,
            default_inactive_lower_rank_count,
            total_spell_count = known_spells.len(),
            "Applied C++ LearnDefaultSkills and LearnSkillRewardedSpells"
        );

        let skill_info_tuples: Vec<(u16, u16, u16, u16, u16, i16, u16)> = skill_info_by_id
            .values()
            .map(|entry| {
                (
                    entry.skill_id,
                    entry.step,
                    entry.rank,
                    entry.starting_rank,
                    entry.max_rank,
                    entry.temp_bonus,
                    entry.perm_bonus,
                )
            })
            .collect();

        self.load_completed_achievements_like_cpp().await;
        self.load_instance_time_restrictions_like_cpp().await;
        self.load_player_account_data_like_cpp(guid).await;
        self.load_character_auras_for_login_like_cpp(&player_lifecycle_port, guid)
            .await;
        // C++ `Player::LoadFromDB` runs `_LoadAuras` before `_LoadInventory`,
        // whose final `_ApplyAllItemMods` pass applies, for each equipment slot,
        // the item-set effect, regular equip spell, and enchantments before
        // advancing to the next item. Keep the replay here so loaded and
        // item-provided auras receive the same slot order.
        let initial_item_mods =
            self.apply_initial_loaded_item_mods_like_cpp(&loaded_equipped_item_guids);
        if initial_item_mods.item_set_auras > 0 {
            info!(
                player_guid = guid.counter(),
                initial_item_set_auras = initial_item_mods.item_set_auras,
                "Applied represented initial item-set auras like C++ Player::_ApplyAllItemMods"
            );
        }
        if initial_item_mods.item_equip_auras > 0 {
            info!(
                player_guid = guid.counter(),
                initial_item_equip_auras = initial_item_mods.item_equip_auras,
                "Applied represented initial item equip auras like C++ Player::_ApplyAllItemMods"
            );
        }
        let loaded_enchantment_updates = initial_item_mods.enchantments;

        // C++ defers `UpdateAllStats` and the saved-health clamp until after
        // `_LoadAuras` and `_LoadInventory` have applied every aura and item
        // modifier. Build the initial self snapshot at the same boundary.
        let (combat, base_mana_like_cpp, current_power0) = if let Some(combat) =
            self.player_login_combat_stats_like_cpp(race, class, level, saved_health, saved_power0)
        {
            combat
        } else {
            warn!(
                "Missing C++ player stats or ChrClasses coefficients for race={race} class={class} level={level}; using fallback"
            );
            let (h, m) = default_health_mana(class);
            let combat = PlayerCombatStats {
                health: restored_saved_health_like_cpp(saved_health, h as i64),
                max_health: h as i64,
                base_mana: m as i32,
                max_mana: m as i64,
                ..PlayerCombatStats::default()
            };
            let max_power0 = primary_max_power_for_class_like_cpp(class, combat.max_mana);
            (combat, m as i32, saved_power0.clamp(0, max_power0.max(0)))
        };

        info!(
            "Player '{}' ({:?}) continuing login at map {} ({}, {}, {}), {} equipped items, \
             HP={} Mana={} AP={} STR/AGI/STA/INT/SPI={:?} Armor={} Dodge={:.1}% Crit={:.1}%",
            name,
            guid,
            map_id,
            pos_x,
            pos_y,
            pos_z,
            item_creates.len(),
            combat.max_health,
            combat.max_mana,
            combat.attack_power,
            combat.stats,
            combat.base_armor,
            combat.dodge_pct,
            combat.crit_pct
        );

        let login_known_spells = self.login_known_spells_after_account_collections_like_cpp();
        let login_favorite_spells =
            favorite_known_spells_for_send_like_cpp(&login_known_spells, &favorite_spell_rows);
        let (spell_history_entries, spell_charge_entries) = self
            .load_character_spell_history_packets_like_cpp(guid)
            .await;
        // Persist the login snapshot so the before-add init helper can re-send spell
        // history/charges on far teleport without a DB round trip. #NEXT.R8.ENTITIES.1229.
        self.record_login_spell_history_packets_like_cpp(
            spell_history_entries.clone(),
            spell_charge_entries.clone(),
        );

        if !self
            .send_login_sequence(
                item_guid_generator,
                player_bootstrap.trait_node_entries.as_ref(),
                creature_spawn_catalogs,
                feature_policy,
                player_grid_loader,
                guid,
                race,
                class,
                gender,
                level,
                display_id,
                &position,
                map_id,
                zone,
                login_homebind,
                validated_persisted_transport_login,
                visible_items,
                inv_slots,
                item_creates,
                combat,
                current_power0,
                base_mana_like_cpp,
                login_known_spells,
                login_favorite_spells,
                spell_history_entries,
                spell_charge_entries,
                action_buttons,
                skill_info_tuples,
                self.account_mount_rows_like_cpp(),
            )
            .await
        {
            self.abort_partial_login_sequence_like_cpp();
            return;
        }
        self.send_item_time_update_plans(&loaded_item_time_updates);
        self.send_item_enchant_time_update_plans(guid, &loaded_non_equipped_enchantment_updates);
        self.send_loaded_equipped_item_enchantment_updates_like_cpp(&loaded_enchantment_updates);
        self.apply_represented_login_spell_reset_if_needed_like_cpp();
        self.apply_represented_login_talent_reset_if_needed_like_cpp();
        let applied_first_login_like_cpp =
            self.apply_represented_first_login_flag_if_needed_like_cpp();
        if applied_first_login_like_cpp {
            self.apply_represented_first_login_cast_spells_with_catalogs_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                player_bootstrap,
            )
            .await;
            self.apply_represented_first_login_explored_zones_with_catalogs_like_cpp(
                player_bootstrap,
            );
            self.apply_represented_first_login_reputation_with_catalogs_like_cpp(player_bootstrap);
        }

        // C++ processes reset-at-login and first-login casts after the initial
        // map packet sequence. Publish only after those normal mutations. The
        // first-login cast closure is not represented losslessly, so that
        // Player remains fail-closed for this entire session.
        if applied_first_login_like_cpp {
            self.tombstone_player_spell_hit_aura_authority_like_cpp();
        } else {
            let _ = self.sync_player_spell_hit_aura_authority_to_canonical_like_cpp();
        }

        // Mark online in DB. This remains best-effort at the existing Rust
        // sequencing point; #432 changes ownership, not login timing.
        let _ = player_lifecycle_port
            .mark_player_online_like_cpp(wow_persistence::PlayerOnlineMarkRequestLikeCpp {
                player_guid: guid.counter() as u32,
            })
            .await;

        // C++ `sScriptMgr->OnPlayerLogin(pCurrChar, firstLogin)`
        // (`CharacterHandler.cpp:1452`), after the completed login and after
        // the login criteria update. Trusted linked modules observe here.
        self.dispatch_module_player_login_like_cpp(modules, first_login);
    }
}
