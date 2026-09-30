//! Composition boundary for the Hotfix DB2 rows that hydrate core `SpellInfo`.

use anyhow::{Result, anyhow};
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    EffectiveCoreSpellDb2StoresLikeCpp,
    SpellAuraRestrictionsStore,
    SpellCastTimesStore,
    SpellCastingRequirementsStore,
    SpellCategoriesStore,
    SpellCategoryStore,
    SpellCooldownsStore,
    SpellDurationStore,
    SpellEffectDb2Store,
    SpellEquippedItemsStore,
    SpellInterruptsStore,
    SpellMiscStore,
    SpellNameEffectiveLoadReportLikeCpp,
    SpellNameStore,
    SpellPowerDifficultyStore,
    SpellPowerStore,
    SpellRadiusStore,
    SpellRangeStore,
    SpellShapeshiftStore,
    SpellStore,
    SpellTargetRestrictionsStore,
    SpellXSpellVisualStore,
};
use wow_persistence::{
    SpellCoreDb2HotfixLoadOutcomeLikeCpp,
    SpellCoreDb2HotfixPersistencePortLikeCpp,
};

mod row_projection;

use row_projection::{
    spell_name_entry_like_cpp,
    spell_categories_entry_like_cpp,
    spell_misc_entry_like_cpp,
    spell_effect_entry_like_cpp,
    spell_shapeshift_entry_like_cpp,
    spell_interrupts_entry_like_cpp,
    spell_cast_times_entry_like_cpp,
    spell_cooldowns_entry_like_cpp,
    spell_casting_requirements_entry_like_cpp,
    spell_power_entry_like_cpp,
    spell_power_difficulty_entry_like_cpp,
    spell_aura_restrictions_entry_like_cpp,
    spell_category_entry_like_cpp,
    spell_duration_entry_like_cpp,
    spell_radius_entry_like_cpp,
    spell_range_entry_like_cpp,
    spell_equipped_items_entry_like_cpp,
    spell_target_restrictions_entry_like_cpp,
    spell_x_spell_visual_entry_like_cpp,
};

fn loaded_rows_like_cpp<T>(outcome: SpellCoreDb2HotfixLoadOutcomeLikeCpp<T>) -> Result<Vec<T>> {
    match outcome {
        SpellCoreDb2HotfixLoadOutcomeLikeCpp::Loaded(rows) => Ok(rows),
        SpellCoreDb2HotfixLoadOutcomeLikeCpp::Failed { reason } => Err(anyhow!(reason)),
    }
}

pub(crate) async fn load_spell_name_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<(SpellNameStore, SpellNameEffectiveLoadReportLikeCpp)> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_name_rows_like_cpp().await)?;
    SpellNameStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_name_entry_like_cpp).collect(),
        removals,
    )
}

