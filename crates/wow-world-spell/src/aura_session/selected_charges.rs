// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Resolve selected aura charges and prove fresh selected modifier absence.
//!
//! The explicit difficulty is the canonical selected `SpellInfo` difficulty,
//! not the raw Aura cast or map difficulty. Callers must resolve it from the
//! canonical spell key authority before using these helpers.

use wow_core::ObjectGuid;
use wow_data::{
    DifficultyStore, ServersideSpellInfoLikeCpp, ServersideSpellStoreLikeCpp,
    SpellAuraOptionsStore, SpellClassOptionsStore, SpellLabelStore, SpellProcStoreLikeCpp,
    SpellStore,
    spell::{
        DataSpellCatalogResolutionError, SpellProcKeyLikeCpp,
        aura_types::{
            SPELL_AURA_ADD_FLAT_MODIFIER, SPELL_AURA_ADD_PCT_MODIFIER,
            SPELL_AURA_ADD_PCT_MODIFIER_BY_SPELL_LABEL,
        },
        resolve_spell_effect_rows,
    },
};
use wow_entities::{AuraApplicationLikeCpp, AuraSubsystem};

use super::selected_catalog::{
    SelectedCatalogError, resolve_selected_value, selected_aura_options_entry,
    selected_spell_family,
};

const IGNORE_CASTER_MODIFIERS_ATTRIBUTE: u32 = 0x2000_0000;
const PROC_CHARGES_OPERATION: i32 = 4;
const CLASS_MODIFIER_ADMISSION_ERROR: &str =
    "Selected aura may have applicable class-mask SpellMods";
const LABEL_MODIFIER_ADMISSION_ERROR: &str = "Selected aura may have applicable label SpellMods";
const MODIFIER_ABSENCE_UNPROVEN: &str =
    "Selected aura modifier absence is not proven by canonical Player auras";

/// Resolve selected `Aura::CalcMaxCharges` metadata without caster modifiers.
///
/// This follows `SpellAuras.cpp:965-976,1088-1090` and
/// `SpellMgr.cpp:490-509` at source `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.
/// The difficulty argument must be the selected `SpellInfo` key's difficulty,
/// resolved by the canonical key authority; raw Aura/map difficulty is not a
/// substitute. A present proc entry overrides AuraOptions even when its
/// charge count is zero. Signed AuraOptions charges retain their full `u32`
/// bit pattern until the final `u8` conversion.
pub fn selected_aura_max_charges(
    spell_id: u32,
    actual_difficulty: u8,
    aura_options: Option<&SpellAuraOptionsStore>,
    proc_store: Option<&SpellProcStoreLikeCpp>,
    difficulties: Option<&DifficultyStore>,
) -> Result<u8, &'static str> {
    let aura_options =
        aura_options.ok_or("Missing effective SpellAuraOptions store for selected aura charges")?;
    let proc_store =
        proc_store.ok_or("Missing canonical SpellProc store for selected aura charges")?;

    let aura_options_charges =
        selected_aura_options_entry(aura_options, spell_id, actual_difficulty, difficulties)
            .map_err(catalog_error)?
            .map_or(0, |entry| entry.proc_charges as u32);

    let has_proc_row_at_u8_difficulty = proc_store
        .proc_entries_by_spell_and_difficulty
        .range(
            SpellProcKeyLikeCpp {
                spell_id,
                difficulty: u32::from(u8::MIN),
            }..=SpellProcKeyLikeCpp {
                spell_id,
                difficulty: u32::from(u8::MAX),
            },
        )
        .next()
        .is_some();

    let proc_entry = resolve_selected_value(
        actual_difficulty,
        difficulties,
        has_proc_row_at_u8_difficulty,
        |difficulty| proc_store.spell_proc_entry_like_cpp(spell_id, u32::from(difficulty)),
    )
    .map_err(catalog_error)?;

    Ok(proc_entry.map_or(aura_options_charges, |entry| entry.charges) as u8)
}

