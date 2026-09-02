//! One-way compatibility hydration into the canonical Player owner.

use crate::session::WorldSession;

pub(crate) fn hydrate_player_presentation_like_cpp(
    session: &WorldSession,
    player: &mut wow_entities::Player,
) -> Option<()> {
    let (zone_id, area_id) = session.player_zone_area_like_cpp();
    player
        .unit_mut()
        .world_mut()
        .set_zone_and_area(zone_id, area_id);
    #[cfg(test)]
    {
        player.gameplay_state_mut().customizations = session
            .loaded_player_customizations_like_cpp
            .iter()
            .map(|choice| wow_entities::PlayerCustomizationChoice {
                option_id: choice.option_id,
                choice_id: choice.choice_id,
            })
            .collect();
    }
    player.gameplay_state_mut().gray_level = session.gray_level(session.player_level_like_cpp());
    for (slot, values) in session
        .loaded_player_visible_items_for_create_like_cpp()?
        .into_iter()
        .enumerate()
    {
        crate::canonical_player_access::set_player_visible_item_values_like_cpp(
            player, slot as u8, values,
        );
    }
    Some(())
}

pub(crate) fn sync_player_zone_area_like_cpp(session: &WorldSession, zone_id: u32, area_id: u32) {
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player
            .unit_mut()
            .world_mut()
            .set_zone_and_area(zone_id, area_id);
    });
}

pub(crate) fn sync_player_liquid_status_like_cpp(session: &WorldSession, status: u32) {
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.gameplay_state_mut().liquid_status = status;
    });
}

pub(crate) fn sync_player_level_like_cpp(session: &WorldSession, level: u8, gray_level: u8) {
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        player.unit_mut().set_level(level);
        player.gameplay_state_mut().gray_level = gray_level;
    });
}

pub(crate) fn sync_player_directory_gameplay_to_canonical_like_cpp(session: &WorldSession) {
    let difficulty = session.represented_dungeon_difficulty_id_like_cpp;
    #[cfg(test)]
    let known_spells = session.known_spells_like_cpp();
    let quest_statuses = session
        .player_quests
        .iter()
        .map(
            |(&quest_id, status)| wow_entities::PlayerQuestStatusRecord {
                quest_id,
                status: status.status,
                explored: false,
                timer_expires_at: None,
            },
        )
        .collect();
    let objective_counts = session
        .player_quests
        .iter()
        .map(|(&quest_id, status)| (quest_id, status.objective_counts.clone()))
        .collect();
    let rewarded = session.rewarded_quests.iter().copied().collect();
    let daily = session
        .daily_quests_completed_like_cpp
        .iter()
        .copied()
        .collect();
    let df = session.df_quests_like_cpp.iter().copied().collect();
    let Some(inventory_item_counts) = session.represented_inventory_item_counts_like_cpp() else {
        return;
    };
    let inventory_item_counts = inventory_item_counts.into_iter().collect();
    let forced_reputation_ranks = session
        .player_forced_reputation_ranks_snapshot_like_cpp()
        .into_iter()
        .map(|(faction, rank)| (faction, rank.as_u8()))
        .collect();
    let pending_share = session
        .represented_pending_quest_sharing_like_cpp
        .map(|pending| (pending.sender_guid, pending.quest_id));
    let transport = session.player_transport_info_like_cpp().map(|transport| {
        wow_entities::PlayerTransportState {
            guid: transport.guid,
            x: transport.x,
            y: transport.y,
            z: transport.z,
            orientation: transport.o,
            seat: transport.seat,
            time: transport.time,
            prev_time: transport.prev_time,
            vehicle_id: transport.vehicle_id,
        }
    });
    let in_vehicle = session.player_vehicle_seat_flags_like_cpp.is_some();
    let has_vehicle_kit = session.player_mount_vehicle_kit_like_cpp.is_some();
    let vehicle_seat = session
        .player_vehicle_seat_id_like_cpp
        .and_then(|seat| i32::try_from(seat).ok())
        .unwrap_or(0);
    let pet_guid = session.represented_pet_guid_like_cpp;
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        let state = player.gameplay_state_mut();
        state.dungeon_difficulty_id = difficulty;
        #[cfg(test)]
        {
            state.spells.known_spells = known_spells.clone();
            state.spells.rows = known_spells
                .iter()
                .copied()
                .map(|spell_id| {
                    (
                        spell_id,
                        wow_entities::PlayerKnownSpellRecord {
                            spell_id,
                            state: wow_entities::PlayerSpellLoadState::Unchanged,
                            active: true,
                            disabled: false,
                            favorite: false,
                            dependent: false,
                        },
                    )
                })
                .collect();
        }
        state.quests.statuses = quest_statuses;
        state.quests.objective_counts_by_quest = objective_counts;
        state.quests.rewarded_quest_ids = rewarded;
        state.quests.daily_quest_ids = daily;
        state.quests.df_quest_ids = df;
        state.quests.pending_share = pending_share;
        state.inventory_item_counts = inventory_item_counts;
        state.forced_reputation_ranks = forced_reputation_ranks;
        state.pass_on_group_loot = session.pass_on_group_loot;
        state.transport = transport;
        state.in_vehicle = in_vehicle;
        state.has_vehicle_kit = has_vehicle_kit;
        state.vehicle_seat = vehicle_seat;
        state.pet_guid = pet_guid;
    });
}
