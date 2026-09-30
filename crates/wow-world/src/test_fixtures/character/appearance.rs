//! Whole appearance operations and canonical observations for integration fixtures.
use crate::session::{WorldSession, SessionPlayerController};
use wow_core::{ObjectGuid, Position};
pub type RepresentedTransmogCriteriaEvent = crate::session::RepresentedTransmogCriteriaEvent;

pub fn attach_appearance_player_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    name: String,
    position: Position,
    map_id: u16,
    race: u8,
    class_id: u8,
    level: u8,
    gender: u8,
) {
    session.attach_player_controller_for_fixture(SessionPlayerController::new(
        guid, name, position, map_id, race, class_id, level, gender,
    ));
}

pub fn appearance_on_item_added_for_test(
    session: &mut WorldSession,
    item: &wow_entities::Item,
) -> Vec<wow_entities::PlayerValuesUpdate> {
    session.on_item_added_to_collection_like_cpp(item)
}

pub fn appearance_item_spec_class_mask_for_test(
    session: &WorldSession,
    item_id: u32,
) -> Option<u32> {
    session.appearance_item_spec_class_mask_for_test(item_id)
}

pub fn appearance_heirloom_rows_for_test(session: &WorldSession) -> Vec<(u32, u32)> {
    session.account_heirloom_rows_like_cpp()
}

pub fn appearance_has_permanent_for_test(session: &WorldSession, id: u32) -> bool {
    session.has_item_appearance_like_cpp(id) == (true, false)
}

pub fn appearance_seed_favorite_for_test(
    session: &WorldSession,
    id: u32,
    state: wow_entities::PlayerFavoriteAppearanceStateLikeCpp,
) {
    session.mutate_canonical_player_like_cpp(|player| {
        player.gameplay_state_mut().collections
            .favorite_item_appearance_entry_like_cpp(id)
            .and_modify(|current| *current = state)
            .or_insert(state);
    });
}

pub fn appearance_seed_temporary_provider_for_test(
    session: &WorldSession,
    id: u32,
    providers: std::collections::HashSet<ObjectGuid>,
) {
    session.mutate_canonical_player_like_cpp(|player| {
        for guid in providers {
            player.gameplay_state_mut().collections
                .add_temporary_item_appearance_like_cpp(id, guid);
        }
    });
}

pub fn enable_appearance_criteria_diagnostics_for_test(session: &mut WorldSession) {
    session.enable_appearance_criteria_diagnostics_for_test();
}

pub fn appearance_criteria_events_for_test(
    session: &WorldSession,
) -> &[RepresentedTransmogCriteriaEvent] {
    session.appearance_criteria_events_for_test()
}

pub fn clear_appearance_criteria_events_for_test(session: &mut WorldSession) {
    session.clear_appearance_criteria_events_for_test();
}

pub fn appearance_active_player_rows_for_test(session: &WorldSession) -> Vec<u32> {
    session.account_transmog_active_player_rows_like_cpp()
}

pub fn appearance_save_plan_for_test(
    session: &mut WorldSession,
) -> Option<wow_entities::AccountItemAppearanceSavePlanLikeCpp> {
    session.account_item_appearance_save_plan_like_cpp()
}

pub fn appearance_install_local_flags_for_test(session: &mut WorldSession, flags: u32) {
    session.set_active_player_local_flags_like_cpp(flags);
}

pub fn appearance_adopt_registered_player_for_test(session: &mut WorldSession) -> bool {
    let identity = super::super::loaded_player_identity_for_test(session);
    if !session.adopt_registered_canonical_player_fixture_like_cpp() {
        return false;
    }
    session.set_loaded_player_identity_like_cpp(
        identity.0, identity.1, identity.2, identity.3, identity.4,
    );
    true
}

pub fn appearance_owner_handle_for_test(session: &WorldSession) -> Option<wow_map::PlayerHandle> {
    session.appearance_owner_handle_for_test()
}
