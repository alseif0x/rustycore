use crate::session::WorldSession;
use std::collections::HashMap;
use wow_entities::PlayerCurrency;

pub fn quest_xp_reward_for_test(session: &WorldSession, quest: &wow_data::quest::QuestTemplate) -> u32 {
    session.quest_xp_reward_like_cpp(quest)
}

pub fn quest_money_reward_for_test(session: &WorldSession, quest: &wow_data::quest::QuestTemplate) -> u32 {
    session.quest_money_reward_like_cpp(quest)
}

pub fn quest_currency_gain_for_test(
    session: &mut WorldSession, id: u32, amount: u32,
    source: wow_constants::currency::CurrencyGainSourceLikeCpp,
) -> Result<Option<wow_data_model::currency::CurrencyGainDelta>, ()> {
    session.add_currency_quest_reward_like_cpp(id, amount, source)
}

pub fn install_player_currencies_for_test(
    session: &mut WorldSession, currencies: HashMap<u32, PlayerCurrency>,
) -> bool {
    session.set_player_currencies_like_cpp(currencies)
}

pub fn player_currencies_snapshot_for_test(session: &WorldSession) -> Option<HashMap<u32, PlayerCurrency>> {
    session.player_currencies_like_cpp()
}

/// The compatibility backing map, deliberately distinct from the canonical observation.
pub fn currency_compatibility_fixture_for_test(session: &WorldSession) -> HashMap<u32, PlayerCurrency> {
    session.quest_fixture_currencies()
}

pub fn mutate_currency_compatibility_fixture_for_test<R>(
    session: &mut WorldSession,
    mutate: impl FnOnce(&mut HashMap<u32, PlayerCurrency>) -> R,
) -> R {
    session.mutate_quest_fixture_currencies(mutate)
}

pub fn quest_player_handle_for_test(session: &WorldSession) -> Option<wow_map::PlayerHandle> {
    session.quest_fixture_player_handle()
}

pub fn remove_current_quest_player_from_map_for_test(session: &mut WorldSession) -> bool {
    session.remove_current_player_from_canonical_current_map_like_cpp()
}
