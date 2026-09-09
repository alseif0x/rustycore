//! Spell handlers operations, part 1 of 2.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #662; every method keeps its original body.

use super::*;

impl WorldSession {
    /// Handle `CMSG_CAST_SPELL` (0x329C).
    ///
    /// Decode movement and the original client request, then enter the shared
    /// application admission/preparation path for both instant and timed casts.
    pub async fn handle_cast_spell_with_catalogs_like_cpp(
        &mut self,
        area_trigger_catalogs: &AreaTriggerCatalogsLikeCpp,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        progression: &crate::session::ProgressionCatalogsLikeCpp,
        player_grid_loader: &crate::session::PlayerGridLoadResolverLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => {
                warn!("handle_cast_spell: no player_guid");
                return;
            }
        };

        let req = match CastSpellRequest::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to parse CMSG_CAST_SPELL: {e}"
                );
                return;
            }
        };

        let original_spell_id = req.spell_id;
        let cast_id = req.cast_id;

        debug!(
            account = self.account_id,
            spell_id = original_spell_id,
            cast_id = ?cast_id,
            target = ?req.target.unit,
            "CMSG_CAST_SPELL"
        );

        // C++ ignores a nonexistent spell before applying embedded movement.
        if self
            .spell_store()
            .and_then(|store| store.get(original_spell_id))
            .is_none()
        {
            warn!(
                account = self.account_id,
                spell_id = original_spell_id,
                "Ignoring cast request without an effective spell"
            );
            return;
        }

        // C++ `WorldSession::HandleCastSpellOpcode` applies an embedded
        // `MoveUpdate` through `HandleMovementOpcode(CMSG_MOVE_STOP, ...)`
        // after validating the `SpellInfo` and before the spell cast request
        // continues.
        if let Some(move_update) = req.move_update.clone() {
            self.handle_movement_info_with_catalogs_like_cpp(
                area_trigger_catalogs,
                creature_spawn_catalogs,
                progression,
                player_grid_loader,
                Some(ClientOpcodes::MoveStop),
                move_update,
            )
            .await;
        }

        let target_guid = if req.target.unit.is_empty() {
            player_guid
        } else {
            req.target.unit
        };
        let request = RepresentedPendingSpellCastRequestLikeCpp {
            cast_id,
            spell_id: original_spell_id,
            casting_unit_guid: player_guid,
            target_guid,
            target_data: crate::spell_cast_adapter::retain_targets(req.target),
            spell_visual: wow_entities::SpellCastVisualLikeCpp {
                spell_visual_id: req.visual.spell_visual_id,
                script_visual_id: 0,
            },
            metadata: crate::session::SpellCastMetadata {
                from_client: true,
                misc: req.misc,
                // C++ `HandleCastSpellOpcode` copies the request trajectory
                // into `m_targets`; `SpellCastTargets::HasTraj()` then gates
                // `CAST_FLAG_ADJUST_MISSILE` in `Spell::SendSpellGo`.
                request_has_trajectory_like_cpp: req.has_trajectory_like_cpp,
                request_trajectory_pitch_like_cpp: req.trajectory_pitch_like_cpp,
                ..Default::default()
            },
        };
        if crate::player_cast::request(self, request) {
            self.tick_pending_spell_cast_request_with_generator_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
            )
            .await;
        }
    }
    #[cfg(test)]
    pub async fn handle_cast_spell(&mut self, pkt: wow_packet::WorldPacket) {
        let area_trigger_catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let progression = self.progression_catalogs_for_test_like_cpp();
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_cast_spell_with_catalogs_like_cpp(
            &area_trigger_catalogs,
            &creature_spawn_catalogs,
            &progression,
            &crate::session::SessionHandlerCatalogsLikeCpp::default().player_grid_loader,
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }
    /// Handle `CMSG_OPEN_ITEM`.
    ///
    /// This ports Trinity's initial validation and fails closed until item loot
    /// storage/generation is represented in Rust.
    pub async fn handle_open_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let open = match OpenItem::read(&mut pkt) {
            Ok(open) => open,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to parse CMSG_OPEN_ITEM: {e}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            slot = open.slot,
            pack_slot = open.pack_slot,
            "CMSG_OPEN_ITEM"
        );

        let Some(item) = self.get_inventory_item_by_pos(open.slot, open.pack_slot) else {
            self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
            return;
        };

        let Some(flags) = self.item_template_flags(item.entry_id) else {
            self.send_equip_error(InventoryResult::ItemNotFound, Some(item.guid), None, 0, 0);
            return;
        };

        let is_wrapped = self
            .resolved_inventory_item_object_like_cpp(item.guid)
            .is_some_and(|runtime_item| runtime_item.is_wrapped());

        if !flags.contains(ItemFlags::HAS_LOOT) && !is_wrapped {
            self.send_equip_error(
                InventoryResult::ClientLockedOut,
                Some(item.guid),
                None,
                0,
                0,
            );
            return;
        }

        let lock_id = self.item_template_lock_id(item.entry_id).unwrap_or(0);
        if lock_id != 0 {
            if !self.lock_entry_exists_like_cpp(u32::from(lock_id)) {
                self.send_equip_error(InventoryResult::ItemLocked, Some(item.guid), None, 0, 0);
                return;
            }

            let item_is_locked = self
                .resolved_inventory_item_object_like_cpp(item.guid)
                .map_or(true, |item_object| item_object.is_locked());
            if item_is_locked {
                self.send_equip_error(InventoryResult::ItemLocked, Some(item.guid), None, 0, 0);
                return;
            }
        }

        if is_wrapped {
            self.open_wrapped_gift_like_cpp(open.slot, open.pack_slot, item.guid)
                .await;
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };

        if !self.loot_table.contains_key(&item.guid) {
            let stored_money = self.load_stored_item_money_like_cpp(item.guid).await;
            let stored_items = self.load_stored_item_items_like_cpp(item.guid).await;
            let loaded_stored_loot = stored_money.is_some() || stored_items.is_some();
            let (coins, mut items) = if loaded_stored_loot {
                (stored_money.unwrap_or(0), stored_items.unwrap_or_default())
            } else {
                let coins = {
                    let (min_money, max_money) = self
                        .load_item_template_addon_money_loot_like_cpp(item.entry_id)
                        .await;
                    self.represented_money_loot_with_rate_like_cpp(
                        min_money,
                        max_money,
                        self.loot_drop_rates_like_cpp().money,
                    )
                };
                let items = self
                    .generate_item_loot_template_entries_like_cpp(item.entry_id)
                    .await;
                (coins, items)
            };
            for entry in &mut items {
                entry.add_allowed_looter_like_cpp(player_guid);
            }
            if !loaded_stored_loot && (coins > 0 || !items.is_empty()) {
                self.save_new_stored_item_loot_like_cpp(item.guid, coins, &items)
                    .await;
            }

            self.loot_table.insert(
                item.guid,
                CreatureLoot {
                    loot_guid: item.guid,
                    coins,
                    unlooted_count: items
                        .iter()
                        .filter(|entry| !entry.taken)
                        .count()
                        .min(u8::MAX as usize) as u8,
                    loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
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
        }

        self.update_inventory_item_object_like_cpp(item.guid, |item_object| {
            item_object.set_loot_generated(true);
        });

        let Some(loot) = self.loot_table.get(&item.guid) else {
            self.send_equip_error(
                InventoryResult::ClientLockedOut,
                Some(item.guid),
                None,
                0,
                0,
            );
            return;
        };

        let items: Vec<LootItemData> = loot
            .items
            .iter()
            .filter(|entry| entry.visible_in_represented_free_for_all_view_like_cpp(player_guid))
            .map(|entry| LootItemData {
                item_type: 0,
                ui_type: entry.free_for_all_ui_type_like_cpp(),
                can_trade_to_tap_list: false,
                loot: ItemInstance {
                    item_id: entry.item_id as i32,
                    ..ItemInstance::default()
                },
                loot_list_id: entry.loot_list_id,
                quantity: entry.quantity,
                loot_item_type: 0,
            })
            .collect();

        let loot_guid = loot.loot_guid;
        let coins = loot.coins;
        self.open_active_item_loot_view_like_cpp(player_guid, item.guid)
            .await;
        self.send_packet(&LootResponse {
            owner: item.guid,
            loot_obj: loot_guid,
            failure_reason: 0,
            acquire_reason: LOOT_TYPE_ITEM_LIKE_CPP,
            loot_method: 0,
            threshold: 2,
            coins,
            items,
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        });
    }
    pub(crate) async fn open_active_item_loot_view_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
    ) {
        if self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.add_active_loot_view_owner_like_cpp(item_guid);
    }
    pub(super) async fn open_wrapped_gift_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) {
        let gift = match self.load_wrapped_gift_row_like_cpp(item_guid).await {
            WrappedGiftLoad::Found(gift) => gift,
            WrappedGiftLoad::Missing => {
                self.destroy_stale_wrapped_gift_like_cpp(bag, slot, item_guid)
                    .await;
                return;
            }
            WrappedGiftLoad::Unavailable => return,
        };

        let Some(durability) = self.apply_wrapped_gift_row_to_runtime_item_like_cpp(
            bag, item_guid, slot, gift.entry, gift.flags,
        ) else {
            return;
        };

        self.persist_wrapped_gift_open_like_cpp(item_guid, gift.entry, gift.flags, durability)
            .await;
    }
    pub(crate) fn apply_wrapped_gift_row_to_runtime_item_like_cpp(
        &mut self,
        bag: u8,
        item_guid: ObjectGuid,
        slot: u8,
        entry: u32,
        flags: u32,
    ) -> Option<u32> {
        let current_item = self.get_inventory_item_by_pos(bag, slot)?;
        if current_item.guid != item_guid {
            return None;
        }

        let max_durability = self.item_template_max_durability(entry);
        let inventory_type = self.item_template_inventory_type(entry);
        let mut durability = None;
        let updated = self.update_inventory_item_object_like_cpp(item_guid, |item_object| {
            if item_object.is_wrapped() && item_object.object().guid() == item_guid {
                durability = Some(apply_wrapped_gift_transform_like_cpp(
                    item_object,
                    entry,
                    flags,
                    max_durability,
                ));
            }
        });
        if !updated {
            return None;
        }
        let durability = durability?;

        if bag == INVENTORY_SLOT_BAG_0 {
            self.update_inventory_item_metadata_like_cpp(slot, item_guid, entry, inventory_type);
        }

        Some(durability)
    }
    pub(super) async fn load_wrapped_gift_row_like_cpp(
        &self,
        item_guid: ObjectGuid,
    ) -> WrappedGiftLoad {
        let Some(port) = self.stored_item_persistence_port_like_cpp() else {
            return WrappedGiftLoad::Unavailable;
        };
        match port
            .load_wrapped_gift_like_cpp(item_guid.counter() as u64)
            .await
        {
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Loaded(row) => {
                WrappedGiftLoad::Found(WrappedGiftRow {
                    entry: row.entry,
                    flags: row.flags,
                })
            }
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Missing => WrappedGiftLoad::Missing,
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(item_guid = item_guid.counter(), error = %reason, "failed to load wrapped gift row");
                WrappedGiftLoad::Unavailable
            }
        }
    }
    pub(super) async fn destroy_stale_wrapped_gift_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(item) = self.get_inventory_item_by_pos(bag, slot) else {
            return;
        };
        if item.guid != item_guid {
            return;
        }
        let Some(port) = self.stored_item_persistence_port_like_cpp() else {
            return;
        };

        let runtime_item = self.resolved_inventory_item_object_like_cpp(item_guid);
        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());

        match port
            .destroy_inventory_item_like_cpp(
                wow_persistence::InventoryItemDestroyPersistenceRequestLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    item_guid: item.db_guid,
                    expire_refund: should_expire_refund,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(item_guid = item_guid.counter(), error = %reason, "failed to destroy stale wrapped gift");
                return;
            }
        }

        self.remove_fully_looted_runtime_item(bag, slot, item.guid);

        if should_expire_refund {
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: item.guid,
            });
        }

        if bag == INVENTORY_SLOT_BAG_0 {
            let mut visible_item_changes = Vec::new();
            let mut virtual_item_changes = Vec::new();
            if (slot as usize) < 19 {
                visible_item_changes.push((slot, 0i32, 0u16, 0u16));
            }
            if (15..=17).contains(&slot) {
                virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
            }

            self.send_player_values_update_from_entity_bridge(
                &[(slot, ObjectGuid::EMPTY)],
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );

            if slot < 19 {
                self.send_stat_update();
            }
        }
    }
    pub(super) async fn persist_wrapped_gift_open_like_cpp(
        &self,
        item_guid: ObjectGuid,
        entry: u32,
        flags: u32,
        durability: u32,
    ) {
        let Some(port) = self.stored_item_persistence_port_like_cpp() else {
            return;
        };
        let outcome = port
            .open_wrapped_gift_like_cpp(wow_persistence::WrappedGiftOpenPersistenceRequestLikeCpp {
                item_guid: item_guid.counter() as u64,
                entry,
                flags,
                durability,
            })
            .await;
        if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
        | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
        {
            warn!(item_guid = item_guid.counter(), entry, error = %reason, "failed to persist wrapped gift open");
        }
    }
    pub(super) async fn load_item_template_addon_money_loot_like_cpp(
        &self,
        item_entry: u32,
    ) -> (u32, u32) {
        let Some(port) = self.item_template_addon_catalog_persistence_port_like_cpp() else {
            return (0, 0);
        };

        match port
            .load_item_template_addon_money_like_cpp(
                wow_persistence::ItemTemplateAddonCatalogRequestLikeCpp { item_entry },
            )
            .await
        {
            wow_persistence::ItemTemplateAddonMoneyOutcomeLikeCpp::Found(row) => {
                match (row.min_money, row.max_money) {
                    (Some(min_money), Some(max_money)) => {
                        if min_money > max_money {
                            // ObjectMgr::LoadItemTemplateAddon swaps invalid item
                            // bounds before storing the template. GameObject addon
                            // money deliberately does not share this normalization.
                            warn!(
                                item_entry,
                                min_money,
                                max_money,
                                "minimum item money loot exceeded maximum; swapping like C++"
                            );
                        }
                        normalize_item_money_loot_bounds_like_cpp(min_money, max_money)
                    }
                    _ => {
                        warn!(
                            item_entry,
                            "failed to decode item_template_addon money loot as C++ uint32 columns"
                        );
                        (0, 0)
                    }
                }
            }
            wow_persistence::ItemTemplateAddonMoneyOutcomeLikeCpp::Missing => (0, 0),
            wow_persistence::ItemTemplateAddonMoneyOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_entry,
                    error = %reason,
                    "failed to load item_template_addon money loot"
                );
                (0, 0)
            }
        }
    }
    pub(super) async fn load_item_template_addon_loot_metadata_like_cpp(
        &self,
        item_entry: u32,
    ) -> ItemTemplateAddonLootMetadataLikeCpp {
        let Some(port) = self.item_template_addon_catalog_persistence_port_like_cpp() else {
            return ItemTemplateAddonLootMetadataLikeCpp::default();
        };

        match port
            .load_item_template_addon_loot_metadata_like_cpp(
                wow_persistence::ItemTemplateAddonCatalogRequestLikeCpp { item_entry },
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
                    item_entry,
                    error = %reason,
                    "failed to load item_template_addon loot metadata"
                );
                ItemTemplateAddonLootMetadataLikeCpp::default()
            }
        }
    }
    pub(super) async fn load_item_template_addon_loot_metadata_for_rows_like_cpp(
        &self,
        rows: &[LootTemplateRow],
    ) -> HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp> {
        let mut item_ids: Vec<u32> = rows
            .iter()
            .filter(|row| row.reference == 0 && row.item_id != 0)
            .map(|row| row.item_id)
            .collect();
        item_ids.sort_unstable();
        item_ids.dedup();

        let mut metadata = HashMap::with_capacity(item_ids.len());
        for item_id in item_ids {
            metadata.insert(
                item_id,
                self.load_item_template_addon_loot_metadata_like_cpp(item_id)
                    .await,
            );
        }

        metadata
    }
    pub(super) async fn generate_item_loot_template_entries_like_cpp(
        &mut self,
        item_entry: u32,
    ) -> Vec<LootEntry> {
        let mut loot_items = Vec::new();
        let mut frames = Vec::new();
        let rows = self
            .load_loot_template_rows_like_cpp(LootTemplateTable::Item, item_entry)
            .await;
        let condition_references = self
            .load_loot_template_condition_reference_rows_like_cpp(&rows)
            .await;
        frames.push(LootTemplateFrame {
            rows,
            condition_references,
            index: 0,
            group_id: 0,
            groups_enqueued: false,
        });

        let mut rng = self.represented_runtime_subrng_like_cpp();
        let mut processed_frames = 0u32;
        while let Some(mut frame) = frames.pop() {
            if frame.group_id != 0 {
                let addon_metadata = self
                    .load_item_template_addon_loot_metadata_for_rows_like_cpp(&frame.rows)
                    .await;
                if let Some(row) = roll_group_loot_row_like_cpp(
                    &frame.rows,
                    frame.group_id,
                    |item_id| self.item_storage_template(item_id).is_some(),
                    |row| {
                        let metadata = addon_metadata
                            .get(&row.item_id)
                            .copied()
                            .unwrap_or_default();
                        self.item_loot_allowed_for_player_like_cpp_representable(
                            row.item_id,
                            row.needs_quest,
                            metadata,
                            &row.conditions,
                            &frame.condition_references,
                        )
                    },
                    |item_id| self.item_drop_rate_like_cpp(item_id),
                    &mut rng,
                ) {
                    let metadata = addon_metadata
                        .get(&row.item_id)
                        .copied()
                        .unwrap_or_default();
                    let flags = self.loot_entry_flags_for_row_like_cpp(&row, metadata);
                    add_loot_template_row_item_like_cpp(
                        &mut loot_items,
                        &row,
                        flags,
                        |item_id| {
                            self.item_storage_template(item_id)
                                .map(|template| template.max_stack_size)
                                .unwrap_or(1)
                        },
                        &mut rng,
                    );
                }
                continue;
            }

            if frame.index >= frame.rows.len() {
                if !frame.groups_enqueued {
                    frame.groups_enqueued = true;
                    let mut groups: Vec<u8> = frame
                        .rows
                        .iter()
                        .filter(|row| row.reference == 0 && row.group_id != 0)
                        .map(|row| row.group_id)
                        .collect();
                    groups.sort_unstable();
                    groups.dedup();

                    if !groups.is_empty() {
                        let rows = frame.rows.clone();
                        let condition_references = frame.condition_references.clone();
                        frames.push(frame);
                        for group_id in groups.into_iter().rev() {
                            frames.push(LootTemplateFrame {
                                rows: rows.clone(),
                                condition_references: condition_references.clone(),
                                index: 0,
                                group_id,
                                groups_enqueued: true,
                            });
                        }
                    }
                }
                continue;
            }
            let row = frame.rows[frame.index].clone();
            frame.index += 1;
            let condition_references = frame.condition_references.clone();
            frames.push(frame);

            if row.loot_mode & LOOT_MODE_DEFAULT_LIKE_CPP == 0 {
                continue;
            }

            if row.group_id != 0 && row.reference == 0 {
                continue;
            }

            if row.reference > 0 {
                if !loot_template_reference_row_can_roll_like_cpp(
                    row.reference,
                    row.chance,
                    row.loot_mode,
                    row.min_count,
                ) {
                    continue;
                }
                if row.chance < 100.0
                    && !roll_chance_with_rate_like_cpp(
                        row.chance,
                        self.loot_drop_rates_like_cpp().item_referenced,
                        &mut rng,
                    )
                {
                    continue;
                }

                let reference_rows = self
                    .load_loot_template_rows_like_cpp(LootTemplateTable::Reference, row.reference)
                    .await;
                let reference_condition_references = self
                    .load_loot_template_condition_reference_rows_like_cpp(&reference_rows)
                    .await;
                let max_count = referenced_loot_max_count_like_cpp(
                    row.max_count,
                    self.loot_drop_rates_like_cpp().item_referenced_amount,
                );
                for _ in 0..max_count {
                    frames.push(LootTemplateFrame {
                        rows: reference_rows.clone(),
                        condition_references: reference_condition_references.clone(),
                        index: 0,
                        group_id: row.group_id,
                        groups_enqueued: false,
                    });
                }
                processed_frames = processed_frames.saturating_add(1);
                if processed_frames > MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP {
                    warn!(
                        item_entry,
                        reference = row.reference,
                        "stopped item loot reference processing after safety cap"
                    );
                    break;
                }
                continue;
            }

            let addon_metadata = self
                .load_item_template_addon_loot_metadata_like_cpp(row.item_id)
                .await;
            if !loot_template_plain_row_can_roll_like_cpp(
                row.item_id,
                row.chance,
                row.needs_quest,
                row.loot_mode,
                row.min_count,
                row.max_count,
                self.item_storage_template(row.item_id).is_some(),
                self.item_loot_allowed_for_player_like_cpp_representable(
                    row.item_id,
                    row.needs_quest,
                    addon_metadata,
                    &row.conditions,
                    &condition_references,
                ),
            ) {
                continue;
            }
            if row.chance < 100.0
                && !roll_chance_with_rate_like_cpp(
                    row.chance,
                    self.item_drop_rate_like_cpp(row.item_id),
                    &mut rng,
                )
            {
                continue;
            }
            let flags = self.loot_entry_flags_for_row_like_cpp(&row, addon_metadata);
            add_loot_template_row_item_like_cpp(
                &mut loot_items,
                &row,
                flags,
                |item_id| {
                    self.item_storage_template(item_id)
                        .map(|template| template.max_stack_size)
                        .unwrap_or(1)
                },
                &mut rng,
            );
        }

        loot_items
    }
    pub(super) async fn load_loot_template_rows_like_cpp(
        &self,
        table: LootTemplateTable,
        entry: u32,
    ) -> Vec<LootTemplateRow> {
        let Some(port) = self.loot_template_catalog_persistence_port_like_cpp() else {
            return Vec::new();
        };

        let persistence_table = match table {
            LootTemplateTable::Item => wow_persistence::LootTemplateTablePersistenceLikeCpp::Item,
            LootTemplateTable::Reference => {
                wow_persistence::LootTemplateTablePersistenceLikeCpp::Reference
            }
        };
        let persistence_rows = match port
            .load_loot_template_rows_like_cpp(persistence_table, entry)
            .await
        {
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    entry,
                    table = table.name(),
                    error = %reason,
                    "failed to load loot template rows"
                );
                return Vec::new();
            }
        };

        let mut rows = persistence_rows
            .into_iter()
            .map(|row| LootTemplateRow {
                item_id: row.item_id,
                reference: row.reference,
                chance: row.chance,
                needs_quest: row.needs_quest,
                loot_mode: row.loot_mode,
                group_id: row.group_id,
                min_count: row.min_count,
                max_count: row.max_count,
                conditions: Vec::new(),
            })
            .collect::<Vec<_>>();

        let condition_source_type = table.condition_source_type_like_cpp();
        for row in &mut rows {
            row.conditions = self
                .load_loot_template_condition_rows_like_cpp(
                    condition_source_type,
                    entry,
                    row.item_id,
                )
                .await;
        }

        rows
    }
}
