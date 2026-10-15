// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `Spell::EffectQuestComplete` (`SpellEffects.cpp`) for the represented
//! session player (#1363 slice 12b2), moved from the World
//! `session/spell_effects/effects_progress.rs`. The spell execution reaches it
//! through `SpellExecutionHostLikeCpp::apply_quest_complete_effect_like_cpp`,
//! whose World impl names this function: spell sits below quest.

use tracing::debug;
use wow_core::ObjectGuid;

use crate::QuestSessionHostLikeCpp;

pub async fn apply_quest_complete_effect_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    target_guid: ObjectGuid,
    quest_id: i32,
) -> Result<(), &'static str> {
    host.invalidate_player_quest_status_authority_like_cpp();
    const QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP_LOCAL: u32 = 0x0000_0400;

    let player_guid = host.player_guid().ok_or("No player GUID")?;
    if target_guid != player_guid {
        return Ok(());
    }

    let Ok(quest_id) = u32::try_from(quest_id) else {
        debug!(
            account = host.core().account_id,
            "Skipping represented quest-complete spell effect with negative MiscValue"
        );
        return Ok(());
    };
    if quest_id == 0 {
        return Ok(());
    }

    let Some(quest) = host
        .hub_ref()
        .catalogs
        .quests
        .store
        .as_deref()
        .and_then(|store| store.get(quest_id))
        .cloned()
    else {
        return Ok(());
    };

    let Some((quest_is_in_log, should_send_event_complete)) =
        crate::mark_represented_quest_explored_like_cpp(
            host,
            quest_id,
            wow_conditions::QUEST_STATUS_FAILED_LIKE_CPP,
        )
    else {
        return Ok(());
    };

    if quest_is_in_log {
        if should_send_event_complete {
            host.send_packet(&wow_packet::packets::quest::QuestUpdateComplete { quest_id });
        }
        crate::complete_represented_quest_after_add_with_generator_like_cpp(
            host,
            item_guid_generator,
            &quest,
        )
        .await;
    } else if (quest.flags & QUEST_FLAGS_TRACKING_EVENT_LIKE_CPP_LOCAL) != 0 {
        if crate::set_represented_quest_rewarded_like_cpp(host, quest_id, true) == false {
            return Ok(());
        }
        let quest_bit = host
            .hub_ref()
            .catalogs
            .quests
            .v2_store
            .as_deref()
            .map(|store| store.get_quest_unique_bit_flag_like_cpp(quest_id))
            .unwrap_or(0);
        if quest_bit != 0 {
            crate::set_loaded_quest_completed_bit_like_cpp(host, quest_bit);
        }
        host.sync_player_registry_state_like_cpp();
    }

    Ok(())
}
