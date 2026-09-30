//! Loot template and loot-condition startup loading.

use anyhow::{Context, Result};
use tracing::info;
use wow_database::{WorldDatabase, WorldStatements};
use wow_loot::{
    LootConditionId, LootConditionLinkReport, LootConditionReferenceUseLikeCpp,
    LootReferenceCheckReport, LootStore, LootStoreKind, LootStores, LootTemplateRow,
};

pub(crate) async fn load_loot_stores_like_cpp(
    world_db: &WorldDatabase,
    item_store: &wow_data::ItemStore,
) -> Result<LootStores> {
    let mut stores = LootStores::new();

    for kind in LootStoreKind::ALL_LIKE_CPP {
        let rows = load_loot_template_rows_like_cpp(world_db, kind).await?;
        let mut store = LootStore::for_kind_like_cpp(kind);
        let accepted = store
            .load_rows_like_cpp(rows, |item_id| item_store.get(item_id).is_some())
            .map_err(|err| anyhow::anyhow!("invalid loot row in {:?}: {:?}", kind, err))?;
        info!(
            table = store.definition().table_name,
            entry_name = store.definition().entry_name,
            rates_allowed = store.definition().rates_allowed,
            accepted_rows = accepted,
            template_ids = store.templates().len(),
            "Loaded C++ loot template store foundation"
        );
        stores.insert(kind, store);
    }

    Ok(stores)
}

pub(crate) fn log_loot_reference_report_like_cpp(report: &LootReferenceCheckReport) {
    if report.is_clean() {
        info!("C++ loot reference verification completed with no gaps");
        return;
    }

    for reference_use in &report.missing_references {
        let store_definition = reference_use.store_kind.definition_like_cpp();
        tracing::warn!(
            table = store_definition.table_name,
            entry = reference_use.entry,
            item_id = reference_use.item_id,
            reference = reference_use.reference,
            "C++ loot reference verification found missing reference_loot_template entry"
        );
    }

    for reference_id in &report.unused_reference_ids {
        tracing::warn!(
            table = LootStoreKind::Reference.definition_like_cpp().table_name,
            entry = *reference_id,
            "C++ loot reference verification found unused reference_loot_template entry"
        );
    }
}

pub(crate) fn log_loot_condition_link_report_like_cpp(report: &LootConditionLinkReport) {
    if report.is_clean() {
        info!(
            linked_conditions = report.linked,
            "C++ loot condition structural linking completed with no gaps"
        );
        return;
    }

    for condition_id in &report.unsupported_source_types {
        tracing::warn!(
            source_type = condition_id.source_type,
            source_group = condition_id.source_group,
            source_entry = condition_id.source_entry,
            "C++ loot condition structural linking found unsupported loot condition source type"
        );
    }

    for missing in &report.missing_templates {
        let store_definition = missing.store_kind.definition_like_cpp();
        tracing::warn!(
            table = store_definition.table_name,
            source_type = missing.condition_id.source_type,
            source_group = missing.condition_id.source_group,
            source_entry = missing.condition_id.source_entry,
            "C++ loot condition structural linking found missing loot template"
        );
    }

    for missing in &report.missing_item_templates {
        let store_definition = missing.store_kind.definition_like_cpp();
        tracing::warn!(
            table = store_definition.table_name,
            source_type = missing.condition_id.source_type,
            source_group = missing.condition_id.source_group,
            source_entry = missing.condition_id.source_entry,
            "C++ loot condition structural linking found missing item template for SourceEntry"
        );
    }

    for missing in &report.missing_template_items {
        let store_definition = missing.store_kind.definition_like_cpp();
        tracing::warn!(
            table = store_definition.table_name,
            source_type = missing.condition_id.source_type,
            source_group = missing.condition_id.source_group,
            source_entry = missing.condition_id.source_entry,
            "C++ loot condition structural linking found SourceEntry absent from loot template"
        );
    }

    for missing in &report.missing_reference_templates {
        tracing::warn!(
            source_type = missing.condition_id.source_type,
            source_group = missing.condition_id.source_group,
            source_entry = missing.condition_id.source_entry,
            reference_id = missing.reference_id,
            "C++ loot condition structural linking found missing condition reference template"
        );
    }
}

pub(crate) async fn load_loot_condition_ids_like_cpp(
    world_db: &WorldDatabase,
) -> Result<Vec<LootConditionId>> {
    let stmt = world_db.prepare(WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_IDS);
    let mut result = world_db.query(&stmt).await?;
    let mut condition_ids = Vec::new();

    if result.is_empty() {
        return Ok(condition_ids);
    }

    loop {
        condition_ids.push(LootConditionId {
            source_type: result.try_read::<i32>(0).unwrap_or(0),
            source_group: result.try_read::<u32>(1).unwrap_or(0),
            source_entry: result.try_read::<u32>(2).unwrap_or(0),
        });

        if !result.next_row() {
            break;
        }
    }

    Ok(condition_ids)
}

