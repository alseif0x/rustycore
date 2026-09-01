// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot sources: creature corpses, chests, gathering nodes, fishing and containers.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use super::*;

impl WorldSession {
    pub(crate) async fn open_represented_gameobject_chest_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.player_is_alive_like_cpp() {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }

        self.record_represented_gameobject_chest_release_metadata_like_cpp(gameobject_guid, source);

        let is_first_represented_unique_use = !self
            .represented_unique_gameobject_uses
            .contains(&gameobject_guid);
        if source.loot_id == 0 && is_first_represented_unique_use {
            self.represented_unique_gameobject_uses
                .insert(gameobject_guid);
            self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
                gameobject.add_unique_use_like_cpp(player_guid);
            });
            if source.should_autostore_push_loot_like_cpp() {
                self.autostore_represented_gameobject_chest_push_loot_like_cpp(
                    gameobject_guid,
                    source,
                )
                .await;
            }
            self.record_represented_gameobject_use_effects_like_cpp(
                gameobject_guid,
                player_guid,
                source.triggered_event_id,
                source.linked_trap_entry,
            );
        }
        let activated_now = self
            .set_represented_gameobject_loot_state_activated_like_cpp(gameobject_guid, player_guid);
        if activated_now {
            let _ =
                self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(gameobject_guid);
        }
        if !source.has_open_loot_like_cpp() {
            return;
        }

        let should_record_generation_effects =
            source.loot_id != 0 && !self.loot_table.contains_key(&gameobject_guid);
        let allowed_looters = if source.is_personal_encounter_loot_like_cpp() {
            Vec::new()
        } else if source.uses_personal_loot_like_cpp() {
            // C++ creates only `m_personalLoot[player]` for a personal chest
            // without a DungeonEncounter; group loot rules never widen it.
            vec![player_guid]
        } else if source.use_group_loot_rules {
            self.represented_group_looters_at_reward_distance_like_cpp(player_guid)
        } else {
            vec![player_guid]
        };
        self.ensure_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            player_guid,
            source,
            &allowed_looters,
        )
        .await;
        if should_record_generation_effects && self.loot_table.contains_key(&gameobject_guid) {
            self.record_represented_gameobject_use_effects_like_cpp(
                gameobject_guid,
                player_guid,
                source.triggered_event_id,
                source.linked_trap_entry,
            );
        }

        if self
            .sync_represented_gameobject_loot_to_canonical_like_cpp(gameobject_guid, player_guid)
            .is_none()
        {
            self.loot_table.remove(&gameobject_guid);
            return;
        }

        let Some(loot) = self.loot_table.get(&gameobject_guid) else {
            return;
        };
        // C++ keeps and sends an empty non-encounter
        // `m_personalLoot[player]`. Encounter generation instead discards
        // empty pools in `GenerateDungeonEncounterPersonalLoot`, so only the
        // former bypasses the generic item/money availability gate.
        let empty_non_encounter_personal_pool = source.uses_personal_loot_like_cpp()
            && !source.is_personal_encounter_loot_like_cpp()
            && loot.allowed_looters.contains(&player_guid);
        if !empty_non_encounter_personal_pool
            && !self.represented_loot_can_be_opened_by_player_like_cpp(
                gameobject_guid,
                loot,
                player_guid,
            )
        {
            return;
        }

        let response = LootResponse {
            owner: gameobject_guid,
            loot_obj: loot.loot_guid,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: self.represented_loot_money_for_player_like_cpp(
                gameobject_guid,
                loot,
                player_guid,
            ),
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        };

        if self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.represented_on_loot_opened_like_cpp(gameobject_guid, player_guid, response);
    }

    pub(crate) async fn open_represented_fishing_hole_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        loot_id: u32,
    ) {
        let player_guid = self.player_guid();
        let should_update_criteria = player_guid.is_some()
            && loot_id != 0
            && self.player_is_alive_like_cpp()
            && self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid);
        self.open_represented_gameobject_personal_loot_like_cpp(
            gameobject_guid,
            loot_id,
            LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
            true,
        )
        .await;
        if should_update_criteria {
            let player_guid = player_guid.expect("checked above");
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::FishingHoleCatchCriteriaUpdated {
                    gameobject_guid,
                    player_guid,
                    gameobject_entry,
                },
            );
        }
    }

    pub(crate) async fn open_represented_fishing_node_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        area_id: u32,
        junk: bool,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.player_is_alive_like_cpp() {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }
        let install_observation =
            self.represented_gameobject_loot_install_observation_like_cpp(gameobject_guid);
        if install_observation.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
            return;
        }

        let loot_type = if junk {
            LOOT_TYPE_FISHING_JUNK_LIKE_CPP
        } else {
            LOOT_TYPE_FISHING_LIKE_CPP
        };
        let loot_mode = if junk {
            LOOT_MODE_JUNK_FISH_LIKE_CPP
        } else {
            LOOT_MODE_DEFAULT_LIKE_CPP
        };
        let items = self
            .generate_represented_fishing_loot_items_like_cpp(area_id, loot_mode)
            .await
            .unwrap_or_else(|| {
                debug!(
                    area_id,
                    gameobject = ?gameobject_guid,
                    junk,
                    "fishing loot template unavailable"
                );
                Vec::new()
            });

        let Some(loot_guid) = self.next_represented_loot_object_guid_like_cpp(gameobject_guid)
        else {
            return;
        };
        self.loot_table.insert(
            gameobject_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items,
                looted_by_player: false,
            },
        );

        if let Some(loot) = self.loot_table.get_mut(&gameobject_guid) {
            mark_loot_allowed_for_player_like_cpp(loot, player_guid);
        }
        let upserted = self
            .loot_table
            .get(&gameobject_guid)
            .cloned()
            .and_then(|loot| {
                install_observation.as_ref().and_then(|observation| {
                    self.upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
                        gameobject_guid,
                        player_guid,
                        loot,
                        false,
                        observation,
                    )
                })
            });
        if upserted.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
            self.loot_table.remove(&gameobject_guid);
            return;
        }

        let Some(loot) = self.loot_table.get(&gameobject_guid) else {
            return;
        };
        if !self.represented_loot_can_be_opened_by_player_like_cpp(
            gameobject_guid,
            loot,
            player_guid,
        ) {
            return;
        }

        let response = LootResponse {
            owner: gameobject_guid,
            loot_obj: loot.loot_guid,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: loot.coins,
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        };

        if self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.represented_on_loot_opened_like_cpp(gameobject_guid, player_guid, response);
    }

    pub(crate) async fn open_represented_gathering_node_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        source: GatheringNodeUseSource,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.player_is_alive_like_cpp() {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }

        let is_first_represented_use = !self
            .represented_unique_gameobject_uses
            .contains(&gameobject_guid);
        if is_first_represented_use {
            self.represented_unique_gameobject_uses
                .insert(gameobject_guid);
            self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
                gameobject.add_unique_use_like_cpp(player_guid);
            });
        }

        self.open_represented_gameobject_personal_loot_like_cpp(
            gameobject_guid,
            source.loot_id,
            LOOT_TYPE_CHEST_LIKE_CPP,
            false,
        )
        .await;

        if is_first_represented_use {
            let xp = self.represented_gathering_node_xp_like_cpp(source.xp_difficulty);
            if xp != 0 {
                self.give_xp(xp, ObjectGuid::EMPTY, 1.0).await;
            }
            self.record_represented_gameobject_use_effects_like_cpp(
                gameobject_guid,
                player_guid,
                source.triggered_event_id,
                source.linked_trap_entry,
            );
        }
        self.record_represented_gathering_node_runtime_state_like_cpp(
            gameobject_guid,
            gameobject_entry,
            player_guid,
            source,
            is_first_represented_use,
        );
        let _ = self
            .queue_gathering_node_gameobject_state_refresh_for_same_map_like_cpp(gameobject_guid);
    }

    fn gathering_node_gameobject_state_refresh_command_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand> {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        Some(SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: self.player_map_id_like_cpp(),
            instance_id: self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0),
            go_type: state.go_type?,
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            go_state: state.go_state.map(|go_state| go_state as i8),
            dynamic_flags: state.dynamic_flags,
            gathering_node_loot_id: state.gathering_node_loot_id,
            personal_loot_uses: state.personal_loot_uses,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        })
    }

    fn chest_gameobject_state_refresh_command_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<SyncChestGameobjectStateAndRefreshLikeCppCommand> {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        let source = state.chest_loot_source?;
        Some(SyncChestGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: self.player_map_id_like_cpp(),
            instance_id: self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0),
            go_type: state.go_type.unwrap_or(GAMEOBJECT_TYPE_CHEST as u8),
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            chest_loot_id: source.loot_id,
            chest_personal_loot_id: source.personal_loot_id,
            chest_push_loot_id: source.push_loot_id,
            chest_quest_id: source.chest_quest_id,
            chest_restock_time_secs: source.chest_restock_time_secs,
            chest_consumable: source.chest_consumable,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        })
    }

    fn goober_gameobject_state_refresh_command_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<SyncGooberGameobjectStateAndRefreshLikeCppCommand> {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        Some(SyncGooberGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: self.player_map_id_like_cpp(),
            instance_id: self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0),
            go_type: state.go_type.unwrap_or(GAMEOBJECT_TYPE_GOOBER as u8),
            gameobject_flags: state.gameobject_flags,
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            go_state: state.go_state.map(|go_state| go_state as i8),
            dynamic_flags: state.dynamic_flags,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        })
    }

    pub(crate) fn queue_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let Some(command) = self.chest_gameobject_state_refresh_command_like_cpp(gameobject_guid)
        else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command.clone()),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    pub(crate) fn queue_goober_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let Some(command) = self.goober_gameobject_state_refresh_command_like_cpp(gameobject_guid)
        else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command.clone()),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    pub(crate) fn queue_visible_gameobject_packet_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                        queued_at: Instant::now(),
                        source_guid: gameobject_guid,
                        map_id: current_map_id,
                        instance_id: current_instance_id,
                        packet_bytes: packet_bytes.clone(),
                    }),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    pub(super) fn represented_creature_is_dead_for_loot_visibility_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> bool {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = self.map_manager.as_ref()
            && let Some(creature) = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .find_creature(map_id, instance_id, creature_guid)
        {
            return !creature.is_alive();
        }

        let Some(map_key) =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        manager
            .find_map(map_key.map_id, map_key.instance_id)
            .and_then(|map| map.map().get_typed_creature(creature_guid))
            .is_some_and(|creature| !creature.is_alive())
    }

    fn queue_gathering_node_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let Some(command) =
            self.gathering_node_gameobject_state_refresh_command_like_cpp(gameobject_guid)
        else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(
                        command.clone(),
                    ),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    fn set_represented_gameobject_loot_state_activated_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        if state.loot_state == Some(LootState::Activated) {
            return false;
        }

        state.loot_state = Some(LootState::Activated);
        state.loot_state_unit_guid = player_guid;
        true
    }

    fn record_represented_gathering_node_runtime_state_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        player_guid: ObjectGuid,
        source: GatheringNodeUseSource,
        is_first_represented_use: bool,
    ) {
        {
            let state = self
                .represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default();
            if is_first_represented_use {
                state.personal_loot_uses = state.personal_loot_uses.saturating_add(1);
            }
            state.go_type = Some(GAMEOBJECT_TYPE_GATHERING_NODE as u8);
            state.gathering_node_loot_id = Some(source.loot_id);
            if state.personal_loot_uses >= source.max_loots {
                state.go_state = Some(GoState::Active);
                state.dynamic_flags |= GO_DYNFLAG_LO_NO_INTERACT;
            }
            state.linked_trap_entry =
                (source.linked_trap_entry != 0).then_some(source.linked_trap_entry);
        }

        let activated_now = self
            .set_represented_gameobject_loot_state_activated_like_cpp(gameobject_guid, player_guid);
        if activated_now && source.despawn_delay_secs != 0 {
            if let Some(state) = self
                .represented_gameobject_use_states
                .get_mut(&gameobject_guid)
            {
                state.despawn_delay_secs = Some(source.despawn_delay_secs);
                state.despawn_delay_until = Some(
                    Instant::now() + Duration::from_secs(u64::from(source.despawn_delay_secs)),
                );
            }
        }

        if is_first_represented_use && source.spell_id != 0 {
            self.apply_represented_gameobject_post_use_spell_like_cpp(
                gameobject_guid,
                player_guid,
                gameobject_entry,
                GAMEOBJECT_TYPE_GATHERING_NODE,
                source.spell_id,
                false,
                RepresentedGameObjectSpellCaster::User,
                player_guid,
            );
        }
    }

    fn record_represented_gameobject_use_effects_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        triggered_event_id: u32,
        linked_trap_entry: u32,
    ) {
        if triggered_event_id != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: triggered_event_id,
                },
            );
        }
        if linked_trap_entry != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                    gameobject_guid,
                    player_guid,
                    trap_entry: linked_trap_entry,
                },
            );
        }
    }

    fn represented_gathering_node_xp_like_cpp(&self, xp_difficulty: u32) -> u32 {
        if xp_difficulty == 0 || xp_difficulty >= 10 {
            return 0;
        }

        self.quest_xp_store
            .as_ref()
            .map(|store| {
                store.player_level_difficulty_xp_like_cpp(
                    self.player_level_like_cpp(),
                    xp_difficulty,
                )
            })
            .unwrap_or(0)
    }

    async fn open_represented_gameobject_personal_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        loot_id: u32,
        loot_type: u8,
        replace_existing: bool,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if loot_id == 0 || !self.player_is_alive_like_cpp() {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }

        // Fishing holes replace this player's personal `Loot` in place. Close
        // the old C++ view before the upsert so its release cannot detach or
        // apply lifecycle state to the freshly generated pool.
        if replace_existing && self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }

        // C++ serializes template generation and `ClearLoot` on the map
        // thread. Rust awaits database-backed template generation, so retain
        // the exact object lifetime and authority tombstone across that await.
        let install_observation =
            self.represented_gameobject_loot_install_observation_like_cpp(gameobject_guid);
        if install_observation.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
            return;
        }

        if !replace_existing
            && let Some(snapshot) = self
                .represented_owned_loot_authority_like_cpp(gameobject_guid)
                .and_then(|authority| authority.snapshot_for_player_like_cpp(player_guid))
        {
            self.loot_table.insert(gameobject_guid, snapshot.loot);
            self.represented_loot_cache_generations_like_cpp
                .insert(gameobject_guid, snapshot.generation);
        }

        if replace_existing || !self.loot_table.contains_key(&gameobject_guid) {
            let items = self
                .generate_represented_gameobject_loot_items_for_store_like_cpp(
                    loot_id,
                    LootStoreKind::Gameobject,
                    LOOT_MODE_DEFAULT_LIKE_CPP,
                    None,
                )
                .await
                .unwrap_or_else(|| {
                    debug!(
                        loot_id,
                        gameobject = ?gameobject_guid,
                        "gameobject personal loot template unavailable"
                    );
                    Vec::new()
                });
            let Some(loot_guid) = self.next_represented_loot_object_guid_like_cpp(gameobject_guid)
            else {
                return;
            };
            self.loot_table.insert(
                gameobject_guid,
                CreatureLoot {
                    loot_guid,
                    coins: 0,
                    unlooted_count: 0,
                    loot_type,
                    dungeon_encounter_id: 0,
                    loot_method: 0,
                    loot_master: ObjectGuid::EMPTY,
                    round_robin_player: ObjectGuid::EMPTY,
                    player_ffa_items: Vec::new(),
                    players_looting: Vec::new(),
                    allowed_looters: Vec::new(),
                    items,
                    looted_by_player: false,
                },
            );
        }

        if let Some(loot) = self.loot_table.get_mut(&gameobject_guid) {
            mark_loot_allowed_for_player_like_cpp(loot, player_guid);
        }
        self.represented_personal_loot_owners
            .insert(gameobject_guid);
        if let Some(loot) = self.loot_table.get(&gameobject_guid).cloned() {
            let upserted = install_observation.as_ref().and_then(|observation| {
                self.upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
                    gameobject_guid,
                    player_guid,
                    loot,
                    replace_existing,
                    observation,
                )
            });
            if upserted.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
                self.loot_table.remove(&gameobject_guid);
                self.represented_personal_loot_owners
                    .remove(&gameobject_guid);
                return;
            }
        }

        let Some(loot) = self.loot_table.get(&gameobject_guid) else {
            return;
        };
        if !self.represented_loot_can_be_opened_by_player_like_cpp(
            gameobject_guid,
            loot,
            player_guid,
        ) {
            return;
        }

        let response = LootResponse {
            owner: gameobject_guid,
            loot_obj: loot.loot_guid,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: loot.coins,
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        };

        if !replace_existing && self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.represented_on_loot_opened_like_cpp(gameobject_guid, player_guid, response);
    }

    pub(super) fn sync_represented_gameobject_loot_to_canonical_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<()> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(gameobject_guid)
        else {
            return (represented_local_loot_fixture_allowed_like_cpp()
                && self.loot_table.contains_key(&gameobject_guid))
            .then_some(());
        };
        let loot = self.loot_table.get(&gameobject_guid)?.clone();
        let is_personal = self
            .represented_personal_loot_owners
            .contains(&gameobject_guid);
        let (shared, personal) = self.represented_loot_authority_pools_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            is_personal,
        )?;
        let installed = authority
            .initialize_pristine_like_cpp(shared, personal)
            .installed();
        if !installed
            && authority
                .snapshot_for_player_like_cpp(player_guid)
                .is_none()
        {
            self.loot_table.remove(&gameobject_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&gameobject_guid);
            return None;
        }
        self.refresh_owned_loot_summary_like_cpp(gameobject_guid);
        let _ = self.reconcile_represented_loot_cache_like_cpp(gameobject_guid, player_guid);
        Some(())
    }

    pub(super) fn upsert_represented_personal_gameobject_loot_authority_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
    ) -> Option<()> {
        let observation =
            self.represented_gameobject_loot_install_observation_like_cpp(gameobject_guid)?;
        self.upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            replace,
            &observation,
        )
    }

    pub(super) fn represented_gameobject_loot_install_observation_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectLootInstallObservationLikeCpp> {
        self.represented_gameobject_loot_install_observation_result_like_cpp(gameobject_guid)?
    }

    /// Preserves the distinction between a missing canonical owner (`None`)
    /// and an owner whose current lifecycle rejects generation (`Some(None)`).
    /// Test-only packet fixtures may fall back only for the former.
    fn represented_gameobject_loot_install_observation_result_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> Option<Option<RepresentedGameObjectLootInstallObservationLikeCpp>> {
        self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
            (gameobject.loot_state() != LootState::JustDeactivated).then(|| {
                let authority = gameobject.loot_authority_like_cpp().clone();
                RepresentedGameObjectLootInstallObservationLikeCpp {
                    object_generation: authority.generation_like_cpp(),
                    authority,
                    loot_lifecycle_revision: gameobject.loot_lifecycle_revision_like_cpp(),
                }
            })
        })
    }

    pub(super) fn upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
        observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
    ) -> Option<()> {
        self.upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            replace,
            false,
            observation,
        )
    }

    pub(super) fn upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
        discard_empty_pool: bool,
        observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
    ) -> Option<()> {
        let (_, mut personal) = self.represented_loot_authority_pools_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            true,
        )?;
        let pool = personal.remove(&player_guid)?;
        if discard_empty_pool && loot_is_looted_like_cpp(&pool) {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                gameobject_guid,
                player_guid,
            );
            return None;
        }
        let installed =
            self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, move |gameobject| {
                gameobject.install_personal_loot_if_lifecycle_like_cpp(
                    &observation.authority,
                    observation.object_generation,
                    observation.loot_lifecycle_revision,
                    player_guid,
                    pool,
                    replace,
                )
            });
        if installed != Some(true) {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                gameobject_guid,
                player_guid,
            );
            return None;
        }
        if !self.reconcile_represented_loot_cache_like_cpp(gameobject_guid, player_guid) {
            return None;
        }
        Some(())
    }

    pub(super) fn sync_represented_creature_loot_to_canonical_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        _player_guid: ObjectGuid,
    ) -> Option<()> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(creature_guid) else {
            return (represented_local_loot_fixture_allowed_like_cpp()
                && self.loot_table.contains_key(&creature_guid))
            .then_some(());
        };
        let loot = self.loot_table.get(&creature_guid)?.clone();
        let is_personal = self
            .represented_personal_loot_owners
            .contains(&creature_guid);
        let (shared, personal) = self.represented_loot_authority_pools_like_cpp(
            creature_guid,
            _player_guid,
            loot,
            is_personal,
        )?;
        let installed = authority
            .initialize_pristine_like_cpp(shared, personal)
            .installed();
        if !installed
            && authority
                .snapshot_for_player_like_cpp(_player_guid)
                .is_none()
        {
            self.loot_table.remove(&creature_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&creature_guid);
            return None;
        }
        self.refresh_owned_loot_summary_like_cpp(creature_guid);
        let _ = self.reconcile_represented_loot_cache_like_cpp(creature_guid, _player_guid);
        Some(())
    }

    pub(super) fn canonical_creature_fully_looted_after_represented_sync_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        player_guid: ObjectGuid,
        fallback_fully_looted: bool,
    ) -> bool {
        if self
            .sync_represented_creature_loot_to_canonical_like_cpp(creature_guid, player_guid)
            .is_some()
        {
            return self
                .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
                    creature.is_fully_looted_like_cpp()
                })
                .unwrap_or(fallback_fully_looted);
        }

        fallback_fully_looted
    }

    pub(super) fn canonical_gameobject_fully_looted_after_represented_sync_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        fallback_fully_looted: bool,
    ) -> bool {
        if self
            .sync_represented_gameobject_loot_to_canonical_like_cpp(gameobject_guid, player_guid)
            .is_some()
        {
            return self
                .canonical_gameobject_is_fully_looted_like_cpp(gameobject_guid)
                .unwrap_or(fallback_fully_looted);
        }

        fallback_fully_looted
    }

    pub(super) async fn represented_ae_loot_creature_targets_like_cpp(
        &mut self,
        main_loot_target: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(player_position) = self.player_position_like_cpp() else {
            return Vec::new();
        };

        let mut candidates: Vec<ObjectGuid> = self
            .world_creature_guids()
            .into_iter()
            .filter(|guid| {
                if *guid == main_loot_target || !guid.is_creature_or_vehicle() {
                    return false;
                }
                self.represented_creature_loot_state_like_cpp(*guid)
                    .is_some_and(|creature| {
                        !creature.is_alive
                            && player_position.is_within_dist(&creature.position, 30.0)
                    })
            })
            .collect();
        candidates.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        let mut result = Vec::new();
        for owner_guid in candidates {
            let Some(creature) = self.represented_creature_loot_state_like_cpp(owner_guid) else {
                continue;
            };
            if !creature.tappers.is_empty() && !creature.tappers.contains(&player_guid) {
                continue;
            }
            // C++ `CMSG_LOOT_UNIT` only reads the Loot created by
            // `Unit::Kill`; it never regenerates a corpse pool. Reconcile the
            // active object-owned generation and fail closed if kill-time
            // generation is absent or the corpse lifetime was retired.
            if !self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid) {
                self.loot_table.remove(&owner_guid);
                continue;
            }

            if self.loot_table.get(&owner_guid).is_some_and(|loot| {
                self.represented_loot_can_be_opened_by_player_like_cpp(
                    owner_guid,
                    loot,
                    player_guid,
                )
            }) {
                result.push(owner_guid);
            }
        }

        result
    }

    pub(crate) async fn ensure_represented_creature_kill_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) {
        let Some(creature) = self.represented_creature_loot_state_like_cpp(creature_guid) else {
            return;
        };
        let Some(loot_owner_guid) = creature.tappers.first().copied() else {
            return;
        };
        let loot_scope_player_guid = if self.current_map_dungeon_state_like_cpp() == Some(false) {
            let connected_tappers =
                self.represented_connected_creature_tappers_like_cpp(&creature.tappers);
            self.player_guid()
                .filter(|player_guid| connected_tappers.contains(player_guid))
                .or_else(|| connected_tappers.first().copied())
                .unwrap_or(loot_owner_guid)
        } else {
            loot_owner_guid
        };

        self.ensure_represented_creature_loot_like_cpp(
            creature_guid,
            loot_owner_guid,
            creature.level,
            creature.entry,
            creature.loot_id,
            creature.gold_min,
            creature.gold_max,
            creature.dungeon_encounter_id,
            &creature.tappers,
            creature.loot_lifecycle_revision,
        )
        .await;
        if self
            .sync_represented_creature_loot_to_canonical_like_cpp(
                creature_guid,
                loot_scope_player_guid,
            )
            .is_none()
        {
            self.loot_table.remove(&creature_guid);
        }
    }

    /// Install kill-time pools only while the exact creature death lifetime
    /// observed before async template generation is still current. C++ runs
    /// `Unit::Kill` and loot creation on one map thread; this lock-scoped CAS
    /// is the Rust equivalent and prevents corpse-removal/respawn ABA.
    pub(super) fn install_represented_creature_kill_loot_if_current_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        expected_authority: &OwnedLootAuthority,
        expected_object_generation: u64,
        expected_loot_lifecycle_revision: u64,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> bool {
        let expected_authority = expected_authority.clone();
        self.mutate_world_creature(creature_guid, move |world_creature| {
            if world_creature.is_alive()
                || world_creature.creature.loot_lifecycle_revision_like_cpp()
                    != expected_loot_lifecycle_revision
                || !world_creature
                    .creature
                    .loot_authority_like_cpp()
                    .shares_storage_like_cpp(&expected_authority)
                || !expected_authority.is_retired_like_cpp()
                || expected_authority.generation_like_cpp() != expected_object_generation
            {
                return false;
            }

            let installed = if expected_object_generation == 0 {
                expected_authority
                    .initialize_pristine_like_cpp(shared, personal)
                    .installed()
            } else {
                expected_authority
                    .replace_retired_generation_like_cpp(
                        expected_object_generation,
                        shared,
                        personal,
                    )
                    .is_some()
            };
            if installed {
                world_creature
                    .creature
                    .sync_loot_summaries_from_authority_like_cpp();
            }
            installed
        })
        .unwrap_or(false)
    }

    async fn ensure_represented_creature_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        loot_owner_guid: ObjectGuid,
        level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
        allowed_looters: &[ObjectGuid],
        expected_loot_lifecycle_revision: u64,
    ) {
        let authority = self.represented_owned_loot_authority_like_cpp(creature_guid);
        if authority.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
            self.loot_table.remove(&creature_guid);
            return;
        }
        let mut retired_object_generation = None;
        if let Some(authority) = authority.as_ref() {
            #[cfg(test)]
            if authority.is_pristine_like_cpp() && self.loot_table.contains_key(&creature_guid) {
                if !self
                    .represented_personal_loot_owners
                    .contains(&creature_guid)
                    && let Some(loot) = self.loot_table.get_mut(&creature_guid)
                {
                    prepare_represented_shared_creature_loot_generation_like_cpp(
                        loot,
                        allowed_looters,
                    );
                }
                if self
                    .sync_represented_creature_loot_to_canonical_like_cpp(
                        creature_guid,
                        loot_owner_guid,
                    )
                    .is_some()
                {
                    // Legacy packet fixtures pre-populate the former session
                    // cache. Install that value once into the typed object-owned
                    // authority instead of silently replacing it with generated
                    // empty loot. This branch does not exist in production.
                    return;
                }
            }
            let snapshot = self
                .player_guid()
                .and_then(|player_guid| authority.snapshot_for_player_like_cpp(player_guid))
                .or_else(|| authority.snapshot_for_player_like_cpp(loot_owner_guid));
            if let Some(snapshot) = snapshot {
                let cache_player = match snapshot.scope {
                    OwnedLootScope::Personal(player_guid) => player_guid,
                    OwnedLootScope::Shared => loot_owner_guid,
                };
                self.cache_represented_owned_loot_snapshot_like_cpp(
                    creature_guid,
                    cache_player,
                    snapshot,
                );
                return;
            }
            if !authority.is_retired_like_cpp() {
                self.loot_table.remove(&creature_guid);
                return;
            }
            retired_object_generation = Some(authority.generation_like_cpp());
            self.loot_table.remove(&creature_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&creature_guid);
        }

        let map_is_dungeon = self.current_map_dungeon_state_like_cpp();
        let connected_tappers =
            self.represented_connected_creature_tappers_like_cpp(allowed_looters);

        // C++ `Unit::Kill` has three distinct ownership shapes:
        // - overworld: one independently generated personal pool per tapper;
        // - dungeon encounter/boss: one independent, lockout-filtered pool per
        //   tapper (`GenerateDungeonEncounterPersonalLoot`);
        // - dungeon trash: exactly one personal pool, keyed by the group's
        //   selected looter (or the first tapper without a group).
        if map_is_dungeon == Some(false)
            || (map_is_dungeon == Some(true) && dungeon_encounter_id != 0)
        {
            let personal_tappers = connected_tappers
                .into_iter()
                .filter(|tapper| {
                    dungeon_encounter_id == 0
                        || !self
                            .represented_locked_dungeon_encounters
                            .contains(&(*tapper, dungeon_encounter_id))
                })
                .collect::<Vec<_>>();
            if personal_tappers.is_empty() {
                self.loot_table.remove(&creature_guid);
                return;
            }

            let Some(personal) = self
                .generate_represented_creature_personal_loot_like_cpp(
                    creature_guid,
                    level,
                    entry,
                    loot_id,
                    gold_min,
                    gold_max,
                    dungeon_encounter_id,
                    &personal_tappers,
                )
                .await
            else {
                return;
            };
            let cache_player = self
                .player_guid()
                .filter(|player_guid| personal.contains_key(player_guid))
                .unwrap_or(personal_tappers[0]);

            if let (Some(authority), Some(expected_generation)) =
                (authority.as_ref(), retired_object_generation)
            {
                if self.install_represented_creature_kill_loot_if_current_like_cpp(
                    creature_guid,
                    authority,
                    expected_generation,
                    expected_loot_lifecycle_revision,
                    None,
                    personal,
                ) {
                    let _ =
                        self.reconcile_represented_loot_cache_like_cpp(creature_guid, cache_player);
                } else {
                    self.loot_table.remove(&creature_guid);
                }
            } else if represented_local_loot_fixture_allowed_like_cpp()
                && let Some(pool) = personal.get(&cache_player).cloned()
            {
                self.loot_table.insert(creature_guid, pool);
            }
            return;
        }

        if map_is_dungeon == Some(true) {
            if connected_tappers.is_empty() {
                self.loot_table.remove(&creature_guid);
                return;
            }
            let selected_looter =
                self.represented_dungeon_trash_looter_like_cpp(&connected_tappers);
            let Some(personal) = self
                .generate_represented_creature_personal_loot_like_cpp(
                    creature_guid,
                    level,
                    entry,
                    loot_id,
                    gold_min,
                    gold_max,
                    0,
                    &[selected_looter],
                )
                .await
            else {
                return;
            };
            let has_loot = personal
                .get(&selected_looter)
                .is_some_and(|loot| !loot_is_looted_like_cpp(loot));

            if let (Some(authority), Some(expected_generation)) =
                (authority.as_ref(), retired_object_generation)
            {
                if self.install_represented_creature_kill_loot_if_current_like_cpp(
                    creature_guid,
                    authority,
                    expected_generation,
                    expected_loot_lifecycle_revision,
                    None,
                    personal,
                ) {
                    let _ = self
                        .reconcile_represented_loot_cache_like_cpp(creature_guid, selected_looter);
                    if has_loot {
                        self.advance_represented_dungeon_trash_looter_like_cpp(&connected_tappers);
                    }
                } else {
                    self.loot_table.remove(&creature_guid);
                }
            } else if represented_local_loot_fixture_allowed_like_cpp()
                && let Some(pool) = personal.get(&selected_looter).cloned()
            {
                self.loot_table.insert(creature_guid, pool);
            }
            return;
        }

        // Missing Map.db2 metadata is not proof of either overworld or
        // dungeon. Preserve the represented shared fallback for legacy test
        // fixtures, but still bind its async install to the exact death token.
        if !self.loot_table.contains_key(&creature_guid) {
            let Some(mut loot) = self
                .generate_represented_creature_loot_like_cpp(
                    creature_guid,
                    loot_owner_guid,
                    level,
                    entry,
                    loot_id,
                    gold_min,
                    gold_max,
                    dungeon_encounter_id,
                )
                .await
            else {
                return;
            };
            prepare_represented_shared_creature_loot_generation_like_cpp(
                &mut loot,
                allowed_looters,
            );
            if let (Some(authority), Some(expected_generation)) =
                (authority.as_ref(), retired_object_generation)
            {
                if self.install_represented_creature_kill_loot_if_current_like_cpp(
                    creature_guid,
                    authority,
                    expected_generation,
                    expected_loot_lifecycle_revision,
                    Some(loot),
                    HashMap::new(),
                ) {
                    let _ = self
                        .reconcile_represented_loot_cache_like_cpp(creature_guid, loot_owner_guid);
                } else {
                    self.loot_table.remove(&creature_guid);
                }
            } else if represented_local_loot_fixture_allowed_like_cpp() {
                self.loot_table.insert(creature_guid, loot);
            }
        }
    }

    async fn ensure_represented_gameobject_chest_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
    ) {
        // C++ creates `m_loot` synchronously in `GameObject::Use`
        // (`GameObject.cpp:2559-2575`). Capture the exact map-owned lifetime
        // before async template work, then revalidate it under the map lock at
        // install time so `ClearLoot`/restock cannot be crossed.
        let install_observation = match self
            .represented_gameobject_loot_install_observation_result_like_cpp(gameobject_guid)
        {
            Some(Some(observation)) => Some(observation),
            Some(None) => {
                self.loot_table.remove(&gameobject_guid);
                self.represented_loot_cache_generations_like_cpp
                    .remove(&gameobject_guid);
                return;
            }
            None if !represented_local_loot_fixture_allowed_like_cpp() => {
                self.loot_table.remove(&gameobject_guid);
                self.represented_loot_cache_generations_like_cpp
                    .remove(&gameobject_guid);
                return;
            }
            None => None,
        };
        let authority = install_observation
            .as_ref()
            .map(|observation| observation.authority.clone());
        let mut install_single_personal_pool = false;
        if let Some(authority) = authority.as_ref() {
            #[cfg(test)]
            if authority.is_pristine_like_cpp() && self.loot_table.contains_key(&gameobject_guid) {
                if !self
                    .represented_personal_loot_owners
                    .contains(&gameobject_guid)
                    && let Some(loot) = self.loot_table.get_mut(&gameobject_guid)
                {
                    prepare_represented_shared_loot_generation_like_cpp(loot, allowed_looters);
                }
                if self
                    .sync_represented_gameobject_loot_to_canonical_like_cpp(
                        gameobject_guid,
                        player_guid,
                    )
                    .is_some()
                {
                    // Test-only bridge for legacy pre-authority packet fixtures;
                    // live gameobjects still require their canonical map owner.
                    return;
                }
            }
            if let Some(snapshot) = authority.snapshot_for_player_like_cpp(player_guid) {
                self.cache_represented_owned_loot_snapshot_like_cpp(
                    gameobject_guid,
                    player_guid,
                    snapshot,
                );
                return;
            }
            let active_authority =
                authority.stamp_like_cpp().lifecycle == OwnedLootAuthorityLifecycle::Active;
            let can_add_personal_pool = active_authority
                && source.uses_personal_loot_like_cpp()
                && !source.is_personal_encounter_loot_like_cpp();
            if can_add_personal_pool {
                // The non-encounter C++ branch adds exactly this opener's
                // `m_personalLoot[player]` pool. Encounter loot is different:
                // it regenerates one topology from GameObject::GetTapList and
                // assigns the whole map. Rust does not yet have that canonical
                // script-owned tap list, so an encounter opener absent from
                // the installed topology must fail closed rather than receive
                // a fabricated singleton pool.
                install_single_personal_pool = true;
            }
            if !authority.is_retired_like_cpp() && !can_add_personal_pool {
                self.loot_table.remove(&gameobject_guid);
                self.represented_loot_cache_generations_like_cpp
                    .remove(&gameobject_guid);
                return;
            }
            self.loot_table.remove(&gameobject_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&gameobject_guid);
        }

        if !self.loot_table.contains_key(&gameobject_guid) {
            let single_personal_looter = install_single_personal_pool.then_some([player_guid]);
            let generation_allowed_looters = single_personal_looter
                .as_ref()
                .map_or(allowed_looters, |looters| looters.as_slice());
            let Some(mut loot) = self
                .generate_represented_gameobject_chest_loot_like_cpp(
                    gameobject_guid,
                    player_guid,
                    source,
                    generation_allowed_looters,
                )
                .await
            else {
                return;
            };
            let personal = self
                .represented_personal_loot_owners
                .contains(&gameobject_guid);
            if !personal {
                prepare_represented_shared_loot_generation_like_cpp(&mut loot, allowed_looters);
            }
            if let Some(observation) = install_observation {
                if source.uses_personal_loot_like_cpp()
                    && (!source.is_personal_encounter_loot_like_cpp()
                        || install_single_personal_pool)
                {
                    if self
                        .upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
                            gameobject_guid,
                            player_guid,
                            loot,
                            false,
                            source.is_personal_encounter_loot_like_cpp(),
                            &observation,
                        )
                        .is_none()
                    {
                        self.loot_table.remove(&gameobject_guid);
                        self.represented_loot_cache_generations_like_cpp
                            .remove(&gameobject_guid);
                    }
                    return;
                }
                let Some((shared, mut personal)) = self.represented_loot_authority_pools_like_cpp(
                    gameobject_guid,
                    player_guid,
                    loot,
                    personal,
                ) else {
                    self.loot_table.remove(&gameobject_guid);
                    self.represented_loot_cache_generations_like_cpp
                        .remove(&gameobject_guid);
                    return;
                };
                if source.is_personal_encounter_loot_like_cpp() {
                    // `GenerateDungeonEncounterPersonalLoot` drops each
                    // per-player `Loot` that is already empty after money,
                    // personal-template and not-normal processing
                    // (`LootMgr.cpp:933-941`).  The non-encounter
                    // `chestPersonalLoot` branch deliberately keeps its empty
                    // `m_personalLoot[player]`, so this filter belongs only to
                    // the encounter topology.
                    personal.retain(|_, pool| !loot_is_looted_like_cpp(pool));
                    self.represented_personal_loot_money
                        .retain(|(owner, player), _| {
                            *owner != gameobject_guid || personal.contains_key(player)
                        });
                    if personal.is_empty() {
                        // C++ assigns an empty `m_personalLoot` map and sends no
                        // loot window.  Keep the authority pristine/retired so
                        // a later `Use` may generate again instead of leaving
                        // an active owner with no selectable pool.
                        self.represented_personal_loot_owners
                            .remove(&gameobject_guid);
                        self.loot_table.remove(&gameobject_guid);
                        self.represented_loot_cache_generations_like_cpp
                            .remove(&gameobject_guid);
                        return;
                    }
                }
                let installed = self
                    .mutate_canonical_gameobject_by_guid_like_cpp(
                        gameobject_guid,
                        move |gameobject| {
                            gameobject.install_loot_authority_if_lifecycle_like_cpp(
                                &observation.authority,
                                observation.object_generation,
                                observation.loot_lifecycle_revision,
                                shared,
                                personal,
                            )
                        },
                    )
                    .unwrap_or(false);
                if !installed {
                    self.loot_table.remove(&gameobject_guid);
                    self.represented_loot_cache_generations_like_cpp
                        .remove(&gameobject_guid);
                    return;
                }
                let _ =
                    self.reconcile_represented_loot_cache_like_cpp(gameobject_guid, player_guid);
            } else if represented_local_loot_fixture_allowed_like_cpp() {
                self.loot_table.insert(gameobject_guid, loot);
            }
        }
    }

    pub(super) async fn generate_represented_gameobject_chest_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
    ) -> Option<CreatureLoot> {
        let personal_loot = source.uses_personal_loot_like_cpp();
        let personal_encounter = source.is_personal_encounter_loot_like_cpp();
        let (loot_method, loot_master, round_robin_player) = self
            .represented_gameobject_chest_group_state_like_cpp(
                source.use_group_loot_rules && !personal_loot,
                player_guid,
            );
        let loot_id = source.open_loot_id_like_cpp();
        let items = if personal_encounter {
            Vec::new()
        } else {
            self.generate_represented_shared_gameobject_loot_items_like_cpp(
                loot_id,
                allowed_looters,
            )
            .await
            .unwrap_or_else(|| {
                if loot_id != 0 {
                    debug!(
                        loot_id,
                        gameobject = ?gameobject_guid,
                        "gameobject loot template unavailable for represented chest"
                    );
                }
                Vec::new()
            })
        };
        let (min_money, max_money) = self
            .load_gameobject_template_addon_money_loot_like_cpp(gameobject_guid.entry())
            .await;
        let coins = self.represented_money_loot_with_rate_like_cpp(
            min_money,
            max_money,
            self.loot_drop_rates_like_cpp().money,
        );

        let loot_guid = self.next_represented_loot_object_guid_like_cpp(gameobject_guid)?;
        let mut loot = CreatureLoot {
            loot_guid,
            coins,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: source.dungeon_encounter_id,
            loot_method,
            loot_master,
            round_robin_player,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items,
            looted_by_player: false,
        };

        if personal_loot {
            loot.coins = 0;
            self.represented_personal_loot_owners
                .insert(gameobject_guid);
            self.represented_personal_loot_money
                .retain(|(owner, _), _| *owner != gameobject_guid);
            let represented_tappers = if personal_encounter && !allowed_looters.is_empty() {
                let mut tappers = allowed_looters
                    .iter()
                    .copied()
                    .filter(|guid| {
                        guid.is_player()
                            && self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
                                *guid,
                                source.dungeon_encounter_id,
                            )
                    })
                    .collect::<Vec<_>>();
                tappers.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
                tappers.dedup();
                tappers
            } else if personal_encounter {
                self.represented_gameobject_personal_encounter_tappers_like_cpp(
                    gameobject_guid,
                    player_guid,
                    source.dungeon_encounter_id,
                )
            } else {
                vec![player_guid]
            };
            for tapper in &represented_tappers {
                if !loot.allowed_looters.contains(tapper) {
                    loot.allowed_looters.push(*tapper);
                }
                let tapper_money = self.represented_money_loot_with_rate_like_cpp(
                    min_money,
                    max_money,
                    self.loot_drop_rates_like_cpp().money,
                );
                self.represented_personal_loot_money
                    .insert((gameobject_guid, *tapper), tapper_money);
            }
            if personal_encounter {
                loot.items = self
                    .generate_represented_gameobject_personal_loot_items_like_cpp(
                        loot_id,
                        &represented_tappers,
                    )
                    .await
                    .unwrap_or_else(|| {
                        if loot_id != 0 {
                            debug!(
                                loot_id,
                                gameobject = ?gameobject_guid,
                                "gameobject personal loot template unavailable for represented chest"
                            );
                        }
                        Vec::new()
                    });
            }
            rebuild_represented_personal_loot_counts_like_cpp(&mut loot);
            if represented_tappers.is_empty() {
                self.represented_personal_loot_owners
                    .remove(&gameobject_guid);
            }
        }

        Some(loot)
    }

    fn represented_gameobject_personal_encounter_tappers_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Vec<ObjectGuid> {
        let Some(tappers) = self.represented_gameobject_tap_lists.get(&gameobject_guid) else {
            return self
                .represented_player_unlocked_for_dungeon_encounter_like_cpp(
                    player_guid,
                    dungeon_encounter_id,
                )
                .into_iter()
                .collect();
        };
        let mut represented_tappers = tappers
            .iter()
            .copied()
            .filter(|guid| guid.is_player())
            .collect::<Vec<_>>();
        represented_tappers.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        represented_tappers.dedup();
        if represented_tappers.is_empty() {
            represented_tappers.push(player_guid);
        }
        represented_tappers.retain(|guid| {
            self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
                *guid,
                dungeon_encounter_id,
            )
        });
        represented_tappers
    }

    pub(super) fn represented_gameobject_chest_group_state_like_cpp(
        &self,
        use_group_loot_rules: bool,
        _player_guid: ObjectGuid,
    ) -> (u8, ObjectGuid, ObjectGuid) {
        if !use_group_loot_rules {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        }
        let Some(group_guid) = self.group_guid else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };
        let Some(registry) = self.group_registry() else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };
        let Some(group) = registry.get(&group_guid) else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };

        // C++ `Loot::FillLoot` assigns round robin only for `LOOT_CORPSE`.
        (
            group.loot_method,
            group.master_looter_guid,
            ObjectGuid::EMPTY,
        )
    }

    async fn generate_represented_gameobject_loot_items_like_cpp(
        &mut self,
        loot_id: u32,
    ) -> Option<Vec<LootEntry>> {
        self.generate_represented_gameobject_loot_items_for_store_like_cpp(
            loot_id,
            LootStoreKind::Gameobject,
            LOOT_MODE_DEFAULT_LIKE_CPP,
            None,
        )
        .await
    }

    async fn generate_represented_shared_gameobject_loot_items_like_cpp(
        &mut self,
        loot_id: u32,
        allowed_looters: &[ObjectGuid],
    ) -> Option<Vec<LootEntry>> {
        self.generate_represented_gameobject_loot_items_for_store_like_cpp(
            loot_id,
            LootStoreKind::Gameobject,
            LOOT_MODE_DEFAULT_LIKE_CPP,
            Some(allowed_looters),
        )
        .await
    }

    async fn generate_represented_gameobject_loot_items_for_store_like_cpp(
        &mut self,
        loot_id: u32,
        store_kind: LootStoreKind,
        loot_mode: u16,
        shared_allowed_looters: Option<&[ObjectGuid]>,
    ) -> Option<Vec<LootEntry>> {
        if loot_id == 0 {
            return Some(Vec::new());
        }

        let mut rng = self.represented_runtime_subrng_like_cpp();
        let stores = self.loot_stores()?;
        let store = stores.get(&store_kind)?;
        let rates = self.loot_drop_rates_like_cpp();
        let condition_ids = store.condition_ids_for_fill_like_cpp(loot_id, store_kind, stores);
        let condition_rows = self
            .load_represented_creature_loot_condition_rows_like_cpp(&condition_ids)
            .await;
        let condition_references = self
            .load_represented_creature_loot_condition_reference_rows_like_cpp(&condition_rows)
            .await;
        let addon_metadata = self
            .load_item_template_addon_loot_metadata_for_item_ids_like_cpp(
                condition_ids.iter().map(|id| id.source_entry),
            )
            .await;
        let defer_eligibility_until_after_roll = shared_allowed_looters.is_some();
        let generated = {
            match store.fill_loot_with_context_like_cpp(
                loot_id,
                store_kind,
                stores,
                LootFillOptions {
                    loot_mode,
                    rates_allowed: true,
                    referenced_amount_rate: rates.item_referenced_amount,
                    item_context: ItemContext::None as u8,
                },
                &mut rng,
                |item_id| {
                    self.item_storage_template(item_id)
                        .map(|template| LootItemTemplateMetadata {
                            max_stack: template.max_stack_size.max(1),
                            has_multi_drop_flag: template.flags.contains(ItemFlags::MULTI_DROP),
                            has_follow_loot_rules_flag: false,
                        })
                },
                |item| self.item_drop_rate_like_cpp(item.item_id),
                |context| {
                    defer_eligibility_until_after_roll
                        || self.represented_creature_loot_item_allowed_like_cpp(
                            context,
                            &condition_rows,
                            &condition_references,
                            &addon_metadata,
                        )
                },
                |item_id, rng| {
                    let random_properties =
                        self.generate_loot_store_random_properties_with_rng_like_cpp(item_id, rng);
                    LootItemRandomProperties {
                        id: random_properties.id,
                        seed: random_properties.seed,
                    }
                },
            ) {
                Ok(generated) => generated,
                Err(LootFillError::MissingLootTemplate { .. }) => Vec::new(),
            }
        };

        Some(
            generated
                .into_iter()
                .map(|item| {
                    let metadata = addon_metadata
                        .get(&item.item_id)
                        .copied()
                        .unwrap_or_default();
                    if let Some(allowed_looters) = shared_allowed_looters {
                        generated_shared_gameobject_loot_item_to_entry_like_cpp(
                            item,
                            metadata,
                            allowed_looters,
                            |context, looter| {
                                self.represented_creature_loot_item_allowed_for_player_like_cpp(
                                    context,
                                    looter,
                                    &condition_rows,
                                    &condition_references,
                                    &addon_metadata,
                                )
                            },
                        )
                    } else {
                        generated_creature_loot_item_to_entry_like_cpp(item, metadata)
                    }
                })
                .collect(),
        )
    }

    async fn generate_represented_fishing_loot_items_like_cpp(
        &mut self,
        area_id: u32,
        loot_mode: u16,
    ) -> Option<Vec<LootEntry>> {
        let mut current_area_id = area_id;
        while current_area_id != 0 {
            let items = self
                .generate_represented_gameobject_loot_items_for_store_like_cpp(
                    current_area_id,
                    LootStoreKind::Fishing,
                    loot_mode,
                    None,
                )
                .await?;
            if !items.is_empty() {
                return Some(items);
            }
            let Some(parent_area_id) = self
                .area_table_store()
                .and_then(|store| store.get(current_area_id))
                .map(|entry| u32::from(entry.parent_area_id))
            else {
                break;
            };
            current_area_id = parent_area_id;
        }

        self.generate_represented_gameobject_loot_items_for_store_like_cpp(
            1,
            LootStoreKind::Fishing,
            loot_mode,
            None,
        )
        .await
    }

    async fn generate_represented_gameobject_personal_loot_items_like_cpp(
        &mut self,
        loot_id: u32,
        tappers: &[ObjectGuid],
    ) -> Option<Vec<LootEntry>> {
        if loot_id == 0 || tappers.is_empty() {
            return Some(Vec::new());
        }

        let mut rng = self.represented_runtime_subrng_like_cpp();
        let stores = self.loot_stores()?;
        let store = stores.get(&LootStoreKind::Gameobject)?;
        let rates = self.loot_drop_rates_like_cpp();
        let condition_ids =
            store.condition_ids_for_fill_like_cpp(loot_id, LootStoreKind::Gameobject, stores);
        let condition_rows = self
            .load_represented_creature_loot_condition_rows_like_cpp(&condition_ids)
            .await;
        let condition_references = self
            .load_represented_creature_loot_condition_reference_rows_like_cpp(&condition_rows)
            .await;
        let addon_metadata = self
            .load_item_template_addon_loot_metadata_for_item_ids_like_cpp(
                condition_ids.iter().map(|id| id.source_entry),
            )
            .await;
        let generated = {
            store
                .fill_personal_loot_with_context_like_cpp(
                    loot_id,
                    LootStoreKind::Gameobject,
                    stores,
                    LootFillOptions {
                        loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                        rates_allowed: true,
                        referenced_amount_rate: rates.item_referenced_amount,
                        item_context: ItemContext::None as u8,
                    },
                    tappers,
                    &mut rng,
                    |item_id| {
                        self.item_storage_template(item_id).map(|template| {
                            LootItemTemplateMetadata {
                                max_stack: template.max_stack_size.max(1),
                                has_multi_drop_flag: template.flags.contains(ItemFlags::MULTI_DROP),
                                has_follow_loot_rules_flag: false,
                            }
                        })
                    },
                    |item| self.item_drop_rate_like_cpp(item.item_id),
                    |context, looter| {
                        self.represented_creature_loot_item_allowed_for_player_like_cpp(
                            context,
                            looter,
                            &condition_rows,
                            &condition_references,
                            &addon_metadata,
                        )
                    },
                    |item_id, rng| {
                        let random_properties = self
                            .generate_loot_store_random_properties_with_rng_like_cpp(item_id, rng);
                        LootItemRandomProperties {
                            id: random_properties.id,
                            seed: random_properties.seed,
                        }
                    },
                )
                .ok()?
        };

        Some(
            generated
                .into_iter()
                .map(|personal_item| {
                    let metadata = addon_metadata
                        .get(&personal_item.item.item_id)
                        .copied()
                        .unwrap_or_default();
                    let mut entry = generated_creature_loot_item_to_entry_like_cpp(
                        personal_item.item,
                        metadata,
                    );
                    entry.add_allowed_looter_like_cpp(personal_item.looter);
                    entry
                })
                .collect(),
        )
    }

    async fn autostore_represented_gameobject_chest_push_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) -> bool {
        if !source.should_autostore_push_loot_like_cpp() {
            return true;
        }

        let items = self
            .generate_represented_gameobject_loot_items_like_cpp(source.push_loot_id)
            .await
            .unwrap_or_else(|| {
                debug!(
                    loot_id = source.push_loot_id,
                    gameobject = ?gameobject_guid,
                    "gameobject push loot template unavailable for represented chest"
                );
                Vec::new()
            });

        let mut all_stored = true;
        for entry in items {
            if !self
                .store_direct_loot_item_like_cpp(&entry, source.dungeon_encounter_id)
                .await
            {
                all_stored = false;
            }
        }

        all_stored
    }

    pub(super) async fn generate_represented_creature_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        loot_owner_guid: ObjectGuid,
        _level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
    ) -> Option<CreatureLoot> {
        let (loot_method, loot_master, round_robin_player) =
            self.represented_creature_loot_group_state_like_cpp(loot_owner_guid);
        let coins = self.represented_money_loot_with_rate_like_cpp(
            gold_min,
            gold_max,
            self.loot_drop_rates_like_cpp().money,
        );

        let items = self
            .generate_represented_creature_loot_items_like_cpp(loot_id)
            .await
            .unwrap_or_else(|| {
                if loot_id != 0 {
                    debug!(
                        entry,
                        loot_id, "creature loot template unavailable for represented corpse"
                    );
                }
                Vec::new()
            });

        let loot_guid = self.next_represented_loot_object_guid_like_cpp(creature_guid)?;
        Some(CreatureLoot {
            loot_guid,
            coins,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id,
            loot_method,
            loot_master,
            round_robin_player,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items,
            looted_by_player: false,
        })
    }

    /// C++ `Unit::Kill` first resolves every tap-list GUID through
    /// `ObjectAccessor::GetPlayer(*creature, guid)`. Only connected players in
    /// the creature's exact map instance receive an overworld personal pool.
    fn represented_connected_creature_tappers_like_cpp(
        &self,
        tappers: &[ObjectGuid],
    ) -> Vec<ObjectGuid> {
        let current_player = self.player_guid();
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let registry = self.player_registry();
        let mut connected = tappers
            .iter()
            .copied()
            .filter(|tapper| {
                if !tapper.is_player() {
                    return false;
                }
                if Some(*tapper) == current_player {
                    return true;
                }
                registry
                    .and_then(|registry| registry.loot_presence(*tapper))
                    .is_some_and(|player| {
                        player.is_in_world
                            && player.map_id == map_id
                            && player.instance_id == instance_id
                    })
            })
            .collect::<Vec<_>>();
        connected.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        connected.dedup();
        connected
    }

    /// Generate one independently rolled C++ personal `Loot` per supplied
    /// player. The caller chooses the ownership set for overworld tappers,
    /// encounter-eligible dungeon tappers, or the single dungeon-trash
    /// selected looter. Every pool is constructed without a Group and remains
    /// an object-owned per-view source of truth.
    #[allow(clippy::too_many_arguments)]
    async fn generate_represented_creature_personal_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        _level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
        tappers: &[ObjectGuid],
    ) -> Option<HashMap<ObjectGuid, CreatureLoot>> {
        let mut personal = HashMap::with_capacity(tappers.len());
        for tapper in tappers {
            let coins = self.represented_money_loot_with_rate_like_cpp(
                gold_min,
                gold_max,
                self.loot_drop_rates_like_cpp().money,
            );
            let items = self
                .generate_represented_creature_loot_items_for_player_like_cpp(loot_id, *tapper)
                .await
                .unwrap_or_else(|| {
                    if loot_id != 0 {
                        debug!(
                            entry,
                            loot_id,
                            tapper = ?tapper,
                            "creature personal loot template unavailable for represented overworld corpse"
                        );
                    }
                    Vec::new()
                });
            let loot_guid = self.next_represented_loot_object_guid_like_cpp(creature_guid)?;
            let mut loot = CreatureLoot {
                loot_guid,
                coins,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items,
                looted_by_player: false,
            };
            mark_loot_allowed_for_player_like_cpp(&mut loot, *tapper);
            rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(&mut loot);
            personal.insert(*tapper, loot);
        }
        Some(personal)
    }

    fn represented_creature_loot_group_state_like_cpp(
        &self,
        loot_owner_guid: ObjectGuid,
    ) -> (u8, ObjectGuid, ObjectGuid) {
        let Some(group_guid) = self.group_guid else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };
        let Some(registry) = self.group_registry() else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };
        let Some(group) = registry.get(&group_guid) else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };

        (group.loot_method, group.master_looter_guid, loot_owner_guid)
    }

    async fn generate_represented_creature_loot_items_like_cpp(
        &mut self,
        loot_id: u32,
    ) -> Option<Vec<LootEntry>> {
        let player_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        self.generate_represented_creature_loot_items_for_player_like_cpp(loot_id, player_guid)
            .await
    }

    pub(super) async fn generate_represented_creature_loot_items_for_player_like_cpp(
        &mut self,
        loot_id: u32,
        player_guid: ObjectGuid,
    ) -> Option<Vec<LootEntry>> {
        if loot_id == 0 {
            return Some(Vec::new());
        }

        let mut rng = self.represented_runtime_subrng_like_cpp();
        let stores = self.loot_stores()?;
        let store = stores.get(&LootStoreKind::Creature)?;
        let rates = self.loot_drop_rates_like_cpp();
        let condition_ids =
            store.condition_ids_for_fill_like_cpp(loot_id, LootStoreKind::Creature, stores);
        let condition_rows = self
            .load_represented_creature_loot_condition_rows_like_cpp(&condition_ids)
            .await;
        let condition_references = self
            .load_represented_creature_loot_condition_reference_rows_like_cpp(&condition_rows)
            .await;
        let addon_metadata = self
            .load_item_template_addon_loot_metadata_for_item_ids_like_cpp(
                condition_ids.iter().map(|id| id.source_entry),
            )
            .await;
        let generated = {
            store
                .fill_loot_with_context_like_cpp(
                    loot_id,
                    LootStoreKind::Creature,
                    stores,
                    LootFillOptions {
                        loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                        rates_allowed: true,
                        referenced_amount_rate: rates.item_referenced_amount,
                        item_context: ItemContext::None as u8,
                    },
                    &mut rng,
                    |item_id| {
                        self.item_storage_template(item_id).map(|template| {
                            LootItemTemplateMetadata {
                                max_stack: template.max_stack_size.max(1),
                                has_multi_drop_flag: template.flags.contains(ItemFlags::MULTI_DROP),
                                has_follow_loot_rules_flag: false,
                            }
                        })
                    },
                    |item| self.item_drop_rate_like_cpp(item.item_id),
                    |context| {
                        self.represented_creature_loot_item_allowed_for_player_like_cpp(
                            context,
                            player_guid,
                            &condition_rows,
                            &condition_references,
                            &addon_metadata,
                        )
                    },
                    |item_id, rng| {
                        let random_properties = self
                            .generate_loot_store_random_properties_with_rng_like_cpp(item_id, rng);
                        LootItemRandomProperties {
                            id: random_properties.id,
                            seed: random_properties.seed,
                        }
                    },
                )
                .ok()?
        };

        Some(
            generated
                .into_iter()
                .map(|item| {
                    let metadata = addon_metadata
                        .get(&item.item_id)
                        .copied()
                        .unwrap_or_default();
                    generated_creature_loot_item_to_entry_like_cpp(item, metadata)
                })
                .collect(),
        )
    }

    async fn load_represented_creature_loot_condition_rows_like_cpp(
        &self,
        condition_ids: &[LootConditionId],
    ) -> HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>> {
        let mut rows_by_id = HashMap::new();
        for &condition_id in condition_ids {
            let rows = self
                .load_represented_creature_loot_condition_rows_for_id_like_cpp(condition_id)
                .await;
            if !rows.is_empty() {
                rows_by_id.insert(condition_id, rows);
            }
        }
        rows_by_id
    }

    async fn load_represented_creature_loot_condition_reference_rows_like_cpp(
        &self,
        condition_rows: &HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>>,
    ) -> HashMap<u32, Vec<LootConditionRowLikeCpp>> {
        let mut references = HashMap::new();
        let mut pending = Vec::new();
        for rows in condition_rows.values() {
            pending.extend(loot_condition_reference_ids_like_cpp(rows));
        }

        while let Some(reference_id) = pending.pop() {
            if references.contains_key(&reference_id) {
                continue;
            }

            let rows = self
                .load_represented_creature_loot_condition_reference_rows_for_id_like_cpp(
                    reference_id,
                )
                .await;
            for nested_reference_id in loot_condition_reference_ids_like_cpp(&rows) {
                if !references.contains_key(&nested_reference_id) {
                    pending.push(nested_reference_id);
                }
            }
            references.insert(reference_id, rows);
        }

        references
    }

    async fn load_represented_creature_loot_condition_reference_rows_for_id_like_cpp(
        &self,
        reference_id: u32,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Ok(reference_source_type) = i32::try_from(reference_id).map(|id| -id) else {
            return Vec::new();
        };

        self.load_represented_creature_loot_condition_rows_for_id_like_cpp(LootConditionId {
            source_type: reference_source_type,
            source_group: 0,
            source_entry: 0,
        })
        .await
    }

    async fn load_represented_creature_loot_condition_rows_for_id_like_cpp(
        &self,
        condition_id: LootConditionId,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Some(port) = self.loot_template_catalog_persistence_port_like_cpp() else {
            return Vec::new();
        };

        let rows = match port
            .load_loot_condition_rows_like_cpp(
                condition_id.source_type,
                condition_id.source_group,
                condition_id.source_entry,
            )
            .await
        {
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    source_type = condition_id.source_type,
                    source_group = condition_id.source_group,
                    source_entry = condition_id.source_entry,
                    error = %reason,
                    "failed to load represented creature loot conditions"
                );
                return Vec::new();
            }
        };

        let mut conditions = Vec::new();
        for row in rows {
            let condition = LootConditionRowLikeCpp {
                else_group: row.else_group,
                condition_type_or_reference: row.condition_type_or_reference,
                condition_target: row.condition_target,
                value1: row.value1,
                value2: row.value2,
                value3: row.value3,
                string_value1: row.string_value1,
                negative: row.negative,
                script_name: row.script_name,
            };
            if !loot_condition_reference_self_references_like_cpp(
                condition_id.source_type,
                condition.condition_type_or_reference,
            ) {
                if let Some(condition) =
                    loot_condition_row_normalize_without_external_stores_like_cpp(condition)
                {
                    conditions.push(condition);
                }
            }
        }

        conditions
    }

    fn represented_creature_loot_item_allowed_like_cpp(
        &self,
        context: LootStoreItemContext,
        condition_rows: &HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>>,
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
        addon_metadata: &HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp>,
    ) -> bool {
        self.represented_creature_loot_item_allowed_for_player_like_cpp(
            context,
            self.player_guid().unwrap_or(ObjectGuid::EMPTY),
            condition_rows,
            condition_references,
            addon_metadata,
        )
    }

    fn represented_creature_loot_item_allowed_for_player_like_cpp(
        &self,
        context: LootStoreItemContext,
        player_guid: ObjectGuid,
        condition_rows: &HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>>,
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
        addon_metadata: &HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp>,
    ) -> bool {
        let Some(template) = self.item_storage_template(context.item.item_id) else {
            return false;
        };
        let Some(player_context) = self.represented_loot_player_context_like_cpp(player_guid)
        else {
            return false;
        };

        let flags2 = self.item_template_flags2_like_cpp(context.item.item_id);
        if represented_item_faction_flags_block_player_like_cpp(flags2, player_context.race) {
            return false;
        }

        let condition_id = LootConditionId {
            source_type: wow_loot::condition_source_type_for_loot_store_kind_like_cpp(
                context.store_kind,
            ),
            source_group: context.entry,
            source_entry: context.item.item_id,
        };
        if !loot_conditions_allow_player_with_references_like_cpp_representable(
            condition_rows
                .get(&condition_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            condition_references,
            |condition| {
                self.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                    condition,
                    &player_context,
                )
            },
        ) {
            return false;
        }

        let addon = addon_metadata
            .get(&context.item.item_id)
            .copied()
            .unwrap_or_default();
        self.item_loot_quest_status_allows_for_player_like_cpp(
            context.item.item_id,
            context.item.needs_quest,
            addon,
            &player_context,
        ) && template.max_stack_size != 0
    }

    pub(super) fn evaluate_creature_loot_condition_for_player_like_cpp_representable(
        &self,
        condition: &LootConditionRowLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> Option<bool> {
        match condition.condition_type_or_reference {
            0 => Some(true),
            2 => {
                if condition.value3 != 0 {
                    return None;
                }
                let item_count = if player_context.is_current {
                    self.direct_inventory_item_count_like_cpp(condition.value1)
                } else {
                    player_context.inventory_item_count(condition.value1)
                };
                Some(item_count >= condition.value2)
            }
            6 => Some(
                player_team_for_race_cpp_representable(player_context.race) == condition.value1,
            ),
            8 => Some(player_context.rewarded_quests.contains(&condition.value1)),
            9 => Some(
                player_context.quest_status(condition.value1) == QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            ),
            14 => Some(player_context.quest_status(condition.value1) == QUEST_STATUS_NONE_LIKE_CPP),
            15 => Some(
                player_class_mask_like_cpp(player_context.class)
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            16 => Some(
                player_race_mask_like_cpp(player_context.race)
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            20 => Some(u32::from(player_context.gender) == condition.value1),
            25 => i32::try_from(condition.value1)
                .ok()
                .map(|spell_id| player_context.known_spells.contains(&spell_id)),
            27 => condition_compare_values_like_cpp(
                condition.value2,
                u32::from(player_context.level),
                condition.value1,
            ),
            28 => Some(
                player_context.quest_status(condition.value1) == QUEST_STATUS_COMPLETE_LIKE_CPP
                    && !player_context.rewarded_quests.contains(&condition.value1),
            ),
            47 => Some(
                player_quest_status_mask_like_cpp(
                    player_context
                        .active_quest_statuses
                        .get(&condition.value1)
                        .copied(),
                    player_context.rewarded_quests.contains(&condition.value1),
                ) & condition.value2
                    != 0,
            ),
            48 => {
                let progress = if player_context.is_current {
                    self.player_quest_objective_progress_like_cpp(condition.value1)
                } else {
                    self.remote_player_quest_objective_progress_like_cpp(
                        condition.value1,
                        player_context,
                    )
                };
                Some(progress == Some(condition.value3 as i32))
            }
            CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP => {
                Some(condition.value1 == TYPEID_PLAYER_LIKE_CPP)
            }
            CONDITION_TYPE_MASK_LIKE_CPP => Some(condition.value1 & PLAYER_TYPE_MASK_LIKE_CPP != 0),
            _ => None,
        }
    }

    pub(super) async fn load_creature_item_template_addon_loot_metadata_like_cpp(
        &self,
        item_id: u32,
    ) -> ItemTemplateAddonLootMetadataLikeCpp {
        let Some(port) = self.item_template_addon_catalog_persistence_port_like_cpp() else {
            return ItemTemplateAddonLootMetadataLikeCpp::default();
        };

        match port
            .load_item_template_addon_loot_metadata_like_cpp(
                wow_persistence::ItemTemplateAddonCatalogRequestLikeCpp {
                    item_entry: item_id,
                },
            )
            .await
        {
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Found(row) => {
                ItemTemplateAddonLootMetadataLikeCpp {
                    flags_cu: row.flags_cu,
                    quest_log_item_id: row.quest_log_item_id,
                }
            }
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Missing => {
                ItemTemplateAddonLootMetadataLikeCpp::default()
            }
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_id,
                    error = %reason,
                    "failed to load item_template_addon loot metadata for creature loot"
                );
                ItemTemplateAddonLootMetadataLikeCpp::default()
            }
        }
    }

    fn canonical_gameobject_owner_for_loot_like_cpp(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?.map();
        let owner_guid = map.get_typed_game_object(guid)?.owner_guid();
        (!owner_guid.is_empty()).then_some(owner_guid)
    }

    pub(super) fn remove_canonical_corpse_lootable_dynamic_flag_like_cpp(
        &mut self,
        corpse_guid: ObjectGuid,
    ) -> bool {
        let Some(map_key) =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        let Some(corpse) = map.map_mut().get_typed_corpse_mut(corpse_guid) else {
            return false;
        };

        corpse.remove_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE);
        true
    }

    pub(super) fn remove_canonical_corpse_lootable_dynamic_flag_if_unviewed_fully_looted_observation_like_cpp(
        &mut self,
        corpse_guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
    ) -> bool {
        let Some(map_key) =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        let Some(corpse) = map.map_mut().get_typed_corpse_mut(corpse_guid) else {
            return false;
        };

        authority
            .with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || corpse.remove_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE),
            )
            .is_some()
    }

    pub(super) fn represented_creature_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureLootStateLikeCpp> {
        self.mutate_world_creature(guid, |creature| RepresentedCreatureLootStateLikeCpp {
            is_alive: creature.is_alive(),
            position: creature.position(),
            level: creature.level(),
            entry: creature.entry(),
            loot_id: creature.loot_id(),
            gold_min: creature.gold_min(),
            gold_max: creature.gold_max(),
            dungeon_encounter_id: creature.dungeon_encounter_id(),
            tappers: creature.creature.tap_list().to_vec(),
            loot_lifecycle_revision: creature.creature.loot_lifecycle_revision_like_cpp(),
        })
    }

    pub(super) fn represented_creature_position_for_loot_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<wow_core::Position> {
        if let Some(position) = self
            .canonical_map_object_position_for_loot_like_cpp(guid, &[AccessorObjectKind::Creature])
        {
            return Some(position);
        }

        self.represented_creature_loot_state_like_cpp(guid)
            .map(|creature| creature.position)
    }

    fn represented_gameobject_loot_state_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectLootStateLikeCpp> {
        if !guid.is_game_object() {
            return None;
        }

        let canonical_position = self.canonical_map_object_position_for_loot_like_cpp(
            guid,
            &[
                AccessorObjectKind::GameObject,
                AccessorObjectKind::Transport,
            ],
        );
        let canonical_owner = self.canonical_gameobject_owner_for_loot_like_cpp(guid);
        let represented_state = self.represented_gameobject_use_states.get(&guid);
        if canonical_position.is_none()
            && represented_state.and_then(|state| state.position).is_none()
            && !self.client_visible_guids_like_cpp.contains(&guid)
        {
            return None;
        }

        Some(RepresentedGameObjectLootStateLikeCpp {
            position: canonical_position
                .or_else(|| represented_state.and_then(|state| state.position)),
            display_id: represented_state.and_then(|state| state.display_id),
            scale: represented_state.map(|state| state.scale).unwrap_or(1.0),
            rotation: represented_state
                .map(|state| state.rotation)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
            go_type: represented_state.and_then(|state| state.go_type),
            interact_radius_override: represented_state
                .and_then(|state| state.interact_radius_override),
            lock_id: represented_state.and_then(|state| state.lock_id),
            owner_guid: canonical_owner
                .or_else(|| represented_state.and_then(|state| state.owner_guid)),
        })
    }

    fn represented_gameobject_exists_for_loot_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.represented_gameobject_loot_state_like_cpp(guid)
            .is_some()
    }

    fn represented_gameobject_spell_lock_range_like_cpp(
        &self,
        lock_id: Option<u32>,
    ) -> Option<f32> {
        let lock_id = lock_id?;
        let lock = self.lock_store()?.get(lock_id)?;
        for i in 0..wow_data::lock::MAX_LOCK_CASE {
            let lock_type = lock.lock_type[i];
            if lock_type == 0 {
                continue;
            }

            if lock_type == LOCK_KEY_SPELL_LIKE_CPP {
                if let Some(range) = self.represented_spell_max_range_like_cpp(lock.index[i]) {
                    return Some(range);
                }
            }

            if lock_type != LOCK_KEY_SKILL_LIKE_CPP {
                break;
            }

            for spell_id in self.known_spells_like_cpp() {
                let Some(spell) = self.spell_store().and_then(|store| store.get(*spell_id)) else {
                    continue;
                };
                let can_open_lock = spell.effects().iter().any(|effect| {
                    effect.effect == SPELL_EFFECT_OPEN_LOCK_LIKE_CPP
                        && effect.effect_misc_value_1 == lock.index[i]
                        && effect.effect_base_points >= i32::from(lock.skill[i])
                });
                if can_open_lock {
                    if let Some(range) = self.represented_spell_max_range_like_cpp(*spell_id) {
                        return Some(range);
                    }
                }
            }
        }

        None
    }

    pub(super) fn represented_gameobject_can_autostore_loot_item_like_cpp(
        &self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(state) = self.represented_gameobject_loot_state_like_cpp(guid) else {
            return false;
        };

        // C++ ref: LootHandler.cpp HandleAutostoreLootItemOpcode skips distance
        // for owned GameObjects and GAMEOBJECT_TYPE_FISHINGHOLE. DB spawns do
        // not carry CreatedBy; apply the owner exception only when runtime GO
        // state explicitly recorded GetOwnerGUID.
        if state.owner_guid == Some(player_guid)
            || state.go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE as u8)
        {
            return true;
        }

        match (self.player_position_like_cpp(), state.position) {
            (Some(player), Some(position)) => {
                let radius = represented_gameobject_interaction_distance_like_cpp(
                    state.go_type,
                    state.interact_radius_override,
                );
                let radius = self
                    .represented_gameobject_spell_lock_range_like_cpp(state.lock_id)
                    .unwrap_or(radius);
                if let Some(display_info) = self.gameobject_display_info_store().and_then(|store| {
                    state
                        .display_id
                        .and_then(|display_id| store.get(display_id))
                }) {
                    represented_gameobject_display_box_contains_like_cpp(
                        position,
                        player,
                        display_info,
                        state.scale,
                        state.rotation,
                        radius,
                    )
                } else {
                    player.is_within_dist(&position, radius)
                }
            }
            _ => true,
        }
    }
}
