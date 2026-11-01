//! Compatibility exports for the Core-owned player session directory.

pub use wow_world_core::session::directory::{
    PlayerAggroCandidateSnapshot, PlayerControlAddress, PlayerDirectoryIdentityLikeCpp,
    PlayerDirectoryPlacementLikeCpp, PlayerDirectoryReliableSendOutcome, PlayerDirectorySendError,
    PlayerGroupPresenceSnapshot, PlayerGroupRewardSnapshot, PlayerInspectSnapshot,
    PlayerLootContextSnapshot, PlayerLootPresenceSnapshot, PlayerMovementDirectoryUpdate,
    PlayerNameQuerySnapshotLikeCpp, PlayerPartyMemberSnapshot, PlayerQuestSharingSnapshot,
    PlayerRegistration, PlayerRegistry, PlayerRuntimeRecipient, PlayerSessionRegistrationLikeCpp,
    PlayerSocialRecipientSnapshot, PlayerVehicleInteractionSnapshot,
    PlayerVisibilityCreateSnapshot, PrepareLootMoneyApplicationLikeCpp,
    PreparedLootMoneyApplicationLikeCpp, SessionPhaseAddressLikeCpp,
    detached_session_phase_rail_like_cpp,
};
