// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordered death-triggered quest failure and its SpellArea callbacks.

use std::collections::BTreeSet;

use wow_data::{
    SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP, SPELL_AREA_FLAG_AUTOREMOVE_LIKE_CPP,
    SPELL_AREA_FLAG_IGNORE_AUTOCAST_ON_QUEST_STATUS_CHANGE_LIKE_CPP, SpellAreaLikeCpp,
};
use wow_world_entities::{CorpseHostLikeCpp, ResurrectionOperationCatalogs};
use wow_world_quest::QuestSessionHostLikeCpp;

/// Source Player.cpp:14928–14997. Each slot is read after all preceding
/// quest callbacks and item removals have finished; triggered casts can
/// change the next slot, the current quest's slot, or the Player's life state.
pub async fn fail_quests_with_flag_like_cpp<H>(
    host: &mut H,
    catalogs: ResurrectionOperationCatalogs<'_>,
    flag: u32,
) -> Result<(), &'static str>
where
    H: QuestSessionHostLikeCpp
        + CorpseHostLikeCpp
        + wow_world_inventory::ItemSessionHostLikeCpp
        + wow_world_spell::PlayerCastHostLikeCpp,
{
    let handle = host
        .core()
        .player_handle_like_cpp
        .ok_or("Quest failure owner is unavailable")?;
    let store = host
        .hub_ref()
        .catalogs
        .quests
        .store
        .clone()
        .ok_or("Quest failure catalog is unavailable")?;
    for slot in 0..wow_world_quest::MAX_QUEST_LOG_SIZE_LIKE_CPP {
        let quest_id = host
            .core()
            .with_owned_player_like_cpp(|player| {
                player.quest_slot_quest_id_like_cpp(slot as u8).unwrap_or(0)
            })
            .ok_or("Quest failure Player is unavailable")?;
        let Some(quest) = store.get(quest_id).filter(|quest| quest.flags & flag != 0) else {
            continue;
        };
        if !wow_world_quest::begin_native_quest_failure_like_cpp(host, quest)? {
            continue;
        }
        send_quest_update_like_cpp(host, catalogs, quest_id).await?;
        wow_world_quest::finish_native_quest_failure_like_cpp(host, quest)?;
        let items = quest
            .objectives
            .iter()
            .filter(|objective| {
                objective.obj_type == wow_constants::quest::QUEST_OBJECTIVE_ITEM_LIKE_CPP
            })
            .map(|objective| (objective.object_id as u32, objective.amount as u32))
            .chain(
                quest
                    .item_drop
                    .iter()
                    .copied()
                    .zip(quest.item_drop_quantity.iter().copied())
                    .filter(|(_, count)| *count != 0),
            );
        for (entry, count) in items {
            let bound = host
                .hub_ref()
                .catalogs
                .item_storage_template(entry)
                .is_some_and(|item| item.bonding == wow_constants::ItemBondingType::Quest);
            if bound {
                wow_world_inventory::destroy_quest_item_count_like_cpp(host, entry, count).await?;
                if host.core().player_handle_like_cpp != Some(handle)
                    || host
                        .core()
                        .with_owned_player_like_cpp(|player| player.guid())
                        != Some(handle.guid())
                {
                    return Err("Quest failure Player incarnation changed during item removal");
                }
            }
        }
    }
    Ok(())
}

/// Source SpellMgr.cpp:713–811. Missing battleground/battlefield owners are
/// retained as errors rather than interpreted as a successful requirement.
fn spell_area_fits<H: QuestSessionHostLikeCpp>(
    host: &H,
    area: &SpellAreaLikeCpp,
    zone: u32,
    sub_area: u32,
) -> Result<bool, &'static str> {
    let (gender, race, battleground) = host
        .core()
        .with_owned_player_like_cpp(|player| {
            (
                player.data().native_sex,
                player.race_like_cpp(),
                player.battleground_instance_id_like_cpp(),
            )
        })
        .ok_or("SpellArea Player is unavailable")?;
    if area.gender != wow_data::GENDER_NONE_LIKE_CPP && area.gender != gender {
        return Ok(false);
    }
    if area.race_mask != 0
        && area.race_mask & wow_data::skill::race_mask_for_race_like_cpp(race) as u64 == 0
    {
        return Ok(false);
    }
    if area.area_id != 0 && area.area_id != zone && area.area_id != sub_area {
        return Ok(false);
    }
    for (quest, mask) in [
        (area.quest_start, area.quest_start_status),
        (area.quest_end, area.quest_end_status),
    ] {
        if quest != 0 {
            let status = wow_world_quest::represented_spell_area_quest_status_like_cpp(host, quest)
                .ok_or("SpellArea quest authority is unavailable")?;
            if (1_u32
                .checked_shl(u32::from(status))
                .ok_or("Invalid SpellArea quest status")?
                & mask)
                == 0
            {
                return Ok(false);
            }
        }
    }
    if area.aura_spell != 0 {
        let spell = area
            .aura_spell
            .checked_abs()
            .ok_or("Invalid SpellArea aura key")?;
        let present = host
            .hub_ref()
            .player_has_visible_aura_spell_like_cpp(spell)
            .ok_or("SpellArea aura authority is unavailable")?;
        if present != (area.aura_spell > 0) {
            return Ok(false);
        }
    }
    if battleground.is_some() {
        return Err("SpellArea battleground spell admission is unavailable");
    }
    if matches!(area.spell_id, 91604 | 56618 | 56617 | 57940 | 58045 | 74411) {
        return Err("SpellArea battlefield requirement owner is unavailable");
    }
    Ok(true)
}

