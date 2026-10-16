// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest-giver menus (`Player::PrepareQuestMenu`) and the prepared-quest
//! publication (`PlayerMenu::SendPreparedQuest`), moved from the World
//! `session/quest_dialog.rs` and `session/quest/publication.rs` (#1363 slice 6a).

use super::*;

const QUEST_MENU_ICON_TURN_IN_LIKE_CPP: u8 = 0;
const QUEST_MENU_ICON_AVAILABLE_LIKE_CPP: u8 = 2;
const QUEST_MENU_ICON_COMPLETE_LIKE_CPP: u8 = 4;

pub(crate) fn represented_quest_menu_item_log_rows_like_cpp(
    menu_items: &[RepresentedPreparedQuestMenuItemLikeCpp],
) -> Vec<(u32, String, u8, bool, bool, u32, u32)> {
    menu_items
        .iter()
        .map(|item| {
            (
                item.quest.id,
                item.quest.log_title.clone(),
                item.quest_icon,
                item.has_starter_relation,
                item.has_involved_relation,
                item.quest.allowable_classes,
                item.quest.flags,
            )
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct RepresentedPreparedQuestMenuItemLikeCpp {
    pub quest: wow_data::quest::QuestTemplate,
    pub quest_icon: u8,
    pub(crate) has_starter_relation: bool,
    pub(crate) has_involved_relation: bool,
}

pub(crate) fn send_represented_prepared_quest_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    source_guid: ObjectGuid,
    menu_items: Vec<RepresentedPreparedQuestMenuItemLikeCpp>,
) {
    if let [menu_item] = menu_items.as_slice() {
        info!(
            ?source_guid,
            quest_id = menu_item.quest.id,
            quest_icon = menu_item.quest_icon,
            starter = menu_item.has_starter_relation,
            involved = menu_item.has_involved_relation,
            title = menu_item.quest.log_title.as_str(),
            "Sending represented single prepared quest like C++"
        );
        send_represented_prepared_single_quest_like_cpp(host, source_guid, menu_item, true);
        return;
    }

    info!(
        ?source_guid,
        quests = ?represented_quest_menu_item_log_rows_like_cpp(
            &menu_items,
        ),
        "Sending represented QuestGiverQuestList like C++"
    );
    host.send_packet(&QuestGiverQuestList {
        guid: source_guid,
        greeting: String::new(),
        greet_emote_delay: 0,
        greet_emote_type: 0,
        quests: menu_items
            .iter()
            .map(|item| quest_list_entry_from_menu_item_like_cpp(host, item))
            .collect(),
    });
}

pub(crate) fn send_represented_prepared_single_quest_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    source_guid: ObjectGuid,
    menu_item: &RepresentedPreparedQuestMenuItemLikeCpp,
    auto_launched: bool,
) {
    let quest = &menu_item.quest;

    if menu_item.quest_icon == QUEST_MENU_ICON_COMPLETE_LIKE_CPP {
        let can_complete = can_reward_quest_represented_bounded_like_cpp(host, quest);
        info!(
            ?source_guid,
            quest_id = quest.id,
            can_complete,
            auto_launched,
            title = quest.log_title.as_str(),
            "Sending represented QuestGiverRequestItems from complete icon like C++"
        );
        send_represented_quest_giver_request_items_with_completion_like_cpp(
            host,
            source_guid,
            quest,
            can_complete,
            auto_launched,
        );
        return;
    }

    if !menu_item.has_starter_relation && !menu_item.has_involved_relation {
        // C++ would `SendCloseGossip()`. This represented seam has no close-gossip
        // packet helper, and relation-derived menu items should not reach this branch.
        return;
    }

    // C++ may auto-accept here; this represented-partial seam intentionally performs no
    // quest log mutation and only chooses the outbound packet.
    if quest.is_turn_in_like_cpp()
        && quest.is_repeatable()
        && !quest.is_daily_or_weekly_like_cpp()
        && !quest.is_monthly_like_cpp()
    {
        info!(
            ?source_guid,
            quest_id = quest.id,
            auto_launched,
            title = quest.log_title.as_str(),
            "Sending represented QuestGiverRequestItems for repeatable turn-in like C++"
        );
        let can_complete = can_complete_repeatable_quest_represented_bounded_like_cpp(host, quest);
        send_represented_quest_giver_request_items_with_completion_like_cpp(
            host,
            source_guid,
            quest,
            can_complete,
            auto_launched,
        );
    } else if quest.is_turn_in_like_cpp()
        && !quest.is_daily_or_weekly_like_cpp()
        && !quest.is_monthly_like_cpp()
    {
        let can_complete = can_reward_quest_represented_bounded_like_cpp(host, quest);
        info!(
            ?source_guid,
            quest_id = quest.id,
            can_complete,
            auto_launched,
            title = quest.log_title.as_str(),
            "Sending represented QuestGiverRequestItems for turn-in like C++"
        );
        send_represented_quest_giver_request_items_with_completion_like_cpp(
            host,
            source_guid,
            quest,
            can_complete,
            auto_launched,
        );
    } else {
        info!(
            ?source_guid,
            quest_id = quest.id,
            auto_launched,
            title = quest.log_title.as_str(),
            "Sending represented QuestGiverQuestDetails like C++"
        );
        send_represented_quest_giver_quest_details_like_cpp(
            host,
            source_guid,
            quest,
            auto_launched,
        );
    }
}

pub fn use_represented_creature_questgiver_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
    creature_entry: u32,
) -> bool {
    let Some(quest_store) = host
        .hub_ref()
        .catalogs
        .quests
        .store
        .as_ref()
        .map(Arc::clone)
    else {
        return false;
    };

    let menu_items =
        represented_creature_quest_menu_items_like_cpp(host, &quest_store, creature_entry);
    let Some(quests) = host.player_quest_gameplay_snapshot_like_cpp() else {
        return false;
    };
    let ender_candidates = quest_store
        .quests_for_ender(creature_entry)
        .iter()
        .map(|quest| {
            (
                quest.id,
                quest.log_title.clone(),
                quest.allowable_races,
                quest.allowable_classes,
                quest.min_level,
                quest.max_level,
                quests
                    .statuses_like_cpp()
                    .get(&quest.id)
                    .map(|status| status.status),
                quests.rewarded_quest_ids_like_cpp().contains(&quest.id),
            )
        })
        .collect::<Vec<_>>();
    let starter_candidates = quest_store
        .quests_for_starter(creature_entry)
        .iter()
        .map(|quest| {
            (
                quest.id,
                quest.log_title.clone(),
                quest.allowable_races,
                quest.allowable_classes,
                quest.min_level,
                quest.max_level,
                quest.is_available_for(
                    host.hub_ref().player_race_like_cpp(),
                    host.hub_ref().player_class_like_cpp(),
                    host.hub_ref().player_level_like_cpp(),
                ),
                can_take_quest(host, quest),
                quests
                    .statuses_like_cpp()
                    .get(&quest.id)
                    .map(|status| status.status),
                quests.rewarded_quest_ids_like_cpp().contains(&quest.id),
            )
        })
        .collect::<Vec<_>>();
    info!(
        creature_entry,
        race = host.hub_ref().player_race_like_cpp(),
        class = host.hub_ref().player_class_like_cpp(),
        level = host.hub_ref().player_level_like_cpp(),
        ender_candidates = ?ender_candidates,
        starter_candidates = ?starter_candidates,
        menu_items = ?represented_quest_menu_item_log_rows_like_cpp(&menu_items),
        "Prepared creature questgiver fallback menu like C++"
    );
    if menu_items.is_empty() {
        return false;
    }

    send_represented_prepared_quest_like_cpp(host, creature_guid, menu_items);
    true
}

