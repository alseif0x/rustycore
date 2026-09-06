//! Owned trait configuration data; header proof alone is not a complete CREATE image.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTraitConfigState {
    pub header: (i32, i32, i32),
    pub details: Option<PlayerTraitConfigDetails>,
}

impl From<(i32, i32, i32)> for PlayerTraitConfigState {
    fn from(header: (i32, i32, i32)) -> Self {
        Self {
            header,
            details: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTraitConfigDetails {
    /// Preserve the dynamic-field insertion order, independently of map key order.
    pub create_index: usize,
    pub local_identifier: i32,
    pub skill_line_id: i32,
    pub trait_system_id: i32,
    pub name: String,
    pub entries: Vec<PlayerTraitEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerTraitEntry {
    pub trait_node_id: i32,
    pub trait_node_entry_id: i32,
    pub rank: i32,
    pub granted_ranks: i32,
}
