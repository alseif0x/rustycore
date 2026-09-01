//! SQLx-free startup source for bounded gameplay rule catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcSpellClickPersistenceRowLikeCpp {
    pub npc_entry: u32,
    pub spell_id: u32,
    pub cast_flags: u8,
    pub user_type: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcVendorPersistenceRowLikeCpp {
    pub entry: u32,
    pub item: i32,
    pub maxcount: u32,
    pub incrtime: u32,
    pub extended_cost: u32,
    pub vendor_type: u8,
    pub bonus_list_ids_raw: String,
    pub player_condition_id: u32,
    pub ignore_filtering: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactionChangePairPersistenceRowLikeCpp {
    pub alliance_id: u32,
    pub horde_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FactionChangePersistenceRowsLikeCpp {
    pub achievements: Vec<FactionChangePairPersistenceRowLikeCpp>,
    pub quests: Vec<FactionChangePairPersistenceRowLikeCpp>,
    pub reputations: Vec<FactionChangePairPersistenceRowLikeCpp>,
    pub spells: Vec<FactionChangePairPersistenceRowLikeCpp>,
    pub titles: Vec<FactionChangePairPersistenceRowLikeCpp>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameplayRuleRowsLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

pub trait GameplayRuleCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_npc_spell_click_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameplayRuleRowsLoadOutcomeLikeCpp<Vec<NpcSpellClickPersistenceRowLikeCpp>>,
    >;

    fn load_npc_vendor_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameplayRuleRowsLoadOutcomeLikeCpp<Vec<NpcVendorPersistenceRowLikeCpp>>,
    >;

    fn load_faction_change_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        GameplayRuleRowsLoadOutcomeLikeCpp<FactionChangePersistenceRowsLikeCpp>,
    >;
}
