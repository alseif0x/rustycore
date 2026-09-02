//! One-way compatibility hydration into the canonical Player owner.

use crate::session::WorldSession;

pub(crate) fn hydrate_player_presentation_like_cpp(
    session: &WorldSession,
    player: &mut wow_entities::Player,
) -> Option<()> {
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
    let quests = session.player_quest_gameplay_snapshot_like_cpp();
    let mount_vehicle_kit = session.player_mount_vehicle_kit_like_cpp.clone();
    let vehicle_seat_flags = session.player_vehicle_seat_flags_like_cpp;
    let vehicle_seat_id = session.player_vehicle_seat_id_like_cpp;
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
        if let Some(quests) = quests.clone() {
            state.quests = quests;
        }
        state.mount_vehicle_kit = mount_vehicle_kit.clone();
        state.vehicle_seat_flags = vehicle_seat_flags;
        state.vehicle_seat_id = vehicle_seat_id;
        state.pet_guid = pet_guid;
    });
}
