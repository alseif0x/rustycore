// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Merge the loaded spell rows into the complete represented PlayerSpellMap during login.

use crate::CharacterLoginHostLikeCpp;
use std::collections::HashSet;
use tracing::warn;
use wow_core::ObjectGuid;
use wow_world_spell::RepresentedPlayerSpellLikeCpp;
use wow_world_spell::RepresentedPlayerSpellStateLikeCpp;

/// `login_authority_complete` is the conjunction of every row family the
/// PlayerSpellMap depends on; the map stays incomplete otherwise.
pub fn finalize_player_spell_map_for_login_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    guid: ObjectGuid,
    loaded_player_spell_rows: Vec<RepresentedPlayerSpellLikeCpp>,
    favorite_spell_rows: &HashSet<i32>,
    skill_rewarded_dependent_spells: &HashSet<i32>,
    skill_rewarded_removed_spells: &HashSet<i32>,
    login_authority_complete: bool,
) {
    // Retain the raw inactive/disabled DB rows and merge spells introduced by
    // represented AddSpell work. Trainer/acquisition decisions need the full
    // logical PlayerSpellMap, not the active-only client projection.
    let canonical_known_spells = host.known_spells_like_cpp().to_vec();
    let canonical_known_spell_ids = canonical_known_spells
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let mut final_player_spell_rows = loaded_player_spell_rows
        .into_iter()
        .map(|mut row| {
            let dependent = host
                .represented_dependent_known_spells_like_cpp()
                .contains(&row.spell_id)
                || skill_rewarded_dependent_spells.contains(&row.spell_id);
            row.favorite = !dependent && favorite_spell_rows.contains(&row.spell_id);
            row.dependent |= dependent;
            if skill_rewarded_removed_spells.contains(&row.spell_id) {
                row.active = false;
                row.disabled = false;
                row.dependent = false;
                row.favorite = false;
                row.state = RepresentedPlayerSpellStateLikeCpp::Removed;
                return (row.spell_id, row);
            }
            if !row.disabled {
                row.active = canonical_known_spell_ids.contains(&row.spell_id);
            }
            (row.spell_id, row)
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    for spell_id in canonical_known_spells {
        final_player_spell_rows
            .entry(spell_id)
            .and_modify(|row| {
                row.disabled = false;
                row.favorite = favorite_spell_rows.contains(&spell_id);
            })
            .or_insert(RepresentedPlayerSpellLikeCpp {
                spell_id,
                active: true,
                disabled: false,
                dependent: host
                    .represented_dependent_known_spells_like_cpp()
                    .contains(&spell_id)
                    || skill_rewarded_dependent_spells.contains(&spell_id),
                favorite: favorite_spell_rows.contains(&spell_id),
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            });
    }
    if login_authority_complete {
        let complete_spell_rows = host.set_complete_represented_player_spell_rows_like_cpp(
            final_player_spell_rows.into_values(),
        );
        if complete_spell_rows {
            host.mark_represented_spell_acquisition_snapshot_complete_like_cpp();
        } else {
            warn!(
                player_guid = guid.counter(),
                "Could not authorize represented post-login PlayerSpellMap"
            );
        }
    } else {
        warn!(
            player_guid = guid.counter(),
            "Keeping represented PlayerSpellMap incomplete after incomplete login authority"
        );
    }
}
