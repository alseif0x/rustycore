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

#[path = "sources/creature.rs"]
mod creature;

mod creature_conditions;
mod gameobject;
mod gameobject_authority;
mod chest_generation;

impl WorldSession {
    async fn open_represented_gameobject_personal_loot_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        loot_id: u32,
        loot_type: u8,
        replace_existing: bool,
    ) {
        self.open_gameobject_personal_loot_operation(item_valuation, gameobject_guid, loot_id, loot_type, replace_existing, LootOperationPolicy::Production).await;
    }

    pub(in crate::handlers::loot) async fn open_gameobject_personal_loot_operation(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        loot_id: u32,
        loot_type: u8,
        replace_existing: bool,
        policy: LootOperationPolicy,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if loot_id == 0 || self.resolved_player_is_alive_like_cpp() != Some(true) {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }

        // Fishing holes replace this player's personal `Loot` in place. Close
        // the old C++ view before the upsert so its release cannot detach or
        // apply lifecycle state to the freshly generated pool.
        if replace_existing && self.has_active_non_item_loot_views_like_cpp() {
            self.release_loot_views_operation(player_guid, policy).await;
        }

        // C++ serializes template generation and `ClearLoot` on the map
        // thread. Rust awaits database-backed template generation, so retain
        // the exact object lifetime and authority tombstone across that await.
        let observation_result = self.represented_gameobject_loot_install_observation_result_like_cpp(gameobject_guid);
        let authority_observed = observation_result.is_some();
        let install_observation = observation_result.flatten();
        if install_observation.is_none() && !policy.permits_local_cache(authority_observed) {
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
            if upserted.is_none() && !policy.permits_local_cache(authority_observed) {
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

    async fn ensure_represented_gameobject_chest_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
        template_money: (u32, u32),
    ) {
        self.ensure_gameobject_chest_loot_with_policy(
            gameobject_guid, player_guid, source, allowed_looters, template_money,
            LootCyclePolicy::Production,
        ).await;
    }

    async fn ensure_gameobject_chest_loot_with_policy(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
        template_money: (u32, u32),
        policy: LootCyclePolicy,
    ) {
        self.ensure_gameobject_chest_loot_operation(gameobject_guid, player_guid, source, allowed_looters, template_money, policy.operation_policy()).await;
    }

    pub(in crate::handlers::loot) async fn ensure_gameobject_chest_loot_operation(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
        template_money: (u32, u32),
        policy: LootOperationPolicy,
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
            None if !policy.permits_local_cache(false) => {
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
            if policy.permits_initial_binding()
                && authority.is_pristine_like_cpp()
                && self.loot_table.contains_key(&gameobject_guid)
            {
                if !self
                    .represented_personal_loot_owners
                    .contains(&gameobject_guid)
                    && let Some(loot) = self.loot_table.get_mut(&gameobject_guid)
                {
                    prepare_represented_shared_loot_generation_like_cpp(loot, allowed_looters);
                }
                if self
                    .sync_gameobject_loot_operation(
                        gameobject_guid,
                        player_guid,
                        policy,
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
                .generate_chest_loot_operation(
                    gameobject_guid,
                    player_guid,
                    source,
                    generation_allowed_looters,
                    template_money,
                    policy,
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
            } else if policy.permits_local_cache(false) {
                self.loot_table.insert(gameobject_guid, loot);
            }
        }
    }

    #[cfg(test)]
    pub(in crate::handlers) async fn open_represented_gameobject_chest_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) {
        let template_money = self
            .world_query_catalogs_like_cpp()
            .and_then(|catalogs| catalogs.gameobject.get(gameobject_guid.entry()))
            .map(|row| (row.min_money, row.max_money))
            .unwrap_or((0, 0));
        let generators = self.id_generators_for_test_like_cpp();
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.open_represented_gameobject_chest_with_template_money_like_cpp(
            generators.item.as_ref(),
            &item_valuation,
            gameobject_guid,
            source,
            template_money,
        )
        .await;
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
}
