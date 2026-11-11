//! Projection of source stored-aura-location dirty-state iteration.

pub struct NewStoredAuraLocation {
    pub spell: u32,
    pub state: NewStoredAuraLocationState,
}

pub enum NewStoredAuraLocationState {
    Unchanged,
    Changed(NewStoredAuraPosition),
    Deleted,
}

pub struct NewStoredAuraPosition {
    pub map: u32,
    pub position: [f32; 4],
}
