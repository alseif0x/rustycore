// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented currency operations and the staged money change (#1363
//! slice 12b7), moved from the World `session/money/operations.rs` and
//! `session/publication/operations.rs`: C++ `Player::GetCurrencyQuantity`,
//! `Player::HasCurrency`, `Player::SetCurrencyFlags`, `Player::AddCurrency`
//! (item refund), `Player::SendCurrencies` and the money-changed staging the
//! transactional callers publish under their money guard. The bodies run over
//! [`VendorSessionHostLikeCpp`]; the canonical Player stays the owner of the
//! currencies and the money.

use wow_constants::item::CurrencyTypes;
use wow_constants::unit::Team;
use wow_data::is_player_meeting_condition_like_cpp;
use wow_entities::{PlayerCurrency, PlayerCurrencyState};
use wow_packet::packets::misc::{SetupCurrency, SetupCurrencyRecord};
use wow_world_core::session::state::hub_support::player_team_for_race_cpp;
use wow_world_core::session::{PlayerCurrencyDelta, currency_max_quantity_cpp};

use crate::vendor_session::{PlayerConditionContextSourceLikeCpp, VendorSessionHostLikeCpp};

/// Currency flags that are not persisted (moved with the setup packet).
const CURRENCY_DB_UNUSED_FLAGS_LIKE_CPP: u8 = 0x13;

/// C++ `Player::GetCurrencyQuantity`.
pub fn player_currency_quantity<H: VendorSessionHostLikeCpp>(
    host: &H,
    currency_id: u32,
) -> Option<u32> {
    let access = host.core().owned_player_currency_access_like_cpp();
    host.split_inventory_ref()
        .0
        .player_currency_quantity_with_access_like_cpp(&access, currency_id)
}

/// C++ `Player::HasCurrency`.
pub fn has_currency<H: VendorSessionHostLikeCpp>(host: &H, currency_id: u32, amount: u32) -> bool {
    player_currency_quantity(host, currency_id).is_some_and(|quantity| quantity >= amount)
}

/// C++ `Player::SetCurrencyFlags` + `Player::SendCurrencies`.
pub fn represented_set_currency_flags_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    currency_id: u32,
    flags: u8,
) -> bool {
    let Some(store) = host.hub_ref().catalogs.currency_types_store.as_ref() else {
        return false;
    };
    if !store.has_record(currency_id) {
        return false;
    }

    let Some(mut currencies) = ({
        let (state, hub) = host.split_inventory_ref();
        state.player_currencies_like_cpp(hub)
    }) else {
        return false;
    };
    if let Some(currency) = currencies.get_mut(&currency_id) {
        if currency.flags != flags {
            currency.flags = flags;
            if currency.state != PlayerCurrencyState::New {
                currency.state = PlayerCurrencyState::Changed;
            }
        }
        if !({
            let (state, mut hub) = host.split_inventory_mut();
            state.set_player_currencies_like_cpp(&mut hub, currencies)
        }) {
            return false;
        }
    }

    let Some(packet) = setup_currencies_packet_like_cpp(host) else {
        return false;
    };
    host.send_packet(&packet);
    true
}

