// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

/// Capability-specific immutable query owner consumed by query/gameobject
/// handlers. It mirrors C++ ObjectMgr startup stores and contains no database
/// handle or mutable gameplay state.
#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "test-fixtures"), derive(Default))]
pub struct ObjectMgrCatalogsLikeCpp {
    pub creature: Arc<wow_data::CreatureQueryCatalogLikeCpp>,
    pub gameobject: Arc<wow_data::GameObjectQueryCatalogLikeCpp>,
    pub gameobject_quest_items: Arc<wow_data::GameObjectQuestItemStoreLikeCpp>,
    pub page_text: Arc<wow_data::PageTextCatalogLikeCpp>,
}
