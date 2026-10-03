// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::ClientOpcodes;
use wow_entities::PlayerEquipmentSetsLikeCpp;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::OwnedEquipmentSetsAccessLikeCpp;

use crate::{
    EquipmentSetsSaveCxLikeCpp, InventoryState, MAX_EQUIPMENT_SET_INDEX_LIKE_CPP,
};

/// Private-state context for packet handlers that mutate equipment sets.
pub struct EquipmentSetsHandlerCxLikeCpp<'a> {
    #[cfg(any(test, feature = "test-fixtures"))]
    state: &'a mut InventoryState,
    owner: OwnedEquipmentSetsAccessLikeCpp<'a>,
}

impl<'a> EquipmentSetsHandlerCxLikeCpp<'a> {
    /// Borrow an InventoryState and its canonical-player capability for one
    /// synchronous equipment-set operation.
    pub fn new(
        _state: &'a mut InventoryState,
        owner: OwnedEquipmentSetsAccessLikeCpp<'a>,
    ) -> Self {
        Self {
            #[cfg(any(test, feature = "test-fixtures"))]
            state: _state,
            owner,
        }
    }

    fn with_owned_equipment_sets_mut_like_cpp<R>(
        &mut self,
        mut f: impl FnMut(&mut PlayerEquipmentSetsLikeCpp) -> R,
    ) -> Option<R> {
        let canonical = self
            .owner
            .with_equipment_sets_mut_like_cpp(|sets| f(sets));
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.owner.owner_handle_absent_like_cpp() {
            return Some(f(&mut self.state.represented_equipment_sets_like_cpp));
        }
        None
    }

    pub fn assign_represented_equipment_set_to_spec_like_cpp(
        &mut self,
        set_id: u32,
        spec_index: u32,
    ) -> bool {
        if set_id >= MAX_EQUIPMENT_SET_INDEX_LIKE_CPP {
            return false;
        }

        self.with_owned_equipment_sets_mut_like_cpp(|sets| {
            sets.assign_set_to_spec_like_cpp(set_id, spec_index as i32)
        })
        .unwrap_or(false)
    }

    pub fn delete_represented_equipment_set_like_cpp(&mut self, id: u64) -> bool {
        self.with_owned_equipment_sets_mut_like_cpp(|sets| sets.delete_set_like_cpp(id))
            .unwrap_or(false)
    }

    /// Decode and apply CMSG_ASSIGN_EQUIPMENT_SET_SPEC synchronously.
    ///
    /// Target C++ `Opcodes.cpp:170` leaves this opcode unhandled and routes it
    /// to `Handle_NULL`; parity for this existing Rust behavior remains an F6
    /// evidence question.
    pub fn handle_assign_equipment_set_spec(&mut self, mut pkt: WorldPacket) {
        let request = match wow_packet::packets::misc::AssignEquipmentSetSpec::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                tracing::warn!("Bad AssignEquipmentSetSpec: {error}");
                return;
            }
        };

        let _assigned = self.assign_represented_equipment_set_to_spec_like_cpp(
            request.set_id,
            request.spec_index,
        );
    }

    /// Decode and apply CMSG_DELETE_EQUIPMENT_SET synchronously.
    pub fn handle_delete_equipment_set(&mut self, mut pkt: WorldPacket) {
        let request = match wow_packet::packets::misc::DeleteEquipmentSet::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                tracing::warn!("Bad DeleteEquipmentSet: {error}");
                return;
            }
        };

        let _deleted = self.delete_represented_equipment_set_like_cpp(request.id);
    }
}

/// Builds an Inventory handler context from a host's state and request catalogs.
pub trait InventoryHandlerHostLikeCpp<C> {
    fn equipment_sets_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> EquipmentSetsHandlerCxLikeCpp<'a>;

    fn equipment_sets_save_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> EquipmentSetsSaveCxLikeCpp<'a>;
}

fn handle_save_equipment_set_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .equipment_sets_save_handler_cx_like_cpp(catalogs)
            .handle_save_equipment_set(pkt);
    })
}

fn handle_assign_equipment_set_spec_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .equipment_sets_handler_cx_like_cpp(catalogs)
            .handle_assign_equipment_set_spec(pkt);
    })
}

fn handle_delete_equipment_set_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .equipment_sets_handler_cx_like_cpp(catalogs)
            .handle_delete_equipment_set(pkt);
    })
}

pub fn register_inventory_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: InventoryHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SaveEquipmentSet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_save_equipment_set",
        handler: handle_save_equipment_set_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AssignEquipmentSetSpec,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_assign_equipment_set_spec",
        handler: handle_assign_equipment_set_spec_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::DeleteEquipmentSet,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_delete_equipment_set",
        handler: handle_delete_equipment_set_thunk::<S, C>,
    })?;
    Ok(())
}
