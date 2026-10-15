// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item-loot admission for item containers: the loot-condition readers, the
//! quest gates and the stored-loot save (#1363 slice 4c, moved from the World
//! `handlers/spell/ops_2.rs`).

use super::*;

use wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
use wow_packet::packets::loot::{LootEntry, LootEntryFlags};
use wow_world_lifecycle::LootTemplateRow;
use wow_world_lifecycle::loot_template_rules::{
    CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP, CONDITION_TYPE_MASK_LIKE_CPP,
    ItemTemplateAddonLootMetadataLikeCpp, PLAYER_TYPE_MASK_LIKE_CPP, TYPEID_PLAYER_LIKE_CPP,
    item_loot_quest_status_allows_like_cpp, loot_entry_flags_for_row_metadata_like_cpp,
    player_class_mask_like_cpp, player_quest_status_mask_like_cpp, player_race_mask_like_cpp,
    player_team_for_race_cpp_representable,
};

pub(crate) fn item_loot_allowed_for_player_like_cpp_representable<H: LootSessionHostLikeCpp>(
    host: &H,
    item_id: u32,
    needs_quest: bool,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    conditions: &[LootConditionRowLikeCpp],
    condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
) -> bool {
    if !loot_conditions_allow_player_with_references_like_cpp_representable(
        conditions,
        condition_references,
        |condition| evaluate_loot_condition_like_cpp_representable(host, condition),
    ) {
        return false;
    }

    let Some(quests) = host.player_quest_gameplay_snapshot_like_cpp() else {
        return false;
    };
    let start_quest_id = host.item_template_start_quest_id(item_id).unwrap_or(0);
    let has_non_none_start_quest_status =
        u32::try_from(start_quest_id).ok().is_some_and(|quest_id| {
            quest_id != 0
                && (quests.statuses_like_cpp().contains_key(&quest_id)
                    || quests.rewarded_quest_ids_like_cpp().contains(&quest_id))
        });

    let has_quest_for_item =
        has_incomplete_quest_objective_for_item_like_cpp_representable(host, item_id)
            || (addon_metadata.quest_log_item_id != 0
                && has_incomplete_quest_objective_for_object_id_like_cpp_representable(
                    host,
                    addon_metadata.quest_log_item_id,
                ))
            || has_incomplete_quest_item_drop_for_item_like_cpp_representable(host, item_id);

    item_loot_quest_status_allows_like_cpp(
        addon_metadata.ignores_quest_status(),
        needs_quest,
        has_non_none_start_quest_status,
        has_quest_for_item,
    )
}

pub(crate) fn loot_entry_flags_for_row_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    row: &LootTemplateRow,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
) -> LootEntryFlags {
    let template = host.item_storage_template(row.item_id);
    loot_entry_flags_for_row_metadata_like_cpp(
        row.needs_quest,
        template
            .map(|template| template.flags)
            .unwrap_or(ItemFlags::empty()),
        addon_metadata,
    )
}

pub(crate) fn has_incomplete_quest_objective_for_item_like_cpp_representable<
    H: LootSessionHostLikeCpp,
>(
    host: &H,
    item_id: u32,
) -> bool {
    let Ok(item_object_id) = i32::try_from(item_id) else {
        return false;
    };

    has_incomplete_quest_objective_for_object_id_like_cpp_representable(host, item_object_id)
}

pub(crate) fn has_incomplete_quest_objective_for_object_id_like_cpp_representable<
    H: LootSessionHostLikeCpp,
>(
    host: &H,
    item_object_id: i32,
) -> bool {
    let Some(quest_store) = &host.hub_ref().catalogs.quests.store else {
        return false;
    };
    let Some(quests) = host.player_quest_gameplay_snapshot_like_cpp() else {
        return false;
    };

    quests.statuses_like_cpp().values().any(|status| {
        if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
            return false;
        }

        let Some(quest) = quest_store.get(status.quest_id) else {
            return false;
        };

        quest
            .objectives
            .iter()
            .enumerate()
            .any(|(fallback_index, objective)| {
                if objective.obj_type != 1 || objective.object_id != item_object_id {
                    return false;
                }

                let storage_index = usize::try_from(objective.storage_index)
                    .ok()
                    .unwrap_or(fallback_index);
                let current = status
                    .objective_counts
                    .get(storage_index)
                    .copied()
                    .unwrap_or(0);
                current < objective.amount.max(1)
            })
    })
}

pub(crate) fn has_incomplete_quest_item_drop_for_item_like_cpp_representable<
    H: LootSessionHostLikeCpp,
