// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private battle_pet capability handlers extracted from the legacy misc owner.
//!
//! The eight C++ `BattlePetHandler.cpp` handlers moved to the
//! `wow-world-application` `ApplicationBattlePet` owner under #1263 F5; their
//! World-side host lives in [`battle_pet_host`]. `DismissCritter` (C++
//! `PetHandler.cpp`) moved its registration to that owner in `#1263 F5
//! remaining families` and its body in #1363 slice 7b. `BattlePetUpdateDisplayNotify` is deliberately
//! **not** registered: 3.4.3 leaves it `STATUS_UNHANDLED` / `Handle_NULL`
//! (`Opcodes.cpp:243`), and the 2026-10-07 #1263 F6 decision (D5,
//! `docs/migration/EXISTING-CODE-DEFECTS.md`) removes the empty registered body
//! that used to claim otherwise.

#[cfg(test)]
use tracing::warn;
#[cfg(test)]
use wow_core::GameTime;

#[cfg(test)]
use wow_packet::packets::misc::BattlePetDeletePet;
#[cfg(test)]
use wow_packet::packets::misc::BattlePetModifyName;

mod battle_pet_host;
#[cfg(test)]
mod test_shims;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/pets/tests/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/pets/mod/test_only.rs"]
mod test_only;
