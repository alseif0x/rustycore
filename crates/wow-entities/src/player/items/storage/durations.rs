// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html
//! Tradeable item tracking, item duration and new-item publication plans.

use super::super::super::*;

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

    pub fn send_new_item_plan(
        &self,
        item: Option<&Item>,
        template: SendNewItemTemplateRef,
        args: SendNewItemArgs,
    ) -> Option<SendNewItemPlan> {
        let item = item?;
        let battle_pet_breed_data = item.get_modifier(ItemModifier::BattlePetBreedData);
        let is_encounter_loot = args.dungeon_encounter_id != 0;
        let delivery =
            if args.broadcast && args.player_in_group && !template.dont_report_loot_log_to_party {
                SendNewItemDelivery::GroupBroadcast
            } else {
                SendNewItemDelivery::Direct
            };
        let modifications = item
            .data()
            .modifiers
            .iter()
            .enumerate()
            .filter_map(|(modifier_type, &value)| {
                (value != 0).then_some(SendNewItemModifier {
                    value: value as i32,
                    modifier_type: modifier_type as u8,
                })
            })
            .collect();

        Some(SendNewItemPlan {
            player_guid: self.guid(),
            item_guid: item.object().guid(),
            item_entry: item.object().entry(),
            item_instance: SendNewItemInstancePlan {
                item_id: item.object().entry(),
                random_properties_seed: item.data().property_seed,
                random_properties_id: item.data().random_properties_id,
                modifications,
            },
            slot: item.bag_slot(),
            slot_in_bag: if item.count() == args.quantity {
                i16::from(item.slot())
            } else {
                -1
            },
            quest_log_item_id: template.quest_log_item_id,
            quantity: args.quantity,
            quantity_in_inventory: args.quantity_in_inventory,
            battle_pet_species_id: item.get_modifier(ItemModifier::BattlePetSpeciesId),
            battle_pet_breed_id: battle_pet_breed_data & 0x00FF_FFFF,
            battle_pet_breed_quality: ((battle_pet_breed_data >> 24) & 0xFF) as u8,
            battle_pet_level: item.get_modifier(ItemModifier::BattlePetLevel),
            pushed: args.pushed,
            created: args.created,
            display_text: if is_encounter_loot {
                SendNewItemDisplayText::EncounterLoot
            } else {
                SendNewItemDisplayText::Normal
            },
            dungeon_encounter_id: args.dungeon_encounter_id,
            is_encounter_loot,
            delivery,
        })
    }
}
