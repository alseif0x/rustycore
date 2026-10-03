use super::LootState;
use wow_constants::UnitDynFlags;
use wow_core::ObjectGuid;
use wow_loot::{
    LootClaimLease, OwnedLootAuthority, OwnedLootScope, OwnedLootSnapshot,
    creature_loot_is_allowed_to_player_like_cpp,
};
use wow_packet::packets::update::UnitDataValuesDeltaUpdate;
use wow_world_core::session::HubRef;

impl LootState {
    pub fn creature_loot_release_values_for_viewer_like_cpp(
        &self,
        hub: HubRef<'_>,
        creature_guid: ObjectGuid,
        viewer_guid: ObjectGuid,
        viewer_has_pending_bind: bool,
        authority: Option<&OwnedLootAuthority>,
        mut update: UnitDataValuesDeltaUpdate,
    ) -> UnitDataValuesDeltaUpdate {
        let Some(object_data) = update.object_data.as_mut() else {
            return update;
        };
        if object_data.dynamic_flags & UnitDynFlags::Lootable as u32 == 0 {
            return update;
        }
        let Some(authority) = authority else {
            // The authority-less path exists only for bounded unit fixtures.
            // Preserve the canonical flag rather than inventing per-viewer
            // ownership without `Creature::GetLootForPlayer` evidence.
            return update;
        };

        // C++ `ViewerDependentValue<ObjectData::DynamicFlags>` removes
        // UNIT_DYNFLAG_LOOTABLE when the complete `Player::isAllowedToLoot`
        // predicate is false. The object-owned authority is the Rust
        // equivalent of `Creature::GetLootForPlayer`; one exhausted personal
        // pool must not hide a different player's still-live pool.
        let creature_is_dead =
            self.represented_creature_is_dead_for_loot_visibility_like_cpp(hub, creature_guid);
        let viewer_can_still_loot = authority
            .snapshot_for_player_like_cpp(viewer_guid)
            .is_some_and(|snapshot| {
                creature_loot_is_allowed_to_player_like_cpp(
                    creature_is_dead,
                    viewer_has_pending_bind,
                    &snapshot.loot,
                    viewer_guid,
                )
            });
        if !viewer_can_still_loot {
            object_data.dynamic_flags &= !(UnitDynFlags::Lootable as u32);
        }
        update
    }

    /// Rebuild every session-local field derived from one authoritative
    /// snapshot. In particular, a reopened personal creature view must restore
    /// its personal-owner marker and per-player money mirror; restoring only
    /// `loot_table` would make the same pool behave as shared loot.
    pub fn cache_represented_owned_loot_snapshot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        _requested_player_guid: ObjectGuid,
        snapshot: OwnedLootSnapshot,
    ) {
        let OwnedLootSnapshot {
            generation,
            scope,
            loot,
        } = snapshot;
        // One WorldSession caches exactly one selected pool for an owner.
        // Generation scratch may have populated money entries for every
        // encounter tapper, but those peer pools now live in the authority;
        // retaining their session-local markers can misclassify a later
        // shared snapshot as personal loot.
        self.represented_personal_loot_money
            .retain(|(owner, _), _| *owner != owner_guid);
        self.represented_personal_loot_owners.remove(&owner_guid);
        match scope {
            OwnedLootScope::Personal(scope_player_guid) => {
                self.represented_personal_loot_owners.insert(owner_guid);
                self.represented_personal_loot_money
                    .insert((owner_guid, scope_player_guid), loot.coins);
            }
            OwnedLootScope::Shared => {}
        }
        self.loot_table.insert(owner_guid, loot);
        self.represented_loot_cache_generations_like_cpp
            .insert(owner_guid, generation);
    }

    pub fn represented_active_loot_claim_generation_matches_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        claim: &LootClaimLease,
    ) -> bool {
        self.active_loot_view_authorities_like_cpp
            .get(&owner_guid)
            .is_some_and(|opened| claim.shares_authority_like_cpp(opened))
            && self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| *opened == claim.generation_like_cpp())
    }
}
