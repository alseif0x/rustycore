//! SQLx-free startup source for ConditionMgr and DisableMgr rows.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionPersistenceRowLikeCpp {
    pub source_type_or_reference_id: i32,
    pub source_group: u32,
    pub source_entry: i32,
    pub source_id: u32,
    pub else_group: u32,
    pub condition_type_or_reference: i32,
    pub condition_target: u8,
    pub condition_value1: u32,
    pub condition_value2: u32,
    pub condition_value3: u32,
    pub condition_string_value1: String,
    pub negative_condition: bool,
    pub error_type: u32,
    pub error_text_id: u32,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisablePersistenceRowLikeCpp {
    pub source_type: u32,
    pub entry: u32,
    pub flags: u16,
    pub params_0: String,
    pub params_1: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionDisableRowsLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

pub trait ConditionDisableCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_condition_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ConditionDisableRowsLoadOutcomeLikeCpp<Vec<ConditionPersistenceRowLikeCpp>>,
    >;

    fn load_disable_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ConditionDisableRowsLoadOutcomeLikeCpp<Vec<DisablePersistenceRowLikeCpp>>,
    >;
}
