// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `authority` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl WorldSession {
    pub(crate) async fn request_represented_remote_master_loot_give_like_cpp(
        &self,
        target: ObjectGuid,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        dungeon_encounter_id: u32,
        entry: LootEntry,
        claim: Option<LootClaimLease>,
    ) -> wow_world_core::session::mailbox::MasterLootGiveResult {
        let (state, hub) = crate::session::split_loot_ref(self);
        state
            .request_represented_remote_master_loot_give_like_cpp(
                hub,
                target,
                owner_guid,
                loot_obj,
                loot_list_id,
                dungeon_encounter_id,
                entry,
                claim,
            )
            .await
    }
}
