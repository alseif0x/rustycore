// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject loot-source interaction and runtime state publication.

use super::*;

#[path = "gameobject_money.rs"]
mod chest_money;
#[path = "gameobject_publication.rs"]
mod state_publication;

impl WorldSession {
    pub(crate) async fn open_represented_fishing_hole_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        loot_id: u32,
    ) {
        self.open_fishing_hole_operation(item_valuation, gameobject_guid, gameobject_entry, loot_id, LootOperationPolicy::Production).await;
    }

    pub(in crate::handlers::loot) async fn open_fishing_hole_operation(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        loot_id: u32,
        policy: LootOperationPolicy,
    ) {
        let player_guid = self.player_guid();
        let should_update_criteria = player_guid.is_some()
            && loot_id != 0
            && self.resolved_player_is_alive_like_cpp() == Some(true)
            && self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid);
        self.open_gameobject_personal_loot_operation(
            item_valuation,
            gameobject_guid,
            loot_id,
            LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
            true,
            policy,
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

    #[cfg(test)]
    pub(crate) async fn open_represented_fishing_hole_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        loot_id: u32,
    ) {
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.open_represented_fishing_hole_with_catalogs_like_cpp(
            &item_valuation,
            gameobject_guid,
            gameobject_entry,
            loot_id,
        )
        .await;
    }

    pub(crate) async fn open_represented_fishing_node_loot_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        area_id: u32,
        junk: bool,
    ) {
        self.open_fishing_node_operation(item_valuation, gameobject_guid, area_id, junk, LootOperationPolicy::Production).await;
    }

    pub(in crate::handlers::loot) async fn open_fishing_node_operation(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        area_id: u32,
        junk: bool,
        policy: LootOperationPolicy,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if self.resolved_player_is_alive_like_cpp() != Some(true) {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }
        let observation_result = self.represented_gameobject_loot_install_observation_result_like_cpp(gameobject_guid);
        let authority_observed = observation_result.is_some();
        let install_observation = observation_result.flatten();
        if install_observation.is_none() && !policy.permits_local_cache(authority_observed) {
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

        let Some(loot_guid) = self.next_loot_guid_operation(gameobject_guid, policy)
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
        if upserted.is_none() && !policy.permits_local_cache(authority_observed) {
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
            self.release_loot_views_operation(player_guid, policy).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.open_loot_view_operation(
            item_valuation,
            gameobject_guid,
            player_guid,
            response,
            policy,
        );
    }

    #[cfg(test)]
    pub(crate) async fn open_represented_fishing_node_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        area_id: u32,
        junk: bool,
    ) {
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.open_represented_fishing_node_loot_with_catalogs_like_cpp(
            &item_valuation,
            gameobject_guid,
            area_id,
            junk,
        )
        .await;
    }

    pub(crate) async fn open_represented_gathering_node_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        source: GatheringNodeUseSource,
    ) {
        self.open_gathering_node_operation(item_valuation, gameobject_guid, gameobject_entry, source, LootOperationPolicy::Production).await;
    }

    pub(in crate::handlers::loot) async fn open_gathering_node_operation(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        source: GatheringNodeUseSource,
        policy: LootOperationPolicy,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if self.resolved_player_is_alive_like_cpp() != Some(true) {
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

        self.open_gameobject_personal_loot_operation(
            item_valuation,
            gameobject_guid,
            source.loot_id,
            LOOT_TYPE_CHEST_LIKE_CPP,
            false,
            policy,
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

    #[cfg(test)]
    pub(crate) async fn open_represented_gathering_node_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        source: GatheringNodeUseSource,
    ) {
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.open_represented_gathering_node_with_catalogs_like_cpp(
            &item_valuation,
            gameobject_guid,
            gameobject_entry,
            source,
        )
        .await;
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

        let xp_store = self.quests.xp_store.as_ref();
        xp_store
            .map(|store| {
                wow_progression::player_level_difficulty_xp(
                    self.player_level_like_cpp(),
                    xp_difficulty,
                    |level| store.get(level).map(|row| &row.difficulty),
                )
            })
            .unwrap_or_default()
    }
}
