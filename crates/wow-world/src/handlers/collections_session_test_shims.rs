// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the toy bodies moved to
//! `wow_world_inventory` (`CMSG_ADD_TOY`) and `wow_world_spell`
//! (`CMSG_USE_TOY`) (#1363 slice 4d): one-call forwards with this session as
//! the host, so no scenario call site changes. The session wrappers whose last
//! production caller was a moved toy body (which now calls the same core,
//! lifecycle or cast-execution operation directly) move here verbatim for
//! their scenarios: `send_equip_error`, `add_account_toy_like_cpp` and
//! `set_active_spell_cast_like_cpp`.

#![cfg(test)]

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;

use crate::session::{SpellCastState, WorldSession};

impl WorldSession {
    pub(crate) async fn handle_add_toy(&mut self, pkt: wow_packet::WorldPacket) {
        wow_world_inventory::handle_add_toy(self, pkt).await
    }

    pub(crate) async fn handle_use_toy(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_spell::handle_use_toy_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            pkt,
        )
        .await;
    }

    pub(crate) fn send_equip_error(
        &self,
        result: InventoryResult,
        item1: Option<ObjectGuid>,
        item2: Option<ObjectGuid>,
        required_level: u32,
        limit_category: u32,
    ) {
        self.core
            .send_equip_error(result, item1, item2, required_level, limit_category)
    }

    pub(crate) fn add_account_toy_like_cpp(
        &mut self,
        item_id: u32,
        is_favorite: bool,
        has_fanfare: bool,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.add_account_toy_like_cpp(&mut hub, item_id, is_favorite, has_fanfare)
    }

    pub(crate) fn set_active_spell_cast_like_cpp(&mut self, cast: Option<SpellCastState>) -> bool {
        self.mutate_cast_execution_like_cpp(|state| state.active = cast)
            .is_some()
    }
}
