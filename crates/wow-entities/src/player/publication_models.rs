use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendNewItemTemplateRef {
    pub quest_log_item_id: u32,
    pub dont_report_loot_log_to_party: bool,
}

impl SendNewItemTemplateRef {
    pub const fn new(quest_log_item_id: u32, dont_report_loot_log_to_party: bool) -> Self {
        Self {
            quest_log_item_id,
            dont_report_loot_log_to_party,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendNewItemArgs {
    pub quantity: u32,
    pub pushed: bool,
    pub created: bool,
    pub broadcast: bool,
    pub dungeon_encounter_id: u32,
    pub player_in_group: bool,
    pub quantity_in_inventory: u32,
}

impl SendNewItemArgs {
    pub const fn new(quantity: u32, pushed: bool, created: bool) -> Self {
        Self {
            quantity,
            pushed,
            created,
            broadcast: false,
            dungeon_encounter_id: 0,
            player_in_group: false,
            quantity_in_inventory: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendNewItemDisplayText {
    Normal,
    EncounterLoot,
    QuestUpdateAddItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendNewItemDelivery {
    Direct,
    GroupBroadcast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendNewItemModifier {
    pub value: i32,
    pub modifier_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendNewItemInstancePlan {
    pub item_id: u32,
    pub random_properties_seed: i32,
    pub random_properties_id: i32,
    pub modifications: Vec<SendNewItemModifier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendNewItemPlan {
    pub player_guid: ObjectGuid,
    pub item_guid: ObjectGuid,
    pub item_entry: u32,
    pub item_instance: SendNewItemInstancePlan,
    pub slot: u8,
    pub slot_in_bag: i16,
    pub quest_log_item_id: u32,
    pub quantity: u32,
    pub quantity_in_inventory: u32,
    pub battle_pet_species_id: u32,
    pub battle_pet_breed_id: u32,
    pub battle_pet_breed_quality: u8,
    pub battle_pet_level: u32,
    pub pushed: bool,
    pub created: bool,
    pub display_text: SendNewItemDisplayText,
    pub dungeon_encounter_id: u32,
    pub is_encounter_loot: bool,
    pub delivery: SendNewItemDelivery,
}