/// Source Player.cpp:15626–15672. Freeze the sets for this one quest, run
/// complete casts in ascending spell order, then remove and refresh visibility.
/// The caller must await this before looking up the next live quest-log slot.
pub async fn send_quest_update_like_cpp<H>(
    host: &mut H,
    catalogs: ResurrectionOperationCatalogs<'_>,
    quest_id: u32,
) -> Result<(), &'static str>
where
    H: QuestSessionHostLikeCpp + CorpseHostLikeCpp + wow_world_spell::PlayerCastHostLikeCpp,
{
    let handle = host
        .core()
        .player_handle_like_cpp
        .ok_or("Quest update owner is unavailable")?;
    let (casts, removals) = {
        let hub = host.hub_ref();
        let store = hub
            .catalogs
            .spell_catalogs
            .spell_area_store
            .as_ref()
            .ok_or("Quest update SpellArea catalog is unavailable")?;
        let rows = store.spell_area_for_quest_map_bounds_like_cpp(quest_id);
        let mut casts = BTreeSet::new();
        let mut removals = BTreeSet::new();
        if !rows.is_empty() {
            let local = hub
                .player_world_local_state_like_cpp()
                .ok_or("Quest update world state is unavailable")?;
            if !local.has_zone_area_authority_like_cpp() {
                return Err("Quest update zone and area authority is unavailable");
            }
            let (zone, area) = local.zone_area_like_cpp();
            for row in rows {
                if row.flags & SPELL_AREA_FLAG_AUTOREMOVE_LIKE_CPP != 0
                    && !spell_area_fits(host, row, zone, area)?
                {
                    removals.insert(row.spell_id);
                } else if row.flags & SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP != 0
                    && row.flags & SPELL_AREA_FLAG_IGNORE_AUTOCAST_ON_QUEST_STATUS_CHANGE_LIKE_CPP
                        == 0
                {
                    casts.insert(row.spell_id);
                }
            }
        }
        removals.retain(|spell| !casts.contains(spell));
        (casts, removals)
    };
    for spell in casts {
        let spell = i32::try_from(spell).map_err(|_| "Invalid quest update spell key")?;
        if !host
            .hub_ref()
            .player_has_visible_aura_spell_like_cpp(spell)
            .ok_or("Quest update aura authority is unavailable")?
        {
            // The generic executor also reports missing native routes and
            // metadata through Result. Retain those failures under the death
            // operation instead of treating an unavailable cast as executed.
            // Ordinary pre-go rejections already return Ok after their failure
            // publication (execution_session/execution.rs:235–260).
            Box::pin(wow_world_spell::execute_server_triggered_spell_like_cpp(
                host,
                catalogs.item_guid_generator,
                catalogs.creature_spawns,
                spell,
                handle.guid(),
                wow_entities::SpellCastMetadata {
                    cast_flags: 1,
                    trigger_cast_flags_like_cpp: wow_entities::TRIGGERED_FULL_MASK_LIKE_CPP,
                    ..Default::default()
                },
            ))
            .await?;
            if host.core().player_handle_like_cpp != Some(handle)
                || host
                    .core()
                    .with_owned_player_like_cpp(|player| player.guid())
                    != Some(handle.guid())
            {
                return Err("Quest update Player incarnation changed during cast");
            }
        }
    }
    for spell in removals {
        host.remove_auras_due_to_spell_like_cpp(
            i32::try_from(spell).map_err(|_| "Invalid quest update removal key")?,
        )?;
    }
    host.update_visible_gameobjects_or_spell_clicks_like_cpp();
    Ok(())
}
