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

#[cfg(test)]
pub(crate) fn hydrate_player_directory_fixture_like_cpp(session: &WorldSession) {
    let known_spells = session.known_spells_fixture_like_cpp();
    let in_vehicle = session.player_vehicle_seat_flags_like_cpp.is_some();
    let has_vehicle_kit = session.player_mount_vehicle_kit_like_cpp.is_some();
    let vehicle_seat = session
        .player_vehicle_seat_id_like_cpp
        .and_then(|seat| i32::try_from(seat).ok())
        .unwrap_or(0);
    let pet_guid = session.represented_pet_guid_like_cpp;
    let _ = session.mutate_canonical_player_like_cpp(|player| {
        let state = player.gameplay_state_mut();
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
        state.in_vehicle = in_vehicle;
        state.has_vehicle_kit = has_vehicle_kit;
        state.vehicle_seat = vehicle_seat;
        state.pet_guid = pet_guid;
    });
}
