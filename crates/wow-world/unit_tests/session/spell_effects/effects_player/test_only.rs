// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `effects_player` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) fn loaded_player_visible_items_for_create_like_cpp(
        &self,
    ) -> Option<[(i32, u16, u16); 19]> {
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        for (slot, item) in self.resolved_inventory_items_like_cpp()? {
            if (slot as usize) < 19 {
                visible_items[slot as usize] = self
                    .resolved_inventory_item_object_like_cpp(item.guid)
                    .map(|item_object| {
                        let (s, h) = crate::session::split_inventory_ref(self);
                        s.loaded_inventory_item_visible_fields_like_cpp(h, &item_object)
                    })
                    .unwrap_or((item.entry_id as i32, 0u16, 0u16));
            }
        }
        Some(visible_items)
    }
}
