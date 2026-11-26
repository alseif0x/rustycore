// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot money application and distribution (#1363 slice 5c; moved from the
//! World `handlers/loot/money.rs`).

use super::*;

pub(super) fn local_loot_money_percent<H: LootSessionHostLikeCpp>(host: &H) -> Option<i32> {
    let hub = host.hub_ref();
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.represented_local_loot_fixture_allowed_like_cpp()
        && hub.core.player_handle_like_cpp.is_none()
    {
        return Some(0);
    }
    hub.core.with_owned_player_like_cpp(|player| {
        wow_world_core::session::PlayerStatAuraView::loot_money_percent(
            player,
            hub.catalogs.player_stat_aura_catalogs(),
        )
    })?
}

pub async fn apply_durable_represented_loot_money_payout_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    notify: LootMoneyNotify,
    durable_applied_amount: u64,
    apply_money: bool,
    publish: bool,
) -> ApplyLootMoneyResultLikeCpp {
    if host.player_guid().is_none() {
        return ApplyLootMoneyResultLikeCpp::TargetMismatch;
    }
    let Some(old_money) = host.resolved_player_money_like_cpp() else {
        return ApplyLootMoneyResultLikeCpp::TargetMismatch;
    };
    let new_money = if apply_money {
        old_money
            .checked_add(durable_applied_amount)
            .filter(|money| *money <= MAX_MONEY_AMOUNT)
            .unwrap_or(old_money)
    } else {
        old_money
    };

    if apply_money && !host.set_player_gold_like_cpp(new_money) {
        return ApplyLootMoneyResultLikeCpp::TargetMismatch;
    }
    if apply_money && durable_applied_amount != 0 {
        host.quest_state_mut()
            .enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
    }
    if publish {
        host.send_packet(&notify);
    }
    if apply_money || publish {
        host.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
    }

    ApplyLootMoneyResultLikeCpp::Applied
}
