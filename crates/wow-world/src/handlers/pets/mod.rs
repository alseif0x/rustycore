// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private battle_pet capability handlers extracted from the legacy misc owner.
//!
//! The eight C++ `BattlePetHandler.cpp` handlers moved to the
//! `wow-world-application` `ApplicationBattlePet` owner under #1263 F5; their
//! World-side host lives in [`battle_pet_host`]. `DismissCritter` (C++
//! `PetHandler.cpp`) keeps its body here and, since `#1263 F5 remaining
//! families`, its registration also lives in that owner. `BattlePetUpdateDisplayNotify` is deliberately
//! **not** registered: 3.4.3 leaves it `STATUS_UNHANDLED` / `Handle_NULL`
//! (`Opcodes.cpp:243`), and the 2026-10-07 #1263 F6 decision (D5,
//! `docs/migration/EXISTING-CODE-DEFECTS.md`) removes the empty registered body
//! that used to claim otherwise.

use tracing::warn;
use wow_core::GameTime;

use wow_packet::ClientPacket;
#[cfg(test)]
use wow_packet::packets::misc::CageBattlePet;
use wow_packet::packets::misc::{BattlePetDeletePet, BattlePetModifyName};
use wow_packet::packets::pet::DismissCritter;

mod battle_pet_host;
#[cfg(test)]
mod test_shims;

impl crate::session::WorldSession {
    pub async fn handle_battle_pet_delete_pet_represented_like_cpp(
        &mut self,
        pkt: wow_packet::WorldPacket,
    ) {
        crate::session::cx_pets(self)
            .handle_battle_pet_delete_pet_represented_like_cpp(pkt)
            .await
    }

    /// CMSG_CAGE_BATTLE_PET — represented cage body.
    ///
    /// C++ registers this handler and forwards only the pet guid to
    /// `BattlePetMgr::CageBattlePet`. The manager then performs the journal,
    /// species, slot, health, inventory, item-store, remove, deleted-packet,
    /// and summoned-companion gates. The archived opcode id is still the
    /// unresolved `0xBADD` placeholder, so this method remains intentionally
    /// unregistered for production dispatch. Until the real inventory path is
    /// wired, this represented body exercises the successful inventory seam.

    #[cfg(test)]
    pub async fn handle_cage_battle_pet_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match CageBattlePet::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "CageBattlePet parse failed: {error}"
                );
                return;
            }
        };

        let _ = self.battle_pet_cage_battle_pet_represented_like_cpp(request.pet_guid, true, true);
    }

    pub async fn handle_battle_pet_modify_name_represented_like_cpp(
        &mut self,
        pkt: wow_packet::WorldPacket,
    ) {
        crate::session::cx_pets(self)
            .handle_battle_pet_modify_name_represented_like_cpp(pkt)
            .await
    }

    /// CMSG_DISMISS_CRITTER — represented companion dismissal.
    ///
    /// C++ reads a full `CritterGUID`, silently ignores missing/non-active
    /// critters, and sends no direct response. Real `TempSummon::UnSummon` and
    /// object update/despawn fanout remain part of the live companion runtime.

    pub async fn handle_dismiss_critter(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match DismissCritter::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.core.account_id,
                    "DismissCritter parse failed: {error}"
                );
                return;
            }
        };

        crate::session::hub_mut(self).represented_dismiss_critter_like_cpp(request.critter_guid);
    }
}

impl crate::session::PetsCx<'_> {
    /// CMSG_BATTLE_PET_DELETE_PET — represented battle-pet removal body.
    ///
    /// C++ registers this handler and forwards only the pet guid to
    /// `BattlePetMgr::RemovePet`, which requires the journal lock and silently
    /// ignores unknown pets. The archived opcode id is the unresolved `0xBADD`
    /// placeholder, so this method is intentionally not registered for
    /// production dispatch until the real client opcode is known.

    pub async fn handle_battle_pet_delete_pet_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match BattlePetDeletePet::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.core.account_id,
                    "BattlePetDeletePet parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_remove_pet_durable_like_cpp(request.pet_guid)
            .await;
    }

    /// CMSG_BATTLE_PET_MODIFY_NAME — represented rename body.
    ///
    /// C++ registers this handler and forwards the parsed guid/name/declined
    /// names to `BattlePetMgr::ModifyName`, which stamps `GameTime::GetGameTime`
    /// inside the manager. The archived opcode id remains the unresolved
    /// `0xBADD` placeholder, so this method is intentionally not registered for
    /// production dispatch until the real client opcode is known.

    pub async fn handle_battle_pet_modify_name_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match BattlePetModifyName::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.core.account_id,
                    "BattlePetModifyName parse failed: {error}"
                );
                return;
            }
        };

        let timestamp = i64::try_from(GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let _ = self
            .battle_pet_modify_name_durable_like_cpp(
                request.pet_guid,
                request.name,
                request.declined_names,
                timestamp,
            )
            .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/pets/tests/mod.rs"]
mod tests;
