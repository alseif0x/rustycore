use super::*;

#[derive(Debug)]
pub(crate) struct TrainerSpellStaticAuthorityLikeCpp {
    pub(crate) safe_cast_spell_ids: BTreeSet<u32>,
    pub(crate) valid_craft_spell_ids: BTreeSet<u32>,
    /// Exact positive `spell_script_names.spell_id` bindings.
    ///
    /// These process-wide sets are also consumed by the spell-hit aura
    /// authority. Keeping the raw exact/all-ranks distinction lets each
    /// session resolve the final rank through the same effective
    /// `SpellChainStoreLikeCpp` instead of treating an empty runtime script
    /// registry as source proof.
    pub(crate) spell_script_exact_spell_ids: BTreeSet<u32>,
    /// Absolute roots from negative `spell_script_names.spell_id` rows.
    pub(crate) spell_script_all_rank_root_spell_ids: BTreeSet<u32>,
    /// C++ `spell_scripts.id & 0x00FF_FFFF` bindings.
    pub(crate) legacy_spell_script_spell_ids: BTreeSet<u32>,
}

#[derive(Debug, Clone, Copy, Default)]
struct TrainerCastWorldHookAuditLikeCpp {
    script_binding: bool,
    legacy_script: bool,
    condition: bool,
    aura_restriction: bool,
    equipped_item_restriction: bool,
    channeled: bool,
    spell_focus_requirement: bool,
    required_area_requirement: bool,
    spell_area_requirement: bool,
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
        || audit.aura_restriction
        || audit.equipped_item_restriction
        || audit.channeled
        || audit.spell_focus_requirement
        || audit.required_area_requirement
        || audit.spell_area_requirement
        || audit.linked_spell)
}

fn trainer_cast_effective_casting_requirement_audit_like_cpp(
    spell_info_present: bool,
    requirements: Option<&SpellCastingRequirementsEntry>,
) -> (bool, bool) {
    (
        !spell_info_present
            || requirements.is_some_and(|requirements| requirements.requires_spell_focus != 0),
        requirements.is_some_and(|requirements| requirements.required_areas_id != 0),
    )
}

fn trainer_cast_has_unsupported_effective_aura_state_restriction_like_cpp(
    entry: &SpellAuraRestrictionsEntry,
) -> bool {
    entry.caster_aura_state != 0
        || entry.target_aura_state != 0
        || entry.exclude_caster_aura_state != 0
        || entry.exclude_target_aura_state != 0
}

