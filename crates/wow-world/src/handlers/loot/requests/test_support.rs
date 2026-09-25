// Copyright (c) 2026 alseif0x
//! Test-only adapters for exercising represented loot request operations.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(in crate::handlers::loot) async fn store_represented_disenchant_loot_winner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        entry: &LootEntry,
        winner_guid: ObjectGuid,
        dungeon_encounter_id: u32,
        claim: Option<&LootClaimLease>,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.store_represented_disenchant_loot_winner_with_generator_like_cpp(
            generator.as_ref(),
            &item_valuation,
            owner_guid,
            loot_obj,
            loot_list_id,
            entry,
            winner_guid,
            dungeon_encounter_id,
            claim,
        )
        .await
    }

    #[cfg(test)]
    pub(in crate::handlers::loot) fn represented_on_loot_opened_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        response: LootResponse,
    ) {
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.represented_on_loot_opened_with_catalogs_like_cpp(
            &item_valuation,
            owner_guid,
            player_guid,
            response,
        );
    }

    #[cfg(test)]
    pub(in crate::handlers::loot) async fn store_direct_loot_item_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.store_direct_loot_item_with_generator_like_cpp(
            generator.as_ref(),
            loot_entry,
            dungeon_encounter_id,
        )
        .await
    }
    /// Persist the complete result of one group-roll disenchant as a single
    /// durable award.
    ///
    /// C++ `LootRoll::Finish` first materializes a temporary
    /// `LOOT_DISENCHANTING` loot and then calls `Loot::AutoStore`.  The C++
    /// loop stores each generated material independently, which is unsafe for
    /// Rust's concurrently shared object authority: a later failure could
    /// reopen the original roll slot after an earlier material was durable.
    /// This bounded divergence keeps C++ generation/inventory rules but plans
    /// every material before creating one SQL transaction.  The detached
    /// transaction worker owns the original roll claim through COMMIT.
    #[cfg(test)]
    pub(in crate::handlers::loot) async fn store_direct_disenchant_batch_like_cpp(
        &mut self,
        loot_entries: &[LootEntry],
        dungeon_encounter_id: u32,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.store_direct_disenchant_batch_with_generator_like_cpp(
            generator.as_ref(),
            loot_entries,
            dungeon_encounter_id,
            claim,
            claim_commit_context,
        )
        .await
    }

    #[cfg(test)]
    pub(in crate::handlers::loot) async fn store_direct_loot_item_from_owner_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.store_direct_loot_item_from_owner_with_generator_like_cpp(
            generator.as_ref(),
            loot_entry,
            dungeon_encounter_id,
            owner_guid,
        )
        .await
    }

    #[cfg(test)]
    pub(in crate::handlers::loot) async fn store_direct_loot_item_with_source_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        stored_item_loot_source: Option<ObjectGuid>,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.store_direct_loot_item_with_source_and_generator_like_cpp(
            generator.as_ref(),
            loot_entry,
            dungeon_encounter_id,
            stored_item_loot_source,
            claim,
            claim_commit_context,
        )
        .await
    }
}
