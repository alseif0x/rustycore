// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest-giver completion packet body moved out of the World shell.
//!
//! C++ `WorldSession::HandleQuestgiverCompleteQuest` (`Handlers/QuestHandler.cpp:533`),
//! registered `Opcodes.cpp:778` as `CMSG_QUEST_GIVER_COMPLETE_QUEST`,
//! `STATUS_LOGGEDIN`/`PROCESS_INPLACE`. The body keeps its original parse, gates,
//! order, log strings and packets; only the access path changed. The World session
//! lends its hub (the represented reads, the disable manager and the offer-reward
//! publication), while the shell-only capabilities (the represented quest gameplay
//! snapshot, the eligibility/reward projections, the involved-source interaction
//! check and the request-items publication) stay behind
//! [`QuestGiverCompleteQuestHostLikeCpp`].

use std::sync::Arc;

use tracing::{debug, info, warn};
use wow_conditions::{QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_NONE_LIKE_CPP};
use wow_core::ObjectGuid;
use wow_data::disable_mgr::DISABLE_TYPE_QUEST;
use wow_data::quest::{QuestStore, QuestTemplate};
use wow_entities::PlayerQuestGameplayState;
use wow_packet::WorldPacket;
use wow_packet::packets::quest::QuestGiverOfferReward;
use wow_world_core::session::HubRef;

use crate::{
    RepresentedQuestCompleteDialogLikeCpp, represented_quest_complete_dialog_like_cpp,
    represented_quest_rewards_block_like_cpp,
};

/// C++ `QUEST_FLAGS_AUTO_COMPLETE` (`SharedDefines.h`); the World handler module
/// declared the same value as `QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP`.
const QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP: u32 = 0x0001_0000;

/// Capabilities of the quest-completion handler that the World shell still owns:
/// the session hub, the represented quest-log snapshot, the eligibility/reward
/// projections built from the canonical Player access plus the cfg(test)
/// fixtures, the involved-source interaction check and the request-items
/// publication. Each method is invoked at the exact point the World body invoked
/// its counterpart, so no step value or resumption is needed.
pub trait QuestGiverCompleteQuestHostLikeCpp {
    /// The session hub, which the moved body uses for the account log field, the
    /// represented player guid, the disable manager and the offer-reward packet.
    fn complete_quest_hub_ref_like_cpp(&self) -> HubRef<'_>;

    /// The session's loaded quest store (`sObjectMgr->GetQuestTemplate` source).
    fn complete_quest_quest_store_like_cpp(&self) -> Option<Arc<QuestStore>>;

    /// C++ `Player::GetQuestStatus`/`m_RewardedQuests` snapshot with the
    /// represented fixture fallback the World session applied at its read point.
    fn complete_quest_player_quest_gameplay_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerQuestGameplayState>;

    /// C++ `object->hasInvolvedQuest(questId)` plus
    /// `Player::CanInteractWithQuestGiver(object)` over the represented
    /// creature/gameobject interaction access.
    fn complete_quest_represented_involved_source_allows_like_cpp(
        &self,
        source_guid: ObjectGuid,
        quest_id: u32,
        quest_store: &QuestStore,
    ) -> bool;

    /// C++ `Player::CanSeeStartQuest` (`Player.cpp:14073-14085`), which reads the
    /// canonical Player access, the quest state and the condition projection.
    fn complete_quest_can_see_start_quest_represented_bounded_like_cpp(
        &self,
        quest: &QuestTemplate,
    ) -> bool;

    /// C++ `Player::CanRewardQuest(quest, false)` in the bounded represented seam.
    fn complete_quest_can_reward_quest_represented_bounded_like_cpp(
        &self,
        quest: &QuestTemplate,
    ) -> bool;

    /// C++ `Player::CanCompleteRepeatableQuest` in the bounded represented seam.
    fn complete_quest_can_complete_repeatable_quest_represented_bounded_like_cpp(
        &self,
        quest: &QuestTemplate,
    ) -> bool;

    /// C++ `PlayerTalkClass->SendQuestGiverRequestItems`, which the World session
    /// still owns because four other quest paths publish it.
    fn send_represented_quest_giver_request_items_with_completion_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        quest: &QuestTemplate,
        can_complete: bool,
        auto_launched: bool,
    );
}

/// C++ `DisableMgr::IsDisabledFor(DISABLE_TYPE_QUEST, questId, nullptr)`
/// (`QuestHandler.cpp:552` `is_quest_disabled` gate).
pub fn quest_is_disabled_like_cpp(hub: HubRef<'_>, quest_id: u32) -> bool {
    hub.catalogs.disable_mgr().is_some_and(|disable_mgr| {
        disable_mgr.is_disabled_for_like_cpp(DISABLE_TYPE_QUEST, quest_id, None, 0, None)
    })
}

/// C++ `ObjectGuid::GetEntry()` for the `SMSG_QUEST_GIVER_*` `giver_creature_id`
/// field. Moved from the World quest handler module, which now delegates here.
pub fn quest_giver_creature_id_from_source_like_cpp(source_guid: ObjectGuid) -> i32 {
    if source_guid.is_any_type_creature() {
        i32::try_from(source_guid.entry()).unwrap_or(0)
    } else {
        0
    }
}

