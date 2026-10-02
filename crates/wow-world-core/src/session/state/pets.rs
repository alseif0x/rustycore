// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `SessionFixtures::pets` state fields: represented pet state, no logic.

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::{
    movement_protocol::UnitMoveTypeLikeCpp, pets::test_fixtures::BattlePetTestFixtureLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_core::ObjectGuid;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::PetStable;

/// Represented pet state, pet stable, react and command state, pet speeds, temporary (un)summon and
/// mount pet-control counters.
pub struct PetState {
    /// Count of C++ temporary pet unsummon side effects requested by movement.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub temporary_pet_unsummon_requests_like_cpp: u32,
    /// Legacy handle-less test fixture for the current Player-owned pet GUID.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_pet_guid_like_cpp: Option<ObjectGuid>,
    /// C++ `Player::m_temporaryUnsummonedPetNumber`, represented until pet DB load/resummon is live.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_temporary_unsummoned_pet_number_like_cpp: u32,
    /// C++ `Player::m_oldpetspell`, used by `RemovePet(nullptr, ..., returnreagent=true)`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_old_pet_spell_like_cpp: u32,
    /// Represented `character_pet`/stable rows until `Pet::LoadPetFromDB` is wired to DB.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_pet_stable_like_cpp: PetStable,
    /// True only after the current Player's complete `character_pet` query
    /// returned no rows. Any later pet load or lifetime mutation revokes this
    /// narrow proof instead of attempting to model pet-to-owner aura casts.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_character_pet_rows_empty_authority_complete_like_cpp: bool,

    /// Represented `Pet::m_unitData->CreatedBySpell` for the active pet until UnitData owns it.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_pet_created_by_spell_like_cpp: u32,
    /// Represented current pet react state for C++ mount/dismount PetMode side effects.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_pet_react_state_like_cpp: u8,
    /// Represented current pet command state for C++ mount/dismount PetMode side effects.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_pet_command_state_like_cpp: u8,
    /// C++ `Player::m_temporaryPetReactState` saved by `DisablePetControlsOnMount`.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub temporary_mount_pet_react_state_like_cpp: Option<u8>,
    /// Count of C++ `DisablePetControlsOnMount` side effects represented until pet runtime is canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub mount_pet_control_disable_requests_like_cpp: u32,
    /// Count of C++ `EnablePetControlsOnDismount` side effects represented until pet runtime is canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub mount_pet_control_enable_requests_like_cpp: u32,
    /// Count of C++ mount/dismount pet resummon calls represented until pet runtime is canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub mount_pet_resummon_requests_like_cpp: u32,
    /// Count of C++ `ResummonPetTemporaryUnSummonedIfAny` calls after near teleport ACK.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub temporary_pet_resummon_requests_like_cpp: u32,
    /// C++ `Player::GetPet()->SetSpeedRate` propagation represented until pet Unit runtime owns it.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_pet_movement_speed_rates_like_cpp: [f32; UnitMoveTypeLikeCpp::COUNT],
    /// Count of represented player speed changes propagated to the active pet.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub represented_pet_speed_propagations_like_cpp: u32,
    /// Handle-less battle-pet state used only by isolated Session tests.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub battle_pet_test_fixture_like_cpp: BattlePetTestFixtureLikeCpp,
}