/// C++ `Player::AddCurrency(..., CurrencyGainSource::ItemRefund)`.
pub fn add_currency_item_refund<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    currency_id: u32,
    amount: u32,
) -> Result<Option<PlayerCurrencyDelta>, ()> {
    if amount == 0 {
        return Ok(None);
    }

    let Some(entry) = host
        .hub_ref()
        .catalogs
        .currency_types_store
        .as_ref()
        .and_then(|store| store.get(currency_id))
        .copied()
    else {
        return Err(());
    };

    let player_team = player_team_for_race_cpp(host.hub_ref().player_race_like_cpp());
    if (entry.is_alliance() && player_team != Team::Alliance)
        || (entry.is_horde() && player_team != Team::Horde)
    {
        return Ok(None);
    }

    if entry.award_condition_id != 0 {
        return Err(());
    }
    if entry.faction_id != 0 || currency_id == CurrencyTypes::Azerite as u32 {
        return Ok(None);
    }

    let mut currencies = ({
        let (state, hub) = host.split_inventory_ref();
        state.player_currencies_like_cpp(hub)
    })
    .ok_or(())?;
    let currency = currencies.entry(currency_id).or_insert(PlayerCurrency {
        state: PlayerCurrencyState::New,
        quantity: 0,
        weekly_quantity: 0,
        tracked_quantity: 0,
        increased_cap_quantity: 0,
        earned_quantity: 0,
        flags: 0,
    });

    if currency.state != PlayerCurrencyState::New {
        currency.state = PlayerCurrencyState::Changed;
    }
    currency.quantity = currency.quantity.saturating_add(amount);

    let scaler = entry.scaler().max(1) as u32;
    let max_quantity = currency_max_quantity_cpp(&entry, currency);
    let delta = PlayerCurrencyDelta {
        currency_id,
        quantity: currency.quantity,
        amount,
        weekly_quantity: ((currency.weekly_quantity / scaler) > 0)
            .then_some(currency.weekly_quantity),
        max_quantity: (max_quantity != 0).then_some(max_quantity),
        total_earned: entry.has_total_earned().then_some(currency.earned_quantity),
        suppress_chat_log: entry.is_suppressing_chat_log(false),
    };
    if !({
        let (state, mut hub) = host.split_inventory_mut();
        state.set_player_currencies_like_cpp(&mut hub, currencies)
    }) {
        return Err(());
    }
    Ok(Some(delta))
}

/// Publish an already-durable absolute money mutation without awaiting.
/// Transactional callers use this while their exclusive money guard is
/// still held, then drop the guard before draining criteria (which can
/// re-enter money persistence through a quest reward).
pub fn stage_player_money_change_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    old_money: u64,
    new_money: u64,
) -> bool {
    if !({
        let (state, mut hub) = host.split_inventory_mut();
        state.set_player_gold_like_cpp(&mut hub, new_money)
    }) {
        return false;
    }
    if old_money != new_money {
        host.enqueue_represented_quest_objective_money_changed_like_cpp(old_money, new_money);
    }
    true
}

/// C++ `Player::SendCurrencies`.
pub fn setup_currencies_packet_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &H,
) -> Option<SetupCurrency> {
    let store = host.hub_ref().catalogs.currency_types_store.as_ref()?;
    let currencies = ({
        let (state, hub) = host.split_inventory_ref();
        state.player_currencies_like_cpp(hub)
    })?;

    let player_team = player_team_for_race_cpp(host.hub_ref().player_race_like_cpp());
    let mut records = Vec::with_capacity(currencies.len());
    for (&currency_id, currency) in &currencies {
        let Some(entry) = store.get(currency_id).copied() else {
            continue;
        };

        if (entry.is_alliance() && player_team != Team::Alliance)
            || (entry.is_horde() && player_team != Team::Horde)
        {
            continue;
        }

        if entry.award_condition_id != 0 {
            if let Some(condition) = host
                .hub_ref()
                .catalogs
                .player_condition_store
                .as_ref()
                .and_then(|store| store.get(entry.award_condition_id as u32))
            {
                let Some(context) = host.represented_player_condition_context_like_cpp() else {
                    continue;
                };
                if !context.as_context(host).is_some_and(|context| {
                    is_player_meeting_condition_like_cpp(condition, &context)
                }) {
                    continue;
                }
            }
        }

        let scaler = entry.scaler().max(1) as u32;
        let max_quantity = currency_max_quantity_cpp(&entry, currency);
        records.push(SetupCurrencyRecord {
            type_id: entry.id as i32,
            quantity: currency.quantity as i32,
            weekly_quantity: ((currency.weekly_quantity / scaler) > 0)
                .then_some(currency.weekly_quantity),
            max_weekly_quantity: entry
                .has_max_earnable_per_week()
                .then_some(entry.max_earnable_per_week),
            tracked_quantity: entry
                .is_tracking_quantity()
                .then_some(currency.tracked_quantity),
            max_quantity: (max_quantity != 0).then_some(max_quantity as i32),
            total_earned: entry
                .has_total_earned()
                .then_some(currency.earned_quantity as i32),
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            flags: currency.flags & !CURRENCY_DB_UNUSED_FLAGS_LIKE_CPP,
        });
    }

    Some(SetupCurrency::from_records(records))
}
