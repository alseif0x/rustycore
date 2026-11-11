//! Frozen WorldSession-owned projections saved in the CharacterDB transaction.

pub struct NewCharacterSessionSave {
    pub tutorials: NewTutorialSave,
    /// Preserve source unordered instance-map iteration order with unique IDs.
    pub instance_restrictions: Vec<NewInstanceRestriction>,
}

pub enum NewTutorialSave {
    Unchanged,
    Changed {
        loaded_from_db: bool,
        values: [u32; 8],
    },
}

pub struct NewInstanceRestriction {
    pub instance_id: u32,
    /// Already projected source clock::to_time_t -> uint64 value. The adapter
    /// neither converts this value nor prunes it against the current time.
    pub release_time: u64,
}
