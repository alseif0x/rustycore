//! Chest money opening shared by production and the explicit legacy fixture cycle.

use super::*;

impl WorldSession {
    pub(crate) async fn open_represented_gameobject_chest_with_template_money_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
        template_money: (u32, u32),
    ) {
        self.open_gameobject_chest_with_policy(
            item_guid_generator, item_valuation, gameobject_guid, source, template_money,
            LootCyclePolicy::Production,
        ).await;
    }

    pub(in crate::handlers::loot) async fn open_gameobject_chest_with_policy(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
        template_money: (u32, u32),
        policy: LootCyclePolicy,
    ) {
        self.open_gameobject_chest_operation(item_guid_generator, item_valuation, gameobject_guid, source, template_money, policy.operation_policy()).await;
    }

    pub(in crate::handlers::loot) async fn open_gameobject_chest_operation(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
        template_money: (u32, u32),
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
                    item_guid_generator,
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
        self.ensure_gameobject_chest_loot_operation(
            gameobject_guid,
            player_guid,
            source,
            &allowed_looters,
            template_money,
            policy,
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
            .sync_gameobject_loot_operation(gameobject_guid, player_guid, policy)
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
            self.release_loot_views_operation(player_guid, policy.release_policy()).await;
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
}
