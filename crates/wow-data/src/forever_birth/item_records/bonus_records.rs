#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemBonusRecord {
    pub id: u32,
    pub values: [i32; 4],
    pub parent: u16,
    pub bonus_type: u8,
    pub order: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AzeriteEmpoweredItemRecord {
    pub id: u32,
    pub item: i32,
    pub tier_unlock_set: u32,
    pub power_set: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemScalingConfigRecord {
    pub id: u32,
    pub offset_curve: i32,
    pub item_level: i32,
    pub required_level: i32,
    pub squish_era: i32,
    pub flags: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemOffsetCurveRecord {
    pub id: u32,
    pub curve: i32,
    pub offset: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemSquishEraRecord {
    pub id: u32,
    pub patch: i32,
    pub curve: i32,
    pub flags: i32,
}
