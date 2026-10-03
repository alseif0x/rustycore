//! Consumed SQL-only specialization/relic inputs; no DB2/Player authority.
pub struct ItemSpecRow {
    pub id: u32,
    pub min_level: u8,
    pub max_level: u8,
    pub item_type: u8,
    pub primary: u8,
    pub secondary: u8,
    pub specialization: u16,
}
pub struct ItemSpecOverrideRow {
    pub id: u32,
    pub specialization: u16,
    pub item: u32,
}
pub struct GemPropertiesRow {
    pub id: u32,
    pub enchantment: u16,
    pub kind: i32,
}
#[derive(Default)]
pub struct ItemSpecRows {
    pub specs: Vec<ItemSpecRow>,
    pub overrides: Vec<ItemSpecOverrideRow>,
    pub gems: Vec<GemPropertiesRow>,
}
pub struct ItemSpecOverlays {
    pub official: ItemSpecRows,
    pub custom: ItemSpecRows,
}
#[derive(Clone, Copy)]
pub struct ItemAddonRow {
    pub id: u32,
    pub flags: u32,
    pub food: u8,
    pub min_money: u32,
    pub max_money: u32,
    pub spell_ppm: f32,
    pub random_bonus_template: u32,
    pub quest_log_item: i32,
}