pub fn use_represented_gameobject_questgiver_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    gameobject_guid: ObjectGuid,
    player_guid: ObjectGuid,
    gameobject_entry: u32,
    source: wow_entities::QuestgiverUseSource,
) -> bool {
    host.world_entities_mut()
        .record_represented_gameobject_use_effect_like_cpp(
            RepresentedGameObjectUseEffect::SendGossip {
                gameobject_guid,
                player_guid,
                gossip_id: source.gossip_id,
            },
        );

    let Some(quest_store) = host
        .hub_ref()
        .catalogs
        .quests
        .store
        .as_ref()
        .map(Arc::clone)
    else {
        return true;
    };

    let menu_items =
        represented_gameobject_quest_menu_items_like_cpp(host, &quest_store, gameobject_entry);
    if !menu_items.is_empty() {
        send_represented_prepared_quest_like_cpp(host, gameobject_guid, menu_items);
    }

    true
}

pub(crate) fn represented_quest_giver_query_creature_menu_item_like_cpp<
    H: QuestSessionHostLikeCpp,
>(
    host: &H,
    quest_store: &wow_data::quest::QuestStore,
    creature_entry: u32,
    quest: &wow_data::quest::QuestTemplate,
) -> Option<RepresentedPreparedQuestMenuItemLikeCpp> {
    if quest_store.creature_has_ender_relation_like_cpp(creature_entry, quest.id) {
        if represented_player_quest_status_is_complete_or_incomplete_like_cpp(host, quest.id) {
            return Some(RepresentedPreparedQuestMenuItemLikeCpp {
                quest: quest.clone(),
                quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                has_starter_relation: false,
                has_involved_relation: true,
            });
        }
        // C++ `HandleQuestgiverQueryQuestOpcode` accepts hasQuest OR
        // hasInvolvedQuest; `PrepareQuestMenu` still checks starters after
        // skipping inactive involved quests on the same source.
    }

    if quest_store.creature_has_starter_relation_like_cpp(creature_entry, quest.id) {
        return represented_starter_quest_menu_item_like_cpp(host, quest);
    }

    None
}

