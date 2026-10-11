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
    pub(crate) async fn handle_cage_battle_pet_represented_like_cpp(
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

    /// CMSG_DISMISS_CRITTER — represented companion dismissal.
    ///
    /// C++ reads a full `CritterGUID`, silently ignores missing/non-active
    /// critters, and sends no direct response. Real `TempSummon::UnSummon` and
    /// object update/despawn fanout remain part of the live companion runtime.

    pub(crate) async fn handle_dismiss_critter(&mut self, mut pkt: wow_packet::WorldPacket) {
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

#[cfg(test)]
#[path = "../../../unit_tests/handlers/pets/tests/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/pets/mod/test_only.rs"]
mod test_only;
