//! Unlock and query operations of void_storage.
//!
//! Divided out of the single inherent impl under #707; every method keeps
//! its name, signature and body.

use super::*;

impl WorldSession {
    pub async fn handle_void_storage_unlock_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: WorldPacket,
    ) {
        let Ok(unlock) = UnlockVoidStorage::read(&mut pkt) else {
            return;
        };
        if self
            .represented_npc_can_interact_with_like_cpp(
                unlock.npc,
                NPCFlags1::VAULT_KEEPER.bits(),
                0,
            )
            .is_none()
            || self.void_storage_is_unlocked_like_cpp()
        {
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(port) = self.void_storage_persistence_port_like_cpp() else {
            return;
        };
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };

        // C++ ModifyMoney clamps at zero; unlocking does not have a separate
        // HasEnoughMoney gate in this audited branch.
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        let new_money = old_money.saturating_sub(VOID_STORAGE_UNLOCK_COST_LIKE_CPP);
        let Some(new_flags) = self.represented_player_flags_value_like_cpp() else {
            return;
        };
        let new_flags = new_flags | crate::session::PLAYER_FLAGS_VOID_UNLOCKED_LIKE_CPP;
        let request = wow_persistence::VoidStorageUnlockWriteRequestLikeCpp {
            player_guid: player_guid.counter() as u64,
            money_before: old_money,
            money_after: new_money,
            player_flags_after: new_flags,
        };
        let Some(money_persistence) = self
            .await_exclusive_player_money_transaction_outcome_like_cpp(
                money_persistence,
                port.persist_void_storage_unlock_like_cpp(request),
                old_money,
                new_money,
                "void-storage unlock",
            )
            .await
        else {
            return;
        };

        if !self.stage_player_money_change_like_cpp(old_money, new_money) {
            self.kick(
                "canonical Player money owner became unavailable after void-storage unlock COMMIT",
            );
            return;
        }
        self.apply_committed_void_storage_unlock_like_cpp();
        self.sync_player_registry_state_like_cpp();
        drop(money_persistence);

        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;
        if old_money != new_money {
            self.send_player_values_update_from_entity_bridge(&[], &[], &[], &[], Some(new_money));
        }
    }

    #[cfg(test)]
    pub async fn handle_void_storage_unlock(&mut self, pkt: WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_void_storage_unlock_with_generator_like_cpp(generators.item.as_ref(), pkt)
            .await;
    }

    pub async fn handle_void_storage_query(&mut self, mut pkt: WorldPacket) {
        let Ok(query) = QueryVoidStorage::read(&mut pkt) else {
            return;
        };
        if self
            .represented_npc_can_interact_with_like_cpp(
                query.npc,
                (NPCFlags1::TRANSMOGRIFIER | NPCFlags1::VAULT_KEEPER).bits(),
                0,
            )
            .is_none()
            || !self.void_storage_is_unlocked_like_cpp()
            || self.represented_void_storage_loaded_like_cpp() != Some(true)
        {
            self.send_packet(&VoidStorageFailed::default());
            return;
        }

        let Some(contents) = self.represented_void_storage_contents_like_cpp() else {
            self.send_packet(&VoidStorageFailed::default());
            return;
        };
        self.send_packet(&contents);
    }
}
