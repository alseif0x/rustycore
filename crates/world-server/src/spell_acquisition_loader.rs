// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Effective spell-acquisition bootstrap.
//!
//! This keeps the dependency-sensitive part of `SpellMgr` startup together:
//! the compact effective metadata catalog, SQL custom attributes, rank chains,
//! learn-skill nodes, and learn-spell edges. The order mirrors
//! `World.cpp::SetInitialWorldSettings` and the projections mirror
//! `SpellMgr.cpp::LoadSpellRanks`, `LoadSpellLearnSkills`, and
//! `LoadSpellLearnSpells`.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

use anyhow::{Context, Result};
use tracing::info;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp, DifficultyStore, SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
    ServersideSpellStoreLikeCpp, SkillStore, SpellAcquisitionCatalogLikeCpp,
    SpellAcquisitionCoverageSeedLikeCpp, SpellAcquisitionDiagnosticSeverityLikeCpp,
    SpellAcquisitionIndeterminateReasonLikeCpp, SpellAcquisitionTalentLookupLikeCpp,
    SpellAuraRestrictionsEntry, SpellAuraRestrictionsStore, SpellChainStoreLikeCpp,
    SpellCustomAttributeKeyLikeCpp, SpellCustomAttributeLoadErrorKindLikeCpp,
    SpellCustomAttributeSourceVariantLikeCpp, SpellCustomAttributeStoreLikeCpp,
    SpellEquippedItemsEntry, SpellEquippedItemsStore, SpellLearnSkillEffectLikeCpp,
    SpellLearnSkillIndeterminateReasonLikeCpp, SpellLearnSkillSourceSpellInfoLikeCpp,
    SpellLearnSkillStoreLikeCpp, SpellLearnSourceSpellInfoLikeCpp, SpellLearnSpellEffectLikeCpp,
    SpellLearnSpellEntry, SpellLearnSpellStoreLikeCpp, SpellLinkedStoreLikeCpp,
    SpellLinkedTypeLikeCpp, SpellPetAuraStoreLikeCpp, SpellReagentsEntry, SpellReagentsStore,
    SpellStore, wdc4::Wdc4Reader,
};
use wow_database::{HotfixDatabase, HotfixStatements, WorldDatabase, WorldStatements};

use wow_data::spell::spell_effect_types::{
    SPELL_EFFECT_DUAL_WIELD, SPELL_EFFECT_LEARN_SPELL, SPELL_EFFECT_SKILL, SPELL_EFFECT_SKILL_STEP,
};
use wow_data::spell_acquisition::{
    SpellAcquisitionEffectLikeCpp, SpellAcquisitionResolvedEffectsLookupLikeCpp,
    SpellAcquisitionResolvedMetadataLookupLikeCpp,
};

#[derive(Debug)]
pub(crate) struct SpellAcquisitionBootstrapLikeCpp {
    pub(crate) catalog: Arc<SpellAcquisitionCatalogLikeCpp>,
    pub(crate) chain_store: Arc<SpellChainStoreLikeCpp>,
    pub(crate) learn_skill_store: Arc<SpellLearnSkillStoreLikeCpp>,
    pub(crate) learn_spell_store: Arc<SpellLearnSpellStoreLikeCpp>,
    pub(crate) custom_attribute_store: Arc<SpellCustomAttributeStoreLikeCpp>,
}

#[derive(Debug)]
pub(crate) struct TrainerSpellStaticAuthorityLikeCpp {
    pub(crate) safe_cast_spell_ids: BTreeSet<u32>,
    pub(crate) valid_craft_spell_ids: BTreeSet<u32>,
}

