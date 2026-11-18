// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! New-item notification planning.

use crate::Item;
use crate::player::{
    Player, SendNewItemArgs, SendNewItemDelivery, SendNewItemDisplayText, SendNewItemInstancePlan,
    SendNewItemModifier, SendNewItemPlan, SendNewItemTemplateRef,
};
use wow_constants::ItemModifier;

impl Player {
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
