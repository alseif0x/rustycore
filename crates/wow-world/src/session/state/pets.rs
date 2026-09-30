// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `WorldSession::pets` sub-state (#1241 F2): moved fields, no logic.

use super::*;

/// Represented pet state, pet stable, react and command state, pet speeds, temporary (un)summon and
/// mount pet-control counters.
pub(crate) struct PetState {
    /// Count of C++ temporary pet unsummon side effects requested by movement.
    #[cfg(test)]
    pub(in crate::session) temporary_pet_unsummon_requests_like_cpp: u32,
    /// Legacy handle-less test fixture for the current Player-owned pet GUID.
    #[cfg(test)]
    pub(crate) represented_pet_guid_like_cpp: Option<ObjectGuid>,
    /// C++ `Player::m_temporaryUnsummonedPetNumber`, represented until pet DB load/resummon is live.
    #[cfg(test)]
    pub(in crate::session) represented_temporary_unsummoned_pet_number_like_cpp: u32,
    /// C++ `Player::m_oldpetspell`, used by `RemovePet(nullptr, ..., returnreagent=true)`.
    #[cfg(test)]
    pub(in crate::session) represented_old_pet_spell_like_cpp: u32,
    /// Represented `character_pet`/stable rows until `Pet::LoadPetFromDB` is wired to DB.
    #[cfg(test)]
    pub(in crate::session) represented_pet_stable_like_cpp: PetStable,
    /// True only after the current Player's complete `character_pet` query
    /// returned no rows. Any later pet load or lifetime mutation revokes this
    /// narrow proof instead of attempting to model pet-to-owner aura casts.
    #[cfg(test)]
    pub(in crate::session) represented_character_pet_rows_empty_authority_complete_like_cpp: bool,

    /// Represented `Pet::m_unitData->CreatedBySpell` for the active pet until UnitData owns it.
    #[cfg(test)]
    pub(in crate::session) represented_pet_created_by_spell_like_cpp: u32,
    /// Represented current pet react state for C++ mount/dismount PetMode side effects.
    #[cfg(test)]
    pub(in crate::session) represented_pet_react_state_like_cpp: u8,
    /// Represented current pet command state for C++ mount/dismount PetMode side effects.
    #[cfg(test)]
    pub(in crate::session) represented_pet_command_state_like_cpp: u8,
    /// C++ `Player::m_temporaryPetReactState` saved by `DisablePetControlsOnMount`.
    #[cfg(test)]
    pub(in crate::session) temporary_mount_pet_react_state_like_cpp: Option<u8>,
    /// Count of C++ `DisablePetControlsOnMount` side effects represented until pet runtime is canonical.
    #[cfg(test)]
    pub(in crate::session) mount_pet_control_disable_requests_like_cpp: u32,
    /// Count of C++ `EnablePetControlsOnDismount` side effects represented until pet runtime is canonical.
    #[cfg(test)]
    pub(in crate::session) mount_pet_control_enable_requests_like_cpp: u32,
    /// Count of C++ mount/dismount pet resummon calls represented until pet runtime is canonical.
    #[cfg(test)]
    pub(in crate::session) mount_pet_resummon_requests_like_cpp: u32,
    /// Count of C++ `ResummonPetTemporaryUnSummonedIfAny` calls after near teleport ACK.
    #[cfg(test)]
    pub(in crate::session) temporary_pet_resummon_requests_like_cpp: u32,
    /// C++ `Player::GetPet()->SetSpeedRate` propagation represented until pet Unit runtime owns it.
    #[cfg(test)]
    pub(in crate::session) represented_pet_movement_speed_rates_like_cpp:
        [f32; UnitMoveTypeLikeCpp::COUNT],
    /// Count of represented player speed changes propagated to the active pet.
    #[cfg(test)]
    pub(in crate::session) represented_pet_speed_propagations_like_cpp: u32,
    /// Handle-less battle-pet state used only by isolated Session tests.
    #[cfg(test)]
    pub(crate) battle_pet_test_fixture_like_cpp: BattlePetTestFixtureLikeCpp,
}
