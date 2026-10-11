// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Equipment-set packet handlers and represented set mutations.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn handle_save_equipment_set(&mut self, pkt: WorldPacket) {
        let Some(generator) = self.equipment_set_guid_generator_for_test_like_cpp() else {
            return;
        };
        self.handle_save_equipment_set_with_generator_like_cpp(generator.as_ref(), pkt)
            .await;
    }
}

#[cfg(test)]
#[path = "../../../../unit_tests/handlers/character/items/equipment_sets/test_only.rs"]
mod test_only;