fn trainer_cast_has_unsupported_difficulty_none_aura_state_restriction_like_cpp(
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
    let selected_difficulty = if has_exact { 0_u32 } else { u32::from(u8::MAX) };
    store
        .resolved_for_difficulty_chain_like_cpp(spell_id, [selected_difficulty])
        .is_some_and(trainer_cast_has_unsupported_effective_aura_state_restriction_like_cpp)
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
/// The difficulty-specific target restriction and per-player immunity/effect
/// mask are resolved separately by `wow-world` immediately before the atomic
/// trainer commit. Spell disables deliberately remain in that dynamic half:
/// C++ `DisableMgr::IsDisabledFor` evaluates caster type, map, area, arena,
/// and battleground state, so excluding every spell that merely has a
/// `disables` row here would reject casts that C++ permits outside that row's
/// scope. Missing runtime disable authority fails closed in the same adapter.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn load_trainer_static_authority_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellAcquisitionStartupPersistencePortLikeCpp,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    spell_store: &SpellStore,
    spell_chains: &SpellChainStoreLikeCpp,
    catalog: &SpellAcquisitionCatalogLikeCpp,
    linked: &SpellLinkedStoreLikeCpp,
    pet_auras: &SpellPetAuraStoreLikeCpp,
    aura_restrictions: &SpellAuraRestrictionsStore,
    casting_requirements: &SpellCastingRequirementsStore,
    equipped_items: &SpellEquippedItemsStore,
    spell_areas: &SpellAreaStoreLikeCpp,
    item_exists: impl Fn(u32) -> bool,
) -> Result<TrainerSpellStaticAuthorityLikeCpp> {
    let audit = loaded_startup_like_cpp(persistence.load_trainer_spell_audit_like_cpp().await)
        .context("Failed to audit trainer spell World hooks")?;
    let mut script_bindings = TrainerSpellScriptBindingsLikeCpp::default();
    for id in audit.script_binding_ids {
        if id > 0 {
            script_bindings.exact_spell_ids.insert(id as u32);
        } else if let Some(id) = id.checked_abs().and_then(|id| u32::try_from(id).ok())
            && id != 0
        {
            script_bindings.all_rank_root_spell_ids.insert(id);
        }
    }
    let legacy_scripts = audit.legacy_script_ids.into_iter().collect::<BTreeSet<_>>();
    let conditions = audit
        .condition_spell_ids
        .into_iter()
        .filter_map(|id| id.checked_abs().and_then(|id| u32::try_from(id).ok()))
        .collect::<BTreeSet<_>>();
    let effective_reagents =
        load_effective_spell_reagents_like_cpp(data_dir, locale, persistence, removals)
            .await
            .context("Failed to compose effective trainer craft reagents")?;

    let mut safe_cast_spell_ids = BTreeSet::new();
    for (spell_id, difficulty_id) in spell_store.spell_info_keys_in_order_like_cpp() {
        if difficulty_id != 0 {
            continue;
        }
        let spell_id_i32 = i32::try_from(spell_id).ok();
        let effective_casting_requirements = spell_id_i32
            .and_then(|spell_id| casting_requirements.entry_for_spell_id_like_cpp(spell_id));
        let (spell_focus_requirement, required_area_requirement) =
            trainer_cast_effective_casting_requirement_audit_like_cpp(
                spell_id_i32
                    .and_then(|spell_id| spell_store.get(spell_id))
                    .is_some(),
                effective_casting_requirements,
            );
        // Shapeshift metadata is intentionally absent from this audit. C++
        // `Trainer::TeachSpell` invokes `CastSpell(..., true)`; the bool
        // constructor selects `TRIGGERED_FULL_MASK`, which contains
        // `TRIGGERED_IGNORE_SHAPESHIFT`, so `Spell::CheckCast` does not call
        // `SpellInfo::CheckShapeshift` for the trainer wrapper. Rejecting
        // stance-gated wrappers here would therefore be stricter than C++.
        let hook_audit = TrainerCastWorldHookAuditLikeCpp {
            script_binding: script_bindings.contains_like_cpp(
                spell_id,
                spell_chains.first_spell_in_chain_like_cpp(spell_id),
            ),
            legacy_script: legacy_scripts.contains(&spell_id),
            condition: conditions.contains(&spell_id),
            // Aura-spell presence/exclusion gates are resolved from the
            // complete player aura authority at purchase time. Unit
            // AuraState remains outside the reduced projection and is the
            // only SpellAuraRestrictions shape that blocks static authority.
            aura_restriction:
                trainer_cast_has_unsupported_difficulty_none_aura_state_restriction_like_cpp(
                    aura_restrictions,
                    spell_id,
                ),
            equipped_item_restriction: equipped_items
                .entry_for_spell_id_like_cpp(i32::try_from(spell_id).unwrap_or(i32::MAX))
                .is_some_and(trainer_cast_has_effective_equipped_item_restriction_like_cpp),
            // The reduced trainer projection does not own channel state or
            // periodic channel updates. Audit the final effective SpellMisc
            // attributes, including official/custom hotfix overlays.
            channeled: spell_id_i32.is_none_or(|spell_id| {
                spell_store.get(spell_id).is_none() || spell_store.is_channeled_like_cpp(spell_id)
            }) || match catalog
                .misc_for_spell_like_cpp(spell_id, u32::from(difficulty_id))
            {
                SpellAcquisitionMetadataLookupLikeCpp::Present(misc) => {
                    misc.is_channeled_checked().unwrap_or(true)
                }
                SpellAcquisitionMetadataLookupLikeCpp::CoveredWithoutRow => false,
                SpellAcquisitionMetadataLookupLikeCpp::MissingCoverage
                | SpellAcquisitionMetadataLookupLikeCpp::Indeterminate(_) => true,
            },
            // The reduced trainer path does not execute C++ `SearchSpellFocus`.
            // C++ builds SpellInfo from the effective SpellCastingRequirements
            // row, so the audit must use that same overlay-composed authority.
            // A missing SpellInfo key remains indeterminate and fails closed.
            spell_focus_requirement,
            // C++ copies DB2 `RequiredAreasID` into SpellInfo and always runs
            // `CheckLocation`, including for this triggered trainer cast.
            // The reduced path cannot resolve AreaGroupMember ancestry yet,
            // so such wrappers must remain outside static authority.
            required_area_requirement,
            // C++ `SpellInfo::CheckLocation` requires at least one matching
            // `spell_area` row whenever the spell has any. The reduced
            // trainer projection does not yet evaluate the player's complete
            // zone/quest/aura/race/gender context, so those wrappers fail
            // closed rather than bypassing the cast gate.
            spell_area_requirement: !spell_areas
                .spell_area_map_bounds_like_cpp(spell_id)
                .is_empty(),
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
        spell_script_exact_spell_ids: script_bindings.exact_spell_ids,
        spell_script_all_rank_root_spell_ids: script_bindings.all_rank_root_spell_ids,
        legacy_spell_script_spell_ids: legacy_scripts,
    })
}

async fn load_effective_spell_reagents_like_cpp(
    data_dir: &str,
    locale: &str,
    persistence: &dyn SpellAcquisitionStartupPersistencePortLikeCpp,
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
        overlay_batches[batch_index] = loaded_startup_like_cpp(
            persistence
                .load_spell_reagents_overlay_like_cpp(official)
                .await,
        )?
        .into_iter()
        .map(|row| SpellReagentsEntry {
            id: row.id,
            spell_id: row.spell_id,
            reagent: row.reagent,
            reagent_count: row.reagent_count,
        })
        .collect();
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


#[cfg(test)]
mod tests;
