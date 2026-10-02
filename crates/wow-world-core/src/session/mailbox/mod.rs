//! Session-independent mailbox protocol, durable rails, and phase admission values.

mod durable;
mod pump;
mod protocol;
mod session_phase_permit;
mod session_phase_rail;

pub use self::durable::{
    DurableCreatureRuntimeCommandsLikeCpp, MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP,
};
pub use self::protocol::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyGroupDifficultyLikeCppCommand,
    ApplyGroupJoinLikeCppCommand, ApplyGroupRemovalLikeCppCommand,
    ApplyGroupSubgroupLikeCppCommand, ApplyLootMoneyLikeCppCommand, ApplyLootMoneyResultLikeCpp,
    ApplyPlayerMeleeResultLikeCppCommand, CancelRepresentedTradeLikeCppCommand,
    CreatureAttackStartLikeCppCommand, CreatureAttackStopLikeCppCommand,
    CreatureMeleeAbsorbConsumptionLikeCpp, DestroyVisibleObjectLikeCppCommand,
    GameEventQuestCompleteClientOutcomeLikeCpp, GameEventQuestCompleteCommandLikeCpp,
    GameEventQuestCompleteResponseLikeCpp, GroupDifficultyKindLikeCpp, KickLikeCppCommand,
    LootRollCommandIdentityLikeCpp, LootRollStoreWinnerCommand, LootRollVoteCommand,
    MasterLootGiveCommand, MasterLootGiveResult, NotifyLootMoneyRemovedLikeCppCommand,
    PlayerMeleeCreatureKillLikeCpp, PlayerMeleeSwingLikeCpp,
    ReconcilePvpCombatExpiryLikeCppCommand, RefreshVisibleWorldCreaturesLikeCppCommand,
    ResetSeasonalQuestStatusCommand, SendAddonIfRegisteredLikeCppCommand,
    SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand,
    SendPartyUpdateLikeCppCommand, SendPlayerSpellIfVisibleLikeCppCommand,
    SendRealmPacketLikeCppCommand, SendRepeatableTurnInRequestItemsLikeCppCommand,
    SendRepresentedDuelCountdownLikeCppCommand, SendRepresentedDuelRequestedLikeCppCommand,
    SendRepresentedTradeStatusLikeCppCommand, SendVisibleObjectValuesUpdateCommand, SessionCommand,
    SetQuestSharingInfoAndSendDetailsCommand, SharedClientVisibleGuidsLikeCpp,
    SharedClientVisibleTransportsLikeCpp, SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand, UnacceptRepresentedTradeLikeCppCommand,
    WorldSessionShutdownFlushLikeCppCommand, WorldSessionShutdownFlushResultLikeCpp,
};
pub use self::session_phase_permit::{
    SessionPhaseClaimLikeCpp, SessionPhasePermitLikeCpp, SessionPhasePermitStateLikeCpp,
    SessionPhaseRevokeLikeCpp,
};
pub use self::session_phase_rail::{
    MapPhaseAdmissionLikeCpp, PendingWorldPhaseFinalizationLikeCpp, RunMapPhasePassLikeCppCommand,
    RunMapPhasePassResultLikeCpp, RunWorldPhasePassLikeCppRequest, RunWorldPhasePassResultLikeCpp,
    SessionPhasePassOutcomeLikeCpp, SessionPhaseRequestLikeCpp,
};

#[cfg(test)]
#[path = "../../../unit_tests/session/mailbox/tests.rs"]
mod tests;
