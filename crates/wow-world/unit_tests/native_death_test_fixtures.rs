// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Explicit complete input for tests that exercise the native death body.

use crate::session::WorldSession;
use std::sync::Arc;

/// The fixture's currently installed spell/Aura/quest rows are the complete
/// input for this Player. Preserve those rows and any supplied real catalogs.
pub(crate) fn prepare(session: &mut WorldSession) {
    if session.catalogs.spell_store().is_none() {
        session.set_spell_store(Arc::new(wow_data::SpellStore::default()));
    }
    session
        .catalogs
        .spell_catalogs
        .serverside_spell_store
        .get_or_insert_with(|| Arc::new(wow_data::ServersideSpellStoreLikeCpp::default()));
    session
        .catalogs
        .spell_catalogs
        .spell_area_store
        .get_or_insert_with(|| Arc::new(wow_data::SpellAreaStoreLikeCpp::default()));
    session
        .catalogs
        .quests
        .store
        .get_or_insert_with(|| Arc::new(wow_data::quest::QuestStore::default()));
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let mut spells = player.spell_runtime_like_cpp().clone();
            spells.set_row_authority_like_cpp(true, true);
            player.replace_spell_runtime_like_cpp(spells);
            player
                .gameplay_state_mut()
                .quests
                .set_status_authority_complete_like_cpp(true);
            player
                .unit_mut()
                .subsystems_mut()
                .auras
                .set_persisted_player_aura_authority_complete_like_cpp(true);
            player
                .inventory_runtime_mut_like_cpp()
                .set_equipment_inventory_authority_complete_like_cpp(true);
        })
        .expect("native death fixture requires its canonical Player");
}
