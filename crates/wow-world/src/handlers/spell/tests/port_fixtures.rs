//! Persistence port and catalog doubles for the spell handler scenarios.
//!
//! Split out of the inline test module under #624; behaviour unchanged.

use super::*;

pub(super) struct ItemTemplateAddonCatalogPortFixtureLikeCpp {
    pub(super) money_requests: Mutex<Vec<ItemTemplateAddonCatalogRequestLikeCpp>>,
    pub(super) money_outcomes: Mutex<VecDeque<ItemTemplateAddonMoneyOutcomeLikeCpp>>,
    pub(super) metadata_requests: Mutex<Vec<ItemTemplateAddonCatalogRequestLikeCpp>>,
    pub(super) metadata_outcomes: Mutex<VecDeque<ItemTemplateAddonLootMetadataOutcomeLikeCpp>>,
}
impl ItemTemplateAddonCatalogPortFixtureLikeCpp {
    pub(super) fn new(
        money_outcomes: impl IntoIterator<Item = ItemTemplateAddonMoneyOutcomeLikeCpp>,
        metadata_outcomes: impl IntoIterator<Item = ItemTemplateAddonLootMetadataOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            money_requests: Mutex::new(Vec::new()),
            money_outcomes: Mutex::new(money_outcomes.into_iter().collect()),
            metadata_requests: Mutex::new(Vec::new()),
            metadata_outcomes: Mutex::new(metadata_outcomes.into_iter().collect()),
        })
    }
}
impl ItemTemplateAddonCatalogPersistencePortLikeCpp for ItemTemplateAddonCatalogPortFixtureLikeCpp {
    fn load_item_template_addon_money_like_cpp<'a>(
        &'a self,
        request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonMoneyOutcomeLikeCpp> {
        self.money_requests.lock().unwrap().push(request);
        let outcome = self
            .money_outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one item-addon money outcome per request");
        Box::pin(async move { outcome })
    }

    fn load_item_template_addon_loot_metadata_like_cpp<'a>(
        &'a self,
        request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonLootMetadataOutcomeLikeCpp> {
        self.metadata_requests.lock().unwrap().push(request);
        let outcome = self
            .metadata_outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one item-addon metadata outcome per request");
        Box::pin(async move { outcome })
    }
}
