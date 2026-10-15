// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the spell-effect bodies moved in #1363
//! slice 12b2 to `wow-world-spell` (the homebind write and the farsight seer
//! consumer): one-call forwards to the moved functions the scenarios call,
//! and the item-flags catalog read, moved verbatim from
//! `session/player_items/catalog.rs` because only the scenarios call it once
//! the moved obtain-spells body reads the catalog through the host. No
//! scenario call site changes.

#![cfg(test)]

use wow_constants::ItemFlags;
use wow_core::ObjectGuid;

use crate::session::{RepresentedHomebindLikeCpp, WorldSession};

impl WorldSession {
    pub(crate) fn set_represented_homebind_like_cpp(
        &mut self,
        homebind: RepresentedHomebindLikeCpp,
    ) -> bool {
        wow_world_spell::set_represented_homebind_like_cpp(self, homebind)
    }
    pub(crate) fn send_set_viewpoint_target_visibility_like_cpp(
        &mut self,
        dynamic_object_guid: ObjectGuid,
    ) -> bool {
        wow_world_spell::send_set_viewpoint_target_visibility_like_cpp(self, dynamic_object_guid)
    }
    pub(crate) fn consume_add_farsight_set_seer_outcome_like_cpp(
        &mut self,
        outcome: &wow_map::map::FarsightDynamicObjectCreateOutcomeLikeCpp,
    ) -> bool {
        wow_world_spell::consume_add_farsight_set_seer_outcome_like_cpp(self, outcome)
    }
    pub(crate) fn item_template_flags(&self, item_id: u32) -> Option<ItemFlags> {
        self.catalogs.item_template_flags(item_id)
    }
}
