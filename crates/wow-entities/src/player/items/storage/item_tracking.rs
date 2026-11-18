// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item duration and soulbound tradeability tracking.

use std::collections::HashSet;

use crate::Item;
use crate::player::{
    ItemDurationRef, Player, PlayerItemTimeUpdate, SoulboundTradeableItemRef,
    UpdateItemDurationAction,
};
use wow_core::ObjectGuid;

impl Player {
    pub fn soulbound_tradeable_items(&self) -> &HashSet<ObjectGuid> {
        &self.soulbound_tradeable_items
    }

    pub fn item_durations(&self) -> &[ObjectGuid] {
        &self.item_durations
    }

    pub fn add_tradeable_item(&mut self, item: &Item) {
        self.soulbound_tradeable_items.insert(item.object().guid());
    }

    pub fn remove_tradeable_item(&mut self, item: &Item) {
        self.soulbound_tradeable_items.remove(&item.object().guid());
    }

    pub fn update_soulbound_trade_items(
        &mut self,
        items: &[SoulboundTradeableItemRef],
    ) -> Vec<ObjectGuid> {
        let player_guid = self.guid();
        let mut removed = Vec::new();
        self.soulbound_tradeable_items.retain(|guid| {
            let keep = items.iter().any(|item| {
                item.guid == *guid && item.owner_guid == player_guid && !item.trade_expired
            });
            if !keep {
                removed.push(*guid);
            }
            keep
        });
        removed
    }

    pub fn add_item_durations(&mut self, item: &Item) -> Option<PlayerItemTimeUpdate> {
        let expiration = item.data().expiration;
        if expiration == 0 {
            return None;
        }

        let item_guid = item.object().guid();
        self.item_durations.push(item_guid);
        Some(PlayerItemTimeUpdate {
            item_guid,
            expiration,
        })
    }

    pub fn remove_item_durations(&mut self, item: &Item) -> bool {
        let item_guid = item.object().guid();
        if let Some(index) = self
            .item_durations
            .iter()
            .position(|stored_guid| *stored_guid == item_guid)
        {
            self.item_durations.remove(index);
            true
        } else {
            false
        }
    }

    pub fn update_item_duration_plan(
        &self,
        items: &[ItemDurationRef],
        time: u32,
        realtime_only: bool,
    ) -> Vec<UpdateItemDurationAction> {
        let mut actions = Vec::new();
        for item_guid in &self.item_durations {
            if let Some(item) = items.iter().find(|item| item.guid == *item_guid) {
                if realtime_only && !item.real_duration {
                    continue;
                }
                if item.expiration == 0 {
                    continue;
                }
                if item.expiration <= time {
                    actions.push(UpdateItemDurationAction::Expire {
                        item_guid: *item_guid,
                    });
                } else {
                    actions.push(UpdateItemDurationAction::UpdateExpiration {
                        item_guid: *item_guid,
                        expiration: item.expiration - time,
                    });
                }
            } else {
                actions.push(UpdateItemDurationAction::MissingItem {
                    item_guid: *item_guid,
                });
            }
        }
        actions
    }

    pub fn send_item_durations_plan(&self, items: &[ItemDurationRef]) -> Vec<PlayerItemTimeUpdate> {
        self.item_durations
            .iter()
            .filter_map(|item_guid| {
                items
                    .iter()
                    .find(|item| item.guid == *item_guid)
                    .map(|item| PlayerItemTimeUpdate {
                        item_guid: *item_guid,
                        expiration: item.expiration,
                    })
            })
            .collect()
    }
}
