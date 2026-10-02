// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;
use wow_packet::packets::misc::FeatureSystemConfigLikeCpp;

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

/// Process-owned support and feature-system policy borrowed by Session edges.
///
/// C++ initializes the support switches in `World.cpp:584-595` and the feature
/// switches in `World.cpp:1597-1599`. `SupportMgr` and the feature-status send
/// helpers read that process state; it is not copied into each `WorldSession`.
#[derive(Debug, Clone, Copy)]
pub struct SupportFeaturePolicyLikeCpp {
    pub support_enabled: bool,
    pub tickets_enabled: bool,
    pub bugs_enabled: bool,
    pub complaints_enabled: bool,
    pub suggestions_enabled: bool,
    pub character_undelete_enabled: bool,
    pub bpay_store_enabled: bool,
    pub max_characters_per_realm: u32,
    pub declined_names_used: bool,
}

impl Default for SupportFeaturePolicyLikeCpp {
    fn default() -> Self {
        Self {
            support_enabled: true,
            tickets_enabled: false,
            bugs_enabled: false,
            complaints_enabled: false,
            suggestions_enabled: false,
            character_undelete_enabled: false,
            bpay_store_enabled: false,
            max_characters_per_realm: 60,
            declined_names_used: false,
        }
    }
}

impl SupportFeaturePolicyLikeCpp {
    pub fn bug_system_enabled_like_cpp(self) -> bool {
        self.support_enabled && self.bugs_enabled
    }

    pub fn complaint_system_enabled_like_cpp(self) -> bool {
        self.support_enabled && self.complaints_enabled
    }

    pub fn suggestion_system_enabled_like_cpp(self) -> bool {
        self.support_enabled && self.suggestions_enabled
    }

    pub(in crate::session) fn feature_system_config_like_cpp(self) -> FeatureSystemConfigLikeCpp {
        FeatureSystemConfigLikeCpp {
            support_tickets_enabled: self.tickets_enabled,
            support_bugs_enabled: self.bugs_enabled,
            support_complaints_enabled: self.complaints_enabled,
            support_suggestions_enabled: self.suggestions_enabled,
            char_undelete_enabled: self.character_undelete_enabled,
            bpay_store_enabled: self.bpay_store_enabled,
        }
    }
}