/// Report whether selected metadata permits a fast absence proof. This check
/// is independent of caster and active aura state; potential class-mask or
/// label modifiers remain rejected for the caller to assess from canonical
/// self-cast Player aura state. The general SpellMod registry and application
/// engine remain outside this proof.
///
/// `actual_difficulty` must be resolved from the canonical selected
/// `SpellInfo` key, not copied from raw Aura/map difficulty. The attribute
/// array is the exact selected SpellInfo attribute set.
pub(crate) fn validate_selected_aura_modifier_absence(
    spell_id: u32,
    actual_difficulty: u8,
    attributes: [u32; 15],
    class_options: Option<&SpellClassOptionsStore>,
    labels: Option<&SpellLabelStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<(), &'static str> {
    if attributes[3] & IGNORE_CASTER_MODIFIERS_ATTRIBUTE != 0 {
        return Ok(());
    }

    let class_options = class_options
        .ok_or("Missing effective SpellClassOptions for selected aura modifier admission")?;
    // SpellClassOptions stores the same uint32 SpellInfo key bits in a signed
    // field; preserve that identity for IDs with the high bit set.
    let signed_spell_id = spell_id as i32;

    let (_, family_flags) = selected_spell_family(
        class_options,
        signed_spell_id,
        actual_difficulty,
        difficulties,
    )
    .map_err(catalog_error)?;

    if family_flags.into_iter().any(|flags| flags != 0) {
        return Err(CLASS_MODIFIER_ADMISSION_ERROR);
    }

    let labels =
        labels.ok_or("Missing effective SpellLabel store for selected aura modifier admission")?;
    let has_label_row = labels
        .entries_like_cpp()
        .any(|entry| entry.spell_id == spell_id);
    if !has_label_row {
        return Ok(());
    }

    let resolved_label_presence =
        resolve_selected_value(actual_difficulty, difficulties, true, |difficulty| {
            (difficulty == 0).then_some(true)
        })
        .map_err(catalog_error)?;

    match resolved_label_presence {
        Some(true) => Err(LABEL_MODIFIER_ADMISSION_ERROR),
        Some(false) | None => Ok(()),
    }
}

/// Resolve the metadata-only potential-modifier rejection from complete
/// self-cast Player aura state. This remains an absence proof: a matching
/// modifier is rejected rather than applied here.
///
/// C++ `Aura::CalcMaxCharges` (`SpellAuras.cpp:965-976`) delegates
/// `SpellModOp::ProcCharges = 4` (`SpellDefines.h:143-149`) to
/// `Player::ApplySpellMod`; `AuraEffect::CalculateSpellMod`
/// (`SpellAuraEffects.cpp:885-925`) creates class-mask and label modifiers.
/// Applicability follows `SpellInfo::IsAffectedBySpellMod`
/// (`SpellInfo.cpp:1784-1818`) and `Player::IsAffectedBySpellmod`
/// (`Player.cpp:21361-21600`) at source `a5f8da2ebf5424bf0450ca4e08843ecbf72577bd`.
pub(crate) fn validate_selected_aura_modifier_absence_from_player_auras(
    spell_id: u32,
    actual_difficulty: u8,
    attributes: [u32; 15],
    caster_guid: ObjectGuid,
    canonical_player_guid: ObjectGuid,
    class_options: Option<&SpellClassOptionsStore>,
    labels: Option<&SpellLabelStore>,
    difficulties: Option<&DifficultyStore>,
    regular_spells: Option<&SpellStore>,
    serverside_spells: Option<&ServersideSpellStoreLikeCpp>,
    player_auras: Option<&AuraSubsystem>,
) -> Result<(), &'static str> {
    let potential_error = match validate_selected_aura_modifier_absence(
        spell_id,
        actual_difficulty,
        attributes,
        class_options,
        labels,
        difficulties,
    ) {
        Ok(()) => return Ok(()),
        Err(error)
            if error == CLASS_MODIFIER_ADMISSION_ERROR
                || error == LABEL_MODIFIER_ADMISSION_ERROR =>
        {
            error
        }
        Err(error) => return Err(error),
    };

    if caster_guid != canonical_player_guid {
        return Err(potential_error);
    }
    let player_auras = player_auras.ok_or(MODIFIER_ABSENCE_UNPROVEN)?;
    if !player_auras.persisted_player_aura_authority_complete_like_cpp()
        || player_auras.spell_hit_aura_authority_tombstoned_like_cpp()
    {
        return Err(MODIFIER_ABSENCE_UNPROVEN);
    }

    let class_options = class_options.ok_or(MODIFIER_ABSENCE_UNPROVEN)?;
    let labels = labels.ok_or(MODIFIER_ABSENCE_UNPROVEN)?;
    let (target_family, target_flags) = selected_spell_family(
        class_options,
        spell_id as i32,
        actual_difficulty,
        difficulties,
    )
    .map_err(catalog_error)?;
    let target_labels = selected_native_labels(spell_id, actual_difficulty, labels, difficulties)?;

    for aura in player_auras.runtime_applications_like_cpp().values() {
        let effects =
            resolve_active_aura_effects(aura, regular_spells, serverside_spells, difficulties)?;

        if let Some(modifier_error) = selected_modifier_match(
            aura.effect_mask,
            &effects,
            aura.spell_id,
            actual_difficulty,
            target_family,
            target_flags,
            &target_labels,
            class_options,
            regular_spells,
            serverside_spells,
            difficulties,
        )? {
            return Err(modifier_error);
        }
    }

    Ok(())
}

