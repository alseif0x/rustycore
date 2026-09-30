//! Ordered jump charge startup composition.

use crate::{catalogs, spell};
use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct JumpChargeCatalogs {
    pub(super) jump_charge_report: wow_data::JumpChargeParamsLoadReportLikeCpp,
    pub(super) _jump_charge_params_store: Arc<wow_data::JumpChargeParamsStoreLikeCpp>,
    pub(super) jump_charge_persistence:
        wow_database::MariaDbJumpChargeCatalogPersistenceAdapterLikeCpp,
    pub(super) spell_x_spell_visual_store: Arc<wow_data::SpellXSpellVisualStore>,
    pub(super) spell_visual_store: wow_data::SpellVisualStore,
}

pub(super) async fn load(
    data_dir: &str,
    locale: &str,
    world_db: &Arc<wow_database::WorldDatabase>,
    spell_core_hotfix_persistence: &dyn wow_persistence::SpellCoreDb2HotfixPersistencePortLikeCpp,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    curve_store: &wow_data::progression_rewards::CurveStore,
) -> anyhow::Result<JumpChargeCatalogs> {
    let spell_visual_store = wow_data::SpellVisualStore::load(data_dir, locale)
        .context("Failed to load SpellVisual.db2 for C++ jump_charge_params validation")?;
    let spell_x_spell_visual_store = Arc::new(
        spell::core_db2_hotfix::load_spell_x_spell_visual_store_like_cpp(
            data_dir,
            locale,
            spell_core_hotfix_persistence,
            db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellXSpellVisual authority for creature casts")?,
    );
    let jump_charge_persistence =
        wow_database::MariaDbJumpChargeCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let jump_charge_params_outcome = catalogs::jump_charge::load_jump_charge_catalog_like_cpp(
        &jump_charge_persistence,
        |id| spell_visual_store.get(id).is_some(),
        |id| curve_store.get(id).is_some(),
    )
    .await
    .context("Failed to load C++ jump_charge_params rows")?;
    for (id, speed) in &jump_charge_params_outcome.report.corrected_invalid_speeds {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` has invalid speed {} for id {}, using default {}",
            speed,
            id,
            wow_data::SPEED_CHARGE_LIKE_CPP
        );
    }
    for (id, gravity) in &jump_charge_params_outcome
        .report
        .corrected_invalid_jump_gravities
    {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` has invalid jumpGravity {} for id {}, using default {}",
            gravity,
            id,
            wow_data::MOVEMENT_GRAVITY_LIKE_CPP
        );
    }
    for (id, spell_visual_id) in &jump_charge_params_outcome
        .report
        .ignored_missing_spell_visuals
    {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` references non-existing SpellVisual {} for id {}, ignored",
            spell_visual_id,
            id
        );
    }
    for (id, progress_curve_id, cpp_logged_spell_visual_id) in &jump_charge_params_outcome
        .report
        .ignored_missing_progress_curves
    {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` references non-existing progress Curve {} for id {}, ignored (C++ log typo would print SpellVisual {:?})",
            progress_curve_id,
            id,
            cpp_logged_spell_visual_id
        );
    }
    for (id, parabolic_curve_id) in &jump_charge_params_outcome
        .report
        .ignored_missing_parabolic_curves
    {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` references non-existing parabolic Curve {} for id {}, ignored",
            parabolic_curve_id,
            id
        );
    }
    info!(
        "Loaded {} C++ jump charge params from {} rows ({} defaults applied, {} invalid optional refs ignored; live EffectJumpCharge consumption pending)",
        jump_charge_params_outcome.report.loaded_params,
        jump_charge_params_outcome.report.rows_seen,
        jump_charge_params_outcome
            .report
            .corrected_invalid_speeds
            .len()
            + jump_charge_params_outcome
                .report
                .corrected_invalid_jump_gravities
                .len(),
        jump_charge_params_outcome
            .report
            .ignored_missing_spell_visuals
            .len()
            + jump_charge_params_outcome
                .report
                .ignored_missing_progress_curves
                .len()
            + jump_charge_params_outcome
                .report
                .ignored_missing_parabolic_curves
                .len()
    );
    let _jump_charge_params_store = Arc::new(jump_charge_params_outcome.store);
    Ok(JumpChargeCatalogs {
        spell_visual_store,
        spell_x_spell_visual_store,
        jump_charge_persistence,
        _jump_charge_params_store,
        jump_charge_report: jump_charge_params_outcome.report,
    })
}
