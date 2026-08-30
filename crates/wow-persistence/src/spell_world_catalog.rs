//! SQLx-free startup source contract for foundational C++ `SpellMgr` World catalogs.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq)]
pub enum SpellWorldCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellRequiredPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub req_spell: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellThreatPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub flat_mod: i32,
    pub pct_mod: f32,
    pub ap_pct_mod: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellLinkedPersistenceRowLikeCpp {
    pub spell_trigger: i32,
    pub spell_effect: i32,
    pub link_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellTotemModelPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub race_id: u8,
    pub display_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellPetAuraPersistenceRowLikeCpp {
    pub spell_id: u32,
    pub effect_index: u8,
    pub pet_entry: u32,
    pub aura_id: u32,
}

/// Independent World-table reads used by foundational C++ `SpellMgr`
/// catalogs. The concrete adapter owns statement identity and tolerant row
/// decoding; `wow-data` retains validation and immutable catalog ownership.
pub trait SpellWorldCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_spell_required_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellRequiredPersistenceRowLikeCpp>,
    >;

    fn load_spell_threat_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellThreatPersistenceRowLikeCpp>,
    >;

    fn load_spell_linked_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellLinkedPersistenceRowLikeCpp>,
    >;

    fn load_spell_totem_model_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellTotemModelPersistenceRowLikeCpp>,
    >;

    fn load_spell_pet_aura_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        SpellWorldCatalogLoadOutcomeLikeCpp<SpellPetAuraPersistenceRowLikeCpp>,
    >;
}
