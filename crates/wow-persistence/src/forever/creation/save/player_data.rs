//! Frozen Player-data lookup/storage projection; canonical resolution is absent.
use std::collections::BTreeMap;

pub struct NewPlayerDataSave {
    /// Dirty DB2 element IDs, not lookup StorageIndex values.
    pub elements: BTreeMap<u32, Option<NewPlayerDataValue>>,
    /// Dirty bitvector word indexes (SQL storageIndex), computed by the canonical
    /// writer as DB2 StorageIndex / PLAYER_DATA_FLAG_VALUE_BITS; not raw bit offsets or flag IDs.
    pub flags: BTreeMap<u32, u64>,
}

pub enum NewPlayerDataValue {
    Int64(i64),
    Float(f32),
}
