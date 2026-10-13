// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Bank interaction kept in the World: the show-bank publication the gossip
//! menu also uses, and the autobank/autostore moves, which run the
//! application storage move. The banker activation and the bank slot purchase
//! moved to `wow_world_inventory::vendor_session` (#1363 slice 8b).

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn handle_autobank_item(&mut self, packet: AutoBankItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_autobank_item_with_generator_like_cpp(
            generators.item.as_ref(),
            &catalogs,
            packet,
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn handle_autostore_bank_item(&mut self, packet: AutoStoreBankItem) {
        let generators = self.id_generators_for_test_like_cpp();
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_autostore_bank_item_with_generator_like_cpp(
            generators.item.as_ref(),
            &catalogs,
            packet,
        )
        .await;
    }

    pub(super) fn send_show_bank_like_cpp(&mut self, banker_guid: ObjectGuid) {
        let (state, hub) = crate::session::split_interaction(self);
        state.send_show_bank_like_cpp(hub, banker_guid)
    }

    /// CMSG_AUTOBANK_ITEM — player moves an inventory item into bank storage.
    ///
    /// C++ ref: `WorldSession::HandleAutoBankItemOpcode`.
    pub(crate) async fn handle_autobank_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        packet: AutoBankItem,
    ) {
        if !self.represented_can_use_current_bank_like_cpp() {
            debug!(
                bag = packet.bag,
                slot = packet.slot,
                account = self.core.account_id,
                "AutoBankItem rejected: player cannot use current bank"
            );
            return;
        }

        let represented_move = RepresentedBankItemMoveLikeCpp {
            to_bank: true,
            inv_update_items: packet.inv_update.items,
            bag: packet.bag,
            slot: packet.slot,
        };
        wow_world_application::execute_inventory_storage_move_like_cpp(
            self,
            item_guid_generator,
            creature_spawn_catalogs,
            packet.bag,
            packet.slot,
            NULL_BAG,
            NULL_SLOT,
            InventoryStorageTargetLikeCpp::Bank,
            InventoryStorageQuestChecksLikeCpp::AutoBankItemRemoved,
            Some(represented_move),
        )
        .await;
    }

    /// CMSG_AUTOSTORE_BANK_ITEM — player moves a bank item back to inventory, or inventory to bank.
    ///
    /// C++ ref: `WorldSession::HandleAutoStoreBankItemOpcode`.
    pub(crate) async fn handle_autostore_bank_item_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        packet: AutoStoreBankItem,
    ) {
        if !self.represented_can_use_current_bank_like_cpp() {
            debug!(
                bag = packet.bag,
                slot = packet.slot,
                account = self.core.account_id,
                "AutoStoreBankItem rejected: player cannot use current bank"
            );
            return;
        }

        let target = autostore_bank_target_like_cpp(packet.bag, packet.slot);
        let represented_move = RepresentedBankItemMoveLikeCpp {
            to_bank: target == InventoryStorageTargetLikeCpp::Bank,
            inv_update_items: packet.inv_update.items,
            bag: packet.bag,
            slot: packet.slot,
        };
        wow_world_application::execute_inventory_storage_move_like_cpp(
            self,
            item_guid_generator,
            creature_spawn_catalogs,
            packet.bag,
            packet.slot,
            NULL_BAG,
            NULL_SLOT,
            target,
            autostore_bank_quest_checks_like_cpp(target),
            Some(represented_move),
        )
        .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/character/bank/test_only.rs"]
mod test_only;
