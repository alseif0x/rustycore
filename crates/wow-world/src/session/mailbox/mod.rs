//! Shell-owned Session mailbox pump and compatibility reexports for its protocol.

mod pump;

pub use wow_world_core::session::SessionPassKind;

pub use wow_world_core::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyGroupDifficultyLikeCppCommand,
    ApplyGroupJoinLikeCppCommand, ApplyGroupRemovalLikeCppCommand,
    ApplyGroupSubgroupLikeCppCommand, ApplyLootMoneyLikeCppCommand, ApplyLootMoneyResultLikeCpp,
    ApplyPlayerMeleeResultLikeCppCommand, CancelRepresentedTradeLikeCppCommand,
    CommittedCreatureMeleeAdmission, CommittedCreatureMeleeReply,
    CreatureAttackStartLikeCppCommand, CreatureAttackStopLikeCppCommand,
    CreatureMeleeAbsorbConsumptionLikeCpp, CreatureMeleePresentationStageLikeCpp,
    DestroyVisibleObjectLikeCppCommand, DurableCreatureRuntimeCommandsLikeCpp,
    GameEventQuestCompleteClientOutcomeLikeCpp, GameEventQuestCompleteCommandLikeCpp,
    GameEventQuestCompleteResponseLikeCpp, GroupDifficultyKindLikeCpp, KickLikeCppCommand,
    LootRollCommandIdentityLikeCpp, LootRollStoreWinnerCommand, LootRollVoteCommand,
    MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP, MapPhaseAdmissionLikeCpp,
    MasterLootGiveCommand, MasterLootGiveResult, NativePlayerDeathAdmission,
    NativePlayerDeathReply, NotifyLootMoneyRemovedLikeCppCommand,
    PendingWorldPhaseFinalizationLikeCpp, PlayerMeleeCreatureKillLikeCpp, PlayerMeleeSwingLikeCpp,
    PlayerRecoveryAdmission, PlayerRecoveryReply, PlayerRecoveryStatus,
    ReconcilePvpCombatExpiryLikeCppCommand, RefreshVisibleWorldCreaturesLikeCppCommand,
    ResetSeasonalQuestStatusCommand, RunCommittedCreatureMeleeRequest,
    RunMapPhasePassLikeCppCommand, RunMapPhasePassResultLikeCpp, RunNativePlayerDeathRequest,
    RunPlayerRecoveryRequest, RunWorldPhasePassLikeCppRequest, RunWorldPhasePassResultLikeCpp,
    SendAddonIfRegisteredLikeCppCommand, SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand,
    SendPartyUpdateLikeCppCommand, SendPlayerAuraUpdateCommand,
    SendPlayerSpellIfVisibleLikeCppCommand, SendRealmPacketLikeCppCommand,
    SendRepeatableTurnInRequestItemsLikeCppCommand, SendRepresentedDuelCountdownLikeCppCommand,
    SendRepresentedDuelRequestedLikeCppCommand, SendRepresentedTradeStatusLikeCppCommand,
    SendVisibleObjectValuesUpdateCommand, SessionCommand, SessionPhaseClaimLikeCpp,
    SessionPhasePassOutcomeLikeCpp, SessionPhasePermitLikeCpp, SessionPhasePermitStateLikeCpp,
    SessionPhaseRequestLikeCpp, SessionPhaseRevokeLikeCpp,
    SetQuestSharingInfoAndSendDetailsCommand, SharedClientVisibleGuidsLikeCpp,
    SharedClientVisibleTransportsLikeCpp, SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand, UnacceptRepresentedTradeLikeCppCommand,
    WorldSessionShutdownFlushLikeCppCommand, WorldSessionShutdownFlushResultLikeCpp,
};
