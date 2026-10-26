// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Resolve selected aura charges and bound fresh self-caster modifier admission.
//!
//! The explicit difficulty is the canonical selected `SpellInfo` difficulty,
//! not the raw Aura cast or map difficulty. Callers must resolve it from the
//! canonical spell key authority before using these helpers.

use wow_data::{
    DifficultyStore, SpellAuraOptionsStore, SpellClassOptionsStore, SpellLabelStore,
    SpellProcStoreLikeCpp,
    spell::{DataSpellCatalogResolutionError, SpellProcKeyLikeCpp},
};

use super::selected_catalog::{
    SelectedCatalogError, resolve_selected_value, selected_aura_options_entry,
    selected_spell_family,
};

const IGNORE_CASTER_MODIFIERS_ATTRIBUTE: u32 = 0x2000_0000;

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
    let aura_options = aura_options
        .ok_or("Missing effective SpellAuraOptions store for selected aura charges")?;
    let proc_store = proc_store
        .ok_or("Missing canonical SpellProc store for selected aura charges")?;

    let aura_options_charges = selected_aura_options_entry(
        aura_options,
        spell_id,
        actual_difficulty,
        difficulties,
    )
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

/// Admit fresh selected self-cast charge calculation only when no unresolved
/// SpellMod can apply. This is deliberately limited to the selected Player
/// casting on itself; `Object.cpp:2274` establishes that Player as its own
/// modifier owner. The general SpellMod registry and application engine remain
/// outside this proof.
///
/// `actual_difficulty` must be resolved from the canonical selected
/// `SpellInfo` key, not copied from raw Aura/map difficulty. The attribute
/// array is the exact selected SpellInfo attribute set.
pub(crate) fn validate_fresh_selected_self_caster_modifiers(
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
        .ok_or("Missing effective SpellClassOptions for selected self-cast modifier admission")?;
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
        return Err("Selected self-cast may have applicable class-mask SpellMods");
    }

    let labels = labels
        .ok_or("Missing effective SpellLabel store for selected self-cast modifier admission")?;
    let has_label_row = labels
        .entries_like_cpp()
        .any(|entry| entry.spell_id == spell_id);
    if !has_label_row {
        return Ok(());
    }

    let resolved_label_presence = resolve_selected_value(
        actual_difficulty,
        difficulties,
        true,
        |difficulty| (difficulty == 0).then_some(true),
    )
    .map_err(catalog_error)?;

    match resolved_label_presence {
        Some(true) => Err("Selected self-cast may have applicable label SpellMods"),
        Some(false) | None => Ok(()),
    }
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
