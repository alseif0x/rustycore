// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Talent packet-handler registrations (#1263 F5, deferred families).
//!
//! C++ source of truth: `src/server/game/Handlers/SkillHandler.cpp:29`
//! `HandleLearnTalentOpcode` and `:43` `HandleConfirmRespecWipeOpcode`;
//! `src/server/game/Server/Protocol/Opcodes.cpp` fixes `CMSG_LEARN_TALENT`
//! `:560` and `CMSG_CONFIRM_RESPEC_WIPE` `:386` (both `STATUS_LOGGEDIN`,
//! `PROCESS_THREADUNSAFE`).
//!
//! The application crate owns this registration family because it already
//! coordinates trainer interaction and spell acquisition (`trainer_handlers`):
//! the respec wipe is a trainer-NPC interaction and learning a talent is a
//! spell acquisition. Only the registration source moved: both entries keep
//! their opcode, session status, packet processing and handler name exactly as
//! the World legacy inventory submitted them, and every packet is handed to the
//! host unread, as the legacy closures did.

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::WorldPacket;

/// The adapter contract between the talent registrations and the World session
/// that executes them.
///
/// **Talent-reset commit and publication stay with the host.** C++ runs
/// `Player::ResetTalents` and only on success publishes `SendTalentsInfoData`
/// and the trainer's `Untalent Visual Effect` cast (`SkillHandler.cpp:65-69`);
/// `Player::LearnTalent` publishes `SendTalentsInfoData` only on success
/// (`:31-32`). The host keeps the Rust equivalent it already owns: trainer
/// admission, the durable reset transaction (money, reset metadata, talent
/// topology) and its COMMIT outcome, and the post-COMMIT publication of the
/// talent data, spell/pet runtime removal, criteria and visual cast. This crate
/// owns only the registration and dispatch of the two opcodes.
pub trait TalentHandlerHostLikeCpp<C> {
    /// C++ `HandleConfirmRespecWipeOpcode` (`SkillHandler.cpp:43`). The legacy
    /// closure destructured the catalog view (`id_generators.item`,
    /// `progression.no_reset_talent_cost`), so the host receives that view.
    fn handle_confirm_respec_wipe_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;

    /// C++ `HandleLearnTalentOpcode` (`SkillHandler.cpp:29`). The legacy closure
    /// destructured the catalog view (`player_bootstrap.talent_tabs`), so the
    /// host receives that view.
    fn handle_learn_talent_with_catalogs_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
        pkt: WorldPacket,
    ) -> HandlerFuture<'a, ()>;
}

fn handle_confirm_respec_wipe_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TalentHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_confirm_respec_wipe_with_catalogs_like_cpp(catalogs, pkt)
}

fn handle_learn_talent_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: TalentHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    session.handle_learn_talent_with_catalogs_like_cpp(catalogs, pkt)
}

/// Register the two C++ `SkillHandler.cpp` talent opcodes.
pub fn register_talent_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: TalentHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::ConfirmRespecWipe,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_confirm_respec_wipe",
        handler: handle_confirm_respec_wipe_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LearnTalent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_learn_talent",
        handler: handle_learn_talent_thunk::<S, C>,
    })?;
    Ok(())
}