#[derive(Debug, Clone, Copy, Default)]
struct TrainerCastWorldHookAuditLikeCpp {
    script_binding: bool,
    legacy_script: bool,
    condition: bool,
    disabled: bool,
    aura_restriction: bool,
    equipped_item_restriction: bool,
    spell_focus_requirement: bool,
    linked_spell: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TrainerSpellScriptBindingsLikeCpp {
    exact_spell_ids: BTreeSet<u32>,
    all_rank_root_spell_ids: BTreeSet<u32>,
}

impl TrainerSpellScriptBindingsLikeCpp {
    fn contains_like_cpp(&self, spell_id: u32, first_rank_spell_id: u32) -> bool {
        self.exact_spell_ids.contains(&spell_id)
            || self.all_rank_root_spell_ids.contains(&first_rank_spell_id)
    }
}

const SPELL_EFFECT_CREATE_ITEM_LIKE_CPP: u32 = 24;
const SPELL_EFFECT_CREATE_RANDOM_ITEM_LIKE_CPP: u32 = 59;
const SPELL_EFFECT_CREATE_LOOT_LIKE_CPP: u32 = 157;

fn trainer_cast_world_hooks_are_static_safe_like_cpp(
    audit: TrainerCastWorldHookAuditLikeCpp,
) -> bool {
    !(audit.script_binding
        || audit.legacy_script
        || audit.condition
        || audit.disabled
        || audit.aura_restriction
        || audit.equipped_item_restriction
        || audit.spell_focus_requirement
        || audit.linked_spell)
}

fn trainer_cast_has_effective_aura_restriction_like_cpp(
    entry: &SpellAuraRestrictionsEntry,
) -> bool {
    entry.caster_aura_state != 0
        || entry.target_aura_state != 0
        || entry.exclude_caster_aura_state != 0
        || entry.exclude_target_aura_state != 0
        || entry.caster_aura_spell != 0
        || entry.target_aura_spell != 0
        || entry.exclude_caster_aura_spell != 0
        || entry.exclude_target_aura_spell != 0
}

fn trainer_cast_has_effective_difficulty_none_aura_restriction_like_cpp(
    store: &SpellAuraRestrictionsStore,
    spell_id: u32,
) -> bool {
    // C++ attaches SpellAuraRestrictions to the exact `(SpellID,
    // Difficulty)` SpellInfo key. Preserve Rust's represented wildcard only
    // when no exact DIFFICULTY_NONE row exists; rows for other difficulties
    // must not contaminate a normal trainer cast.
    let has_exact = store
        .entries_for_spell_id_like_cpp(spell_id)
        .any(|entry| entry.difficulty_id == 0);
    let selected_difficulty = if has_exact { 0 } else { u8::MAX };
    store
        .entries_for_spell_id_like_cpp(spell_id)
        .filter(|entry| entry.difficulty_id == selected_difficulty)
        .any(trainer_cast_has_effective_aura_restriction_like_cpp)
}

fn trainer_cast_has_effective_equipped_item_restriction_like_cpp(
    entry: &SpellEquippedItemsEntry,
) -> bool {
    // C++ `SpellInfo::IsItemFitToSpellRequirements` treats class -1 as item
    // neutral before consulting either mask.
    entry.equipped_item_class != -1
}

fn trainer_cast_effects_are_static_safe_like_cpp(
    spell_id: u32,
    effects: &[SpellAcquisitionEffectLikeCpp],
    mut has_pet_aura: impl FnMut(u8) -> bool,
) -> bool {
    let mut has_acquisition_effect = false;
    for effect in effects {
        let (Ok(effect_type), Ok(effect_index)) =
            (effect.effect_type_checked(), effect.effect_index_checked())
        else {
            return false;
        };
        if has_pet_aura(effect_index) {
            return false;
        }
        match effect_type {
            SPELL_EFFECT_LEARN_SPELL
            | SPELL_EFFECT_SKILL
            | SPELL_EFFECT_SKILL_STEP
            | SPELL_EFFECT_DUAL_WIELD => {
                has_acquisition_effect = true;
                // This bounded runtime does not yet own C++'s mechanic/state
                // immunity containers. Keep accepted trainer effects neutral
                // in both dimensions so active immunity auras can be compared
                // exactly rather than guessed at purchase time.
                if effect.effect_mechanic_raw != 0 || effect.effect_aura_raw != 0 {
                    return false;
                }
                if effect_type != SPELL_EFFECT_SKILL && !effect.targets_player_like_cpp() {
                    return false;
                }
            }
            0 => {}
            // The effective 3.4.3 audit found these two inert DUMMY
            // effects in the otherwise deterministic riding closure.
            3 if matches!(spell_id, 33_388 | 34_090) => {}
            other if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(other) => {}
            _ => return false,
        }
    }
    has_acquisition_effect
}

/// Build the immutable, process-wide half of trainer wrapper authority.
///
/// This deliberately proves only the narrow acquisition-effect closure. Every
/// world-table hook that can alter a cast is queried from the final DB, and
/// every unsupported effective effect/target remains absent from the result.
/// The per-player immunity/effect mask is resolved separately by `wow-world`
/// immediately before the atomic trainer commit.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_trainer_static_authority_like_cpp(
    data_dir: &str,
    locale: &str,
    hotfix_db: &HotfixDatabase,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    world_db: &WorldDatabase,
    spell_store: &SpellStore,
    spell_chains: &SpellChainStoreLikeCpp,
    catalog: &SpellAcquisitionCatalogLikeCpp,
    linked: &SpellLinkedStoreLikeCpp,
    pet_auras: &SpellPetAuraStoreLikeCpp,
    aura_restrictions: &SpellAuraRestrictionsStore,
    equipped_items: &SpellEquippedItemsStore,
    item_exists: impl Fn(u32) -> bool,
) -> Result<TrainerSpellStaticAuthorityLikeCpp> {
    let script_bindings = load_spell_script_bindings_like_cpp(
        world_db,
        WorldStatements::SEL_TRAINER_CAST_SCRIPT_BINDING_IDS,
    )
    .await
    .context("Failed to audit spell_script_names for trainer casts")?;
    let legacy_scripts = load_unsigned_spell_id_set_like_cpp(
        world_db,
        WorldStatements::SEL_TRAINER_CAST_LEGACY_SCRIPT_IDS,
    )
    .await
    .context("Failed to audit spell_scripts for trainer casts")?;
    let conditions = load_signed_spell_id_set_like_cpp(
        world_db,
        WorldStatements::SEL_TRAINER_CAST_CONDITION_IDS,
    )
    .await
    .context("Failed to audit spell cast/implicit-target conditions")?;
    let disabled = load_unsigned_spell_id_set_like_cpp(
        world_db,
        WorldStatements::SEL_TRAINER_CAST_DISABLED_IDS,
    )
    .await
    .context("Failed to audit disabled trainer casts")?;
    let effective_reagents =
        load_effective_spell_reagents_like_cpp(data_dir, locale, hotfix_db, removals)
            .await
            .context("Failed to compose effective trainer craft reagents")?;

    let mut safe_cast_spell_ids = BTreeSet::new();
    for (spell_id, difficulty_id) in spell_store.spell_info_keys_in_order_like_cpp() {
        if difficulty_id != 0 {
            continue;
        }
        let hook_audit = TrainerCastWorldHookAuditLikeCpp {
            script_binding: script_bindings.contains_like_cpp(
                spell_id,
                spell_chains.first_spell_in_chain_like_cpp(spell_id),
            ),
            legacy_script: legacy_scripts.contains(&spell_id),
            condition: conditions.contains(&spell_id),
            disabled: disabled.contains(&spell_id),
            aura_restriction: trainer_cast_has_effective_difficulty_none_aura_restriction_like_cpp(
                aura_restrictions,
                spell_id,
            ),
            equipped_item_restriction: equipped_items
                .entry_for_spell_id_like_cpp(i32::try_from(spell_id).unwrap_or(i32::MAX))
                .is_some_and(trainer_cast_has_effective_equipped_item_restriction_like_cpp),
            // The reduced trainer path does not execute C++ `SearchSpellFocus`.
            // Missing SpellInfo is equally indeterminate and must fail closed.
            spell_focus_requirement: i32::try_from(spell_id)
                .ok()
                .and_then(|spell_id| spell_store.get(spell_id))
                .is_none_or(|spell_info| spell_info.requires_spell_focus_like_cpp()),
            linked_spell: [
                SpellLinkedTypeLikeCpp::Cast,
                SpellLinkedTypeLikeCpp::Hit,
                SpellLinkedTypeLikeCpp::Aura,
            ]
            .into_iter()
            .any(|kind| linked.get_spell_linked_like_cpp(kind, spell_id).is_some()),
        };
        if !trainer_cast_world_hooks_are_static_safe_like_cpp(hook_audit) {
            continue;
        }
        let effects = match catalog.difficulty_none_effects_like_cpp(spell_id) {
            wow_data::SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) => effects,
            wow_data::SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage
            | wow_data::SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_) => continue,
        };
        if trainer_cast_effects_are_static_safe_like_cpp(spell_id, effects, |effect_index| {
            pet_auras
                .get_pet_aura_like_cpp(spell_id, effect_index)
                .is_some()
        }) {
            safe_cast_spell_ids.insert(spell_id);
        }
    }

    // C++ `SpellMgr::IsSpellValid` evaluates the final SpellEffect payload,
    // recursively follows LEARN_SPELL, and validates every created item and
    // positive reagent. Derive this authority from the same effective catalog
    // so official/custom overlays cannot leave a stale dataset-specific pin.
    let mut effective_effects_by_spell = BTreeMap::new();
    for (spell_id, difficulty_id) in spell_store.spell_info_keys_in_order_like_cpp() {
        if difficulty_id != 0 {
            continue;
        }
        if let wow_data::SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) =
            catalog.difficulty_none_effects_like_cpp(spell_id)
        {
            effective_effects_by_spell.insert(spell_id, effects.to_vec());
        }
    }
    let valid_craft_spell_ids = derive_valid_craft_spell_ids_like_cpp(
        &effective_effects_by_spell,
        &effective_reagents,
        item_exists,
    );

    Ok(TrainerSpellStaticAuthorityLikeCpp {
        safe_cast_spell_ids,
        valid_craft_spell_ids,
    })
}

const SPELL_REAGENTS_OVERLAY_SQL_LIKE_CPP: &str = concat!(
    "SELECT ID, SpellID, Reagent1, Reagent2, Reagent3, Reagent4, ",
    "Reagent5, Reagent6, Reagent7, Reagent8, ReagentCount1, ReagentCount2, ",
    "ReagentCount3, ReagentCount4, ReagentCount5, ReagentCount6, ",
    "ReagentCount7, ReagentCount8 FROM spell_reagents ",
    "WHERE (`VerifiedBuild` > 0) = ?"
);

