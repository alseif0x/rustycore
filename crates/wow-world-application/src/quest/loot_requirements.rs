// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player-specific quest predicates used while resolving GameObject loot.

use wow_loot::LootStoreKind;
use wow_world_core::session::{
    OwnedInventoryAccessLikeCpp, QuestObjectiveAccessLikeCpp, SessionCatalogs,
};
use wow_world_inventory::InventoryState;

use super::{SessionQuestState, objective_progress::current_quest_gameplay_snapshot_like_cpp};

pub fn represented_gameobject_loot_ids_have_quest_loot_for_player_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
    catalogs: &SessionCatalogs,
    inventory: &InventoryState,
    quest_state: &SessionQuestState,
    loot_ids: impl IntoIterator<Item = u32>,
    consumer_test: bool,
) -> bool {
    let Some(stores) = catalogs.loot_stores.as_ref() else {
        return false;
    };
    let Some(store) = stores.get(&LootStoreKind::Gameobject) else {
        return false;
    };
    loot_ids.into_iter().filter(|id| *id != 0).any(|loot_id| {
        store.have_quest_loot_for_player_like_cpp(loot_id, stores.as_ref(), |item_id| {
            represented_player_has_quest_for_loot_item_like_cpp(
                owner,
                inventory_access,
                catalogs,
                inventory,
                quest_state,
                item_id,
                consumer_test,
            )
        })
    })
}

fn represented_player_has_quest_for_loot_item_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
    catalogs: &SessionCatalogs,
    inventory: &InventoryState,
    quest_state: &SessionQuestState,
    item_id: u32,
    consumer_test: bool,
) -> bool {
    represented_current_player_has_incomplete_quest_objective_for_item_like_cpp(
        owner,
        catalogs,
        quest_state,
        item_id,
        consumer_test,
    ) || catalogs
        .item_template_addon_quest_log_item_id_like_cpp(item_id)
        .is_some_and(|quest_log_item_id| {
            quest_log_item_id != 0
                && represented_current_player_has_incomplete_quest_objective_for_item_like_cpp(
                    owner,
                    catalogs,
                    quest_state,
                    quest_log_item_id,
                    consumer_test,
                )
        })
        || represented_current_player_has_incomplete_quest_item_drop_for_item_like_cpp(
            owner,
            inventory_access,
            catalogs,
            inventory,
            quest_state,
            item_id,
            consumer_test,
        )
}

fn represented_current_player_has_incomplete_quest_objective_for_item_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    catalogs: &SessionCatalogs,
    quest_state: &SessionQuestState,
    item_id: u32,
    consumer_test: bool,
) -> bool {
    let Ok(item_object_id) = i32::try_from(item_id) else {
        return false;
    };
    let Some(quests) = current_quest_gameplay_snapshot_like_cpp(owner, quest_state, consumer_test)
    else {
        return false;
    };
    let Some(store) = catalogs.quests.store.as_ref() else {
        return false;
    };
    wow_entities::player_has_incomplete_quest_objective_for_object_id_like_cpp(
        quests.statuses_like_cpp(),
        |id| store.get(id).map(|quest| quest.objective_rules_like_cpp()),
        item_object_id,
    )
}

fn represented_current_player_has_incomplete_quest_item_drop_for_item_like_cpp(
    owner: &QuestObjectiveAccessLikeCpp<'_>,
    inventory_access: &OwnedInventoryAccessLikeCpp<'_>,
    catalogs: &SessionCatalogs,
    inventory: &InventoryState,
    quest_state: &SessionQuestState,
    item_id: u32,
    consumer_test: bool,
) -> bool {
    let Some(quest_store) = catalogs.quests.store.as_ref() else {
        return false;
    };
    let Some(quests) = current_quest_gameplay_snapshot_like_cpp(owner, quest_state, consumer_test)
    else {
        return false;
    };
    quests.statuses_like_cpp().values().any(|status| {
        if status.status != wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP {
            return false;
        }
        let Some(quest) = quest_store.get(status.quest_id) else {
            return false;
        };
        quest
            .item_drop
            .iter()
            .enumerate()
            .any(|(index, drop_item_id)| {
                if *drop_item_id != item_id {
                    return false;
                }
                let Some(template) = catalogs.item_storage_template(item_id) else {
                    return false;
                };
                let quantity = quest.item_drop_quantity[index];
                let mut max_allowed_count = if quantity != 0 {
                    quantity
                } else {
                    template.max_stack_size
                };
                if template.max_count > 0 {
                    max_allowed_count = max_allowed_count.min(template.max_count as u32);
                }
                inventory
                    .represented_inventory_item_counts_with_access_like_cpp(inventory_access)
                    .is_some_and(|counts| {
                        counts.get(&item_id).copied().unwrap_or(0) < max_allowed_count
                    })
            })
    })
}
