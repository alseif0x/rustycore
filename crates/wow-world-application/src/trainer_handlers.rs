// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trainer packet-handler registrations for the C++ `NPCHandler.cpp` trainer
//! family (#1263 F5 tail).
//!
//! C++ source of truth: `src/server/game/Handlers/NPCHandler.cpp:98`
//! `WorldSession::HandleTrainerListOpcode`, `:113`
//! `WorldSession::SendTrainerList` and `:132`
//! `WorldSession::HandleTrainerBuySpellOpcode`, plus
//! `src/server/game/Entities/Creature/Trainer.cpp:41` `Trainer::SendSpells` and
//! `:79` `Trainer::TeachSpell`. The application crate owns this registration
//! tail because it is the area that coordinates NPC admission, offer
//! publication and spell acquisition for those two opcodes; the handler bodies
//! stay in their existing owners. Only the registration source moved: both
//! entries keep their opcode, session status, packet processing and handler
//! name exactly as the World registry submitted them.

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::ClientPacket;
use wow_packet::WorldPacket;

/// The narrow host entry points the trainer registration tail needs.
///
/// The World session owns the trainer list publication and the admission
/// catalysts of the purchase, which this crate reaches only through these two
/// calls; the purchase itself is the production
/// [`crate::handle_trainer_buy_spell_with_generator_like_cpp`] operation.
pub trait TrainerHandlerHostLikeCpp<C> {
    /// C++ `HandleTrainerListOpcode` (`NPCHandler.cpp:98-132`).
    fn handle_trainer_list_like_cpp<'a>(
        &'a mut self,
        hello: wow_packet::packets::gossip::Hello,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `HandleTrainerBuySpellOpcode` (`NPCHandler.cpp:132`), served by the
    /// production admitted purchase with the packet's catalogs.
    fn handle_trainer_buy_spell_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;
}

fn handle_trainer_list_thunk<'a, S, C>(
    session: &'a mut S,
    _catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TrainerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match wow_packet::packets::gossip::Hello::read(&mut pkt) {
            Ok(hello) => session.handle_trainer_list_like_cpp(hello).await,
            Err(e) => tracing::warn!("Failed to read TrainerList: {e}"),
        }
    })
}

fn handle_trainer_buy_spell_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TrainerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .handle_trainer_buy_spell_like_cpp(catalogs, pkt)
            .await
    })
}

/// Register the two C++ `NPCHandler.cpp` trainer opcodes.
pub fn register_trainer_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: TrainerHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TrainerList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_trainer_list",
        handler: handle_trainer_list_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::TrainerBuySpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_trainer_buy_spell",
        handler: handle_trainer_buy_spell_thunk::<S, C>,
    })?;
    Ok(())
}