pub(crate) fn represented_quest_giver_query_gameobject_menu_item_like_cpp<
    H: QuestSessionHostLikeCpp,
>(
    host: &H,
    quest_store: &wow_data::quest::QuestStore,
    gameobject_entry: u32,
    quest: &wow_data::quest::QuestTemplate,
) -> Option<RepresentedPreparedQuestMenuItemLikeCpp> {
    if quest_store.gameobject_has_ender_relation_like_cpp(gameobject_entry, quest.id) {
        if represented_player_quest_status_is_complete_or_incomplete_like_cpp(host, quest.id) {
            return Some(RepresentedPreparedQuestMenuItemLikeCpp {
                quest: quest.clone(),
                quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                has_starter_relation: false,
                has_involved_relation: true,
            });
        }
        // C++ `HandleQuestgiverQueryQuestOpcode` accepts hasQuest OR
        // hasInvolvedQuest; `PrepareQuestMenu` still checks starters after
        // skipping inactive involved quests on the same source.
    }

    if quest_store.gameobject_has_starter_relation_like_cpp(gameobject_entry, quest.id) {
        return represented_starter_quest_menu_item_like_cpp(host, quest);
    }

    None
}

pub fn represented_creature_quest_menu_items_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest_store: &wow_data::quest::QuestStore,
    creature_entry: u32,
) -> Vec<RepresentedPreparedQuestMenuItemLikeCpp> {
    let mut menu_items = Vec::new();

    // C++ `Player::PrepareQuestMenu`: involved/ender relations first, icon 4 for
    // represented COMPLETE/INCOMPLETE local quest status.
    for quest in quest_store.quests_for_ender(creature_entry) {
        if represented_player_quest_status_is_complete_or_incomplete_like_cpp(host, quest.id) {
            menu_items.push(RepresentedPreparedQuestMenuItemLikeCpp {
                quest: quest.clone(),
                quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                has_starter_relation: false,
                has_involved_relation: true,
            });
        }
    }

    // Starter relations second, preserving C++ quest-icon selection for later
    // `SendPreparedQuest` single-item auto-open.
    for quest in quest_store.quests_for_starter(creature_entry) {
        if let Some(menu_item) = represented_starter_quest_menu_item_like_cpp(host, quest) {
            menu_items.push(menu_item);
        }
    }

    menu_items
}

pub(crate) fn represented_gameobject_quest_menu_items_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest_store: &wow_data::quest::QuestStore,
    gameobject_entry: u32,
) -> Vec<RepresentedPreparedQuestMenuItemLikeCpp> {
    let mut menu_items = Vec::new();

    // C++ `Player::PrepareQuestMenu`: GO involved/ender relations first, then
    // starters; do not contaminate GameObject sources with Creature relations.
    for quest in quest_store.quests_for_gameobject_ender(gameobject_entry) {
        if represented_player_quest_status_is_complete_or_incomplete_like_cpp(host, quest.id) {
            menu_items.push(RepresentedPreparedQuestMenuItemLikeCpp {
                quest: quest.clone(),
                quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                has_starter_relation: false,
                has_involved_relation: true,
            });
        }
    }

    for quest in quest_store.quests_for_gameobject_starter(gameobject_entry) {
        if let Some(menu_item) = represented_starter_quest_menu_item_like_cpp(host, quest) {
            menu_items.push(menu_item);
        }
    }

    menu_items
}