fn selected_native_labels(
    spell_id: u32,
    actual_difficulty: u8,
    labels: &SpellLabelStore,
    difficulties: Option<&DifficultyStore>,
) -> Result<Vec<u32>, &'static str> {
    let spell_labels = labels
        .entries_like_cpp()
        .filter(|entry| entry.spell_id == spell_id)
        .map(|entry| entry.label_id)
        .collect::<Vec<_>>();
    if spell_labels.is_empty() {
        return Ok(Vec::new());
    }

    let selected = resolve_selected_value(actual_difficulty, difficulties, true, |difficulty| {
        (difficulty == 0).then_some(true)
    })
    .map_err(catalog_error)?;
    Ok(if selected == Some(true) {
        spell_labels
    } else {
        Vec::new()
    })
}

#[derive(Clone, Copy)]
struct SelectedAuraEffect {
    index: u32,
    effect_type: i64,
    aura_type: i32,
    misc_value_1: i32,
    misc_value_2: i32,
    class_mask: [u32; 4],
}

enum SelectedSpellSource<'a> {
    Regular(u8),
    Server(&'a ServersideSpellInfoLikeCpp, u8),
}

fn select_spell_source<'a>(
    spell_id: u32,
    requested_difficulty: u8,
    regular_spells: Option<&'a SpellStore>,
    serverside_spells: Option<&'a ServersideSpellStoreLikeCpp>,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<SelectedSpellSource<'a>>, &'static str> {
    if regular_spells.is_none() && serverside_spells.is_none() {
        return Err(MODIFIER_ABSENCE_UNPROVEN);
    }
    let has_any_key = regular_spells
        .is_some_and(|store| store.contains_spell_info_any_difficulty_like_cpp(spell_id))
        || serverside_spells.is_some_and(|store| {
            (u8::MIN..=u8::MAX).any(|difficulty| {
                store
                    .get_serverside_spell_like_cpp(spell_id, u32::from(difficulty))
                    .is_some()
            })
        });

    wow_data::spell::resolve_spell_catalog_value(
        requested_difficulty,
        difficulties,
        has_any_key,
        |difficulty| {
            let regular_key = regular_spells.is_some_and(|store| {
                store.contains_spell_info_exact_like_cpp(spell_id, difficulty)
            });
            let server_spell = serverside_spells.and_then(|store| {
                store.get_serverside_spell_like_cpp(spell_id, u32::from(difficulty))
            });
            match (regular_key, server_spell) {
                (true, Some(_)) => Some(Err(MODIFIER_ABSENCE_UNPROVEN)),
                (true, None) => Some(Ok(SelectedSpellSource::Regular(difficulty))),
                (false, Some(spell)) => Some(Ok(SelectedSpellSource::Server(spell, difficulty))),
                (false, None) => None,
            }
        },
    )
    .map_err(catalog_error)?
    .transpose()
}

fn resolve_active_aura_effects(
    aura: &AuraApplicationLikeCpp,
    regular_spells: Option<&SpellStore>,
    serverside_spells: Option<&ServersideSpellStoreLikeCpp>,
    difficulties: Option<&DifficultyStore>,
) -> Result<Vec<SelectedAuraEffect>, &'static str> {
    let spell_id = aura.spell_id as u32;
    let source = select_spell_source(
        spell_id,
        aura.difficulty_id,
        regular_spells,
        serverside_spells,
        difficulties,
    )?
    .ok_or(MODIFIER_ABSENCE_UNPROVEN)?;

    match source {
        SelectedSpellSource::Regular(actual_difficulty) => {
            let store = regular_spells.ok_or(MODIFIER_ABSENCE_UNPROVEN)?;
            let rows =
                resolve_spell_effect_rows(store, aura.spell_id, actual_difficulty, difficulties)
                    .map_err(|_| MODIFIER_ABSENCE_UNPROVEN)?;
            Ok(rows
                .into_iter()
                .map(|effect| SelectedAuraEffect {
                    index: effect.effect_index,
                    effect_type: i64::from(effect.effect),
                    aura_type: effect.effect_aura,
                    misc_value_1: effect.effect_misc_value_1,
                    misc_value_2: effect.effect_misc_value_2,
                    class_mask: effect.effect_spell_class_mask,
                })
                .collect())
        }
        SelectedSpellSource::Server(spell, actual_difficulty) => {
            server_effect_rows(spell, actual_difficulty)
        }
    }
}

fn selected_source_family(
    spell_id: i32,
    target_difficulty: u8,
    regular_spells: Option<&SpellStore>,
    serverside_spells: Option<&ServersideSpellStoreLikeCpp>,
    class_options: &SpellClassOptionsStore,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<u32>, &'static str> {
    let Some(source) = select_spell_source(
        spell_id as u32,
        target_difficulty,
        regular_spells,
        serverside_spells,
        difficulties,
    )?
    else {
        return Ok(None);
    };
    match source {
        SelectedSpellSource::Regular(source_difficulty) => {
            let (family, _) =
                selected_spell_family(class_options, spell_id, source_difficulty, difficulties)
                    .map_err(catalog_error)?;
            Ok(Some(u32::from(family)))
        }
        SelectedSpellSource::Server(spell, _) => Ok(Some(spell.row.spell_family_name)),
    }
}

