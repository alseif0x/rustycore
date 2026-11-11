//! Frozen aura persistence projections; canonical eligibility/key logic is absent.

pub struct NewAuraSave {
    /// Required result of canonical Aura::CanBeSaved, not a fallback predicate.
    pub saveable: bool,
    /// Complete ObjectGuid::GetRawValue bytes; never low-counter projections.
    pub caster_guid: [u8; 16],
    pub item_guid: [u8; 16],
    pub spell: u32,
    /// Masks already resolved by the future canonical Aura::GenerateKey owner.
    pub effect_mask: u32,
    pub recalculate_mask: u32,
    pub difficulty: u8,
    pub stack_count: u8,
    pub max_duration: i32,
    pub remaining_time: i32,
    pub charges: u8,
    pub cast_item: u32,
    pub cast_item_level: i32,
    /// Preserve the source non-null AuraEffect iteration order.
    pub effects: Vec<NewAuraEffect>,
}

pub struct NewAuraEffect {
    pub index: u8,
    pub amount: f64,
    pub base_amount: f64,
}
