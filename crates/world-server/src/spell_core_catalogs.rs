//! Core SpellInfo and pet spell catalogs produced during ordered startup.

use super::*;

pub(super) struct SpellCoreCatalogs {
    pub(super) spell_core_hotfix_persistence:
        wow_database::MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp,
    pub(super) spell_levels_store: Arc<wow_data::SpellLevelsStore>,
    pub(super) spell_acquisition_startup_persistence:
        wow_database::MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp,
    pub(super) spell_name_store: wow_data::SpellNameStore,
    pub(super) spell_store: wow_data::SpellStore,
}

pub(super) async fn load_spell_core_catalogs(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<HotfixDatabase>,
    world_db: &Arc<WorldDatabase>,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    creature_family_store: &wow_data::CreatureFamilyStore,
    skill_store: &wow_data::SkillStore,
    creature_template_lifecycle_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
) -> Result<SpellCoreCatalogs> {
    let spell_core_hotfix_persistence =
        wow_database::MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            hotfix_db,
        ));
    let spell_levels_store = Arc::new(
        spell::core_db2_hotfix::load_spell_levels_store(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellLevels store")?,
    );
    info!(
        "Loaded {} effective spell level rows",
        spell_levels_store.len()
    );
    let spell_acquisition_startup_persistence =
        wow_database::MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp::new(
            Arc::clone(hotfix_db),
            Arc::clone(world_db),
        );
    let (spell_name_store, spell_name_load_report) =
        spell::core_db2_hotfix::load_spell_name_store_like_cpp(
            &data_dir,
            &locale,
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
        &data_dir,
        &locale,
        &spell_info_key_hotfix_persistence,
        &spell_name_store,
        db2_hotfix_removals,
    )
    .await
    .context("Failed to load SpellInfo key authority")?;
    let spell_store = spell::core_db2_hotfix::load_spell_store_like_cpp(
        &data_dir,
        &locale,
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
    Ok(SpellCoreCatalogs {
        spell_core_hotfix_persistence,
        spell_levels_store,
        spell_acquisition_startup_persistence,
        spell_name_store,
        spell_store,
    })
}
