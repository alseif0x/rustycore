//! Effective SpellInfo and pet spell progression startup.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;

use super::spell_info_startup;
use crate::spell;

pub(super) struct SpellPetStartup {
    pub(super) spell_name_load_report: wow_data::SpellNameEffectiveLoadReportLikeCpp,
    pub(super) spell_info_key_hotfix_persistence:
        wow_database::MariaDbSpellInfoKeyHotfixPersistenceAdapterLikeCpp,
    pub(super) spell_acquisition_startup_persistence:
        wow_database::MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp,
    pub(super) spell_core_hotfix_persistence:
        wow_database::MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp,
    pub(super) spell_store: wow_data::SpellStore,
    pub(super) spell_name_store: wow_data::SpellNameStore,
    pub(super) spell_info: spell_info_startup::SpellInfoStartup,
    pub(super) spell_shapeshift_form_store: Arc<wow_data::SpellShapeshiftFormStore>,
    pub(super) spell_cooldowns_store: Arc<wow_data::SpellCooldownsStore>,
    pub(super) spell_duration_store: Arc<wow_data::SpellDurationStore>,
    pub(super) spell_procs_per_minute_store: Arc<wow_data::SpellProcsPerMinuteStore>,
    pub(super) pet_family_spell_store: Arc<wow_data::PetFamilySpellStoreLikeCpp>,
    pub(super) pet_default_spell_store: Arc<wow_data::PetDefaultSpellStoreLikeCpp>,
    pub(super) pet_levelup_spell_store: Arc<wow_data::PetLevelupSpellStoreLikeCpp>,
    pub(super) spell_levels_store: Arc<wow_data::SpellLevelsStore>,
    pub(super) creature_family_store: Arc<wow_data::CreatureFamilyStore>,
}

