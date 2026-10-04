// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Currency mutations performed while a quest reward is being prepared.

use wow_constants::{CurrencyTypes, Team};
use wow_data::CurrencyTypesStore;
use wow_entities::{PlayerCurrency, PlayerCurrencyState};
use wow_packet::packets::misc::SetCurrency;
use wow_world_core::session::{
    CurrencyGainSourceLikeCpp, PacketPublicationAccessLikeCpp, PlayerCurrencyDelta,
    QuestRewardPlayerAccessLikeCpp, currency_max_quantity_cpp,
};
use wow_world_inventory::InventoryState;

pub(super) async fn grant_quest_reward_currencies_like_cpp(
    inventory: &mut InventoryState,
    player: &QuestRewardPlayerAccessLikeCpp<'_>,
    publication: &PacketPublicationAccessLikeCpp<'_>,
    currency_types: Option<&CurrencyTypesStore>,
    quest: &wow_data::quest::QuestTemplate,
    choice_item_id: u32,
    choice_loot_item_type: u8,
    currency_choice_type: u8,
) -> bool {
    let gain_source = quest.currency_gain_source_like_cpp();

    if choice_loot_item_type == currency_choice_type
        && choice_item_id != 0
        && currency_types.is_some_and(|store| store.has_record(choice_item_id))
    {
        for ((currency_id, count), item_type) in quest
            .reward_choice_items
            .iter()
            .zip(quest.reward_choice_item_types.iter())
        {
            if *currency_id == 0
                || *item_type != currency_choice_type
                || *currency_id != choice_item_id
            {
                continue;
            }
            if !grant_quest_reward_currency_like_cpp(
                inventory,
                player,
                publication,
                currency_types,
                *currency_id,
                *count,
                gain_source,
            )
            .await
            {
                return false;
            }
        }
    }

    for (currency_id, count) in quest
        .reward_currencies
        .iter()
        .zip(quest.reward_currency_amounts.iter())
    {
        if *currency_id == 0 || *count == 0 {
            continue;
        }
        if !grant_quest_reward_currency_like_cpp(
            inventory,
            player,
            publication,
            currency_types,
            *currency_id,
            *count,
            gain_source,
        )
        .await
        {
            return false;
        }
    }

    true
}

async fn grant_quest_reward_currency_like_cpp(
    inventory: &mut InventoryState,
    player: &QuestRewardPlayerAccessLikeCpp<'_>,
    publication: &PacketPublicationAccessLikeCpp<'_>,
    currency_types: Option<&CurrencyTypesStore>,
    currency_id: u32,
    amount: u32,
    gain_source: CurrencyGainSourceLikeCpp,
) -> bool {
    let Some(currency_snapshot) =
        inventory.player_currencies_with_quest_reward_access_like_cpp(player)
    else {
        return false;
    };
    let delta = match add_currency_quest_reward_like_cpp(
        inventory,
        player,
        currency_types,
        currency_id,
        amount,
        gain_source,
    ) {
        Ok(delta) => delta,
        Err(()) => {
            inventory.set_player_currencies_with_quest_reward_access_like_cpp(
                player,
                currency_snapshot,
            );
            return false;
        }
    };

    // `_SaveCurrency` writes the complete state in the operation's closing transaction.
    if let Some(delta) = delta {
        let (Some(quantity), Some(amount)) = (
            i32::try_from(delta.quantity).ok(),
            i32::try_from(delta.amount).ok(),
        ) else {
            return true;
        };
        let mut packet = SetCurrency {
            type_id: delta.currency_id as i32,
            quantity,
            flags: 0,
            weekly_quantity: delta
                .weekly_quantity
                .and_then(|value| i32::try_from(value).ok()),
            tracked_quantity: None,
            max_quantity: delta
                .max_quantity
                .and_then(|value| i32::try_from(value).ok()),
            total_earned: delta
                .total_earned
                .and_then(|value| i32::try_from(value).ok()),
            suppress_chat_log: delta.suppress_chat_log,
            quantity_change: Some(amount),
            quantity_gain_source: Some(gain_source as i32),
            quantity_lost_source: None,
            first_craft_operation_id: None,
            next_recharge_time: None,
            recharge_cycle_start_time: None,
            overflown_currency_id: None,
        };
        packet.suppress_chat_log = delta.suppress_chat_log;
        publication.send_packet(&packet);
    }

    true
}

fn add_currency_quest_reward_like_cpp(
    inventory: &mut InventoryState,
    player: &QuestRewardPlayerAccessLikeCpp<'_>,
    currency_types: Option<&CurrencyTypesStore>,
    currency_id: u32,
    amount: u32,
    gain_source: CurrencyGainSourceLikeCpp,
) -> Result<Option<PlayerCurrencyDelta>, ()> {
    if amount == 0 {
        return Ok(None);
    }
    let Some(entry) = currency_types.and_then(|store| store.get(currency_id)).copied() else {
        return Err(());
    };

    let player_team = wow_world_core::session::state::hub_support::player_team_for_race_cpp(
        player.player_race_like_cpp(),
    );
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

    let ignore_caps = matches!(
        gain_source,
        CurrencyGainSourceLikeCpp::QuestRewardIgnoreCaps
            | CurrencyGainSourceLikeCpp::WorldQuestRewardIgnoreCaps
    );
    let mut currencies = inventory
        .player_currencies_with_quest_reward_access_like_cpp(player)
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

    let weekly_cap = entry.max_earnable_per_week;
    let mut applied = amount;
    if !ignore_caps {
        if weekly_cap != 0 && currency.weekly_quantity.saturating_add(applied) > weekly_cap {
            applied = weekly_cap.saturating_sub(currency.weekly_quantity);
        }
        let max_quantity = currency_max_quantity_cpp(&entry, currency);
        if max_quantity != 0 && currency.quantity.saturating_add(applied) > max_quantity {
            applied = max_quantity.saturating_sub(currency.quantity);
        }
    }
    if applied == 0 {
        return Ok(None);
    }

    if currency.state != PlayerCurrencyState::New {
        currency.state = PlayerCurrencyState::Changed;
    }
    currency.quantity = currency.quantity.saturating_add(applied);
    if !ignore_caps {
        if weekly_cap != 0 {
            currency.weekly_quantity = currency.weekly_quantity.saturating_add(applied);
        }
        if entry.is_tracking_quantity() {
            currency.tracked_quantity = currency.tracked_quantity.saturating_add(applied);
        }
        if entry.has_total_earned() {
            currency.earned_quantity = currency.earned_quantity.saturating_add(applied);
        }
    }

    let scaler = entry.scaler().max(1) as u32;
    let max_quantity = currency_max_quantity_cpp(&entry, currency);
    let delta = PlayerCurrencyDelta {
        currency_id,
        quantity: currency.quantity,
        amount: applied,
        weekly_quantity: ((currency.weekly_quantity / scaler) > 0)
            .then_some(currency.weekly_quantity),
        max_quantity: (max_quantity != 0).then_some(max_quantity),
        total_earned: entry.has_total_earned().then_some(currency.earned_quantity),
        suppress_chat_log: entry.is_suppressing_chat_log(false),
    };
    if !inventory.set_player_currencies_with_quest_reward_access_like_cpp(player, currencies) {
        return Err(());
    }
    Ok(Some(delta))
}
