// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player spell-map dispatch and its spell-row, trait and override
//! operations, moved from the World `session/spell_state/spell.rs` (#1363
//! slice 9a).

use super::*;
use crate::{RepresentedPlayerSpellRuntimeLikeCpp, represented_player_spell_runtime_like_cpp};

pub fn represented_talent_spell_id_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    talent_id: u32,
    rank: u8,
) -> Option<i32> {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_talent_spell_id_like_cpp(hub, talent_id, rank)
}
pub fn represented_talent_override_spell_pair_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    talent_id: u32,
) -> Option<(i32, i32)> {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_talent_override_spell_pair_like_cpp(hub, talent_id)
}
/// Take one spell's trait definition id (C++ `Player::RemoveSpell`).
pub fn take_represented_trait_definition_id_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
) -> Option<i32> {
    host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.take_trait_definition_id_like_cpp(spell_id)
    })
    .flatten()
}
/// Install one loaded spell row, keeping the authoritative set when it
/// exists and otherwise retaining it as a fallback grant.
pub fn install_loaded_spell_row_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    row: wow_entities::PlayerKnownSpellRecord,
    complete_rows: Option<std::collections::BTreeMap<i32, wow_entities::PlayerKnownSpellRecord>>,
) -> bool {
    host.mutate_player_spell_runtime_like_cpp(|runtime| {
        if let Some(mut rows) = complete_rows {
            rows.insert(spell_id, row);
            runtime.replace_rows_like_cpp(rows, true);
        } else {
            runtime.insert_fallback_row_like_cpp(spell_id, row);
        }
    })
    .is_some()
}
/// Drop the trait definitions and config headers a login reset discards.
pub fn begin_represented_trait_config_load_like_cpp<H: SpellbookHostLikeCpp>(host: &mut H) -> bool {
    host.mutate_player_spell_runtime_like_cpp(
        wow_entities::PlayerSpellRuntimeState::begin_trait_config_load_like_cpp,
    )
    .is_some()
}
/// Complete one represented trait-config load.
pub fn complete_represented_trait_config_rows_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    configs: Vec<(i32, i32, i32, i32)>,
    entries_empty: bool,
) -> Option<bool> {
    host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.complete_trait_config_load_like_cpp(configs, entries_empty)
    })
}
/// Drop every trait and override edge, as login does before rebuilding a
/// fresh C++ Player.
pub fn clear_represented_trait_and_override_state_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
) -> bool {
    host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.clear_trait_and_override_state_like_cpp();
    })
    .is_some()
}
/// Drop the loaded trait-config headers and return them to unhydrated.
pub fn clear_represented_trait_config_rows_like_cpp<H: SpellbookHostLikeCpp>(host: &mut H) -> bool {
    host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.clear_trait_config_rows_like_cpp();
    })
    .is_some()
}
/// Forget the rows retained while no authoritative load exists.
pub fn clear_represented_fallback_spell_rows_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
) -> bool {
    host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.clear_fallback_rows_like_cpp();
    })
    .is_some()
}
pub fn invalidate_represented_player_spell_rows_like_cpp<H: SpellbookHostLikeCpp>(host: &mut H) {
    let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.clear_rows_like_cpp();
    });
    invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp(host);
}
pub fn set_complete_represented_player_spell_rows_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
) -> bool {
    replace_loaded_represented_player_spell_rows_like_cpp(host, rows, true)
}
pub fn replace_loaded_represented_player_spell_rows_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    rows: impl IntoIterator<Item = RepresentedPlayerSpellLikeCpp>,
    complete: bool,
) -> bool {
    invalidate_represented_spell_acquisition_auxiliary_authority_like_cpp(host);
    let mut exact_rows = BTreeMap::new();
    for row in rows {
        if row.spell_id <= 0
            || exact_rows
                .insert(row.spell_id, canonical_player_spell_record_like_cpp(row))
                .is_some()
        {
            invalidate_represented_player_spell_rows_like_cpp(host);
            return false;
        }
    }
    host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.replace_loaded_spell_rows_like_cpp(exact_rows, complete);
    })
    .is_some()
}
pub(super) fn cleanup_removed_spell_titan_grip_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
) {
    let Some(spell_store) = host.hub_ref().catalogs.spell_store() else {
        return;
    };
    let Some(spell_info) = spell_store.get(spell_id) else {
        return;
    };
    if !spell_store.is_passive_like_cpp(spell_id)
        || !spell_info
            .has_effect_like_cpp(wow_data::spell::spell_effect_types::SPELL_EFFECT_TITAN_GRIP)
    {
        return;
    }

    let Some(penalty_spell_id) = host.core().mutate_canonical_player_like_cpp(|player| {
        player
            .can_titan_grip()
            .then_some(player.titan_grip_penalty_spell_id())
    }) else {
        return;
    };
    let Some(penalty_spell_id) = penalty_spell_id else {
        return;
    };

    if penalty_spell_id > 0 {
        let _ = host.remove_represented_auras_due_to_spell_like_cpp(
            i32::try_from(penalty_spell_id).unwrap_or(i32::MAX),
        );
    }

    let _ = host.core().mutate_canonical_player_like_cpp(|player| {
        player.set_can_titan_grip(false, 0);
    });
}
pub fn add_represented_override_spell_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    overriden_spell_id: i32,
    new_spell_id: i32,
) {
    if overriden_spell_id <= 0 || new_spell_id <= 0 {
        return;
    }
    let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.add_override_spell_like_cpp(overriden_spell_id, new_spell_id);
    });
}
pub fn remove_represented_override_spell_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    overriden_spell_id: i32,
    new_spell_id: i32,
) {
    let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.remove_override_spell_like_cpp(overriden_spell_id, new_spell_id);
    });
}
pub fn represented_override_spells_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> HashMap<i32, BTreeSet<i32>> {
    host.with_player_spell_runtime_like_cpp(|runtime| {
        runtime
            .override_spells_like_cpp()
            .iter()
            .map(|(&id, values)| (id, values.clone()))
            .collect()
    })
    .unwrap_or_default()
}
pub fn complete_represented_override_spells_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> Option<HashMap<i32, BTreeSet<i32>>> {
    host.with_player_spell_runtime_like_cpp(|runtime| {
        runtime.override_spells_complete_like_cpp().then(|| {
            runtime
                .override_spells_like_cpp()
                .iter()
                .map(|(&id, values)| (id, values.clone()))
                .collect()
        })
    })
    .flatten()
}
pub fn complete_represented_spell_trait_definition_ids_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> Option<HashMap<i32, i32>> {
    host.with_player_spell_runtime_like_cpp(|runtime| {
        runtime.trait_definition_ids_complete_like_cpp().then(|| {
            runtime
                .trait_definition_ids_like_cpp()
                .iter()
                .map(|(&id, &value)| (id, value))
                .collect()
        })
    })
    .flatten()
}
pub fn set_complete_represented_spell_trait_definition_ids_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    traits: impl IntoIterator<Item = (i32, i32)>,
) -> bool {
    host.core()
        .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    // Keep caller-provided iteration outside the owner guard. The login
    // loader supplies its already materialized exact-traits Vec here.
    let traits = traits.into_iter().collect();
    host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.replace_complete_trait_definition_ids_like_cpp(traits)
    })
    .unwrap_or(false)
}
pub fn complete_represented_player_spell_rows_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> Option<BTreeMap<i32, RepresentedPlayerSpellLikeCpp>> {
    host.with_player_spell_runtime_like_cpp(|runtime| {
        (runtime.rows_loaded_like_cpp() && runtime.rows_complete_like_cpp()).then(|| {
            runtime
                .rows_like_cpp()
                .iter()
                .map(|(&id, row)| (id, represented_player_spell_record_like_cpp(row)))
                .collect()
        })
    })
    .flatten()
}