fn server_effect_rows(
    spell: &ServersideSpellInfoLikeCpp,
    actual_difficulty: u8,
) -> Result<Vec<SelectedAuraEffect>, &'static str> {
    let mut seen = [false; 32];
    let mut effects = Vec::with_capacity(spell.effects.len());
    for effect in &spell.effects {
        if effect.difficulty_id != u32::from(actual_difficulty) {
            return Err(MODIFIER_ABSENCE_UNPROVEN);
        }
        let index = usize::try_from(effect.effect_index).map_err(|_| MODIFIER_ABSENCE_UNPROVEN)?;
        let Some(seen_effect) = seen.get_mut(index) else {
            return Err(MODIFIER_ABSENCE_UNPROVEN);
        };
        if *seen_effect {
            return Err(MODIFIER_ABSENCE_UNPROVEN);
        }
        *seen_effect = true;
        effects.push(SelectedAuraEffect {
            index: index as u32,
            effect_type: i64::from(effect.effect),
            aura_type: effect.effect_aura,
            misc_value_1: effect.effect_misc_value[0],
            misc_value_2: effect.effect_misc_value[1],
            class_mask: effect.effect_spell_class_mask.map(|mask| mask as u32),
        });
    }
    Ok(effects)
}

#[allow(clippy::too_many_arguments)]
fn selected_modifier_match(
    active_effect_mask: u32,
    effects: &[SelectedAuraEffect],
    source_spell_id: i32,
    target_difficulty: u8,
    target_family: u16,
    target_flags: [u32; 4],
    target_labels: &[u32],
    class_options: &SpellClassOptionsStore,
    regular_spells: Option<&SpellStore>,
    serverside_spells: Option<&ServersideSpellStoreLikeCpp>,
    difficulties: Option<&DifficultyStore>,
) -> Result<Option<&'static str>, &'static str> {
    let mut seen = [false; 32];
    for effect in effects {
        let index = usize::try_from(effect.index).map_err(|_| MODIFIER_ABSENCE_UNPROVEN)?;
        let Some(seen_effect) = seen.get_mut(index) else {
            return Err(MODIFIER_ABSENCE_UNPROVEN);
        };
        if *seen_effect {
            return Err(MODIFIER_ABSENCE_UNPROVEN);
        }
        *seen_effect = true;
    }

    for index in 0..32 {
        if active_effect_mask & (1_u32 << index) == 0 {
            continue;
        }
        let effect = effects
            .iter()
            .find(|effect| effect.index == index)
            .filter(|effect| effect.effect_type > 0)
            .ok_or(MODIFIER_ABSENCE_UNPROVEN)?;
        if effect.misc_value_1 != PROC_CHARGES_OPERATION {
            continue;
        }

        match effect.aura_type {
            SPELL_AURA_ADD_FLAT_MODIFIER | SPELL_AURA_ADD_PCT_MODIFIER => {
                if !target_flags
                    .iter()
                    .zip(effect.class_mask)
                    .any(|(target, source)| *target & source != 0)
                {
                    continue;
                }
                // C++ looks up mod->spellId at the selected target
                // SpellInfo::Difficulty before comparing family names
                // (`SpellInfo.cpp:1794`). Effect metadata above instead uses
                // this aura application's own difficulty.
                let Some(source_family) = selected_source_family(
                    source_spell_id,
                    target_difficulty,
                    regular_spells,
                    serverside_spells,
                    class_options,
                    difficulties,
                )?
                else {
                    continue;
                };
                if source_family == u32::from(target_family) {
                    return Ok(Some(CLASS_MODIFIER_ADMISSION_ERROR));
                }
            }
            SPELL_AURA_ADD_PCT_MODIFIER_BY_SPELL_LABEL => {
                if target_labels.contains(&(effect.misc_value_2 as u32))
                    && select_spell_source(
                        source_spell_id as u32,
                        target_difficulty,
                        regular_spells,
                        serverside_spells,
                        difficulties,
                    )?
                    .is_some()
                {
                    return Ok(Some(LABEL_MODIFIER_ADMISSION_ERROR));
                }
            }
            _ => {}
        }
    }
    Ok(None)
}

fn catalog_error(error: SelectedCatalogError) -> &'static str {
    match error {
        DataSpellCatalogResolutionError::DifficultyUnavailable => {
            "Selected spell catalog difficulty is unavailable"
        }
        DataSpellCatalogResolutionError::DifficultyCycle => {
            "Selected spell catalog difficulty chain contains a cycle"
        }
    }
}
