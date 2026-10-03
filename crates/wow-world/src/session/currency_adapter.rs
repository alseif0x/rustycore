// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Currency adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{HashMap, PlayerCurrency, WorldSession};

pub(crate) use wow_world_core::session::PlayerCurrencyDelta;
pub(in crate::session) use wow_world_core::session::currency_max_quantity_cpp;

pub(crate) use wow_world_application::RepresentedQuestObjectiveProgressEventLikeCpp;

pub(crate) use wow_constants::currency::CurrencyGainSourceLikeCpp;

pub(crate) use wow_world_core::session::state::hub_support::player_team_for_race_cpp;

impl WorldSession {
    pub(crate) fn set_player_currencies_like_cpp(
        &mut self,
        currencies: HashMap<u32, PlayerCurrency>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.set_player_currencies_like_cpp(&mut hub, currencies)
    }

    pub(crate) fn clear_player_currencies_like_cpp(&mut self) -> bool {
        self.set_player_currencies_like_cpp(HashMap::new())
    }

    pub(crate) fn player_currencies_like_cpp(&self) -> Option<HashMap<u32, PlayerCurrency>> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.player_currencies_like_cpp(hub)
    }
}
