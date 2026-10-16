// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapters for the leaf bodies moved by #1363 slice 4d: the area
//! trigger and taxi activation and the group receivers, reconciliation and
//! publication tail (`wow_world_instances`), and `CMSG_ADD_TOY`
//! (`wow_world_inventory`). Each method is the same-named session operation,
//! split or field; the area-trigger catalog access moved here from
//! `session/instances/host.rs` with its two new fields.

use std::sync::Arc;

use wow_constants::InventoryResult;
use wow_data::{
    AreaTriggerDb2Store, AreaTriggerScriptStoreLikeCpp, AreaTriggerStore,
    TavernAreaTriggerStoreLikeCpp,
};
use wow_entities::{Item, PlayerInventoryItem as InventoryItem};
use wow_social::group::GroupDifficultyKindLikeCpp;
use wow_world_core::session::{HubMut, HubRef};
use wow_world_instances::{AreaTriggerCatalogAccessLikeCpp, GroupSessionHostLikeCpp};
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_social::SessionSocialLimits;

use crate::session::{AreaTriggerCatalogsLikeCpp, WorldSession};

impl AreaTriggerCatalogAccessLikeCpp for AreaTriggerCatalogsLikeCpp {
    fn destinations(&self) -> &Arc<AreaTriggerStore> {
        &self.destinations
    }
    fn taverns(&self) -> &Arc<TavernAreaTriggerStoreLikeCpp> {
        &self.taverns
    }
    fn db2(&self) -> &Arc<AreaTriggerDb2Store> {
        &self.db2
    }
    fn scripts(&self) -> &Arc<AreaTriggerScriptStoreLikeCpp> {
        &self.scripts
    }
}

/// The `ScriptMgr::OnAreaTrigger` dispatcher receives this session, so it stays
/// a World type and is invoked here exactly as the moved body invoked it.
impl wow_world_instances::AreaTriggerHostLikeCpp for WorldSession {
    fn dispatch_area_trigger_script_like_cpp(
        &mut self,
        catalogs: &AreaTriggerCatalogsLikeCpp,
        script_id: wow_data::ScriptIdLikeCpp,
        trigger_id: u32,
        entered: bool,
    ) -> Option<bool> {
        WorldSession::dispatch_area_trigger_script_like_cpp(
            self,
            catalogs.script_dispatcher.as_ref(),
            script_id,
            trigger_id,
            entered,
        )
    }
}

impl GroupSessionHostLikeCpp for WorldSession {
    fn split_social_ref(&self) -> (&SessionSocialLimits, HubRef<'_>) {
        crate::session::split_social_ref(self)
    }
    fn split_social_mut(&mut self) -> (&mut SessionSocialLimits, HubMut<'_>) {
        crate::session::split_social_mut(self)
    }
    fn lifecycle(&self) -> &SessionLifecycleState {
        &self.lifecycle
    }
    fn update_visible_gameobjects_or_spell_clicks_like_cpp(&self) -> usize {
        WorldSession::update_visible_gameobjects_or_spell_clicks_like_cpp(self)
    }
    fn apply_group_difficulty_like_cpp(
        &mut self,
        group_guid: u64,
        difficulty_id: u32,
        kind: GroupDifficultyKindLikeCpp,
    ) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        cx.apply_group_difficulty_like_cpp(group_guid, difficulty_id, kind);
    }
}

impl wow_world_inventory::AddToyHostLikeCpp for WorldSession {
    fn can_use_inventory_item_represented_like_cpp(
        &self,
        item: &InventoryItem,
        runtime_item: Option<&Item>,
    ) -> InventoryResult {
        WorldSession::can_use_inventory_item_represented_like_cpp(self, item, runtime_item)
    }
}
