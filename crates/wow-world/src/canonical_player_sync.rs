//! One-way compatibility hydration into the canonical Player owner.

use crate::session::WorldSession;

pub(crate) fn hydrate_player_presentation_like_cpp(
    session: &WorldSession,
    player: &mut wow_entities::Player,
) -> Option<()> {
    #[cfg(test)]
    {
        player.gameplay_state_mut().customizations = session
            .lifecycle
            .loaded_player_customizations_for_test_like_cpp()
            .iter()
            .map(|choice| wow_entities::PlayerCustomizationChoice {
                option_id: choice.option_id,
                choice_id: choice.choice_id,
            })
            .collect();
    }
    player.set_gray_level_like_cpp(
        crate::session::hub_ref(&session)
            .gray_level(crate::session::hub_ref(&session).player_level_like_cpp()),
    );
    // C++ constructs Player before LoadFromDB / _LoadInventory
    // (CharacterHandler.cpp:1065-1070; Player.cpp:17748). Production inventory
    // already belongs to Player: querying it here would require the very
    // handle that this initial construction is about to install. Only old
    // unit fixtures hydrate equipment from their Session-side input.
    #[cfg(test)]
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
    let _ = session.core.mutate_canonical_player_like_cpp(|player| {
        player.set_liquid_status_like_cpp(status);
    });
}

#[cfg(test)]
pub(crate) fn hydrate_player_directory_fixture_like_cpp(session: &WorldSession) {
    wow_world_application::PlayerRegistryHydrationContext::new(
        session.core.player_registry_hydration_access_like_cpp(),
        &session.spell_state,
        &session.quest_state,
        (
            &session.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
            &session.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
            &session.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
            &session.fixtures.pets.represented_pet_guid_like_cpp,
        ),
        true,
    )
    .hydrate();
}
