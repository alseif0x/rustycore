use wow_core::ObjectGuid;
use wow_data::{DifficultyStore, SpellLevelsStore};
use wow_map::{MapManager, PlayerHandle};

use super::selected_catalog::{SelectedCatalogError, selected_spell_level};
use super::selected_reaction::{selected_owner_context, selected_owner_map, with_unit_in_map};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectedCasterLevelError {
    OwnerUnavailable,
    OwnerMapUnavailable,
    CasterUnavailable,
    SpellLevelsUnavailable,
    DifficultyUnavailable,
    DifficultyCycle,
    ReactionUnavailable,
}

/// Resolve the caster level retained by a newly-created selected Aura.
///
/// C++ `SpellAuras.cpp:398-410,463-468` first resolves the raw caster in the
/// target Unit's map context, then captures either that Unit's current level or
/// the selected SpellInfo's known SpellLevel. The caller supplies the already
/// borrowed canonical map owner so this lookup needs no session or directory.
pub(crate) fn selected_caster_level(
    manager: &MapManager,
    owner: PlayerHandle,
    raw_caster_guid: ObjectGuid,
    spell_id: u32,
    difficulty_id: u8,
    spell_levels: Option<&SpellLevelsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<u8, SelectedCasterLevelError> {
    let owner_snapshot = selected_owner_context(manager, owner)?;

    if raw_caster_guid == owner.guid() {
        return Ok(owner_snapshot.level);
    }

    if !raw_caster_guid.is_unit() {
        return spell_level_fallback(spell_id, difficulty_id, spell_levels, difficulties);
    }

    let owner_map_key = selected_owner_map(manager, owner, owner_snapshot)?;
    if let Some(level) = with_unit_in_map(manager, owner_map_key, raw_caster_guid, |unit| {
        unit.unit().data().level as u8
    })? {
        return Ok(level);
    }

    spell_level_fallback(spell_id, difficulty_id, spell_levels, difficulties)
}

fn spell_level_fallback(
    spell_id: u32,
    difficulty_id: u8,
    spell_levels: Option<&SpellLevelsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<u8, SelectedCasterLevelError> {
    let spell_levels = spell_levels.ok_or(SelectedCasterLevelError::SpellLevelsUnavailable)?;
    selected_spell_level(spell_levels, spell_id, difficulty_id, difficulties).map_err(|error| {
        match error {
            SelectedCatalogError::DifficultyUnavailable => {
                SelectedCasterLevelError::DifficultyUnavailable
            }
            SelectedCatalogError::DifficultyCycle => SelectedCasterLevelError::DifficultyCycle,
        }
    })
}
