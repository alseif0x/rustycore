//! Creature loot template and personal-pool generation. Async IO stays in APP.
use super::*;

impl WorldSession {
    pub(in crate::handlers::loot) async fn generate_represented_creature_loot_like_cpp(
        &mut self, creature_guid: ObjectGuid, loot_owner_guid: ObjectGuid, level: u8,
        entry: u32, loot_id: u32, gold_min: u32, gold_max: u32, dungeon_encounter_id: u32,
    ) -> Option<CreatureLoot> {
        self.generate_creature_loot(creature_guid, loot_owner_guid, level, entry,
            loot_id, gold_min, gold_max, dungeon_encounter_id, None).await.ok()
    }

    pub(super) async fn generate_creature_loot(
        &mut self,
        creature_guid: ObjectGuid,
        loot_owner_guid: ObjectGuid,
        _level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
        original: OriginalMeleeLoot<'_>,
    ) -> Result<CreatureLoot, MeleeLootError> {
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

        let loot_guid = self.next_creature_generation_guid(creature_guid, original)?;
        Ok(CreatureLoot {
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
    pub(super) fn represented_connected_creature_tappers_like_cpp(
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
    pub(super) async fn generate_represented_creature_personal_loot_like_cpp(
        &mut self, creature_guid: ObjectGuid, level: u8, entry: u32,
        loot_id: u32, gold_min: u32, gold_max: u32, dungeon_encounter_id: u32,
        tappers: &[ObjectGuid],
    ) -> Option<HashMap<ObjectGuid, CreatureLoot>> {
        self.generate_creature_personal_loot(creature_guid, level, entry, loot_id,
            gold_min, gold_max, dungeon_encounter_id, tappers, None).await.ok()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn generate_creature_personal_loot(
        &mut self,
        creature_guid: ObjectGuid,
        _level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
        tappers: &[ObjectGuid],
        mut original: OriginalMeleeLoot<'_>,
    ) -> Result<HashMap<ObjectGuid, CreatureLoot>, MeleeLootError> {
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
            let loot_guid = self.next_creature_generation_guid(creature_guid,
                original.as_mut().map(|(tick, operation)| (*tick, &mut **operation)))?;
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
        Ok(personal)
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

}
