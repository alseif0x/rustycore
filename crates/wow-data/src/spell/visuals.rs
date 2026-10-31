//! Spell visual relation selection and visual-override potential metadata.

use super::*;
use crate::{
    ConditionPresenceCatalog,
    difficulty::DifficultyStore,
    spell_db2::{SpellXSpellVisualEntry, SpellXSpellVisualStore},
};

/// The caster categories that affect whether a stored caster condition applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellVisualCasterKind {
    NoCaster,
    Player,
    OtherUnit,
}

/// A known visual row needs predicate evaluation that this catalog does not own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellVisualSelectionError {
    ConditionPresenceUnavailable,
    PlayerConditionEvaluationUnavailable { condition_id: u32 },
    UnitConditionEvaluationUnavailable { condition_id: u16 },
}

/// Resolve the complete `SpellXSpellVisual` vector at the first difficulty in
/// the requested fallback chain that has any rows.
///
/// Source: `SpellMgr.cpp:2595-2605` inserts each row by descending
/// `CasterPlayerConditionID`; equal conditions insert before the older row,
/// producing descending `RecordID`. `SpellMgr.cpp:2680-2683` copies the first
/// nonempty fallback vector without merging later difficulties.
pub fn resolve_spell_visual_rows<'a>(
    store: &'a SpellXSpellVisualStore,
    spell_id: u32,
    requested_difficulty: u8,
    difficulties: Option<&DifficultyStore>,
) -> Result<Vec<&'a SpellXSpellVisualEntry>, DataSpellCatalogResolutionError> {
    let has_any_row = store
        .entries_like_cpp()
        .any(|entry| entry.spell_id == spell_id);

    let resolved = resolve_spell_catalog_value(
        requested_difficulty,
        difficulties,
        has_any_row,
        |difficulty_id| {
            let mut rows = store
                .entries_like_cpp()
                .filter(|entry| entry.spell_id == spell_id && entry.difficulty_id == difficulty_id)
                .collect::<Vec<_>>();
            rows.sort_unstable_by(|left, right| {
                right
                    .caster_player_condition_id
                    .cmp(&left.caster_player_condition_id)
                    .then_with(|| right.id.cmp(&left.id))
            });
            (!rows.is_empty()).then_some(rows)
        },
    )?;

    Ok(resolved.unwrap_or_default())
}

/// Select the first visual row whose condition records do not require an
/// unsupported predicate evaluation. Missing condition records are not active
/// constraints; a present record must be evaluated or skipped by caster kind.
pub fn select_spell_visual_id(
    ordered_rows: &[&SpellXSpellVisualEntry],
    condition_presence: Option<&ConditionPresenceCatalog>,
    caster_kind: SpellVisualCasterKind,
) -> Result<u32, SpellVisualSelectionError> {
    if ordered_rows.is_empty() {
        return Ok(0);
    }

    let condition_presence =
        condition_presence.ok_or(SpellVisualSelectionError::ConditionPresenceUnavailable)?;

    for row in ordered_rows {
        if condition_presence.contains_player_condition(row.caster_player_condition_id) {
            match caster_kind {
                SpellVisualCasterKind::Player => {
                    return Err(
                        SpellVisualSelectionError::PlayerConditionEvaluationUnavailable {
                            condition_id: row.caster_player_condition_id,
                        },
                    );
                }
                SpellVisualCasterKind::NoCaster | SpellVisualCasterKind::OtherUnit => continue,
            }
        }

        if condition_presence.contains_unit_condition(u32::from(row.caster_unit_condition_id)) {
            match caster_kind {
                SpellVisualCasterKind::NoCaster => continue,
                SpellVisualCasterKind::Player | SpellVisualCasterKind::OtherUnit => {
                    return Err(
                        SpellVisualSelectionError::UnitConditionEvaluationUnavailable {
                            condition_id: row.caster_unit_condition_id,
                        },
                    );
                }
            }
        }

        return Ok(row.id);
    }

    Ok(0)
}

const SPELL_AURA_OVERRIDE_SPELL_VISUAL: i32 = 403;

pub(super) fn is_visual_override_aura(effect_type: u32, aura_type: i32) -> bool {
    use spell_effect_types::*;

    aura_type == SPELL_AURA_OVERRIDE_SPELL_VISUAL
        && matches!(
            effect_type,
            SPELL_EFFECT_APPLY_AURA
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY
                | SPELL_EFFECT_APPLY_AREA_AURA_RAID
                | SPELL_EFFECT_APPLY_AREA_AURA_FRIEND
                | SPELL_EFFECT_APPLY_AREA_AURA_ENEMY
                | SPELL_EFFECT_APPLY_AREA_AURA_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_OWNER
                | SPELL_EFFECT_APPLY_AURA_ON_PET
                | SPELL_EFFECT_APPLY_AREA_AURA_SUMMONS
                | SPELL_EFFECT_APPLY_AREA_AURA_PARTY_NONRANDOM
                | SPELL_EFFECT_PERSISTENT_AREA_AURA
        )
}
