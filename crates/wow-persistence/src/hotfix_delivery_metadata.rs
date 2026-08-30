//! SQLx-free Hotfix source contract for client-delivery metadata.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixBlobPersistenceRowLikeCpp {
    pub table_hash: u32,
    pub record_id: i32,
    pub locale: String,
    pub blob: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixDataPersistenceRowLikeCpp {
    pub push_id: i32,
    pub unique_id: u32,
    pub table_hash: u32,
    pub record_id: i32,
    pub status: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixOptionalDataPersistenceRowLikeCpp {
    pub table_hash: u32,
    pub record_id: i32,
    pub locale: String,
    pub key: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotfixDeliveryMetadataLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// C++ `DB2Manager` Hotfix delivery sources, kept staged so startup can
/// preserve blob -> data -> optional-data application and failure behavior.
pub trait HotfixDeliveryMetadataPersistencePortLikeCpp: Send + Sync {
    fn load_hotfix_blob_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixBlobPersistenceRowLikeCpp>,
    >;

    fn load_hotfix_data_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixDataPersistenceRowLikeCpp>,
    >;

    fn load_hotfix_optional_data_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        HotfixDeliveryMetadataLoadOutcomeLikeCpp<HotfixOptionalDataPersistenceRowLikeCpp>,
    >;
}
