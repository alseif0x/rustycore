use std::collections::HashMap;

use tracing::warn;
use wow_constants::BagFamilyMask;
use wow_core::ObjectGuid;
use wow_loot::{
    LootConditionRowLikeCpp, loot_condition_reference_ids_like_cpp,
    loot_condition_reference_self_references_like_cpp,
    loot_condition_row_normalize_without_external_stores_like_cpp,
};
use wow_packet::packets::loot::{LootEntry, LootEntryFlags};
use wow_world_core::session::HubRef;

use super::SessionLifecycleState;
use super::stored_item_loot_contracts::{
    LootTemplateRow, LootTemplateTable, WrappedGiftLoad, WrappedGiftRow,
    stored_item_row_can_load_like_cpp_representable, stored_loot_item_should_persist_like_cpp,
};

impl SessionLifecycleState {
    pub async fn load_wrapped_gift_row_like_cpp(
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

    pub async fn persist_wrapped_gift_open_like_cpp(
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

    pub async fn load_loot_template_rows_like_cpp(
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

    pub async fn load_loot_template_condition_rows_like_cpp(
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

    pub async fn load_loot_template_condition_reference_rows_like_cpp(
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

    pub async fn load_loot_template_condition_reference_rows_for_id_like_cpp(
        &self,
        reference_id: u32,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Ok(reference_source_type) = i32::try_from(reference_id).map(|id| -id) else {
            return Vec::new();
        };

        self.load_loot_template_condition_rows_like_cpp(reference_source_type, 0, 0)
            .await
    }

    pub async fn load_stored_item_money_like_cpp(
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

    pub async fn load_stored_item_items_like_cpp(
        &self,
        hub: HubRef<'_>,
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
                hub.catalogs.item_storage_template(row.item_id).is_some(),
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

    pub async fn save_new_stored_item_loot_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_guid: wow_core::ObjectGuid,
        money: u32,
        items: &[LootEntry],
    ) {
        let Some(port) = self.stored_item_persistence_port_like_cpp() else {
            return;
        };
        let mut rows = Vec::new();
        for item in items {
            let template = hub.catalogs.item_storage_template(item.item_id);
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
}