async fn load_effective_spell_reagents_like_cpp(
    data_dir: &str,
    locale: &str,
    hotfix_db: &HotfixDatabase,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> Result<BTreeMap<u32, [i32; 8]>> {
    let db2_path = Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("SpellReagents.db2");
    let table_hash = Wdc4Reader::open(&db2_path)
        .with_context(|| format!("failed to read table hash from {}", db2_path.display()))?
        .table_hash();
    let store =
        SpellReagentsStore::load(data_dir, locale).context("failed to load SpellReagents.db2")?;
    let base_rows = store.entries_like_cpp().cloned().collect::<Vec<_>>();
    let mut overlay_batches = [Vec::new(), Vec::new()];
    for (batch_index, official) in [true, false].into_iter().enumerate() {
        let mut statement =
            hotfix_db.prepare(HotfixStatements::base(SPELL_REAGENTS_OVERLAY_SQL_LIKE_CPP));
        statement.set_bool(0, official);
        let mut result = hotfix_db.query(&statement).await?;
        if result.is_empty() {
            continue;
        }
        loop {
            overlay_batches[batch_index].push(SpellReagentsEntry {
                id: result.try_read::<u32>(0).unwrap_or(0),
                spell_id: result.try_read::<i32>(1).unwrap_or(0),
                reagent: std::array::from_fn(|index| {
                    result.try_read::<i32>(2 + index).unwrap_or(0)
                }),
                reagent_count: std::array::from_fn(|index| {
                    result.try_read::<i16>(10 + index).unwrap_or(0)
                }),
            });
            if !result.next_row() {
                break;
            }
        }
    }
    let [official_rows, custom_rows] = overlay_batches;
    let removed_record_ids = removals
        .removed_records_in_order_like_cpp()
        .into_iter()
        .filter_map(|(removed_table_hash, record_id)| {
            (removed_table_hash == table_hash).then_some(record_id)
        });
    Ok(compose_effective_spell_reagents_like_cpp(
        base_rows,
        official_rows,
        custom_rows,
        removed_record_ids,
    ))
}

fn compose_effective_spell_reagents_like_cpp(
    base_rows: impl IntoIterator<Item = SpellReagentsEntry>,
    official_rows: impl IntoIterator<Item = SpellReagentsEntry>,
    custom_rows: impl IntoIterator<Item = SpellReagentsEntry>,
    removed_record_ids: impl IntoIterator<Item = i32>,
) -> BTreeMap<u32, [i32; 8]> {
    let mut rows_by_record_id = BTreeMap::new();
    for row in base_rows {
        rows_by_record_id.insert(row.id, row);
    }
    for row in official_rows {
        rows_by_record_id.insert(row.id, row);
    }
    for row in custom_rows {
        rows_by_record_id.insert(row.id, row);
    }
    let removed_record_ids = removed_record_ids.into_iter().collect::<BTreeSet<_>>();
    rows_by_record_id.retain(|record_id, _| {
        i32::try_from(*record_id)
            .ok()
            .is_some_and(|record_id| !removed_record_ids.contains(&record_id))
    });

    let mut reagents_by_spell_id = BTreeMap::new();
    for row in rows_by_record_id.into_values() {
        if let Ok(spell_id) = u32::try_from(row.spell_id)
            && spell_id != 0
        {
            // C++ iterates DB2 storage and assigns the relation; retaining
            // record-ID order preserves its last-record-wins duplicate shape.
            reagents_by_spell_id.insert(spell_id, row.reagent);
        }
    }
    reagents_by_spell_id
}

fn derive_valid_craft_spell_ids_like_cpp(
    effects_by_spell: &BTreeMap<u32, Vec<SpellAcquisitionEffectLikeCpp>>,
    reagents_by_spell: &BTreeMap<u32, [i32; 8]>,
    item_exists: impl Fn(u32) -> bool,
) -> BTreeSet<u32> {
    fn is_valid(
        spell_id: u32,
        effects_by_spell: &BTreeMap<u32, Vec<SpellAcquisitionEffectLikeCpp>>,
        reagents_by_spell: &BTreeMap<u32, [i32; 8]>,
        item_exists: &impl Fn(u32) -> bool,
        visiting: &mut BTreeSet<u32>,
        memo: &mut BTreeMap<u32, bool>,
    ) -> bool {
        if let Some(valid) = memo.get(&spell_id) {
            return *valid;
        }
        let Some(effects) = effects_by_spell.get(&spell_id) else {
            return false;
        };
        // C++ assumes acyclic LEARN_SPELL data and would recurse forever on a
        // cycle. The startup authority must fail closed instead.
        if !visiting.insert(spell_id) {
            return false;
        }

        let is_loot_crafting = effects.iter().any(|effect| {
            matches!(
                effect.effect_type_checked(),
                Ok(SPELL_EFFECT_CREATE_RANDOM_ITEM_LIKE_CPP | SPELL_EFFECT_CREATE_LOOT_LIKE_CPP)
            )
        });
        let mut need_check_reagents = false;
        let mut valid = true;
        for effect in effects {
            let Ok(effect_type) = effect.effect_type_checked() else {
                valid = false;
                break;
            };
            match effect_type {
                SPELL_EFFECT_CREATE_ITEM_LIKE_CPP | SPELL_EFFECT_CREATE_LOOT_LIKE_CPP => {
                    need_check_reagents = true;
                    let Ok(item_id) = effect.item_type_checked() else {
                        valid = false;
                        break;
                    };
                    if (item_id == 0 && !is_loot_crafting)
                        || (item_id != 0 && !item_exists(item_id))
                    {
                        valid = false;
                        break;
                    }
                }
                SPELL_EFFECT_LEARN_SPELL => {
                    let Ok(learned_spell_id) = effect.trigger_spell_id_checked() else {
                        valid = false;
                        break;
                    };
                    if !is_valid(
                        learned_spell_id,
                        effects_by_spell,
                        reagents_by_spell,
                        item_exists,
                        visiting,
                        memo,
                    ) {
                        valid = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if valid && need_check_reagents {
            valid = reagents_by_spell
                .get(&spell_id)
                .into_iter()
                .flatten()
                .all(|reagent_id| {
                    *reagent_id <= 0 || u32::try_from(*reagent_id).ok().is_some_and(item_exists)
                });
        }
        visiting.remove(&spell_id);
        memo.insert(spell_id, valid);
        valid
    }

    let craft_spell_ids = effects_by_spell.iter().filter_map(|(spell_id, effects)| {
        effects
            .iter()
            .any(|effect| {
                matches!(
                    effect.effect_type_checked(),
                    Ok(SPELL_EFFECT_CREATE_ITEM_LIKE_CPP | SPELL_EFFECT_CREATE_LOOT_LIKE_CPP)
                )
            })
            .then_some(*spell_id)
    });
    let mut memo = BTreeMap::new();
    craft_spell_ids
        .filter(|spell_id| {
            is_valid(
                *spell_id,
                effects_by_spell,
                reagents_by_spell,
                &item_exists,
                &mut BTreeSet::new(),
                &mut memo,
            )
        })
        .collect()
}

async fn load_unsigned_spell_id_set_like_cpp(
    world_db: &WorldDatabase,
    statement: WorldStatements,
) -> Result<BTreeSet<u32>> {
    let prepared = world_db.prepare(statement);
    let mut result = world_db.query(&prepared).await?;
    let mut ids = BTreeSet::new();
    if !result.is_empty() {
        loop {
            if let Some(id) = result.try_read::<u32>(0) {
                ids.insert(id);
            }
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(ids)
}

async fn load_signed_spell_id_set_like_cpp(
    world_db: &WorldDatabase,
    statement: WorldStatements,
) -> Result<BTreeSet<u32>> {
    let prepared = world_db.prepare(statement);
    let mut result = world_db.query(&prepared).await?;
    let mut ids = BTreeSet::new();
    if !result.is_empty() {
        loop {
            if let Some(id) = result.try_read::<i32>(0)
                && let Some(id) = id.checked_abs().and_then(|id| u32::try_from(id).ok())
            {
                ids.insert(id);
            }
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(ids)
}

async fn load_spell_script_bindings_like_cpp(
    world_db: &WorldDatabase,
    statement: WorldStatements,
) -> Result<TrainerSpellScriptBindingsLikeCpp> {
    let prepared = world_db.prepare(statement);
    let mut result = world_db.query(&prepared).await?;
    let mut bindings = TrainerSpellScriptBindingsLikeCpp::default();
    if !result.is_empty() {
        loop {
            if let Some(id) = result.try_read::<i32>(0) {
                if id > 0 {
                    bindings.exact_spell_ids.insert(id as u32);
                } else if let Some(id) = id.checked_abs().and_then(|id| u32::try_from(id).ok())
                    && id != 0
                {
                    bindings.all_rank_root_spell_ids.insert(id);
                }
            }
            if !result.next_row() {
                break;
            }
        }
    }
    Ok(bindings)
}

#[derive(Debug, PartialEq, Eq)]
enum LearnSkillProjectionLikeCpp {
    Covered(SpellLearnSkillSourceSpellInfoLikeCpp),
    Indeterminate(SpellLearnSkillIndeterminateReasonLikeCpp),
}

/// Compose the effective spell-acquisition stores in C++ startup order.
///
/// C++ anchors:
/// - `World.cpp:1858-1956`: custom attributes precede ranks/learn stores;
/// - `SpellMgr.cpp:812-902`: rank chains from final SkillLineAbility rows;
/// - `SpellMgr.cpp:947-1135`: learn-skill and learn-spell projections;
/// - `SpellMgr.cpp:2608-2690`: slot-by-slot effect fallback and first-present
///   singleton metadata.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_like_cpp(
    data_dir: &str,
    locale: &str,
    hotfix_db: &HotfixDatabase,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    world_db: &WorldDatabase,
    spell_store: &SpellStore,
    serverside_spell_store: &ServersideSpellStoreLikeCpp,
    difficulty_store: &DifficultyStore,
    skill_store: &SkillStore,
) -> Result<SpellAcquisitionBootstrapLikeCpp> {
    let regular_keys = spell_store.spell_info_keys_in_order_like_cpp();
    let serverside_keys = serverside_spell_store
        .spell_infos_by_spell_and_difficulty
        .keys()
        .copied()
        .collect::<Vec<_>>();

    let mut coverage = Vec::with_capacity(regular_keys.len() + serverside_keys.len());
    coverage.extend(regular_keys.iter().map(|(spell_id, difficulty_id)| {
        SpellAcquisitionCoverageSeedLikeCpp::covered(*spell_id, u32::from(*difficulty_id))
    }));
    coverage.extend(serverside_keys.iter().map(|key| {
        SpellAcquisitionCoverageSeedLikeCpp::indeterminate(
            key.spell_id,
            key.difficulty_id,
            SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
        )
    }));
    let indeterminate_seed_count = serverside_keys.len();

    let catalog = Arc::new(
        SpellAcquisitionCatalogLikeCpp::load_effective_like_cpp(
            data_dir, locale, hotfix_db, removals, coverage,
        )
        .await
        .context("Failed to load the effective spell-acquisition catalog")?,
    );
    let catalog_indeterminate_diagnostics = catalog
        .diagnostics_like_cpp()
        .iter()
        .filter(|diagnostic| {
            diagnostic.severity == SpellAcquisitionDiagnosticSeverityLikeCpp::Indeterminate
        })
        .count();
    info!(
        regular_seed_count = regular_keys.len(),
        serverside_seed_count = serverside_keys.len(),
        indeterminate_seed_count,
        diagnostic_count = catalog.diagnostics_like_cpp().len(),
        indeterminate_diagnostic_count = catalog_indeterminate_diagnostics,
        removed_row_count = catalog.removed_rows_like_cpp().len(),
        "Loaded effective spell-acquisition catalog"
    );

    // C++ applies SQL custom attributes to every exact SpellInfo variant
    // before deriving Talent flags and before rank/learn stores are built.
    let custom_variants = custom_attribute_variants_like_cpp(
        &regular_keys,
        serverside_spell_store,
        difficulty_store,
        &catalog,
    );
    let mut custom_attribute_outcome =
        SpellCustomAttributeStoreLikeCpp::load_for_variants_like_cpp(world_db, |spell_id| {
            custom_variants.get(&spell_id).cloned().unwrap_or_default()
        })
        .await
        .context("Failed to load C++ spell_custom_attr rows")?;
    let custom_talent_unknown_keys =
        custom_talent_unknown_keys_like_cpp(&custom_attribute_outcome.errors);
    let mut derived_talent_variant_count = 0usize;
    for (spell_id, variants) in &custom_variants {
        if matches!(
            catalog.talent_membership_like_cpp(*spell_id),
            SpellAcquisitionTalentLookupLikeCpp::Talent
        ) {
            derived_talent_variant_count += apply_proven_talent_to_variants_like_cpp(
                &mut custom_attribute_outcome.store,
                *spell_id,
                variants.iter().map(|variant| variant.difficulty),
            );
        }
    }
    info!(
        loaded_row_count = custom_attribute_outcome.loaded_row_count,
        applied_variant_count = custom_attribute_outcome.applied_variant_count,
        derived_talent_variant_count,
        error_count = custom_attribute_outcome.errors.len(),
        talent_indeterminate_variant_count = custom_talent_unknown_keys.len(),
        "Loaded C++ spell custom attributes before rank and learn stores"
    );
    let custom_attribute_store = Arc::new(custom_attribute_outcome.store);

    let regular_difficulty_none_ids = regular_keys
        .iter()
        .filter_map(|(spell_id, difficulty_id)| (*difficulty_id == 0).then_some(*spell_id))
        .collect::<BTreeSet<_>>();
    let spell_exists_at_difficulty_none = |spell_id| {
        resolved_difficulty_none_like_cpp(
            spell_store,
            serverside_spell_store,
            difficulty_store,
            spell_id,
        )
        .is_some()
    };

    let chain_outcome =
        SpellChainStoreLikeCpp::from_skill_line_ability_rank_rows_with_diagnostics_like_cpp(
            skill_store
                .skill_line_ability_rank_rows_like_cpp()
                .iter()
                .cloned(),
            spell_exists_at_difficulty_none,
        );
    let invalid_rank_source_count = chain_outcome
        .diagnostics_in_order_like_cpp
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic,
                wow_data::SpellChainLoadDiagnosticLikeCpp::InvalidEffectiveSkillLineAbilityRankEndpoints {
                    ..
                }
            )
        })
        .count();
    info!(
        node_count = chain_outcome.store.chains_by_spell_id.len(),
        rank_load_diagnostic_count = chain_outcome.diagnostics_in_order_like_cpp.len(),
        invalid_effective_source_count = invalid_rank_source_count,
        "Loaded represented C++ spell rank-chain nodes from effective SkillLineAbility rows"
    );
    let chain_store = Arc::new(chain_outcome.store);

    let serverside_difficulty_none_count = serverside_keys
        .iter()
        .filter(|key| key.difficulty_id == 0)
        .count();
    let mut learn_skill_sources = Vec::new();
    let mut learn_skill_indeterminate_sources = BTreeMap::new();
    for key in serverside_keys.iter().filter(|key| key.difficulty_id == 0) {
        learn_skill_indeterminate_sources.insert(
            key.spell_id,
            SpellLearnSkillIndeterminateReasonLikeCpp::EffectiveMetadata(vec![
                SpellAcquisitionIndeterminateReasonLikeCpp::ServerSideMetadataUnavailable,
            ]),
        );
    }
    for spell_id in &regular_difficulty_none_ids {
        let chain = difficulty_chain_like_cpp(difficulty_store, 0);
        let effects = match catalog
            .resolved_effects_for_difficulty_chain_like_cpp(*spell_id, chain.iter().copied())
        {
            SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => effects,
            SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { difficulty_id } => {
                learn_skill_indeterminate_sources.insert(
                    *spell_id,
                    SpellLearnSkillIndeterminateReasonLikeCpp::MissingEffectiveCoverage {
                        difficulty_id,
                    },
                );
                continue;
            }
            SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(reasons) => {
                learn_skill_indeterminate_sources.insert(
                    *spell_id,
                    SpellLearnSkillIndeterminateReasonLikeCpp::EffectiveMetadata(reasons),
                );
                continue;
            }
        };
        match project_learn_skill_source_like_cpp(*spell_id, &effects) {
            LearnSkillProjectionLikeCpp::Covered(source) => learn_skill_sources.push(source),
            LearnSkillProjectionLikeCpp::Indeterminate(reason) => {
                learn_skill_indeterminate_sources.insert(*spell_id, reason);
            }
        }
    }
    let mut learn_skill_outcome =
        SpellLearnSkillStoreLikeCpp::from_spell_infos_like_cpp(learn_skill_sources);
    for (spell_id, reason) in learn_skill_indeterminate_sources {
        learn_skill_outcome
            .store
            .mark_spell_learn_skill_indeterminate_like_cpp(spell_id, reason);
    }
    info!(
        loaded_node_count = learn_skill_outcome.store.skill_by_spell_id.len(),
        compatibility_error_count = learn_skill_outcome.errors.len(),
        indeterminate_source_count = learn_skill_outcome.store.indeterminate_by_spell_id.len(),
        "Loaded C++ spell-learn-skill entries from effective acquisition metadata"
    );
    let learn_skill_store = Arc::new(learn_skill_outcome.store);

    // SQL spell_learn_spell validation needs only GetSpellInfo(NONE)
    // existence and effective IS_TALENT. Keep this map separate from the
    // richer auto-edge projection so missing Effects/Misc cannot reject a
    // C++-valid SQL row.
    let all_spell_ids = regular_keys
        .iter()
        .map(|(spell_id, _)| *spell_id)
        .chain(serverside_keys.iter().map(|key| key.spell_id))
        .collect::<BTreeSet<_>>();
    let mut sql_source_infos = BTreeMap::new();
    let mut sql_talent_indeterminate_sources = 0usize;
    for spell_id in all_spell_ids {
        let Some(resolved_difficulty) = resolved_difficulty_none_like_cpp(
            spell_store,
            serverside_spell_store,
            difficulty_store,
            spell_id,
        ) else {
            continue;
        };
        let custom_proves_talent = custom_attribute_store
            .attributes_for_spell_difficulty_like_cpp(spell_id, resolved_difficulty)
            & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
            != 0;
        let custom_talent_unknown =
            custom_talent_unknown_keys.contains(&(spell_id, resolved_difficulty));
        let Some(is_talent) = effective_talent_like_cpp(
            custom_proves_talent,
            custom_talent_unknown,
            catalog.talent_membership_like_cpp(spell_id),
        ) else {
            sql_talent_indeterminate_sources += 1;
            continue;
        };
        sql_source_infos.insert(
            spell_id,
            SpellLearnSourceSpellInfoLikeCpp {
                spell_id,
                difficulty_none: true,
                is_talent,
                is_passive: false,
                has_skill_step_effect: false,
                learn_spell_effects: Vec::new(),
            },
        );
    }

    let mut automatic_sources = Vec::new();
    let mut automatic_indeterminate_sources = serverside_difficulty_none_count;
    for spell_id in &regular_difficulty_none_ids {
        let chain = difficulty_chain_like_cpp(difficulty_store, 0);
        let effects = match catalog
            .resolved_effects_for_difficulty_chain_like_cpp(*spell_id, chain.iter().copied())
        {
            SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => effects,
            SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { .. }
            | SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(_) => {
                automatic_indeterminate_sources += 1;
                continue;
            }
        };
        let is_passive = match catalog
            .resolved_misc_for_difficulty_chain_like_cpp(*spell_id, chain.iter().copied())
        {
            SpellAcquisitionResolvedMetadataLookupLikeCpp::Present(misc) => {
                let Ok(is_passive) = misc.is_passive_checked() else {
                    automatic_indeterminate_sources += 1;
                    continue;
                };
                is_passive
            }
            SpellAcquisitionResolvedMetadataLookupLikeCpp::CoveredWithoutRow => false,
            SpellAcquisitionResolvedMetadataLookupLikeCpp::MissingCoverage { .. }
            | SpellAcquisitionResolvedMetadataLookupLikeCpp::Indeterminate(_) => {
                automatic_indeterminate_sources += 1;
                continue;
            }
        };
        let custom_proves_talent = custom_attribute_store
            .attributes_for_spell_difficulty_like_cpp(*spell_id, 0)
            & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
            != 0;
        let custom_talent_unknown = custom_talent_unknown_keys.contains(&(*spell_id, 0));
        let Some(is_talent) = effective_talent_like_cpp(
            custom_proves_talent,
            custom_talent_unknown,
            catalog.talent_membership_like_cpp(*spell_id),
        ) else {
            automatic_indeterminate_sources += 1;
            continue;
        };
        match project_learn_spell_source_like_cpp(*spell_id, is_talent, is_passive, &effects) {
            Some(source) => automatic_sources.push(source),
            None => automatic_indeterminate_sources += 1,
        }
    }

    let mut invalid_dependency_rows = 0usize;
    let dependency_rows = catalog
        .effective_dependency_rows_like_cpp()
        .filter_map(|row| {
            let (Ok(spell_id), Ok(learn_spell_id), Ok(overrides_spell_id)) = (
                row.spell_id_checked(),
                row.learned_spell_id_checked(),
                row.overrides_spell_id_checked(),
            ) else {
                invalid_dependency_rows += 1;
                return None;
            };
            let (Ok(spell_id), Ok(learn_spell_id), Ok(overrides_spell_id)) = (
                i32::try_from(spell_id),
                i32::try_from(learn_spell_id),
                i32::try_from(overrides_spell_id.unwrap_or(0)),
            ) else {
                invalid_dependency_rows += 1;
                return None;
            };
            Some(SpellLearnSpellEntry {
                id: row.record_id,
                spell_id,
                learn_spell_id,
                overrides_spell_id,
            })
        })
        .collect::<Vec<_>>();

    let sql_source_infos_for_lookup = sql_source_infos;
    let learn_spell_outcome = SpellLearnSpellStoreLikeCpp::load_like_cpp(
        world_db,
        automatic_sources,
        dependency_rows,
        |spell_id| sql_source_infos_for_lookup.get(&spell_id).cloned(),
        spell_exists_at_difficulty_none,
    )
    .await
    .context("Failed to load C++ spell_learn_spell rows")?;
    info!(
        sql_loaded_row_count = learn_spell_outcome.sql_loaded_row_count,
        canonical_loaded_edge_count = learn_spell_outcome.dbc_loaded_row_count,
        validation_error_count = learn_spell_outcome.errors.len(),
        warning_count = learn_spell_outcome.warnings.len(),
        sql_result_empty = learn_spell_outcome.sql_result_empty,
        sql_talent_indeterminate_source_count = sql_talent_indeterminate_sources,
        automatic_indeterminate_source_count = automatic_indeterminate_sources,
        invalid_dependency_row_count = invalid_dependency_rows,
        "Loaded C++ spell-learn-spell graph from separated SQL and canonical projections"
    );
    let learn_spell_store = Arc::new(learn_spell_outcome.store);

    Ok(SpellAcquisitionBootstrapLikeCpp {
        catalog,
        chain_store,
        learn_skill_store,
        learn_spell_store,
        custom_attribute_store,
    })
}

/// C++ starts with the requested difficulty and then follows
/// `FallbackDifficultyID` while the current Difficulty row exists. Rust stops
/// a malformed cycle, and retains the final fallback id even when it has no
/// Difficulty row so the metadata resolver can perform the same final lookup.
fn difficulty_chain_like_cpp(
    difficulty_store: &DifficultyStore,
    requested_difficulty: u32,
) -> Vec<u32> {
    let mut chain = vec![requested_difficulty];
    let mut visited = BTreeSet::from([requested_difficulty]);
    let mut current = requested_difficulty;
    while let Some(difficulty) = difficulty_store.get(current) {
        let fallback = u32::from(difficulty.fallback_difficulty_id);
        if !visited.insert(fallback) {
            break;
        }
        chain.push(fallback);
        current = fallback;
    }
    chain
}

fn exact_spell_variant_exists_like_cpp(
    spell_store: &SpellStore,
    serverside_spell_store: &ServersideSpellStoreLikeCpp,
    spell_id: u32,
    difficulty_id: u32,
) -> bool {
    u8::try_from(difficulty_id)
        .ok()
        .is_some_and(|difficulty_id| {
            spell_store.contains_spell_info_exact_like_cpp(spell_id, difficulty_id)
        })
        || serverside_spell_store
            .get_serverside_spell_like_cpp(spell_id, difficulty_id)
            .is_some()
}

fn resolved_difficulty_none_like_cpp(
    spell_store: &SpellStore,
    serverside_spell_store: &ServersideSpellStoreLikeCpp,
    difficulty_store: &DifficultyStore,
    spell_id: u32,
) -> Option<u32> {
    difficulty_chain_like_cpp(difficulty_store, 0)
        .into_iter()
        .find(|difficulty_id| {
            exact_spell_variant_exists_like_cpp(
                spell_store,
                serverside_spell_store,
                spell_id,
                *difficulty_id,
            )
        })
}

fn custom_attribute_variants_like_cpp(
    regular_keys: &[(u32, u8)],
    serverside_spell_store: &ServersideSpellStoreLikeCpp,
    difficulty_store: &DifficultyStore,
    catalog: &SpellAcquisitionCatalogLikeCpp,
) -> BTreeMap<u32, Vec<SpellCustomAttributeSourceVariantLikeCpp>> {
    let mut variants = BTreeMap::<u32, Vec<SpellCustomAttributeSourceVariantLikeCpp>>::new();
    for (spell_id, difficulty_id) in regular_keys {
        let difficulty = u32::from(*difficulty_id);
        let chain = difficulty_chain_like_cpp(difficulty_store, difficulty);
        let effect_types =
            match catalog.resolved_effects_for_difficulty_chain_like_cpp(*spell_id, chain) {
                SpellAcquisitionResolvedEffectsLookupLikeCpp::Covered(effects) => effects
                    .into_iter()
                    .map(SpellAcquisitionEffectLikeCpp::effect_type_checked)
                    .collect::<Result<Vec<_>, _>>()
                    .ok(),
                SpellAcquisitionResolvedEffectsLookupLikeCpp::MissingCoverage { .. }
                | SpellAcquisitionResolvedEffectsLookupLikeCpp::Indeterminate(_) => None,
            };
        variants
            .entry(*spell_id)
            .or_default()
            .push(SpellCustomAttributeSourceVariantLikeCpp {
                spell_id: *spell_id,
                difficulty,
                effect_types,
            });
    }
    for (key, spell_info) in &serverside_spell_store.spell_infos_by_spell_and_difficulty {
        // The server-side effect loader has already validated the type domain.
        // Keep the checked conversion here so this boundary remains fail-closed
        // if that loader's contract changes.
        let effect_types = spell_info
            .effects
            .iter()
            .map(|effect| u32::try_from(effect.effect).ok())
            .collect::<Option<Vec<_>>>();
        variants
            .entry(key.spell_id)
            .or_default()
            .push(SpellCustomAttributeSourceVariantLikeCpp {
                spell_id: key.spell_id,
                difficulty: key.difficulty_id,
                effect_types,
            });
    }
    for spell_variants in variants.values_mut() {
        spell_variants.sort_by_key(|variant| variant.difficulty);
    }
    variants
}

/// A rejected SHARE_DAMAGE variant is relevant to talent classification only
/// when that same SQL row also requested IS_TALENT.
fn custom_talent_unknown_keys_like_cpp(
    errors: &[wow_data::SpellCustomAttributeLoadErrorLikeCpp],
) -> BTreeSet<(u32, u32)> {
    errors
        .iter()
        .filter_map(|error| {
            (error.kind
                == SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable
                && error.attributes & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP != 0)
                .then(|| {
                    error
                        .difficulty
                        .map(|difficulty| (error.spell_id, difficulty))
                })
                .flatten()
        })
        .collect()
}

/// The second half of C++ `LoadSpellInfoCustomAttributes` derives
/// `SPELL_ATTR0_CU_IS_TALENT` from final Talent rows for every exact
/// SpellInfo variant, after applying the SQL bits.
fn apply_proven_talent_to_variants_like_cpp(
    store: &mut SpellCustomAttributeStoreLikeCpp,
    spell_id: u32,
    difficulties: impl IntoIterator<Item = u32>,
) -> usize {
    let mut applied = 0usize;
    for difficulty in difficulties {
        *store
            .attributes_by_spell_and_difficulty
            .entry(SpellCustomAttributeKeyLikeCpp {
                spell_id,
                difficulty,
            })
            .or_default() |= SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP;
        applied += 1;
    }
    applied
}

fn effective_talent_like_cpp(
    custom_proves_talent: bool,
    custom_talent_unknown: bool,
    talent_lookup: SpellAcquisitionTalentLookupLikeCpp<'_>,
) -> Option<bool> {
    if custom_proves_talent {
        return Some(true);
    }
    match talent_lookup {
        SpellAcquisitionTalentLookupLikeCpp::Talent => Some(true),
        SpellAcquisitionTalentLookupLikeCpp::NotTalent if !custom_talent_unknown => Some(false),
        SpellAcquisitionTalentLookupLikeCpp::NotTalent
        | SpellAcquisitionTalentLookupLikeCpp::Indeterminate(_) => None,
    }
}

fn project_learn_skill_source_like_cpp(
    spell_id: u32,
    effects: &[&SpellAcquisitionEffectLikeCpp],
) -> LearnSkillProjectionLikeCpp {
    let mut projected_effects = Vec::new();
    for effect in effects {
        let effect_type = match effect.effect_type_checked() {
            Ok(effect_type) => effect_type,
            Err(invalid) => {
                return LearnSkillProjectionLikeCpp::Indeterminate(
                    SpellLearnSkillIndeterminateReasonLikeCpp::InvalidEffectiveValue {
                        record_id: effect.record_id,
                        field: invalid.field,
                        raw: invalid.raw,
                    },
                );
            }
        };
        match effect_type {
            SPELL_EFFECT_SKILL => {
                let misc_value = match effect
                    .misc_value_id_checked(0)
                    .ok()
                    .and_then(|value| i32::try_from(value).ok())
                {
                    Some(value) => value,
                    None => {
                        return LearnSkillProjectionLikeCpp::Indeterminate(
                            SpellLearnSkillIndeterminateReasonLikeCpp::InvalidEffectiveValue {
                                record_id: effect.record_id,
                                field: "SpellEffect.EffectMiscValue",
                                raw: effect.effect_misc_value_raw[0],
                            },
                        );
                    }
                };
                let domain = match effect.base_points_die_sides_domain_checked() {
                    Ok(domain) => domain,
                    Err(invalid) => {
                        return LearnSkillProjectionLikeCpp::Indeterminate(
                            SpellLearnSkillIndeterminateReasonLikeCpp::InvalidEffectiveValue {
                                record_id: effect.record_id,
                                field: invalid.field,
                                raw: invalid.raw,
                            },
                        );
                    }
                };
                let Some(calc_value) = domain.deterministic_value() else {
                    return LearnSkillProjectionLikeCpp::Indeterminate(
                        SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
                            record_id: effect.record_id,
                            domain,
                        },
                    );
                };
                projected_effects.push(SpellLearnSkillEffectLikeCpp {
                    effect: effect_type,
                    misc_value,
                    calc_value,
                });
                break;
            }
            SPELL_EFFECT_DUAL_WIELD => {
                projected_effects.push(SpellLearnSkillEffectLikeCpp {
                    effect: effect_type,
                    misc_value: 0,
                    calc_value: 0,
                });
                break;
            }
            _ => {}
        }
    }
    LearnSkillProjectionLikeCpp::Covered(SpellLearnSkillSourceSpellInfoLikeCpp {
        spell_id,
        difficulty_none: true,
        effects: projected_effects,
    })
}

fn project_learn_spell_source_like_cpp(
    spell_id: u32,
    is_talent: bool,
    is_passive: bool,
    effects: &[&SpellAcquisitionEffectLikeCpp],
) -> Option<SpellLearnSourceSpellInfoLikeCpp> {
    let mut has_skill_step_effect = false;
    let mut learn_spell_effects = Vec::new();
    for effect in effects {
        let effect_type = effect.effect_type_checked().ok()?;
        has_skill_step_effect |= effect_type == SPELL_EFFECT_SKILL_STEP;
        if effect_type == SPELL_EFFECT_LEARN_SPELL {
            learn_spell_effects.push(SpellLearnSpellEffectLikeCpp {
                trigger_spell: effect.trigger_spell_id_checked().ok()?,
                target_unit_pet: effect.targets_unit_pet_like_cpp(),
            });
        }
    }
    Some(SpellLearnSourceSpellInfoLikeCpp {
        spell_id,
        difficulty_none: true,
        is_talent,
        is_passive,
        has_skill_step_effect,
        learn_spell_effects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_data::DifficultyEntry;

    fn difficulty(id: u32, fallback_difficulty_id: u8) -> DifficultyEntry {
        DifficultyEntry {
            id,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id,
            toggle_difficulty_id: 0,
        }
    }

    fn acquisition_effect(
        record_id: u32,
        effect_index: i64,
        effect_type: u32,
        misc_value: i64,
        trigger_spell: i64,
    ) -> SpellAcquisitionEffectLikeCpp {
        SpellAcquisitionEffectLikeCpp {
            record_id,
            spell_id_raw: 100,
            difficulty_id_raw: 0,
            effect_index_raw: effect_index,
            effect_type_raw: i64::from(effect_type),
            effect_aura_raw: 0,
            effect_mechanic_raw: 0,
            effect_attributes_raw: 0,
            effect_base_points_raw: 0,
            effect_die_sides_raw: 0,
            effect_chain_targets_raw: 0,
            effect_points_per_resource_bits: 0.0f32.to_bits(),
            effect_real_points_per_level_bits: 0.0f32.to_bits(),
            effect_coefficient_bits: 0.0f32.to_bits(),
            effect_variance_bits: 0.0f32.to_bits(),
            effect_trigger_spell_raw: trigger_spell,
            effect_item_type_raw: 0,
            effect_misc_value_raw: [misc_value, 0],
            implicit_target_raw: [0, 0],
        }
    }

    fn reagent_row(id: u32, spell_id: i32, reagents: [i32; 8]) -> SpellReagentsEntry {
        SpellReagentsEntry {
            id,
            spell_id,
            reagent: reagents,
            reagent_count: [0; 8],
        }
    }

    #[test]
    fn trainer_craft_reagents_use_db2_official_custom_then_final_removal_order() {
        let effective = compose_effective_spell_reagents_like_cpp(
            [
                reagent_row(7, 12_716, [1, 0, 0, 0, 0, 0, 0, 0]),
                reagent_row(8, 13_240, [4, 0, 0, 0, 0, 0, 0, 0]),
            ],
            [reagent_row(7, 12_716, [2, 0, 0, 0, 0, 0, 0, 0])],
            [reagent_row(7, 12_716, [3, 0, 0, 0, 0, 0, 0, 0])],
            [8],
        );

        assert_eq!(
            effective,
            BTreeMap::from([(12_716, [3, 0, 0, 0, 0, 0, 0, 0])])
        );
    }

    #[test]
    fn trainer_craft_authority_uses_effective_outputs_reagents_and_loot_zero_branch() {
        let mut create_item = acquisition_effect(1, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        create_item.spell_id_raw = 700;
        create_item.effect_item_type_raw = 10_577;
        let mut missing_output = create_item.clone();
        missing_output.record_id = 2;
        missing_output.spell_id_raw = 701;
        missing_output.effect_item_type_raw = 17_771;
        let mut loot_zero = acquisition_effect(3, 0, SPELL_EFFECT_CREATE_LOOT_LIKE_CPP, 0, 0);
        loot_zero.spell_id_raw = 702;
        let mut create_zero = acquisition_effect(4, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        create_zero.spell_id_raw = 703;
        let mut missing_reagent = create_item.clone();
        missing_reagent.record_id = 5;
        missing_reagent.spell_id_raw = 704;

        let effects = BTreeMap::from([
            (700, vec![create_item]),
            (701, vec![missing_output]),
            (702, vec![loot_zero]),
            (703, vec![create_zero]),
            (704, vec![missing_reagent]),
        ]);
        let reagents = BTreeMap::from([
            (700, [100, -1, 200, 0, 0, 0, 0, 0]),
            (704, [300, 0, 0, 0, 0, 0, 0, 0]),
        ]);
        let existing_items = BTreeSet::from([10_577, 100, 200]);

        assert_eq!(
            derive_valid_craft_spell_ids_like_cpp(&effects, &reagents, |item_id| {
                existing_items.contains(&item_id)
            }),
            BTreeSet::from([700, 702])
        );
    }

    #[test]
    fn trainer_craft_authority_recursively_rejects_invalid_learned_spell_and_cycles() {
        let mut parent_create = acquisition_effect(1, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        parent_create.spell_id_raw = 800;
        parent_create.effect_item_type_raw = 10_577;
        let mut parent_learn = acquisition_effect(2, 1, SPELL_EFFECT_LEARN_SPELL, 0, 801);
        parent_learn.spell_id_raw = 800;
        let mut child_create = acquisition_effect(3, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        child_create.spell_id_raw = 801;
        child_create.effect_item_type_raw = 17_771;

        let effects = BTreeMap::from([
            (800, vec![parent_create, parent_learn]),
            (801, vec![child_create]),
        ]);
        assert!(
            derive_valid_craft_spell_ids_like_cpp(&effects, &BTreeMap::new(), |item_id| {
                item_id == 10_577
            })
            .is_empty()
        );

        let mut cycle_create = acquisition_effect(4, 0, SPELL_EFFECT_CREATE_ITEM_LIKE_CPP, 0, 0);
        cycle_create.spell_id_raw = 900;
        cycle_create.effect_item_type_raw = 10_577;
        let mut cycle_learn = acquisition_effect(5, 1, SPELL_EFFECT_LEARN_SPELL, 0, 900);
        cycle_learn.spell_id_raw = 900;
        assert!(
            derive_valid_craft_spell_ids_like_cpp(
                &BTreeMap::from([(900, vec![cycle_create, cycle_learn])]),
                &BTreeMap::new(),
                |item_id| item_id == 10_577,
            )
            .is_empty()
        );
    }

    #[test]
    fn trainer_craft_reagent_overlay_query_covers_all_cpp_reagent_slots() {
        assert_eq!(
            SPELL_REAGENTS_OVERLAY_SQL_LIKE_CPP,
            concat!(
                "SELECT ID, SpellID, Reagent1, Reagent2, Reagent3, Reagent4, ",
                "Reagent5, Reagent6, Reagent7, Reagent8, ReagentCount1, ReagentCount2, ",
                "ReagentCount3, ReagentCount4, ReagentCount5, ReagentCount6, ",
                "ReagentCount7, ReagentCount8 FROM spell_reagents ",
                "WHERE (`VerifiedBuild` > 0) = ?"
            )
        );
    }

    #[test]
    fn trainer_cast_static_world_hook_audit_fails_closed_for_every_dynamic_hook() {
        assert!(trainer_cast_world_hooks_are_static_safe_like_cpp(
            TrainerCastWorldHookAuditLikeCpp::default()
        ));

        for audit in [
            TrainerCastWorldHookAuditLikeCpp {
                script_binding: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                legacy_script: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                condition: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                disabled: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                aura_restriction: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                equipped_item_restriction: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                spell_focus_requirement: true,
                ..Default::default()
            },
            TrainerCastWorldHookAuditLikeCpp {
                linked_spell: true,
                ..Default::default()
            },
        ] {
            assert!(!trainer_cast_world_hooks_are_static_safe_like_cpp(audit));
        }
    }

    #[test]
    fn trainer_script_bindings_preserve_cpp_signed_rank_semantics() {
        let bindings = TrainerSpellScriptBindingsLikeCpp {
            exact_spell_ids: BTreeSet::from([200]),
            all_rank_root_spell_ids: BTreeSet::from([100]),
        };

        assert!(bindings.contains_like_cpp(200, 200));
        assert!(!bindings.contains_like_cpp(201, 200));
        assert!(bindings.contains_like_cpp(101, 100));
        assert!(!bindings.contains_like_cpp(301, 300));
    }

    #[test]
    fn trainer_cast_static_restriction_audit_ignores_cpp_neutral_db2_rows() {
        let neutral_aura = SpellAuraRestrictionsEntry {
            id: 1,
            difficulty_id: 0,
            caster_aura_state: 0,
            target_aura_state: 0,
            exclude_caster_aura_state: 0,
            exclude_target_aura_state: 0,
            caster_aura_spell: 0,
            target_aura_spell: 0,
            exclude_caster_aura_spell: 0,
            exclude_target_aura_spell: 0,
            spell_id: 100,
        };
        assert!(!trainer_cast_has_effective_aura_restriction_like_cpp(
            &neutral_aura
        ));

        let mut effective_aura = neutral_aura.clone();
        effective_aura.target_aura_spell = 200;
        assert!(trainer_cast_has_effective_aura_restriction_like_cpp(
            &effective_aura
        ));

        let mut raid_restriction = effective_aura.clone();
        raid_restriction.id = 2;
        raid_restriction.difficulty_id = 16;
        let restrictions = SpellAuraRestrictionsStore::from_entries([
            neutral_aura.clone(),
            raid_restriction.clone(),
        ]);
        let raid_only = SpellAuraRestrictionsStore::from_entries([raid_restriction.clone()]);
        assert!(
            !trainer_cast_has_effective_difficulty_none_aura_restriction_like_cpp(&raid_only, 100,),
            "a nonzero-difficulty-only row does not apply to DIFFICULTY_NONE"
        );
        assert!(
            !trainer_cast_has_effective_difficulty_none_aura_restriction_like_cpp(
                &restrictions,
                100,
            ),
            "a restriction on another difficulty must not contaminate DIFFICULTY_NONE"
        );

        let mut wildcard_restriction = raid_restriction;
        wildcard_restriction.id = 3;
        wildcard_restriction.difficulty_id = u8::MAX;
        let wildcard_only =
            SpellAuraRestrictionsStore::from_entries([wildcard_restriction.clone()]);
        assert!(
            trainer_cast_has_effective_difficulty_none_aura_restriction_like_cpp(
                &wildcard_only,
                100,
            ),
            "the represented wildcard applies when no exact difficulty row exists"
        );
        let exact_over_wildcard =
            SpellAuraRestrictionsStore::from_entries([neutral_aura, wildcard_restriction]);
        assert!(
            !trainer_cast_has_effective_difficulty_none_aura_restriction_like_cpp(
                &exact_over_wildcard,
                100,
            ),
            "an exact DIFFICULTY_NONE row takes precedence over the wildcard"
        );

        assert!(
            !trainer_cast_has_effective_equipped_item_restriction_like_cpp(
                &SpellEquippedItemsEntry {
                    id: 2,
                    spell_id: 100,
                    equipped_item_class: -1,
                    equipped_item_inv_types: i32::MAX,
                    equipped_item_subclass: i32::MAX,
                }
            )
        );
        assert!(
            trainer_cast_has_effective_equipped_item_restriction_like_cpp(
                &SpellEquippedItemsEntry {
                    id: 3,
                    spell_id: 100,
                    equipped_item_class: 2,
                    equipped_item_inv_types: 0,
                    equipped_item_subclass: 0,
                }
            )
        );
    }

    #[test]
    fn trainer_cast_static_effect_audit_accepts_only_player_acquisition_closure() {
        let learn = acquisition_effect(1, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        let skill = acquisition_effect(2, 1, SPELL_EFFECT_SKILL, 164, 0);
        let noop = acquisition_effect(3, 2, 0, 0, 0);

        assert!(trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[learn.clone(), skill, noop.clone()],
            |_| false,
        ));
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[noop],
            |_| false,
        ));
    }

    #[test]
    fn trainer_cast_static_effect_audit_rejects_target_pet_aura_and_invalid_rows() {
        let mut pet_target = acquisition_effect(1, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        pet_target.implicit_target_raw = [5, 0];
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[pet_target],
            |_| false,
        ));

        let learn = acquisition_effect(2, 1, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[learn],
            |effect_index| effect_index == 1,
        ));

        let mut invalid_index = acquisition_effect(3, 256, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        invalid_index.implicit_target_raw = [1, 0];
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[invalid_index],
            |_| false,
        ));

        let mut mechanic = acquisition_effect(4, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        mechanic.effect_mechanic_raw = 3;
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[mechanic],
            |_| false,
        ));

        let mut aura = acquisition_effect(5, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        aura.effect_aura_raw = 79;
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[aura],
            |_| false,
        ));
    }

    #[test]
    fn trainer_cast_static_effect_audit_pins_only_the_audited_riding_dummy() {
        let learn = acquisition_effect(1, 0, SPELL_EFFECT_LEARN_SPELL, 0, 200);
        let dummy = acquisition_effect(2, 1, 3, 0, 0);

        assert!(trainer_cast_effects_are_static_safe_like_cpp(
            33_388,
            &[learn.clone(), dummy.clone()],
            |_| false,
        ));
        assert!(!trainer_cast_effects_are_static_safe_like_cpp(
            100,
            &[learn, dummy],
            |_| false,
        ));
    }

    #[test]
    fn fallback_chain_keeps_final_missing_lookup_like_cpp() {
        let store = DifficultyStore::from_entries([difficulty(5, 4), difficulty(4, 3)]);

        assert_eq!(difficulty_chain_like_cpp(&store, 5), vec![5, 4, 3]);
    }

    #[test]
    fn fallback_chain_stops_invalid_cycle() {
        let store = DifficultyStore::from_entries([difficulty(5, 4), difficulty(4, 5)]);

        assert_eq!(difficulty_chain_like_cpp(&store, 5), vec![5, 4]);
    }

    #[test]
    fn proven_talent_wins_over_unknown_custom_variant() {
        assert_eq!(
            effective_talent_like_cpp(false, true, SpellAcquisitionTalentLookupLikeCpp::Talent,),
            Some(true)
        );
        assert_eq!(
            effective_talent_like_cpp(false, true, SpellAcquisitionTalentLookupLikeCpp::NotTalent,),
            None
        );
        assert_eq!(
            effective_talent_like_cpp(true, true, SpellAcquisitionTalentLookupLikeCpp::NotTalent,),
            Some(true)
        );
    }

    #[test]
    fn custom_talent_uncertainty_uses_rejected_attribute_bits() {
        let errors = [
            wow_data::SpellCustomAttributeLoadErrorLikeCpp {
                spell_id: 100,
                difficulty: Some(0),
                attributes: wow_data::SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP,
                kind:
                    SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable,
            },
            wow_data::SpellCustomAttributeLoadErrorLikeCpp {
                spell_id: 100,
                difficulty: Some(2),
                attributes: wow_data::SPELL_ATTR0_CU_SHARE_DAMAGE_LIKE_CPP
                    | SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
                kind:
                    SpellCustomAttributeLoadErrorKindLikeCpp::ShareDamageEffectCoverageUnavailable,
            },
        ];

        assert_eq!(
            custom_talent_unknown_keys_like_cpp(&errors),
            BTreeSet::from([(100, 2)])
        );
    }

    #[test]
    fn proven_talent_is_published_for_every_exact_variant_like_cpp() {
        let mut store = SpellCustomAttributeStoreLikeCpp::default();

        assert_eq!(
            apply_proven_talent_to_variants_like_cpp(&mut store, 100, [0, 2]),
            2
        );
        assert_ne!(
            store.attributes_for_spell_difficulty_like_cpp(100, 0)
                & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
            0
        );
        assert_ne!(
            store.attributes_for_spell_difficulty_like_cpp(100, 2)
                & SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP,
            0
        );
    }

    #[test]
    fn learn_skill_projection_stops_at_first_matching_effect_like_cpp() {
        let unrelated = acquisition_effect(1, 0, 0, 0, 0);
        let dual_wield = acquisition_effect(2, 1, SPELL_EFFECT_DUAL_WIELD, 0, 0);
        let mut skill = acquisition_effect(3, 2, SPELL_EFFECT_SKILL, -1, 0);
        skill.effect_base_points_raw = i64::from(i32::MAX) + 1;

        let LearnSkillProjectionLikeCpp::Covered(source) =
            project_learn_skill_source_like_cpp(100, &[&unrelated, &dual_wield, &skill])
        else {
            panic!("the first qualifying dual-wield effect must remain covered");
        };

        assert_eq!(
            source.effects,
            vec![SpellLearnSkillEffectLikeCpp {
                effect: SPELL_EFFECT_DUAL_WIELD,
                misc_value: 0,
                calc_value: 0,
            }]
        );
    }

    #[test]
    fn learn_skill_projection_rejects_zero_skill_id() {
        let skill = acquisition_effect(1, 0, SPELL_EFFECT_SKILL, 0, 0);

        assert_eq!(
            project_learn_skill_source_like_cpp(100, &[&skill]),
            LearnSkillProjectionLikeCpp::Indeterminate(
                SpellLearnSkillIndeterminateReasonLikeCpp::InvalidEffectiveValue {
                    record_id: 1,
                    field: "SpellEffect.EffectMiscValue",
                    raw: 0,
                }
            )
        );
    }

    #[test]
    fn rng_dependent_first_skill_is_explicit_and_does_not_fall_through() {
        let mut ranged_skill = acquisition_effect(1, 0, SPELL_EFFECT_SKILL, 755, 0);
        ranged_skill.effect_base_points_raw = 4;
        ranged_skill.effect_die_sides_raw = 3;
        let dual_wield = acquisition_effect(2, 1, SPELL_EFFECT_DUAL_WIELD, 0, 0);

        assert_eq!(
            project_learn_skill_source_like_cpp(100, &[&ranged_skill, &dual_wield]),
            LearnSkillProjectionLikeCpp::Indeterminate(
                SpellLearnSkillIndeterminateReasonLikeCpp::RngDependentCalcValue {
                    record_id: 1,
                    domain: wow_data::AcquisitionValueDomainLikeCpp {
                        minimum: 5,
                        maximum: 7,
                    },
                }
            )
        );
    }

    #[test]
    fn covered_spell_without_qualifying_effect_stays_distinct_from_indeterminate() {
        let unrelated = acquisition_effect(1, 0, 0, 0, 0);

        assert_eq!(
            project_learn_skill_source_like_cpp(100, &[&unrelated]),
            LearnSkillProjectionLikeCpp::Covered(SpellLearnSkillSourceSpellInfoLikeCpp {
                spell_id: 100,
                difficulty_none: true,
                effects: Vec::new(),
            })
        );
    }

    #[test]
    fn learn_spell_projection_keeps_skill_step_and_checked_trigger_like_cpp() {
        let skill_step = acquisition_effect(1, 0, SPELL_EFFECT_SKILL_STEP, 0, 0);
        let learn_spell = acquisition_effect(2, 1, SPELL_EFFECT_LEARN_SPELL, 0, 200);

        let source =
            project_learn_spell_source_like_cpp(100, false, true, &[&skill_step, &learn_spell])
                .unwrap();

        assert!(source.has_skill_step_effect);
        assert!(source.is_passive);
        assert_eq!(
            source.learn_spell_effects,
            vec![SpellLearnSpellEffectLikeCpp {
                trigger_spell: 200,
                target_unit_pet: false,
            }]
        );
    }
}
