//! Frozen saved-equipment/legacy-transmog data; ID allocation remains external.

/// The containing map's ordered key is distinct from this value's saved GUID.
/// Private names/icons deliberately have no Debug implementation.
pub struct NewSavedEquipmentSet {
    pub guid: u64,
    pub set_index: u32,
    pub name: String,
    pub icon: String,
    pub ignore_mask: u32,
    pub kind: NewSavedSetKind,
    pub state: NewSavedSetState,
}

pub enum NewSavedSetKind {
    Equipment {
        assigned_spec: Option<i32>,
        items: [u64; 19],
    },
    Transmog {
        appearances: [i32; 19],
        enchants: [i32; 2],
    },
}

#[derive(Clone, Copy)]
pub enum NewSavedSetState {
    Unchanged,
    Changed,
    New,
    Deleted,
}
