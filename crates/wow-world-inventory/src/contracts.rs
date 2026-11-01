// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountItemAppearanceSavePlanLikeCpp {
    pub appearance_blocks: Vec<(u32, u32)>,
    pub favorite_inserts: Vec<u32>,
    pub favorite_deletes: Vec<u32>,
}

impl AccountItemAppearanceSavePlanLikeCpp {
    pub fn is_empty(&self) -> bool {
        self.appearance_blocks.is_empty()
            && self.favorite_inserts.is_empty()
            && self.favorite_deletes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountTransmogIllusionSavePlanLikeCpp {
    pub illusion_blocks: Vec<(u32, u32)>,
}

impl AccountTransmogIllusionSavePlanLikeCpp {
    pub fn is_empty(&self) -> bool {
        self.illusion_blocks.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedEquipmentSetSavedLikeCpp {
    pub guid: u64,
    pub set_type: wow_entities::PlayerEquipmentSetTypeLikeCpp,
    pub raw_set_type: i32,
    pub set_id: u32,
    pub generated_new_guid: bool,
}

pub const DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP: [u32; 7] = [
    3,  // Lifestealing
    13, // Crusader
    22, // Striking
    23, // Agility
    34, // Hide Weapon Enchant
    43, // Beastslayer
    44, // Titanguard
];

pub const MAX_EQUIPMENT_SET_INDEX_LIKE_CPP: u32 = 20;
