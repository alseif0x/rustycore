// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapters for the travel bodies that moved to
//! `wow_world_instances` (`CMSG_AREA_TRIGGER`) and `wow_world_entities`
//! (`CMSG_ACTIVATE_TAXI`) (#1363 slice 4d). Each method is the same-named
//! session operation or catalog field; the area-trigger catalog access moved
//! here from `session/instances/host.rs` with its two new fields.

use std::sync::Arc;

use wow_data::{
    AreaTriggerDb2Store, AreaTriggerScriptStoreLikeCpp, AreaTriggerStore,
    TavernAreaTriggerStoreLikeCpp,
};
use wow_world_instances::AreaTriggerCatalogAccessLikeCpp;

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

/// The area-trigger opcode's one World operation (#1363 slice 4d): the
/// `ScriptMgr::OnAreaTrigger` dispatcher receives this session, so it stays a
/// World type and is invoked here exactly as the body invoked it.
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

impl wow_world_entities::ActivateTaxiHostLikeCpp for WorldSession {
    fn represented_taxi_usable_mount_displays_like_cpp(&self, flying_mount_id: u32) -> Vec<i32> {
        WorldSession::represented_taxi_usable_mount_displays_like_cpp(self, flying_mount_id)
    }
}
