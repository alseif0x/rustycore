//! Gameobject loot template generation, pool inputs and push-loot storage.
use super::*;

impl WorldSession {
    pub(in crate::handlers::loot) async fn generate_represented_gameobject_chest_loot_with_template_money_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
        template_money: (u32, u32),
    ) -> Option<CreatureLoot> {
        self.generate_chest_loot_operation(gameobject_guid, player_guid, source, allowed_looters, template_money, LootOperationPolicy::Production).await
    }

    pub(in crate::handlers::loot) async fn generate_chest_loot_operation(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
        template_money: (u32, u32),
        policy: LootOperationPolicy,
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

        let loot_guid = self.next_loot_guid_operation(gameobject_guid, policy)?;
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
    pub(in crate::handlers::loot) async fn generate_represented_gameobject_chest_loot_like_cpp(
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

    pub(super) fn represented_gameobject_personal_encounter_tappers_like_cpp(
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

    pub(in crate::handlers::loot) fn represented_gameobject_chest_group_state_like_cpp(
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

    pub(super) async fn generate_represented_gameobject_loot_items_like_cpp(
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

    pub(super) async fn generate_represented_shared_gameobject_loot_items_like_cpp(
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

    pub(super) async fn generate_represented_gameobject_loot_items_for_store_like_cpp(
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

    pub(super) async fn generate_represented_fishing_loot_items_like_cpp(
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

    pub(super) async fn generate_represented_gameobject_personal_loot_items_like_cpp(
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

    pub(super) async fn autostore_represented_gameobject_chest_push_loot_like_cpp(
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

}
