// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only dungeon-finding entry points.
//!
//! `#1263 F5 remaining families`: the `CMSG_DF_GET_SYSTEM_INFO` registration
//! moved to the `ApplicationDungeonFinding` area registrar. `#1363 slice 4a`:
//! its body and the LFG lock info moved to `wow_world_social::lfg_lock_info`.

use wow_world_social::{
    LFG_LOCKSTATUS_MISSING_ACHIEVEMENT_LIKE_CPP, LFG_LOCKSTATUS_MISSING_ITEM_LIKE_CPP,
    LFG_LOCKSTATUS_QUEST_NOT_COMPLETED_LIKE_CPP,
};

mod test_shims;

impl crate::session::WorldSession {
    pub(crate) async fn handle_df_get_system_info(&mut self, pkt: wow_packet::WorldPacket) {
        let store = self
            .lfg_dungeon_store_like_cpp()
            .cloned()
            .unwrap_or_default();
        wow_world_social::handle_df_get_system_info_with_catalog_like_cpp(
            self,
            store.as_ref(),
            pkt,
        )
        .await;
    }
}

#[path = "../../../unit_tests/handlers/dungeon_finding/tests/mod.rs"]
mod tests;
