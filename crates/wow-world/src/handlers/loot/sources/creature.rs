// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature loot generation, persistence observations, and source snapshots.

use super::*;

impl WorldSession {
    pub(in crate::handlers::loot) fn sync_represented_creature_loot_to_canonical_like_cpp(
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

    pub(in crate::handlers::loot) fn canonical_creature_fully_looted_after_represented_sync_like_cpp(
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

    pub(in crate::handlers::loot) async fn represented_ae_loot_creature_targets_like_cpp(
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
    pub(in crate::handlers::loot) fn install_represented_creature_kill_loot_if_current_like_cpp(
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
                        || self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
                            *tapper,
                            dungeon_encounter_id,
                        )
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

    pub(in crate::handlers::loot) async fn generate_represented_creature_loot_like_cpp(
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
        let Some(group_guid) = self.resolved_group_guid_like_cpp() else {
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

    pub(in crate::handlers::loot) async fn generate_represented_creature_loot_items_for_player_like_cpp(
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

    pub(in crate::handlers::loot) fn represented_creature_loot_state_like_cpp(
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

    pub(in crate::handlers::loot) fn represented_creature_position_for_loot_like_cpp(
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
}
