// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Bank flag admission, mutation and complete Player VALUES publication.

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{
    NpcInteractionAccessLikeCpp, OwnedInventoryAccessLikeCpp, PacketPublicationAccessLikeCpp,
};
use wow_world_interaction::InteractionState;
use wow_world_inventory::InventoryState;

pub fn can_use_current_bank_with_access_like_cpp(
    interaction: &InteractionState,
    npc: &NpcInteractionAccessLikeCpp<'_>,
    inventory: &OwnedInventoryAccessLikeCpp<'_>,
) -> bool {
    let Some(banker_guid) = interaction.player_interaction_source_guid_with_access_like_cpp(npc)
    else {
        return false;
    };
    if Some(banker_guid) == inventory.player_guid_like_cpp() {
        return true;
    }
    npc.represented_npc_can_interact_with_like_cpp(
        banker_guid,
        wow_constants::NPCFlags1::BANKER.bits(),
        0,
    )
    .is_some()
}

pub struct BankSlotFlagApplicationCxLikeCpp<'a> {
    state: &'a mut InventoryState,
    interaction: &'a InteractionState,
    npc: NpcInteractionAccessLikeCpp<'a>,
    inventory: OwnedInventoryAccessLikeCpp<'a>,
    publication: PacketPublicationAccessLikeCpp<'a>,
    item_store: Option<&'a std::sync::Arc<wow_data::ItemStore>>,
    item_stats_store: Option<&'a std::sync::Arc<wow_data::ItemStatsStore>>,
}

impl<'a> BankSlotFlagApplicationCxLikeCpp<'a> {
    pub fn new(
        state: &'a mut InventoryState,
        interaction: &'a InteractionState,
        npc: NpcInteractionAccessLikeCpp<'a>,
        inventory: OwnedInventoryAccessLikeCpp<'a>,
        publication: PacketPublicationAccessLikeCpp<'a>,
        item_store: Option<&'a std::sync::Arc<wow_data::ItemStore>>,
        item_stats_store: Option<&'a std::sync::Arc<wow_data::ItemStatsStore>>,
    ) -> Self {
        Self {
            state,
            interaction,
            npc,
            inventory,
            publication,
            item_store,
            item_stats_store,
        }
    }

    /// Existing Rust behavior only: target Opcodes.cpp:289 registers this
    /// opcode STATUS_UNHANDLED / Handle_NULL. That difference remains F6.
    pub fn change_bank_bag_slot_flag_like_cpp(
        &mut self,
        packet: wow_packet::packets::misc::ChangeBankBagSlotFlag,
    ) {
        if !can_use_current_bank_with_access_like_cpp(self.interaction, &self.npc, &self.inventory)
        {
            tracing::debug!(
                account = self.npc.account_id_like_cpp(),
                "ChangeBankBagSlotFlag rejected: player cannot use current bank"
            );
            return;
        }
        let Ok(slot) = usize::try_from(packet.slot) else {
            return;
        };
        if slot >= 7 {
            tracing::debug!(
                slot = packet.slot,
                account = self.npc.account_id_like_cpp(),
                "ChangeBankBagSlotFlag rejected: invalid bank bag slot"
            );
            return;
        }
        if packet.flag >= u32::BITS {
            tracing::debug!(
                flag = packet.flag,
                account = self.npc.account_id_like_cpp(),
                "ChangeBankBagSlotFlag rejected: invalid flag bit"
            );
            return;
        }
        let Some(current) = self
            .state
            .represented_bank_bag_slot_flag_with_access_like_cpp(&self.inventory, slot)
        else {
            return;
        };
        let mask = 1u32 << packet.flag;
        let updated = if packet.enabled {
            current | mask
        } else {
            current & !mask
        };
        if !self
            .state
            .set_represented_bank_bag_slot_flag_with_access_like_cpp(&self.inventory, slot, updated)
        {
            return;
        }
        self.state
            .send_player_bank_bag_slot_flag_update_with_access_like_cpp(
                &self.inventory,
                &self.publication,
                self.item_store,
                self.item_stats_store,
                slot,
                updated,
            );
    }
}

pub trait BankHandlerHostLikeCpp<C> {
    fn bank_slot_flag_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> BankSlotFlagApplicationCxLikeCpp<'a>;
}

fn thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match wow_packet::packets::misc::ChangeBankBagSlotFlag::read(&mut pkt) {
            Ok(change) => session
                .bank_slot_flag_handler_cx_like_cpp(catalogs)
                .change_bank_bag_slot_flag_like_cpp(change),
            Err(e) => tracing::warn!("Failed to read ChangeBankBagSlotFlag: {e}"),
        }
    })
}

pub fn register_bank_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: BankHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ChangeBankBagSlotFlag,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_change_bank_bag_slot_flag",
        handler: thunk::<S, C>,
    })?;
    Ok(())
}
