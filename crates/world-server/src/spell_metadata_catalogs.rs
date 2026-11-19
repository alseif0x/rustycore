//! Spell metadata and ability catalogs produced during ordered startup.

use super::*;

pub(super) struct SpellMetadataCatalogs {
    pub(super) spell_category_store: Arc<wow_data::SpellCategoryStore>,
    pub(super) spell_aura_options_store: Arc<wow_data::SpellAuraOptionsStore>,
    pub(super) spell_aura_restrictions_store: Arc<wow_data::SpellAuraRestrictionsStore>,
    pub(super) spell_casting_requirements_store: Arc<wow_data::SpellCastingRequirementsStore>,
    pub(super) spell_class_options_store: Arc<wow_data::SpellClassOptionsStore>,
    pub(super) spell_label_store: Arc<wow_data::SpellLabelStore>,
    pub(super) spell_equipped_items_store: Arc<wow_data::SpellEquippedItemsStore>,
    pub(super) spell_target_restrictions_store: Arc<wow_data::SpellTargetRestrictionsStore>,
    pub(super) spell_misc_store: Arc<wow_data::SpellMiscStore>,
    pub(super) spell_procs_per_minute_store: Arc<wow_data::SpellProcsPerMinuteStore>,
    pub(super) spell_duration_store: Arc<wow_data::SpellDurationStore>,
    pub(super) spell_cooldowns_store: Arc<wow_data::SpellCooldownsStore>,
}

pub(super) async fn load_spell_metadata_catalogs(
    data_dir: &str,
    locale: &str,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    spell_core_hotfix_persistence: &wow_database::MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp,
    creature_family_store: &wow_data::CreatureFamilyStore,
    skill_store: &wow_data::SkillStore,
    spell_levels_store: &wow_data::SpellLevelsStore,
    spell_store: &wow_data::SpellStore,
) -> Result<SpellMetadataCatalogs> {
    let spell_category_store = Arc::new(
        spell::core_db2_hotfix::load_spell_category_store_like_cpp(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellCategory authority")?,
    );
    info!(
        "Loaded {} spell categories from SpellCategory.db2",
        spell_category_store.len()
    );
    let spell_aura_options_store = Arc::new(
        spell::core_db2_hotfix::load_spell_aura_options_store(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellAuraOptions store")?,
    );
    info!(
        "Loaded {} effective spell aura options rows",
        spell_aura_options_store.len()
    );
    let spell_aura_restrictions_store = Arc::new(
        spell::core_db2_hotfix::load_spell_aura_restrictions_store_like_cpp(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellAuraRestrictions authority")?,
    );
    info!(
        "Loaded {} spell aura restriction rows",
        spell_aura_restrictions_store.len()
    );
    let spell_casting_requirements_store = Arc::new(
        spell::core_db2_hotfix::load_spell_casting_requirements_store_like_cpp(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellCastingRequirements authority")?,
    );
    info!(
        "Loaded {} spell casting requirement rows",
        spell_casting_requirements_store.len()
    );
    let spell_class_options_store = Arc::new(
        spell::core_db2_hotfix::load_spell_class_options_store(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellClassOptions store")?,
    );
    info!(
        "Loaded {} effective spell class options rows",
        spell_class_options_store.len()
    );
    // C++ `sSpellMgr` label authority, read by `SpellInfo::HasLabel`
    // (`SpellInfo.cpp`) for the damage-taken-from-caster-by-label term.
    let spell_label_store = Arc::new(
        spell::core_db2_hotfix::load_spell_label_store(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellLabel store")?,
    );
    info!(
        "Loaded {} effective spell label rows",
        spell_label_store.len()
    );
    let spell_equipped_items_store = Arc::new(
        spell::core_db2_hotfix::load_spell_equipped_items_store_like_cpp(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellEquippedItems authority")?,
    );
    info!(
        "Loaded {} spell equipped items rows",
        spell_equipped_items_store.len()
    );
    let spell_target_restrictions_store = Arc::new(
        spell::core_db2_hotfix::load_spell_target_restrictions_store_like_cpp(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellTargetRestrictions authority")?,
    );
    info!(
        "Loaded {} spell target restriction rows",
        spell_target_restrictions_store.len()
    );
    let spell_misc_store = Arc::new(
        spell::core_db2_hotfix::load_spell_misc_store_like_cpp(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellMisc authority")?,
    );
    info!(
        "Loaded {} effective spell misc rows",
        spell_misc_store.len()
    );
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
                is_passive: spell_misc_store.is_passive_like_cpp(spell_id),
            })
        },
    ));
    info!(
        "Loaded {} pet family passive spells for {} families",
        pet_family_spell_store.spell_count(),
        pet_family_spell_store.family_count()
    );
    let spell_procs_per_minute_store = Arc::new(
        spell::core_db2_hotfix::load_spell_procs_per_minute_store(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellProcsPerMinute store")?,
    );
    info!(
        "Loaded {} spell procs-per-minute rows",
        spell_procs_per_minute_store.len()
    );
    let spell_duration_store = Arc::new(
        spell::core_db2_hotfix::load_spell_duration_store_like_cpp(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellDuration authority")?,
    );
    info!("Loaded {} spell duration rows", spell_duration_store.len());
    let spell_cooldowns_store = Arc::new(
        spell::core_db2_hotfix::load_spell_cooldowns_store_like_cpp(
            &data_dir,
            &locale,
            spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellCooldowns authority")?,
    );
    info!("Loaded {} spell cooldown rows", spell_cooldowns_store.len());
    Ok(SpellMetadataCatalogs {
        spell_category_store,
        spell_aura_options_store,
        spell_aura_restrictions_store,
        spell_casting_requirements_store,
        spell_class_options_store,
        spell_label_store,
        spell_equipped_items_store,
        spell_target_restrictions_store,
        spell_misc_store,
        spell_procs_per_minute_store,
        spell_duration_store,
        spell_cooldowns_store,
    })
}