pub(crate) async fn load_spell_store_like_cpp(
    data_dir: &str,
    locale: &str,
    seed: SpellStore,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellStore> {
    let categories = loaded_rows_like_cpp(persistence.load_spell_categories_rows_like_cpp().await)?;
    let categories = SpellCategoriesStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        categories.into_iter().map(spell_categories_entry_like_cpp),
        removals,
    )?;
    let misc = loaded_rows_like_cpp(persistence.load_spell_misc_rows_like_cpp().await)?;
    let misc = SpellMiscStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        misc.into_iter().map(spell_misc_entry_like_cpp),
        removals,
    )?;
    let effect = loaded_rows_like_cpp(persistence.load_spell_effect_rows_like_cpp().await)?;
    let effect = SpellEffectDb2Store::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        effect.into_iter().map(spell_effect_entry_like_cpp),
        removals,
    )?;
    let shapeshift = loaded_rows_like_cpp(persistence.load_spell_shapeshift_rows_like_cpp().await)?;
    let shapeshift = SpellShapeshiftStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        shapeshift.into_iter().map(spell_shapeshift_entry_like_cpp),
        removals,
    )?;
    let interrupts = loaded_rows_like_cpp(persistence.load_spell_interrupts_rows_like_cpp().await)?;
    let interrupts = SpellInterruptsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        interrupts.into_iter().map(spell_interrupts_entry_like_cpp),
        removals,
    )?;
    let cast_times = loaded_rows_like_cpp(persistence.load_spell_cast_times_rows_like_cpp().await)?;
    let cast_times = SpellCastTimesStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        cast_times.into_iter().map(spell_cast_times_entry_like_cpp),
        removals,
    )?;
    let cooldowns = loaded_rows_like_cpp(persistence.load_spell_cooldowns_rows_like_cpp().await)?;
    let cooldowns = SpellCooldownsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        cooldowns.into_iter().map(spell_cooldowns_entry_like_cpp),
        removals,
    )?;
    let casting_requirements = loaded_rows_like_cpp(
        persistence
            .load_spell_casting_requirements_rows_like_cpp()
            .await,
    )?;
    let casting_requirements =
        SpellCastingRequirementsStore::load_effective_from_hotfix_rows_like_cpp(
            data_dir,
            locale,
            casting_requirements
                .into_iter()
                .map(spell_casting_requirements_entry_like_cpp),
            removals,
        )?;
    let power = loaded_rows_like_cpp(persistence.load_spell_power_rows_like_cpp().await)?;
    let power = SpellPowerStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        power.into_iter().map(spell_power_entry_like_cpp),
        removals,
    )?;
    let power_difficulty = loaded_rows_like_cpp(
        persistence
            .load_spell_power_difficulty_rows_like_cpp()
            .await,
    )?;
    let power_difficulty = SpellPowerDifficultyStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        power_difficulty
            .into_iter()
            .map(spell_power_difficulty_entry_like_cpp),
        removals,
    )?;

    Ok(
        seed.hydrate_effective_core_db2_like_cpp(EffectiveCoreSpellDb2StoresLikeCpp::new(
            categories,
            misc,
            effect,
            shapeshift,
            interrupts,
            cast_times,
            cooldowns,
            casting_requirements,
            power,
            power_difficulty,
        )),
    )
}

pub(crate) async fn load_spell_casting_requirements_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellCastingRequirementsStore> {
    let rows = loaded_rows_like_cpp(
        persistence
            .load_spell_casting_requirements_rows_like_cpp()
            .await,
    )?;
    SpellCastingRequirementsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter()
            .map(spell_casting_requirements_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_misc_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellMiscStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_misc_rows_like_cpp().await)?;
    SpellMiscStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_misc_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_cooldowns_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellCooldownsStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_cooldowns_rows_like_cpp().await)?;
    SpellCooldownsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_cooldowns_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_aura_restrictions_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellAuraRestrictionsStore> {
    let rows = loaded_rows_like_cpp(
        persistence
            .load_spell_aura_restrictions_rows_like_cpp()
            .await,
    )?;
    SpellAuraRestrictionsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_aura_restrictions_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_category_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellCategoryStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_category_rows_like_cpp().await)?;
    SpellCategoryStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_category_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_duration_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellDurationStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_duration_rows_like_cpp().await)?;
    SpellDurationStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_duration_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_radius_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellRadiusStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_radius_rows_like_cpp().await)?;
    SpellRadiusStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_radius_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_range_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellRangeStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_range_rows_like_cpp().await)?;
    SpellRangeStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_range_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_equipped_items_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellEquippedItemsStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_equipped_items_rows_like_cpp().await)?;
    SpellEquippedItemsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_equipped_items_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_target_restrictions_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellTargetRestrictionsStore> {
    let rows = loaded_rows_like_cpp(
        persistence
            .load_spell_target_restrictions_rows_like_cpp()
            .await,
    )?;
    SpellTargetRestrictionsStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter()
            .map(spell_target_restrictions_entry_like_cpp),
        removals,
    )
}

pub(crate) async fn load_spell_x_spell_visual_store_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellCoreDb2HotfixPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<SpellXSpellVisualStore> {
    let rows = loaded_rows_like_cpp(persistence.load_spell_x_spell_visual_rows_like_cpp().await)?;
    SpellXSpellVisualStore::load_effective_from_hotfix_rows_like_cpp(
        data_dir,
        locale,
        rows.into_iter().map(spell_x_spell_visual_entry_like_cpp),
        removals,
    )
}

#[cfg(test)]
mod tests;
