// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Source Aura visual selection for the bounded selected-aura consumers.

use wow_core::ObjectGuid;
use wow_data::{
    ConditionPresenceCatalog, DifficultyStore, ServersideSpellStoreLikeCpp, SpellStore,
    spell::{SpellVisualCasterKind, SpellVisualSelectionError},
    spell_db2::SpellXSpellVisualStore,
};
use wow_map::{MapManager, PlayerHandle};

use super::selected_reaction::{
    SelectedUnit, selected_owner_context, selected_owner_map, with_unit_in_map,
};

/// Resolve the source `Aura` visual from its raw caster GUID and immutable catalogs.
///
/// The caller supplies its already-held canonical MapManager guard. Runtime
/// aura-403 state is intentionally not read here; either catalog reporting
/// override potential leaves the active override source and order unresolved.
pub(crate) fn selected_aura_spell_visual_id(
    manager: &MapManager,
    owner: PlayerHandle,
    raw_caster_guid: ObjectGuid,
    spell_id: u32,
    difficulty_id: u8,
    spell_store: Option<&SpellStore>,
    serverside_spell_store: Option<&ServersideSpellStoreLikeCpp>,
    visual_store: Option<&SpellXSpellVisualStore>,
    condition_presence: Option<&ConditionPresenceCatalog>,
    difficulties: Option<&DifficultyStore>,
) -> Result<u32, &'static str> {
    let owner_context = selected_owner_context(manager, owner)
        .map_err(|_| "Canonical selected aura owner is unavailable for visual selection")?;

    let caster_kind = if raw_caster_guid == owner.guid() {
        SpellVisualCasterKind::Player
    } else if !raw_caster_guid.is_unit() {
        SpellVisualCasterKind::NoCaster
    } else {
        let map_key = selected_owner_map(manager, owner, owner_context)
            .map_err(|_| "Selected aura caster map is unavailable for visual selection")?;
        let caster = with_unit_in_map(manager, map_key, raw_caster_guid, |unit| match unit {
            SelectedUnit::Player(_) => SpellVisualCasterKind::Player,
            SelectedUnit::Creature { .. } => SpellVisualCasterKind::OtherUnit,
        })
        .map_err(|_| "Selected aura caster is inconsistent for visual selection")?;
        caster.unwrap_or(SpellVisualCasterKind::NoCaster)
    };

    if caster_kind != SpellVisualCasterKind::NoCaster {
        let spell_store = spell_store
            .ok_or("Missing complete SpellStore for selected aura visual override check")?;
        let serverside_spell_store = serverside_spell_store
            .ok_or("Missing serverside spell catalog for selected aura visual override check")?;
        let regular_override = spell_store.may_override_spell_visual(spell_id);
        let serverside_override = serverside_spell_store.may_override_spell_visual(spell_id);
        if regular_override || serverside_override {
            return Err("Selected aura visual override source/order is unavailable");
        }
    }

    let visual_store =
        visual_store.ok_or("Missing SpellXSpellVisual store for selected aura creation")?;
    let rows = wow_data::spell::resolve_spell_visual_rows(
        visual_store,
        spell_id,
        difficulty_id,
        difficulties,
    )
    .map_err(|_| "Unable to resolve selected aura SpellXSpellVisual difficulty")?;

    wow_data::spell::select_spell_visual_id(&rows, condition_presence, caster_kind).map_err(
        |error| match error {
            SpellVisualSelectionError::ConditionPresenceUnavailable => {
                "Missing ConditionPresenceCatalog for selected aura visual"
            }
            SpellVisualSelectionError::PlayerConditionEvaluationUnavailable { .. } => {
                "Selected aura player condition evaluation is unavailable"
            }
            SpellVisualSelectionError::UnitConditionEvaluationUnavailable { .. } => {
                "Selected aura unit condition evaluation is unavailable"
            }
        },
    )
}
