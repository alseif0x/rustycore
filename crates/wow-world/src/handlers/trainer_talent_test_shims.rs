// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the trainer and talent-point bodies
//! moved in #1363 slice 12a: `handle_trainer_buy_spell` (verbatim from
//! `handlers/trainer.rs`, where it was a nested `#[cfg(test)]` method) and
//! one-call forwards to the moved functions the scenarios call, with this
//! session as the host, so no scenario call site changes.

#![cfg(test)]

use std::sync::Arc;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn handle_trainer_buy_spell(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let selection = self
            .battle_pet_selection_store_like_cpp()
            .cloned()
            .unwrap_or_else(|| {
                Arc::new(wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp::default())
            });
        wow_world_application::handle_trainer_buy_spell_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            selection.as_ref(),
            pkt,
        )
        .await;
    }
}
