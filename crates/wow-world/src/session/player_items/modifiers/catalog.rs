//! catalog for the existing modifiers owner.

use super::*;

impl WorldSession {
    /// Set the C++ ItemLimitCategory.db2 store for this session.
    pub fn set_item_limit_category_store(&mut self, store: Arc<ItemLimitCategoryStore>) {
        self.items.limit_category_store = Some(store);
    }
    /// Set the C++ ItemLimitCategoryCondition.db2 store for this session.
    pub fn set_item_limit_category_condition_store(
        &mut self,
        store: Arc<ItemLimitCategoryConditionStore>,
    ) {
        self.items.limit_category_condition_store = Some(store);
    }
    /// C++ `sItemLimitCategoryStore.LookupEntry(limitCategory)`.
    pub(crate) fn item_limit_category_template_like_cpp(
        &self,
        limit_category_id: u32,
    ) -> Option<ItemLimitCategoryTemplate> {
        if limit_category_id == 0 {
            return None;
        }

        let entry = self
            .items
            .limit_category_store
            .as_ref()
            .and_then(|store| store.get(limit_category_id))?;

        let mut quantity = entry.quantity;
        if let Some(condition_store) = self.items.limit_category_condition_store.as_ref() {
            let context_holder = self.represented_player_condition_context_like_cpp()?;
            let context = context_holder.as_context(self)?;
            for condition in condition_store.conditions_for_parent_like_cpp(entry.id) {
                let player_condition = self
                    .player_condition_store
                    .as_ref()
                    .and_then(|store| store.get(condition.player_condition_id));
                if player_condition.is_none_or(|condition| {
                    is_player_meeting_condition_like_cpp(condition, &context)
                }) {
                    quantity = (i16::from(quantity) + i16::from(condition.add_quantity)) as u8;
                }
            }
        }

        Some(ItemLimitCategoryTemplate {
            id: entry.id,
            quantity,
            flags: entry.flags,
        })
    }
    /// Set the item stats store for this session.
    pub fn set_item_bonus_db2_store(&mut self, store: Arc<ItemBonusDb2Store>) {
        self.items.bonus_db2_store = Some(store);
    }
    pub fn set_item_set_store(&mut self, store: Arc<ItemSetStore>) {
        self.items.set_store = Some(store);
    }
    pub fn set_item_set_spell_store(&mut self, store: Arc<ItemSetSpellStore>) {
        self.spell_catalogs.item_set_spell_store = Some(store);
    }
    pub(crate) fn item_set_for_item_id_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<&wow_data::ItemSetEntry> {
        self.items
            .set_store
            .as_ref()
            .and_then(|store| store.item_set_for_item_id_like_cpp(item_id))
    }
    pub(crate) fn item_set_spells_like_cpp(
        &self,
        item_set_id: u32,
    ) -> Vec<&wow_data::ItemSetSpellEntry> {
        self.spell_catalogs
            .item_set_spell_store
            .as_ref()
            .map(|store| store.item_set_spells_like_cpp(item_set_id))
            .unwrap_or_default()
    }
}