>(
    host: &H,
    item_id: u32,
) -> bool {
    let Some(quest_store) = &host.hub_ref().catalogs.quests.store else {
        return false;
    };
    let Some(quests) = host.player_quest_gameplay_snapshot_like_cpp() else {
        return false;
    };

    quests.statuses_like_cpp().values().any(|status| {
        if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
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

                let Some(template) = host.item_storage_template(item_id) else {
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

                direct_inventory_item_count_like_cpp_representable(host, item_id)
                    .is_some_and(|count| count < max_allowed_count)
            })
    })
}

pub(crate) fn direct_inventory_item_count_like_cpp_representable<H: LootSessionHostLikeCpp>(
    host: &H,
    item_id: u32,
) -> Option<u32> {
    let (state, hub) = host.split_inventory_ref();
    state.direct_inventory_item_count_like_cpp_representable(hub, item_id)
}

pub(crate) fn evaluate_loot_condition_like_cpp_representable<H: LootSessionHostLikeCpp>(
    host: &H,
    condition: &LootConditionRowLikeCpp,
) -> Option<bool> {
    let quests = host.player_quest_gameplay_snapshot_like_cpp()?;
    match condition.condition_type_or_reference {
        0 => Some(true),
        2 => {
            if condition.value3 != 0 {
                return None;
            }
            Some(
                direct_inventory_item_count_like_cpp_representable(host, condition.value1)?
                    >= condition.value2,
            )
        }
        6 => Some(
            player_team_for_race_cpp_representable(host.hub_ref().player_race_like_cpp())
                == condition.value1,
        ),
        8 => Some(
            quests
                .rewarded_quest_ids_like_cpp()
                .contains(&condition.value1),
        ),
        9 => Some(
            quests
                .statuses_like_cpp()
                .get(&condition.value1)
                .is_some_and(|status| status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP),
        ),
        14 => Some(
            !quests.statuses_like_cpp().contains_key(&condition.value1)
                && !quests
                    .rewarded_quest_ids_like_cpp()
                    .contains(&condition.value1),
        ),
        15 => Some(
            player_class_mask_like_cpp(host.hub_ref().player_class_like_cpp())
                .is_some_and(|mask| mask & condition.value1 != 0),
        ),
        16 => Some(
            player_race_mask_like_cpp(host.hub_ref().player_race_like_cpp())
                .is_some_and(|mask| mask & condition.value1 != 0),
        ),
        20 => Some(u32::from(host.hub_ref().player_gender_like_cpp()) == condition.value1),
        25 => i32::try_from(condition.value1)
            .ok()
            .map(|spell_id| host.known_spells_like_cpp().contains(&spell_id)),
        27 => condition_compare_values_like_cpp(
            condition.value2,
            u32::from(host.hub_ref().player_level_like_cpp()),
            condition.value1,
        ),
        28 => Some(
            quests
                .statuses_like_cpp()
                .get(&condition.value1)
                .is_some_and(|status| status.status == 2)
                && !quests
                    .rewarded_quest_ids_like_cpp()
                    .contains(&condition.value1),
        ),
        47 => Some(
            player_quest_status_mask_like_cpp(
                quests
                    .statuses_like_cpp()
                    .get(&condition.value1)
                    .map(|status| status.status),
                quests
                    .rewarded_quest_ids_like_cpp()
                    .contains(&condition.value1),
            ) & condition.value2
                != 0,
        ),
        48 => Some(
            player_quest_objective_progress_like_cpp_representable(host, condition.value1)
                == Some(condition.value3 as i32),
        ),
        CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP => Some(condition.value1 == TYPEID_PLAYER_LIKE_CPP),
        CONDITION_TYPE_MASK_LIKE_CPP => Some(condition.value1 & PLAYER_TYPE_MASK_LIKE_CPP != 0),
        _ => None,
    }
}

pub(crate) fn player_quest_objective_progress_like_cpp_representable<H: LootSessionHostLikeCpp>(
    host: &H,
    objective_id: u32,
) -> Option<i32> {
    let quest_store = host.hub_ref().catalogs.quests.store.as_ref()?;
    let quests = host.player_quest_gameplay_snapshot_like_cpp()?;

    for status in quests.statuses_like_cpp().values() {
        let Some(quest) = quest_store.get(status.quest_id) else {
            continue;
        };
        let Some((_, objective)) = quest
            .objectives
            .iter()
            .enumerate()
            .find(|(_, objective)| objective.id == objective_id)
        else {
            continue;
        };
        let objective_index = objective.storage_index.max(0) as usize;
        return Some(
            status
                .objective_counts
                .get(objective_index)
                .copied()
                .unwrap_or(0),
        );
    }

    None
}

pub(crate) async fn save_new_stored_item_loot_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    item_guid: wow_core::ObjectGuid,
    money: u32,
    items: &[LootEntry],
) {
    let (state, hub) = (host.lifecycle(), host.hub_ref());
    state
        .save_new_stored_item_loot_like_cpp(hub, item_guid, money, items)
        .await
}