pub fn represented_cast_spell_info_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    spell_info: &wow_data::SpellInfo,
) -> wow_data::SpellInfo {
    let override_spells = crate::represented_override_spells_like_cpp(host);
    if let Some(overrides) = override_spells.get(&spell_info.spell_id) {
        if let Some(store) = host.hub_ref().catalogs.spell_store() {
            for new_spell_id in overrides {
                if let Some(new_info) = store.get(*new_spell_id) {
                    return new_info.clone();
                }
            }
        }
    }

    spell_info.clone()
}

pub fn player_spell_runtime_snapshot_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> Option<RepresentedPlayerSpellRuntimeLikeCpp> {
    let canonical = host
        .core()
        .owned_spell_acquisition_access_like_cpp()
        .with_player_spell_runtime_like_cpp(represented_player_spell_runtime_like_cpp);
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        return Some(
            host.spell_state()
                .represented_spell_runtime_fixture_like_cpp(),
        );
    }
    canonical
}

pub fn with_player_spell_runtime_like_cpp<H: SpellbookHostLikeCpp + ?Sized, R>(
    host: &H,
    query: impl FnOnce(&wow_entities::PlayerSpellRuntimeState) -> R,
) -> Option<R> {
    let (state, hub) = host.split_spell_state_ref();
    state.with_player_spell_runtime_like_cpp(hub, world_test_consumer_like_cpp(host), query)
}
