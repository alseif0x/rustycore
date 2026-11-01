// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the spell-state bodies moved to
//! `wow-world-spell` in #1363 slice 12b3: one-call forwards to the moved
//! functions the scenarios call, so no scenario call site changes, and the
//! spell-row and mount-source reads, which have no production caller.
#![cfg(test)]

use wow_world_spell::{RepresentedPlayerSpellLikeCpp, RepresentedPlayerSpellStateLikeCpp};

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) fn represented_update_zone_script_aura_source_is_hit_inert_like_cpp(&self) -> bool {
        wow_world_spell::represented_update_zone_script_aura_source_is_hit_inert_like_cpp(self)
    }
    pub(crate) fn represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(&self) -> bool {
        wow_world_spell::represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(self)
    }
    pub(crate) fn represented_login_passive_spell_cast_gate_like_cpp(&self, spell_id: i32) -> bool {
        wow_world_spell::represented_login_passive_spell_cast_gate_like_cpp(self, spell_id)
    }
    pub(crate) fn player_target_spell_is_hit_inert_like_cpp(
        &self,
        spell_id: u32,
        difficulty_id: u8,
    ) -> bool {
        wow_world_spell::player_target_spell_is_hit_inert_like_cpp(self, spell_id, difficulty_id)
    }
    pub(crate) fn has_represented_self_res_spell_like_cpp(&self, spell_id: i32) -> bool {
        wow_world_spell::has_represented_self_res_spell_like_cpp(self, spell_id)
    }
    /// Moved verbatim from `session/spell_state/spell.rs` (no production
    /// caller; it was `#[allow(dead_code)]` there).
    pub(crate) fn represented_player_spell_rows_like_cpp(
        &self,
    ) -> Vec<RepresentedPlayerSpellLikeCpp> {
        let Some(runtime) = wow_world_spell::player_spell_runtime_snapshot_like_cpp(self) else {
            return Vec::new();
        };
        let mut rows = runtime
            .known_spells
            .iter()
            .copied()
            .map(|spell_id| RepresentedPlayerSpellLikeCpp {
                spell_id,
                active: true,
                disabled: false,
                dependent: runtime.dependent_known_spells.contains(&spell_id),
                favorite: runtime.favorite_known_spells.contains(&spell_id),
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            })
            .collect::<Vec<_>>();

        rows.extend(
            runtime
                .removed_known_spells
                .iter()
                .copied()
                .map(|spell_id| RepresentedPlayerSpellLikeCpp {
                    spell_id,
                    active: false,
                    disabled: false,
                    dependent: false,
                    favorite: false,
                    state: RepresentedPlayerSpellStateLikeCpp::Removed,
                }),
        );
        rows.sort_by_key(|spell| spell.spell_id);
        rows
    }
    /// Moved verbatim from `session/spell_state/spell.rs` (no production
    /// caller; it was `#[allow(dead_code)]` there).
    pub(crate) fn represented_mount_source_spell_usable_like_cpp(&self, spell_id: u32) -> bool {
        let Some(mount) = self
            .catalogs
            .mount_store
            .as_ref()
            .and_then(|store| store.get_by_source_spell_id_like_cpp(spell_id))
        else {
            return false;
        };

        self.represented_meets_player_condition_id_like_cpp(mount.player_condition_id)
    }
}