pub(super) async fn load_spell_pet_startup(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<wow_database::HotfixDatabase>,
    world_db: &Arc<wow_database::WorldDatabase>,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    skill_store: &wow_data::SkillStore,
    creature_template_lifecycle_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
) -> anyhow::Result<SpellPetStartup> {
    let creature_family_store = Arc::new(
        wow_data::CreatureFamilyStore::load(data_dir, locale)
            .context("Failed to load CreatureFamily.db2")?,
    );
    info!(
        "Loaded {} creature family rows from CreatureFamily.db2",
        creature_family_store.len()
    );
    let spell_levels_store = Arc::new(
        wow_data::SpellLevelsStore::load(data_dir, locale)
            .context("Failed to load SpellLevels.db2")?,
    );
    info!(
        "Loaded {} spell level rows from SpellLevels.db2",
        spell_levels_store.len()
    );
    let spell_core_hotfix_persistence =
        wow_database::MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            hotfix_db,
        ));
    let spell_acquisition_startup_persistence =
        wow_database::MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp::new(
            Arc::clone(hotfix_db),
            Arc::clone(world_db),
        );
    let (spell_name_store, spell_name_load_report) =
        spell::core_db2_hotfix::load_spell_name_store_like_cpp(
            data_dir,
            locale,
            &spell_core_hotfix_persistence,
            db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellName store")?;
    info!(
        "Loaded {} effective SpellName rows ({} SQL overlay rows; {} removed rows; {} final DB2 removals total)",
        spell_name_store.len(),
        spell_name_load_report.overlay_rows,
        spell_name_load_report.removed_rows,
        db2_hotfix_removals.len()
    );
    let spell_info_key_hotfix_persistence =
        wow_database::MariaDbSpellInfoKeyHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            hotfix_db,
        ));
    let spell_store_seed = spell::info_key_hotfix::load_spell_store_seed_like_cpp(
        data_dir,
        locale,
        &spell_info_key_hotfix_persistence,
        &spell_name_store,
        db2_hotfix_removals,
    )
    .await
    .context("Failed to load SpellInfo key authority")?;
    let spell_store = spell::core_db2_hotfix::load_spell_store_like_cpp(
        data_dir,
        locale,
        spell_store_seed,
        &spell_core_hotfix_persistence,
        db2_hotfix_removals,
    )
    .await
    .context("Failed to load SpellStore")?;
    info!(
        "Loaded {} hydrated spells and {} exact regular SpellInfo keys from SpellStore",
        spell_store.len(),
        spell_store.spell_info_key_count_like_cpp()
    );
    let pet_levelup_spell_store = Arc::new(wow_data::PetLevelupSpellStoreLikeCpp::load_like_cpp(
        creature_family_store.entries_like_cpp(),
        skill_store,
        |spell_id| {
            let spell = spell_store.get(spell_id)?;
            let spell_id = u32::try_from(spell.spell_id).ok()?;
            let spell_level = spell_levels_store
                .entry_for_spell_difficulty_like_cpp(spell_id, 0)
                .map(|entry| u32::try_from(entry.spell_level).unwrap_or(0))
                .unwrap_or(0);

            Some(wow_data::PetLevelupSpellInfoLikeCpp {
                id: spell_id,
                spell_level,
            })
        },
    ));
    info!(
        "Loaded {} pet levelup spells for {} families",
        pet_levelup_spell_store.count(),
        pet_levelup_spell_store.family_count()
    );
    let pet_default_spell_store = Arc::new(wow_data::PetDefaultSpellStoreLikeCpp::load_like_cpp(
        spell_store
            .iter()
            .map(|spell| wow_data::PetDefaultSpellInfoLikeCpp {
                difficulty_none: true,
                effects: spell
                    .effects()
                    .iter()
                    .map(|effect| wow_data::PetDefaultSpellEffectLikeCpp {
                        effect: effect.effect,
                        misc_value: effect.effect_misc_value_1,
                    })
                    .collect(),
            }),
        creature_template_lifecycle_store
            .entries_like_cpp()
            .map(|template| {
                let mut spells = [0; wow_data::MAX_CREATURE_SPELL_DATA_SLOT_LIKE_CPP];
                spells.copy_from_slice(
                    &template.spells[..wow_data::MAX_CREATURE_SPELL_DATA_SLOT_LIKE_CPP],
                );
                wow_data::PetDefaultSpellCreatureTemplateLikeCpp {
                    entry: template.entry,
                    family: template.family,
                    spells,
                }
            }),
        pet_levelup_spell_store.as_ref(),
    ));
    info!(
        "Loaded {} summonable creature default spell templates",
        pet_default_spell_store.count()
    );
    let spell_info = spell_info_startup::load_spell_info_startup(
        data_dir,
        locale,
        &spell_core_hotfix_persistence,
        db2_hotfix_removals,
    )
    .await?;
    let pet_family_spell_store = Arc::new(wow_data::PetFamilySpellStoreLikeCpp::load_like_cpp(
        skill_store,
        creature_family_store.entries_like_cpp(),
        spell_levels_store
            .entries_like_cpp()
            .map(|entry| wow_data::PetFamilySpellLevelLikeCpp {
                spell_id: i32::try_from(entry.spell_id).unwrap_or(0),
                difficulty_id: u32::from(entry.difficulty_id),
                spell_level: entry.spell_level,
            }),
        |spell_id| {
            let spell = spell_store.get(spell_id)?;
            let spell_id = u32::try_from(spell.spell_id).ok()?;
            Some(wow_data::PetFamilySpellInfoLikeCpp {
                id: spell_id,
                is_passive: spell_info.spell_misc_store.is_passive_like_cpp(spell_id),
            })
        },
    ));
    info!(
        "Loaded {} pet family passive spells for {} families",
        pet_family_spell_store.spell_count(),
        pet_family_spell_store.family_count()
    );
    let spell_procs_per_minute_store = Arc::new(
        wow_data::SpellProcsPerMinuteStore::load(data_dir, locale)
            .context("Failed to load SpellProcsPerMinute.db2")?,
    );
    info!(
        "Loaded {} spell procs-per-minute rows",
        spell_procs_per_minute_store.len()
    );
    let spell_duration_store = Arc::new(
        spell::core_db2_hotfix::load_spell_duration_store_like_cpp(
            data_dir,
            locale,
            &spell_core_hotfix_persistence,
            db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellDuration authority")?,
    );
    info!("Loaded {} spell duration rows", spell_duration_store.len());
    let spell_cooldowns_store = Arc::new(
        spell::core_db2_hotfix::load_spell_cooldowns_store_like_cpp(
            data_dir,
            locale,
            &spell_core_hotfix_persistence,
            db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellCooldowns authority")?,
    );
    info!("Loaded {} spell cooldown rows", spell_cooldowns_store.len());
    let spell_shapeshift_form_store = Arc::new(
        wow_data::SpellShapeshiftFormStore::load(data_dir, locale)
            .context("Failed to load SpellShapeshiftForm.db2")?,
    );
    info!(
        "Loaded {} spell shapeshift form rows",
        spell_shapeshift_form_store.len()
    );
    Ok(SpellPetStartup {
        creature_family_store,
        spell_levels_store,
        pet_levelup_spell_store,
        pet_default_spell_store,
        pet_family_spell_store,
        spell_procs_per_minute_store,
        spell_duration_store,
        spell_cooldowns_store,
        spell_shapeshift_form_store,
        spell_info,
        spell_name_store,
        spell_store,
        spell_core_hotfix_persistence,
        spell_acquisition_startup_persistence,
        spell_info_key_hotfix_persistence,
        spell_name_load_report,
    })
}