pub(crate) fn represented_starter_quest_menu_item_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> Option<RepresentedPreparedQuestMenuItemLikeCpp> {
    if !can_take_quest(host, quest) {
        return None;
    }

    let quest_icon = if quest.is_turn_in_like_cpp()
        && (!quest.is_repeatable()
            || quest.is_daily_like_cpp()
            || quest.is_weekly_like_cpp()
            || quest.is_monthly_like_cpp())
    {
        QUEST_MENU_ICON_TURN_IN_LIKE_CPP
    } else if quest.is_turn_in_like_cpp() {
        QUEST_MENU_ICON_COMPLETE_LIKE_CPP
    } else if represented_player_quest_status_is_none_like_cpp(host, quest.id) {
        QUEST_MENU_ICON_AVAILABLE_LIKE_CPP
    } else {
        return None;
    };

    Some(RepresentedPreparedQuestMenuItemLikeCpp {
        quest: quest.clone(),
        quest_icon,
        has_starter_relation: true,
        has_involved_relation: false,
    })
}

pub(crate) fn quest_list_entry_from_menu_item_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    menu_item: &RepresentedPreparedQuestMenuItemLikeCpp,
) -> QuestListEntry {
    let quest = &menu_item.quest;
    QuestListEntry {
        quest_id: quest.id,
        // C++ `PlayerMenu::SendQuestGiverQuestListMessage` writes
        // `QuestMenuItem::QuestIcon`, not `Quest::GetQuestType`.
        quest_type: menu_item.quest_icon,
        quest_level: 0,
        quest_max_scaling_level: 0,
        quest_flags: quest.flags,
        quest_flags_ex: quest.flags_ex,
        repeatable: quest.is_turn_in_like_cpp()
            && quest.is_repeatable()
            && !quest.is_daily_or_weekly_like_cpp()
            && !quest.is_monthly_like_cpp(),
        important: represented_quest_is_important_like_cpp(host, quest),
        title: quest.log_title.clone(),
    }
}

pub fn represented_creature_gossip_text_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    creature_entry: u32,
) -> Vec<wow_packet::packets::gossip::ClientGossipText> {
    let Some(quest_store) = host.hub_ref().catalogs.quests.store.as_ref() else {
        return Vec::new();
    };

    let starter_candidates = quest_store
        .quests_for_starter(creature_entry)
        .iter()
        .map(|quest| {
            (
                quest.id,
                quest.allowable_races,
                quest.allowable_classes,
                quest.min_level,
                quest.max_level,
                crate::can_take_quest(host, quest),
            )
        })
        .collect::<Vec<_>>();
    let menu_items =
        crate::represented_creature_quest_menu_items_like_cpp(host, quest_store, creature_entry);
    info!(
        creature_entry,
        race = host.hub_ref().player_race_like_cpp(),
        class = host.hub_ref().player_class_like_cpp(),
        level = host.hub_ref().player_level_like_cpp(),
        starter_candidates = ?starter_candidates,
        quests = ?menu_items.iter().map(|item| item.quest.id).collect::<Vec<_>>(),
        "Prepared creature gossip quest text like C++"
    );

    menu_items
        .iter()
        .map(|item| gossip_text_from_menu_item_like_cpp(host, item))
        .collect()
}
fn gossip_text_from_menu_item_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    menu_item: &RepresentedPreparedQuestMenuItemLikeCpp,
) -> wow_packet::packets::gossip::ClientGossipText {
    let quest = &menu_item.quest;
    wow_packet::packets::gossip::ClientGossipText {
        quest_id: quest.id as i32,
        content_tuning_id: 0,
        quest_type: i32::from(menu_item.quest_icon),
        quest_level: quest.quest_level,
        quest_max_scaling_level: quest.quest_max_scaling_level,
        quest_flags: quest.flags,
        quest_flags_ex: quest.flags_ex,
        repeatable: quest.is_turn_in_like_cpp()
            && quest.is_repeatable()
            && !quest.is_daily_or_weekly_like_cpp()
            && !quest.is_monthly_like_cpp(),
        important: crate::represented_quest_is_important_like_cpp(host, quest),
        quest_title: quest.log_title.clone(),
    }
}