/// CMSG_QUEST_GIVER_COMPLETE_QUEST — player talks to quest-ender NPC.
/// If objectives are done: show reward dialog. Else: show "still need X" dialog.
/// Legacy non-canonical note: QuestHandler.HandleQuestGiverCompleteQuest
pub async fn handle_quest_giver_complete_quest_like_cpp<H>(host: &mut H, mut pkt: WorldPacket)
where
    H: QuestGiverCompleteQuestHostLikeCpp + Send,
{
    let guid = match pkt.read_packed_guid() {
        Ok(g) => g,
        Err(_) => {
            warn!("QuestGiverCompleteQuest: failed to read GUID");
            return;
        }
    };
    let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
    let from_script: bool = pkt.read_bit().unwrap_or(false);

    info!(
        account = host.complete_quest_hub_ref_like_cpp().core.account_id,
        ?guid,
        quest_id,
        from_script,
        "Received QuestGiverCompleteQuest like C++"
    );

    let quest_store = match host.complete_quest_quest_store_like_cpp() {
        Some(s) => s,
        None => return,
    };

    let quest = match quest_store.get(quest_id) {
        Some(q) => q,
        None => {
            warn!(
                account = host.complete_quest_hub_ref_like_cpp().core.account_id,
                quest_id, "QuestGiverCompleteQuest: unknown quest"
            );
            return;
        }
    };

    if quest_is_disabled_like_cpp(host.complete_quest_hub_ref_like_cpp(), quest_id) {
        debug!(
            account = host.complete_quest_hub_ref_like_cpp().core.account_id,
            quest_id, "QuestGiverCompleteQuest: quest disabled"
        );
        return;
    }

    if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0 {
        if from_script
            || !host.complete_quest_represented_involved_source_allows_like_cpp(
                guid,
                quest_id,
                &quest_store,
            )
        {
            warn!(
                account = host.complete_quest_hub_ref_like_cpp().core.account_id,
                ?guid,
                quest_id,
                from_script,
                "QuestGiverCompleteQuest: represented involved source rejected"
            );
            return;
        }
    } else if !from_script
        || host.complete_quest_hub_ref_like_cpp().core.player_guid() != Some(guid)
    {
        warn!(
            account = host.complete_quest_hub_ref_like_cpp().core.account_id,
            ?guid,
            quest_id,
            from_script,
            "QuestGiverCompleteQuest: auto-complete source is not script/player"
        );
        return;
    }

    // C++ `HandleQuestgiverCompleteQuest` (QuestHandler.cpp:559) rejects only a
    // quest the player can neither see nor hold:
    // `!CanSeeStartQuest(quest) && GetQuestStatus(id) == QUEST_STATUS_NONE`.
    // A quest that is visible but has no status entry still reaches the dialog
    // selection below; `has_quest` alone would drop it.
    let represented_status = host
        .complete_quest_player_quest_gameplay_snapshot_like_cpp()
        .and_then(|state| state.statuses_like_cpp().get(&quest_id).map(|qs| qs.status))
        .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP);
    if !host.complete_quest_can_see_start_quest_represented_bounded_like_cpp(quest)
        && represented_status == QUEST_STATUS_NONE_LIKE_CPP
    {
        warn!(
            account = host.complete_quest_hub_ref_like_cpp().core.account_id,
            quest_id,
            "QuestGiverCompleteQuest: possible hacking attempt, quest is neither visible nor held"
        );
        return;
    }

    // C++ `GetQuestStatus(packet.QuestID) != QUEST_STATUS_COMPLETE` plus the
    // `HasQuestObjectiveType(QUEST_OBJECTIVE_ITEM)` dialog choice.
    let is_complete = represented_status == QUEST_STATUS_COMPLETE_LIKE_CPP;
    let can_reward_quest = host.complete_quest_can_reward_quest_represented_bounded_like_cpp(quest);
    let can_complete_repeatable_quest =
        host.complete_quest_can_complete_repeatable_quest_represented_bounded_like_cpp(quest);
    match represented_quest_complete_dialog_like_cpp(
        quest,
        is_complete,
        can_reward_quest,
        can_complete_repeatable_quest,
    ) {
        RepresentedQuestCompleteDialogLikeCpp::RequestItems {
            can_complete,
            auto_launched,
        } => {
            host.send_represented_quest_giver_request_items_with_completion_like_cpp(
                guid,
                quest,
                can_complete,
                auto_launched,
            );
        }
        RepresentedQuestCompleteDialogLikeCpp::OfferReward { auto_launched } => {
            host.complete_quest_hub_ref_like_cpp()
                .core
                .packet_publication_access_like_cpp()
                .send_packet(&QuestGiverOfferReward {
                    giver_guid: guid,
                    giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
                    quest_id,
                    quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
                    suggested_party_members: quest.suggested_group_num,
                    rewards: represented_quest_rewards_block_like_cpp(quest),
                    title: quest.log_title.clone(),
                    reward_text: quest.quest_completion_log.clone(),
                    auto_launched,
                });
        }
    }
}
