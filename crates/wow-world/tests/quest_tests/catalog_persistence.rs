//! Persistence-port fixtures for quest and item catalog loading scenarios.

use std::sync::Arc;
use wow_persistence::{
    ItemTemplateAddonCatalogPersistencePortLikeCpp, ItemTemplateAddonCatalogRequestLikeCpp,
    ItemTemplateAddonLootMetadataOutcomeLikeCpp, ItemTemplateAddonMoneyOutcomeLikeCpp,
    PersistenceFutureLikeCpp, QuestPoiBlobLoadRowLikeCpp, QuestPoiLoadOutcomeLikeCpp,
    QuestPoiPersistencePortLikeCpp,
};

pub(crate) struct QuestPoiPortFixtureLikeCpp(pub(super) QuestPoiLoadOutcomeLikeCpp);

impl QuestPoiPersistencePortLikeCpp for QuestPoiPortFixtureLikeCpp {
    fn load_quest_poi_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<'_, QuestPoiLoadOutcomeLikeCpp> {
        let outcome = self.0.clone();
        Box::pin(async move { outcome })
    }
}

pub(crate) struct ItemTemplateAddonCatalogPortFixtureLikeCpp {
    pub(super) requests: std::sync::Mutex<Vec<ItemTemplateAddonCatalogRequestLikeCpp>>,
    outcomes:
        std::sync::Mutex<std::collections::VecDeque<ItemTemplateAddonLootMetadataOutcomeLikeCpp>>,
}

impl ItemTemplateAddonCatalogPortFixtureLikeCpp {
    pub(super) fn new(
        outcomes: impl IntoIterator<Item = ItemTemplateAddonLootMetadataOutcomeLikeCpp>,
    ) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }
}

impl ItemTemplateAddonCatalogPersistencePortLikeCpp for ItemTemplateAddonCatalogPortFixtureLikeCpp {
    fn load_item_template_addon_money_like_cpp<'a>(
        &'a self,
        _request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonMoneyOutcomeLikeCpp> {
        panic!("quest source-item lookup never requests item-addon money")
    }

    fn load_item_template_addon_loot_metadata_like_cpp<'a>(
        &'a self,
        request: ItemTemplateAddonCatalogRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, ItemTemplateAddonLootMetadataOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("one item-addon metadata outcome per uncached request");
        Box::pin(async move { outcome })
    }
}

pub(crate) fn quest_poi_blob_row_like_cpp(quest_id: i32, idx1: i32) -> QuestPoiBlobLoadRowLikeCpp {
    QuestPoiBlobLoadRowLikeCpp {
        quest_id,
        blob_index: 1,
        idx1,
        objective_index: -1,
        quest_objective_id: 2,
        quest_object_id: 3,
        map_id: 571,
        ui_map_id: 486,
        priority: 4,
        flags: 5,
        world_effect_id: 6,
        player_condition_id: 7,
        navigation_player_condition_id: 8,
        spawn_tracking_id: 9,
        always_allow_merging_blobs: false,
    }
}
