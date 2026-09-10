//! SQLx-free World catalog contract for on-demand loot templates.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LootTemplateTablePersistenceLikeCpp {
    Item,
    Disenchant,
    Reference,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LootTemplatePersistenceRowLikeCpp {
    pub item_id: u32,
    pub reference: u32,
    pub chance: f32,
    pub needs_quest: bool,
    pub loot_mode: u16,
    pub group_id: u8,
    pub min_count: u8,
    pub max_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootConditionPersistenceRowLikeCpp {
    pub else_group: u32,
    pub condition_type_or_reference: i32,
    pub condition_target: u8,
    pub value1: u32,
    pub value2: u32,
    pub value3: u32,
    pub string_value1: String,
    pub negative: bool,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LootTemplateCatalogOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// C++ `LootStore` source capability. Gameplay retains reference expansion,
/// condition evaluation and random selection; the adapter owns row decoding.
pub trait LootTemplateCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_loot_template_rows_like_cpp(
        &self,
        table: LootTemplateTablePersistenceLikeCpp,
        entry: u32,
    ) -> PersistenceFutureLikeCpp<
        '_,
        LootTemplateCatalogOutcomeLikeCpp<LootTemplatePersistenceRowLikeCpp>,
    >;

    fn load_loot_condition_rows_like_cpp(
        &self,
        source_type: i32,
        source_group: u32,
        source_entry: u32,
    ) -> PersistenceFutureLikeCpp<
        '_,
        LootTemplateCatalogOutcomeLikeCpp<LootConditionPersistenceRowLikeCpp>,
    >;
}
