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

impl WorldSession {
    async fn open_represented_gameobject_personal_loot_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        loot_id: u32,
        loot_type: u8,
        replace_existing: bool,
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
        self.represented_on_loot_opened_with_catalogs_like_cpp(
            item_valuation,
            gameobject_guid,
            player_guid,
            response,
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
                .generate_represented_gameobject_chest_loot_with_template_money_like_cpp(
                    gameobject_guid,
                    player_guid,
                    source,
                    generation_allowed_looters,
                    template_money,
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

    pub(super) async fn generate_represented_gameobject_chest_loot_with_template_money_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
        template_money: (u32, u32),
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
        let (min_money, max_money) = template_money;
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

    #[cfg(test)]
    pub(crate) async fn open_represented_gameobject_chest_like_cpp(
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

    #[cfg(test)]
    pub(super) async fn generate_represented_gameobject_chest_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
    ) -> Option<CreatureLoot> {
        let template_money = self
            .world_query_catalogs_like_cpp()
            .and_then(|catalogs| catalogs.gameobject.get(gameobject_guid.entry()))
            .map(|row| (row.min_money, row.max_money))
            .unwrap_or((0, 0));
        self.generate_represented_gameobject_chest_loot_with_template_money_like_cpp(
            gameobject_guid,
            player_guid,
            source,
            allowed_looters,
            template_money,
        )
        .await
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
        let Some(group_guid) = self.resolved_group_guid_like_cpp() else {
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
        item_guid_generator: &wow_core::ObjectGuidGenerator,
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
                .store_direct_loot_item_with_generator_like_cpp(
                    item_guid_generator,
                    &entry,
                    source.dungeon_encounter_id,
                )
                .await
            {
                all_stored = false;
            }
        }

        all_stored
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
