//! SQLx-free World source contract for item random-enchantment templates.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemRandomEnchantmentPersistenceRowLikeCpp {
    pub group_id: u32,
    pub enchantment_id: u32,
    pub chance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp {
    Loaded(Vec<ItemRandomEnchantmentPersistenceRowLikeCpp>),
    Failed { reason: String },
}

pub trait ItemRandomEnchantmentCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, ItemRandomEnchantmentCatalogLoadOutcomeLikeCpp>;
}
