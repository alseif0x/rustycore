//! Feature-gated observations of existing quest test backing values.
use super::*;

impl WorldSession {
    pub(crate) fn quest_fixture_player_handle(&self) -> Option<wow_map::PlayerHandle> {
        self.player_handle_like_cpp
    }

    pub(crate) fn quest_fixture_currencies(&self) -> HashMap<u32, PlayerCurrency> {
        self.player_currencies.clone()
    }

    pub(crate) fn mutate_quest_fixture_currencies<R>(
        &mut self,
        mutate: impl FnOnce(&mut HashMap<u32, PlayerCurrency>) -> R,
    ) -> R {
        mutate(&mut self.player_currencies)
    }
}
