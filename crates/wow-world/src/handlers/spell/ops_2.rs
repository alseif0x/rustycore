//! Spell handlers operations, part 2 of 2.
//!
//! The inherent `WorldSession` impl is divided by responsibility under
//! #662; every method keeps its original body.

use super::*;

impl WorldSession {
    pub(super) async fn load_loot_template_condition_rows_like_cpp(
        &self,
        source_type: i32,
        source_group: u32,
        source_entry: u32,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Some(port) = self.loot_template_catalog_persistence_port_like_cpp() else {
            return Vec::new();
        };

        let rows = match port
            .load_loot_condition_rows_like_cpp(source_type, source_group, source_entry)
            .await
        {
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    source_type,
                    source_group,
                    source_entry,
                    error = %reason,
                    "failed to load loot template condition rows"
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
                source_type,
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
    pub(super) async fn load_loot_template_condition_reference_rows_like_cpp(
        &self,
        rows: &[LootTemplateRow],
    ) -> HashMap<u32, Vec<LootConditionRowLikeCpp>> {
        let mut references = HashMap::new();
        let mut pending = Vec::new();
        for row in rows {
            pending.extend(loot_condition_reference_ids_like_cpp(&row.conditions));
        }

        while let Some(reference_id) = pending.pop() {
            if references.contains_key(&reference_id) {
                continue;
            }

            let reference_rows = self
                .load_loot_template_condition_reference_rows_for_id_like_cpp(reference_id)
                .await;
            for nested_reference_id in loot_condition_reference_ids_like_cpp(&reference_rows) {
                if !references.contains_key(&nested_reference_id) {
                    pending.push(nested_reference_id);
                }
            }
            references.insert(reference_id, reference_rows);
        }

        references
    }
    pub(super) async fn load_loot_template_condition_reference_rows_for_id_like_cpp(
        &self,
        reference_id: u32,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Ok(reference_source_type) = i32::try_from(reference_id).map(|id| -id) else {
            return Vec::new();
        };

        self.load_loot_template_condition_rows_like_cpp(reference_source_type, 0, 0)
            .await
    }
    pub(super) fn item_loot_allowed_for_player_like_cpp_representable(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
        conditions: &[LootConditionRowLikeCpp],
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
    ) -> bool {
        if !self.loot_conditions_allow_player_with_references_like_cpp_representable(
            conditions,
            condition_references,
        ) {
            return false;
        }

        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let start_quest_id = self.item_template_start_quest_id(item_id).unwrap_or(0);
        let has_non_none_start_quest_status =
            u32::try_from(start_quest_id).ok().is_some_and(|quest_id| {
                quest_id != 0
                    && (quests.statuses.contains_key(&quest_id)
                        || quests.rewarded_quest_ids.contains(&quest_id))
            });

        let has_quest_for_item = self
            .has_incomplete_quest_objective_for_item_like_cpp_representable(item_id)
            || (addon_metadata.quest_log_item_id != 0
                && self.has_incomplete_quest_objective_for_object_id_like_cpp_representable(
                    addon_metadata.quest_log_item_id,
                ))
            || self.has_incomplete_quest_item_drop_for_item_like_cpp_representable(item_id);

        item_loot_quest_status_allows_like_cpp(
            addon_metadata.ignores_quest_status(),
            needs_quest,
            has_non_none_start_quest_status,
            has_quest_for_item,
        )
    }
    pub(super) fn loot_entry_flags_for_row_like_cpp(
        &self,
        row: &LootTemplateRow,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    ) -> LootEntryFlags {
        let template = self.item_storage_template(row.item_id);
        loot_entry_flags_for_row_metadata_like_cpp(
            row.needs_quest,
            template
                .map(|template| template.flags)
                .unwrap_or(ItemFlags::empty()),
            addon_metadata,
        )
    }
    pub(super) fn has_incomplete_quest_objective_for_item_like_cpp_representable(
        &self,
        item_id: u32,
    ) -> bool {
        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };

        self.has_incomplete_quest_objective_for_object_id_like_cpp_representable(item_object_id)
    }
    pub(super) fn has_incomplete_quest_objective_for_object_id_like_cpp_representable(
        &self,
        item_object_id: i32,
    ) -> bool {
        let Some(quest_store) = &self.quests.store else {
            return false;
        };
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        quests.statuses.values().any(|status| {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                return false;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                return false;
            };

            quest
                .objectives
                .iter()
                .enumerate()
                .any(|(fallback_index, objective)| {
                    if objective.obj_type != 1 || objective.object_id != item_object_id {
                        return false;
                    }

                    let storage_index = usize::try_from(objective.storage_index)
                        .ok()
                        .unwrap_or(fallback_index);
                    let current = status
                        .objective_counts
                        .get(storage_index)
                        .copied()
                        .unwrap_or(0);
                    current < objective.amount.max(1)
                })
        })
    }
    pub(super) fn has_incomplete_quest_item_drop_for_item_like_cpp_representable(
        &self,
        item_id: u32,
    ) -> bool {
        let Some(quest_store) = &self.quests.store else {
            return false;
        };
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        quests.statuses.values().any(|status| {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
                return false;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                return false;
            };

            quest
                .item_drop
                .iter()
                .enumerate()
                .any(|(index, drop_item_id)| {
                    if *drop_item_id != item_id {
                        return false;
                    }

                    let Some(template) = self.item_storage_template(item_id) else {
                        return false;
                    };

                    let quantity = quest.item_drop_quantity[index];
                    let mut max_allowed_count = if quantity != 0 {
                        quantity
                    } else {
                        template.max_stack_size
                    };
                    if template.max_count > 0 {
                        max_allowed_count = max_allowed_count.min(template.max_count as u32);
                    }

                    self.direct_inventory_item_count_like_cpp_representable(item_id)
                        .is_some_and(|count| count < max_allowed_count)
                })
        })
    }
    pub(super) fn direct_inventory_item_count_like_cpp_representable(
        &self,
        item_id: u32,
    ) -> Option<u32> {
        Some(
            self.resolved_inventory_items_like_cpp()?
                .values()
                .filter(|inventory_item| inventory_item.entry_id == item_id)
                .filter_map(|inventory_item| {
                    self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
                })
                .filter(|item| !item.is_in_trade())
                .fold(0_u32, |total, item| total.saturating_add(item.count())),
        )
    }
    pub(super) fn loot_conditions_allow_player_with_references_like_cpp_representable(
        &self,
        conditions: &[LootConditionRowLikeCpp],
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
    ) -> bool {
        loot_conditions_allow_player_with_references_like_cpp_representable(
            conditions,
            condition_references,
            |condition| self.evaluate_loot_condition_like_cpp_representable(condition),
        )
    }
    pub(super) fn evaluate_loot_condition_like_cpp_representable(
        &self,
        condition: &LootConditionRowLikeCpp,
    ) -> Option<bool> {
        let quests = self.player_quest_gameplay_snapshot_like_cpp()?;
        match condition.condition_type_or_reference {
            0 => Some(true),
            2 => {
                if condition.value3 != 0 {
                    return None;
                }
                Some(
                    self.direct_inventory_item_count_like_cpp_representable(condition.value1)?
                        >= condition.value2,
                )
            }
            6 => Some(
                player_team_for_race_cpp_representable(self.player_race_like_cpp())
                    == condition.value1,
            ),
            8 => Some(quests.rewarded_quest_ids.contains(&condition.value1)),
            9 => Some(
                quests
                    .statuses
                    .get(&condition.value1)
                    .is_some_and(|status| status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP),
            ),
            14 => Some(
                !quests.statuses.contains_key(&condition.value1)
                    && !quests.rewarded_quest_ids.contains(&condition.value1),
            ),
            15 => Some(
                player_class_mask_like_cpp(self.player_class_like_cpp())
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            16 => Some(
                player_race_mask_like_cpp(self.player_race_like_cpp())
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            20 => Some(u32::from(self.player_gender_like_cpp()) == condition.value1),
            25 => i32::try_from(condition.value1)
                .ok()
                .map(|spell_id| self.known_spells_like_cpp().contains(&spell_id)),
            27 => condition_compare_values_like_cpp(
                condition.value2,
                u32::from(self.player_level_like_cpp()),
                condition.value1,
            ),
            28 => Some(
                quests
                    .statuses
                    .get(&condition.value1)
                    .is_some_and(|status| status.status == 2)
                    && !quests.rewarded_quest_ids.contains(&condition.value1),
            ),
            47 => Some(
                player_quest_status_mask_like_cpp(
                    quests
                        .statuses
                        .get(&condition.value1)
                        .map(|status| status.status),
                    quests.rewarded_quest_ids.contains(&condition.value1),
                ) & condition.value2
                    != 0,
            ),
            48 => Some(
                self.player_quest_objective_progress_like_cpp_representable(condition.value1)
                    == Some(condition.value3 as i32),
            ),
            CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP => {
                Some(condition.value1 == TYPEID_PLAYER_LIKE_CPP)
            }
            CONDITION_TYPE_MASK_LIKE_CPP => Some(condition.value1 & PLAYER_TYPE_MASK_LIKE_CPP != 0),
            _ => None,
        }
    }
    pub(super) fn player_quest_objective_progress_like_cpp_representable(
        &self,
        objective_id: u32,
    ) -> Option<i32> {
        let quest_store = self.quests.store.as_ref()?;
        let quests = self.player_quest_gameplay_snapshot_like_cpp()?;

        for status in quests.statuses.values() {
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            let Some((_, objective)) = quest
                .objectives
                .iter()
                .enumerate()
                .find(|(_, objective)| objective.id == objective_id)
            else {
                continue;
            };
            let objective_index = objective.storage_index.max(0) as usize;
            return Some(
                status
                    .objective_counts
                    .get(objective_index)
                    .copied()
                    .unwrap_or(0),
            );
        }

        None
    }
    pub(super) async fn load_stored_item_money_like_cpp(
        &self,
        item_guid: wow_core::ObjectGuid,
    ) -> Option<u32> {
        let port = self.stored_item_persistence_port_like_cpp()?;
        match port
            .load_stored_item_money_like_cpp(item_guid.counter() as u64)
            .await
        {
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Loaded(money) => Some(money),
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Missing => None,
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_guid = item_guid.counter(),
                    error = %reason,
                    "failed to load stored item loot money"
                );
                None
            }
        }
    }
    pub(super) async fn load_stored_item_items_like_cpp(
        &self,
        item_guid: wow_core::ObjectGuid,
    ) -> Option<Vec<LootEntry>> {
        let port = self.stored_item_persistence_port_like_cpp()?;
        let rows = match port
            .load_stored_item_loot_like_cpp(item_guid.counter() as u64)
            .await
        {
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Missing => return None,
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_guid = item_guid.counter(),
                    error = %reason,
                    "failed to load stored item loot rows"
                );
                return None;
            }
        };

        let mut items = Vec::new();
        for row in rows {
            if stored_item_row_can_load_like_cpp_representable(
                row.item_id,
                row.count,
                row.item_index,
                row.blocked,
                row.needs_quest,
                row.random_properties_id,
                row.random_properties_seed,
                row.context,
                self.item_storage_template(row.item_id).is_some(),
            ) {
                items.push(LootEntry {
                    loot_list_id: row.item_index as u8,
                    item_id: row.item_id,
                    quantity: row.count,
                    random_properties_id: row.random_properties_id,
                    random_properties_seed: row.random_properties_seed,
                    item_context: row.context,
                    flags: LootEntryFlags {
                        follow_loot_rules: row.follow_loot_rules,
                        freeforall: row.free_for_all,
                        blocked: row.blocked,
                        counted: row.counted,
                        under_threshold: row.under_threshold,
                        needs_quest: row.needs_quest,
                    },
                    allowed_looters: Vec::new(),
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                });
            }
        }

        Some(items)
    }
    pub(super) async fn save_new_stored_item_loot_like_cpp(
        &self,
        item_guid: wow_core::ObjectGuid,
        money: u32,
        items: &[LootEntry],
    ) {
        let Some(port) = self.stored_item_persistence_port_like_cpp() else {
            return;
        };
        let mut rows = Vec::new();
        for item in items {
            let template = self.item_storage_template(item.item_id);
            if !stored_loot_item_should_persist_like_cpp(
                template.is_some(),
                template
                    .map(|t| t.bag_family)
                    .unwrap_or(BagFamilyMask::NONE),
            ) {
                continue;
            }

            rows.push(wow_persistence::StoredItemLootPersistenceRowLikeCpp {
                item_id: item.item_id,
                count: item.quantity,
                item_index: u32::from(item.loot_list_id),
                follow_loot_rules: item.flags.follow_loot_rules,
                free_for_all: item.flags.freeforall,
                blocked: item.flags.blocked,
                counted: item.flags.counted,
                under_threshold: item.flags.under_threshold,
                needs_quest: item.flags.needs_quest,
                random_properties_id: item.random_properties_id,
                random_properties_seed: item.random_properties_seed,
                context: item.item_context,
            });
        }
        let outcome = port
            .save_stored_item_loot_like_cpp(wow_persistence::StoredItemLootSaveRequestLikeCpp {
                item_guid: item_guid.counter() as u64,
                money,
                items: rows,
            })
            .await;
        if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
        | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
        {
            warn!(
                item_guid = item_guid.counter(),
                money,
                error = %reason,
                "failed to save stored item loot rows"
            );
        }
    }
    /// Handle `CMSG_SPELL_CLICK`.
    ///
    /// C++ `WorldSession::HandleSpellClick` resolves an in-world creature, pet,
    /// or vehicle and then delegates to `Unit::HandleSpellClick`. Rust already
    /// represents the packet shape plus spellclick stores/conditions/visibility;
    /// executing spellclick casts, vehicle seat handling, and AI callbacks stays
    /// in the next bounded runtime slice.
    pub async fn handle_spell_click_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let spell_click = match SpellClick::read(&mut pkt) {
            Ok(spell_click) => spell_click,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to parse CMSG_SPELL_CLICK: {e}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            target = ?spell_click.unit_guid,
            try_auto_dismount = spell_click.try_auto_dismount,
            "CMSG_SPELL_CLICK"
        );

        let plan = self.represented_handle_spell_click_plan_like_cpp(spell_click.unit_guid);
        debug!(
            account = self.account_id,
            target = ?spell_click.unit_guid,
            casts = plan.casts.len(),
            exact_context_unrepresented = plan.exact_context_unrepresented,
            ai_on_spell_click_unrepresented = plan.ai_on_spell_click_unrepresented,
            "CMSG_SPELL_CLICK represented execution plan"
        );
        let outcome = self
            .execute_represented_spell_click_plan_with_generator_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                spell_click.unit_guid,
                &plan,
            )
            .await;
        debug!(
            account = self.account_id,
            target = ?spell_click.unit_guid,
            planned_casts = outcome.planned_casts,
            executed_casts = outcome.executed_casts,
            skipped_unrepresented_caster = outcome.skipped_unrepresented_caster,
            skipped_unrepresented_target = outcome.skipped_unrepresented_target,
            skipped_unrepresented_original_caster = outcome.skipped_unrepresented_original_caster,
            failed_casts = outcome.failed_casts,
            "CMSG_SPELL_CLICK represented execution outcome"
        );
    }
    #[cfg(test)]
    pub async fn handle_spell_click(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_spell_click_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            pkt,
        )
        .await;
    }
    /// Handle `CMSG_CANCEL_CAST` — player cancels an in-progress cast.
    pub async fn handle_cancel_cast(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CancelCast::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CancelCast parse failed: {error}"
                );
                return;
            }
        };

        self.cancel_client_cast_request_like_cpp(
            (request.spell_id != 0).then_some(request.spell_id as i32),
        );
    }
    /// Handle `CMSG_CANCEL_AURA` — player requests removing a cancelable owned aura.
    pub async fn handle_cancel_aura(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CancelAura::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CancelAura parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            spell_id = request.spell_id,
            caster_guid = ?request.caster_guid,
            "CMSG_CANCEL_AURA parsed"
        );
        let Some(spell_store) = self.spell_store() else {
            return;
        };
        if spell_store.get(request.spell_id).is_none()
            || spell_store.has_attribute0_like_cpp(
                request.spell_id,
                wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL,
            )
        {
            return;
        }
        if spell_store.is_channeled_like_cpp(request.spell_id) {
            self.interrupt_current_channeled_spell_like_cpp(request.spell_id);
            return;
        }
        if spell_store.is_passive_like_cpp(request.spell_id) {
            return;
        }
        self.remove_represented_cancelable_owned_aura_like_cpp(
            request.spell_id,
            request.caster_guid,
        );
    }
    /// Handle `CMSG_CANCEL_AUTO_REPEAT_SPELL`.
    pub async fn handle_cancel_auto_repeat_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = CancelAutoRepeatSpell::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "CancelAutoRepeatSpell parse failed: {error}"
            );
        }
        // C++ interrupts CURRENT_AUTOREPEAT_SPELL. Rust does not yet represent
        // a separate auto-repeat current-spell slot, so this remains silent.
    }
    /// Handle `CMSG_CANCEL_CHANNELLING` — player stops a channelled spell.
    pub async fn handle_cancel_channelling(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CancelChannelling::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CancelChannelling parse failed: {error}"
                );
                return;
            }
        };

        let Some(spell_store) = self.spell_store() else {
            return;
        };

        if spell_store.get(request.channel_spell).is_none()
            || spell_store.has_attribute0_like_cpp(
                request.channel_spell,
                wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL,
            )
        {
            return;
        }

        debug!(
            account = self.account_id,
            channel_spell = request.channel_spell,
            reason = request.reason,
            "CMSG_CANCEL_CHANNELLING parsed"
        );
        self.interrupt_current_channeled_spell_like_cpp(request.channel_spell);
    }
    /// Handle `CMSG_CANCEL_GROWTH_AURA`.
    pub async fn handle_cancel_growth_aura(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = CancelGrowthAura::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "CancelGrowthAura parse failed: {error}"
            );
        }
        self.remove_represented_growth_auras_cancelable_like_cpp();
    }
    /// Handle the represented `CMSG_CANCEL_MOD_SPEED_NO_CONTROL_AURAS`.
    ///
    /// The inspected opcode table assigns this packet to the shared unresolved
    /// `0xBADD` value, so `WorldSession` probes this handler from that branch
    /// and falls through to other 0xBADD packet shapes when the target does not
    /// match C++ `Player::GetUnitBeingMoved()`.
    pub async fn try_handle_cancel_mod_speed_no_control_auras_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) -> bool {
        let request = match CancelModSpeedNoControlAuras::read(&mut pkt) {
            Ok(request) if pkt.is_empty() => request,
            _ => return false,
        };
        if self.player_moved_unit_guid_like_cpp() != Some(request.target_guid) {
            return false;
        }

        self.remove_represented_mod_speed_no_control_auras_cancelable_like_cpp();
        true
    }
    /// Handle `CMSG_CANCEL_MOUNT_AURA`.
    pub async fn handle_cancel_mount_aura(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = CancelMountAura::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "CancelMountAura parse failed: {error}"
            );
        }
        self.remove_represented_mount_auras_cancelable_like_cpp();
    }
    /// Handle `CMSG_CANCEL_QUEUED_SPELL`.
    pub async fn handle_cancel_queued_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = CancelQueuedSpell::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "CancelQueuedSpell parse failed: {error}"
            );
            return;
        }
        // C++ cancels `Player::CancelPendingCastRequest`, not the current
        // non-melee spell. The represented queue is separate from
        // `active_spell_cast`, so this keeps casts already in progress alive.
        self.cancel_pending_spell_cast_request_like_cpp();
    }
    /// Handle `CMSG_SELF_RES`.
    pub async fn handle_self_res_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match SelfRes::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(account = self.account_id, "SelfRes parse failed: {error}");
                return;
            }
        };

        debug!(
            account = self.account_id,
            spell_id = request.spell_id,
            "CMSG_SELF_RES parsed"
        );
        if !self.has_represented_self_res_spell_like_cpp(request.spell_id) {
            return;
        }
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        // C++ `HandleSelfResOpcode` uses
        // `CastSpell(_player, SpellID, GetMap()->GetDifficultyID())`, whose
        // trigger flags are TRIGGERED_NONE: not a triggered cast, and the
        // global cooldown applies.
        if self
            .execute_server_triggered_spell_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                request.spell_id,
                player_guid,
                crate::session::SpellCastMetadata::default(),
            )
            .await
            .is_ok()
        {
            self.remove_represented_self_res_spell_like_cpp(request.spell_id);
        }
    }
    #[cfg(test)]
    pub async fn handle_self_res(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_self_res_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            pkt,
        )
        .await;
    }
    /// Handle `CMSG_PET_CANCEL_AURA`.
    pub async fn handle_pet_cancel_aura(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match PetCancelAura::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "PetCancelAura parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            pet_guid = ?request.pet_guid,
            spell_id = request.spell_id,
            "CMSG_PET_CANCEL_AURA parsed"
        );
        self.cancel_represented_pet_aura_like_cpp(request.pet_guid, request.spell_id);
    }
    /// Handle `CMSG_TOTEM_DESTROYED`.
    pub async fn handle_totem_destroyed(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match TotemDestroyed::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "TotemDestroyed parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            slot = request.slot,
            totem_guid = ?request.totem_guid,
            "CMSG_TOTEM_DESTROYED parsed"
        );
        self.destroy_represented_totem_like_cpp(request.slot, request.totem_guid);
    }
    pub(crate) fn is_spell_disabled_for_player_like_cpp(&self, spell_id: i32) -> bool {
        let Some(disable_mgr) = self.disable_mgr() else {
            return false;
        };

        let map_id = u32::from(self.player_map_id_like_cpp());
        let Some((_, area_id)) = self.player_zone_area_like_cpp() else {
            return true;
        };
        let map_instance_type = self
            .map_store()
            .and_then(|store| store.get(map_id))
            .map(|entry| entry.instance_type);

        disable_mgr.is_disabled_for_like_cpp(
            DISABLE_TYPE_SPELL,
            spell_id as u32,
            Some(DisableWorldObjectRefLikeCpp {
                type_id: TypeId::Player,
                map_id,
                area_id,
                is_pet: false,
                is_battle_arena: map_instance_type == Some(MAP_ARENA_LIKE_CPP),
                is_battleground: map_instance_type == Some(MAP_BATTLEGROUND_LIKE_CPP),
                player_map_difficulty: None,
            }),
            0,
            self.map_store().map(|store| store.as_ref()),
        )
    }
}
