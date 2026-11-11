//! SQL-free projection of the unordered trait dirty-state iteration.

pub struct NewTraitChange {
    /// Dirty-map key used by deletes; it need not equal a found config's ID.
    pub key: i32,
    pub state: NewTraitState,
}

pub enum NewTraitState {
    Unchanged,
    Changed(Option<NewTraitConfig>),
    New,
    Removed,
    Temporary,
}

/// No Debug implementation: the source config name is private character data.
pub struct NewTraitConfig {
    pub id: i32,
    pub kind: i32,
    pub specialization: i32,
    pub combat_flags: i32,
    pub local_identifier: i32,
    pub skill_line: i32,
    pub system: i32,
    pub variation: i32,
    pub name: String,
    /// Preserve the source entry-vector order.
    pub entries: Vec<NewTraitEntry>,
}

pub struct NewTraitEntry {
    pub node: i32,
    pub entry: i32,
    pub rank: i32,
}
