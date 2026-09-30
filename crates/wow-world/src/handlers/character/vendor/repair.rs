//! repair for the existing vendor owner.

use super::*;

impl WorldSession {

    /// CMSG_REPAIR_ITEM — player repairs item at a repair vendor.
    /// C++ ref: WorldSession::HandleRepairItemOpcode.
    pub async fn handle_repair_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        repair: RepairItem,
    ) {
        let Some(repair_npc) = self.represented_npc_can_interact_with_like_cpp(
            repair.npc_guid,
            NPCFlags1::REPAIR.bits(),
            0,
        ) else {
            debug!(
                npc_guid = ?repair.npc_guid,
                account = self.account_id,
                "RepairItem rejected: NPC missing, out of range, dead, or lacks REPAIR flag"
            );
            return;
        };

        self.remove_represented_feign_death_if_needed_like_cpp();

        // C++ uses GetReputationPriceDiscount(unit) and RATE_REPAIRCOST.
        let discount_mod = self.reputation_price_discount_for_faction_template_like_cpp(
            repair_npc.faction_template_id,
        );
        let repair_cost_rate = self.repair_cost_rate_like_cpp();

        if !repair.item_guid.is_empty() {
            let repaired = self
                .repair_inventory_item_durability_with_generator_like_cpp(
                    item_guid_generator,
                    repair.item_guid,
                    true,
                    discount_mod,
                    repair_cost_rate,
                )
                .await;
            debug!(
                npc_guid = ?repair.npc_guid,
                item_guid = ?repair.item_guid,
                repaired,
                account = self.account_id,
                "RepairItem single-item represented runtime"
            );
            return;
        }

        if repair.use_guild_bank {
            let repaired = self
                .repair_all_inventory_item_durability_with_guild_bank_and_generator_like_cpp(
                    item_guid_generator,
                    discount_mod,
                    repair_cost_rate,
                )
                .await;
            debug!(
                npc_guid = ?repair.npc_guid,
                repaired,
                account = self.account_id,
                "RepairItem all-items represented guild-bank runtime"
            );
            return;
        }

        let repaired = self
            .repair_all_inventory_item_durability_with_player_money_and_generator_like_cpp(
                item_guid_generator,
                discount_mod,
                repair_cost_rate,
            )
            .await;
        debug!(
            npc_guid = ?repair.npc_guid,
            repaired,
            account = self.account_id,
            "RepairItem all-items represented runtime"
        );
    }

    #[cfg(test)]
    pub async fn handle_repair_item(&mut self, repair: RepairItem) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_repair_item_with_generator_like_cpp(generators.item.as_ref(), repair)
            .await;
    }
}