pub(crate) async fn load_loot_condition_reference_uses_like_cpp(
    world_db: &WorldDatabase,
) -> Result<Vec<LootConditionReferenceUseLikeCpp>> {
    let stmt = world_db.prepare(WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_REFERENCE_USES);
    let mut result = world_db.query(&stmt).await?;
    let mut reference_uses = Vec::new();

    if result.is_empty() {
        return Ok(reference_uses);
    }

    loop {
        reference_uses.push(LootConditionReferenceUseLikeCpp {
            condition_id: LootConditionId {
                source_type: result.try_read::<i32>(0).unwrap_or(0),
                source_group: result.try_read::<u32>(1).unwrap_or(0),
                source_entry: result.try_read::<u32>(2).unwrap_or(0),
            },
            reference_id: result.try_read::<u32>(3).unwrap_or(0),
        });

        if !result.next_row() {
            break;
        }
    }

    Ok(reference_uses)
}

pub(crate) async fn load_condition_reference_template_ids_like_cpp(
    world_db: &WorldDatabase,
) -> Result<Vec<u32>> {
    let stmt = world_db.prepare(WorldStatements::SEL_CONDITION_REFERENCE_TEMPLATE_IDS);
    let mut result = world_db.query(&stmt).await?;
    let mut template_ids = Vec::new();

    if result.is_empty() {
        return Ok(template_ids);
    }

    loop {
        template_ids.push(result.try_read::<u32>(0).unwrap_or(0));

        if !result.next_row() {
            break;
        }
    }

    Ok(template_ids)
}

pub(crate) async fn load_loot_template_rows_like_cpp(
    world_db: &WorldDatabase,
    kind: LootStoreKind,
) -> Result<Vec<LootTemplateRow>> {
    let statement = loot_store_all_rows_statement_like_cpp(kind);
    let stmt = world_db.prepare(statement);
    let mut result = world_db.query(&stmt).await?;
    let mut rows = Vec::new();

    if result.is_empty() {
        return Ok(rows);
    }

    loop {
        // Trinity's QuestRequired column is a signed TINYINT(1). Reading it
        // as u8 makes sqlx reject the signed MySQL type; defaulting that
        // decode failure to zero turns every quest-only drop into normal
        // loot. Fail startup instead of silently disabling the quest gate.
        let quest_required = result
            .try_read::<i8>(4)
            .with_context(|| format!("failed to decode {kind:?} loot QuestRequired as TINYINT"))?;
        rows.push(LootTemplateRow {
            entry: result.try_read::<u32>(0).unwrap_or(0),
            item: wow_loot::LootStoreItem {
                item_id: result.try_read::<u32>(1).unwrap_or(0),
                reference: result.try_read::<u32>(2).unwrap_or(0),
                chance: result.try_read::<f32>(3).unwrap_or(0.0),
                needs_quest: loot_quest_required_from_signed_db_like_cpp(quest_required),
                loot_mode: result.try_read::<u16>(5).unwrap_or(0),
                group_id: result.try_read::<u8>(6).unwrap_or(0),
                min_count: result.try_read::<u8>(7).unwrap_or(0),
                max_count: result.try_read::<u8>(8).unwrap_or(0),
            },
        });

        if !result.next_row() {
            break;
        }
    }

    Ok(rows)
}

pub(crate) const fn loot_quest_required_from_signed_db_like_cpp(value: i8) -> bool {
    value != 0
}

pub(crate) fn loot_store_all_rows_statement_like_cpp(kind: LootStoreKind) -> WorldStatements {
    match kind {
        LootStoreKind::Creature => WorldStatements::SEL_CREATURE_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Disenchant => WorldStatements::SEL_DISENCHANT_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Fishing => WorldStatements::SEL_FISHING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Gameobject => WorldStatements::SEL_GAMEOBJECT_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Item => WorldStatements::SEL_ITEM_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Mail => WorldStatements::SEL_MAIL_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Milling => WorldStatements::SEL_MILLING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Pickpocketing => WorldStatements::SEL_PICKPOCKETING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Prospecting => WorldStatements::SEL_PROSPECTING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Reference => WorldStatements::SEL_REFERENCE_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Skinning => WorldStatements::SEL_SKINNING_LOOT_TEMPLATE_ALL_ROWS,
        LootStoreKind::Spell => WorldStatements::SEL_SPELL_LOOT_TEMPLATE_ALL_ROWS,
    }
}
