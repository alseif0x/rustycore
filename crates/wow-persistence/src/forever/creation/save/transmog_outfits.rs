//! Frozen modern-outfit projections, keyed externally by ordered dirty IDs.

/// Private outfit names deliberately have no Debug implementation.
pub struct NewTransmogOutfit {
    pub name: String,
    pub icon: u32,
    pub situations_enabled: bool,
    pub situations: Vec<NewTransmogOutfitSituation>,
    pub slots: Vec<NewTransmogOutfitSlot>,
}

pub struct NewTransmogOutfitSituation {
    pub situation_id: u32,
    pub spec_id: u32,
    pub loadout_id: u32,
    pub equipment_set_id: u32,
}

pub struct NewTransmogOutfitSlot {
    pub slot: i8,
    pub slot_option: u8,
    pub sheathe_category: u8,
    pub item_modified_appearance_id: u32,
    pub appearance_display_type: u8,
    pub spell_item_enchantment_id: u32,
    pub illusion_display_type: u8,
    pub flags: u32,
}
