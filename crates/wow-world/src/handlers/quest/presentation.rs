//! Quest catalog to packet presentation, with existing read and log order.
use super::*;

pub(crate) fn quest_giver_creature_id_from_source_like_cpp(source_guid: ObjectGuid) -> i32 {
    if source_guid.is_any_type_creature() {
        i32::try_from(source_guid.entry()).unwrap_or(0)
    } else {
        0
    }
}


pub(crate) fn represented_quest_completion_npc_response_like_cpp(
    quest_store: &wow_data::quest::QuestStore,
    raw_quest_ids: &[i32],
) -> Vec<QuestCompletionNpc> {
    raw_quest_ids
        .iter()
        .filter_map(|&raw_quest_id| {
            let quest_id = u32::try_from(raw_quest_id).ok()?;
            if quest_store.get(quest_id).is_none() {
                return None;
            }

            let mut npcs = Vec::new();
            for creature_entry in quest_store.creature_ender_entries_for_quest_like_cpp(quest_id) {
                let Ok(entry) = i32::try_from(creature_entry) else {
                    debug!(
                        quest_id,
                        creature_entry,
                        "QueryQuestCompletionNPCs: creature entry exceeds signed i32 response field"
                    );
                    continue;
                };
                npcs.push(entry);
            }

            for go_entry in quest_store.gameobject_ender_entries_for_quest_like_cpp(quest_id) {
                npcs.push((go_entry | 0x8000_0000) as i32);
            }

            Some(QuestCompletionNpc {
                quest_id: raw_quest_id,
                npcs,
            })
        })
        .collect()
}


impl WorldSession {
    pub(in crate::handlers::quest) fn represented_quest_dialog_classification_like_cpp(
        quest: &wow_data::quest::QuestTemplate,
        quest_info: Option<&wow_data::progression_rewards::QuestInfoStore>,
    ) -> crate::handlers::quest::dialog_status::QuestDialogClassificationLikeCpp {
        crate::handlers::quest::dialog_status::QuestDialogClassificationLikeCpp::new(
            quest.flags,
            quest.flags_ex,
            quest_info.and_then(|store| store.get(quest.quest_info_id as u32)),
        )
    }

}
