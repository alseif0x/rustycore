use anyhow::{Context, Result};
use std::sync::Arc;
use tracing::info;

pub(super) struct InventoryBaseCatalogs {
    pub(super) item_set_spell_store: Arc<wow_data::ItemSetSpellStore>,
    pub(super) item_set_store: Arc<wow_data::ItemSetStore>,
    pub(super) pvp_item_store: Arc<wow_data::PvpItemStore>,
    pub(super) item_bonus_db2_store: Arc<wow_data::ItemBonusDb2Store>,
    pub(super) item_limit_category_condition_store: Arc<wow_data::ItemLimitCategoryConditionStore>,
    pub(super) item_limit_category_store: Arc<wow_data::ItemLimitCategoryStore>,
    pub(super) item_price_base_store: Arc<wow_data::ItemPriceBaseStore>,
    pub(super) item_child_equipment_store: Arc<wow_data::ItemChildEquipmentStore>,
    pub(super) item_store: Arc<wow_data::ItemStore>,
    pub(super) item_extended_cost_store: Arc<wow_data::ItemExtendedCostStore>,
    pub(super) item_currency_cost_store: Arc<wow_data::ItemCurrencyCostStore>,
    pub(super) item_class_store: Arc<wow_data::ItemClassStore>,
    pub(super) bank_bag_slot_prices_store: Arc<wow_data::BankBagSlotPricesStore>,
    pub(super) import_price_stores: Arc<wow_data::ImportPriceStores>,
    pub(super) currency_types_store: Arc<wow_data::CurrencyTypesStore>,
}

pub(super) fn load(
    data_dir: &str,
    locale: &str,
) -> Result<InventoryBaseCatalogs> {
    let currency_types_store = Arc::new(
        wow_data::CurrencyTypesStore::load(&data_dir, &locale)
            .context("Failed to load CurrencyTypes.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} currencies from CurrencyTypes.db2",
        currency_types_store.len()
    );

    let import_price_stores = Arc::new(
        wow_data::ImportPriceStores::load(&data_dir, &locale)
            .context("Failed to load ImportPrice*.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded ImportPrice*.db2 stores");

    let bank_bag_slot_prices_store = Arc::new(
        wow_data::BankBagSlotPricesStore::load(&data_dir, &locale).context(
            "Failed to load BankBagSlotPrices.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} bank bag slot price rows from BankBagSlotPrices.db2",
        bank_bag_slot_prices_store.len()
    );

    let item_class_store = Arc::new(
        wow_data::ItemClassStore::load(&data_dir, &locale)
            .context("Failed to load ItemClass.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item classes from ItemClass.db2",
        item_class_store.len()
    );

    let item_currency_cost_store = Arc::new(
        wow_data::ItemCurrencyCostStore::load(&data_dir, &locale)
            .context("Failed to load ItemCurrencyCost.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item currency costs from ItemCurrencyCost.db2",
        item_currency_cost_store.len()
    );

    let item_extended_cost_store = Arc::new(
        wow_data::ItemExtendedCostStore::load(&data_dir, &locale)
            .context("Failed to load ItemExtendedCost.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item extended costs from ItemExtendedCost.db2",
        item_extended_cost_store.len()
    );

    let item_store = Arc::new(
        wow_data::ItemStore::load(&data_dir, &locale)
            .context("Failed to load Item.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} items from Item.db2", item_store.len());
    let item_child_equipment_store = Arc::new(
        wow_data::ItemChildEquipmentStore::load(&data_dir, &locale).context(
            "Failed to load ItemChildEquipment.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item child-equipment rows from ItemChildEquipment.db2",
        item_child_equipment_store.len()
    );

    let item_price_base_store = Arc::new(
        wow_data::ItemPriceBaseStore::load(&data_dir, &locale)
            .context("Failed to load ItemPriceBase.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item price base rows from ItemPriceBase.db2",
        item_price_base_store.len()
    );

    let item_limit_category_store = Arc::new(
        wow_data::ItemLimitCategoryStore::load(&data_dir, &locale).context(
            "Failed to load ItemLimitCategory.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item limit categories from ItemLimitCategory.db2",
        item_limit_category_store.len()
    );

    let item_limit_category_condition_store = Arc::new(
        wow_data::ItemLimitCategoryConditionStore::load(&data_dir, &locale).context(
            "Failed to load ItemLimitCategoryCondition.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item limit category conditions from ItemLimitCategoryCondition.db2",
        item_limit_category_condition_store.len()
    );

    let item_bonus_db2_store = Arc::new(
        wow_data::ItemBonusDb2Store::load(&data_dir, &locale)
            .context("Failed to load ItemBonus.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item bonus rows from ItemBonus.db2",
        item_bonus_db2_store.len()
    );
    let pvp_item_store = Arc::new(
        wow_data::PvpItemStore::load(&data_dir, &locale)
            .context("Failed to load PVPItem.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} PvP item bonus rows from PVPItem.db2",
        pvp_item_store.len()
    );
    let item_set_store = Arc::new(
        wow_data::ItemSetStore::load(&data_dir, &locale)
            .context("Failed to load ItemSet.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item set rows from ItemSet.db2",
        item_set_store.len()
    );
    let item_set_spell_store = Arc::new(
        wow_data::ItemSetSpellStore::load(&data_dir, &locale)
            .context("Failed to load ItemSetSpell.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item set spell rows from ItemSetSpell.db2",
        item_set_spell_store.len()
    );

    Ok(InventoryBaseCatalogs {
        currency_types_store,
        import_price_stores,
        bank_bag_slot_prices_store,
        item_class_store,
        item_currency_cost_store,
        item_extended_cost_store,
        item_store,
        item_child_equipment_store,
        item_price_base_store,
        item_limit_category_store,
        item_limit_category_condition_store,
        item_bonus_db2_store,
        pvp_item_store,
        item_set_store,
        item_set_spell_store,
    })
}
