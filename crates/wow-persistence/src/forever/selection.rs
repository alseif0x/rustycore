//! 02245dcd CharacterDatabase.cpp CHAR_SEL_ENUM/CHAR_SEL_ENUM_CUSTOMIZATIONS.
//! Complete query-holder projection, SQL-free and deliberately without Debug.

#[derive(Clone, Copy, Default)]
pub struct VisualItemRow {
    pub item_id: u32,
    pub visible_item_id: u32,
    pub subclass: u8,
    pub inventory_type: u8,
    pub display_id: u32,
    pub display_enchant_id: u32,
    pub secondary_appearance_id: i32,
    pub sheathe_category: u8,
}

#[derive(Clone, Default)]
pub struct CharacterRow {
    pub guid: u64,
    pub name: String,
    /// CharacterCache.cpp's surname projection, not a legacy enum offset.
    pub surname: String,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub level: u8,
    pub zone: u16,
    pub map: u16,
    pub position: [f32; 3],
    pub guild: u64,
    pub player_flags: u32,
    pub at_login: u16,
    pub pet_entry: u32,
    pub pet_display: u32,
    pub pet_level: u16,
    pub active_ban_guid: u64,
    pub slot: u8,
    pub create_time: i64,
    pub logout_time: i64,
    pub active_talent_group: u8,
    pub last_login_build: u32,
    pub personal_tabard: [i32; 5],
    pub equipment: [VisualItemRow; 19],
    pub declined_genitive: Option<String>,
}

#[derive(Clone, Copy)]
pub struct CustomizationRow {
    pub guid: u64,
    pub option: u32,
    pub choice: u32,
}

/// Both queries completed before this value exists. This is not a claim of a
/// repeatable-read database snapshot: the C++ holder also issues two queries.
#[derive(Clone, Default)]
pub struct SelectionRows {
    pub characters: Vec<CharacterRow>,
    pub customizations: Vec<CustomizationRow>,
}
