// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the represented login-load bodies moved
//! to `wow-world-character` in #1363 slice 12b4: one-call forwards to the
//! moved functions the scenarios call, so no scenario call site changes, and
//! the vault clear, which has no production caller left.
#![cfg(test)]

use crate::session::{RepresentedVoidStorageItemLikeCpp, WorldSession};

impl WorldSession {
    pub(crate) fn clear_represented_void_storage_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.clear_represented_void_storage_like_cpp(&mut hub)
    }
    pub(crate) fn mark_represented_void_storage_loaded_like_cpp(&mut self) {
        wow_world_character::mark_represented_void_storage_loaded_like_cpp(self)
    }
    pub(crate) fn prepare_represented_void_storage_login_load_like_cpp(&mut self) -> bool {
        wow_world_character::prepare_represented_void_storage_login_load_like_cpp(self)
    }
    pub(crate) fn load_represented_void_storage_row_like_cpp(
        &mut self,
        slot: u8,
        item: RepresentedVoidStorageItemLikeCpp,
    ) -> bool {
        wow_world_character::load_represented_void_storage_row_like_cpp(self, slot, item)
    }
    pub(crate) fn load_represented_group_difficulties_like_cpp(&mut self) -> bool {
        wow_world_character::load_represented_group_difficulties_like_cpp(self)
    }
    pub(crate) fn load_represented_group_by_db_store_id_like_cpp(
        &mut self,
        db_store_id: u32,
    ) -> bool {
        wow_world_character::load_represented_group_by_db_store_id_like_cpp(self, db_store_id)
    }
    pub(crate) fn begin_represented_trait_config_authority_load_like_cpp(&mut self) {
        wow_world_character::begin_represented_trait_config_authority_load_like_cpp(self)
    }
    pub(crate) fn complete_represented_trait_config_authority_load_like_cpp(
        &mut self,
        configs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
        entries_empty: bool,
    ) -> bool {
        wow_world_character::complete_represented_trait_config_authority_load_like_cpp(
            self,
            configs,
            entries_empty,
        )
    }
}
