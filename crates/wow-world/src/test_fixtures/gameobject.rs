use crate::session::{RepresentedGameObjectUseState, WorldSession};
use wow_core::{ObjectGuid, Position};

pub fn set_represented_gameobject_use_type_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    go_type: u8,
) {
    let mut state = RepresentedGameObjectUseState::default();
    state.go_type = Some(go_type);
    session.represented_gameobject_use_states.insert(guid, state);
}

pub fn record_represented_gameobject_runtime_state_for_test(
    session: &mut WorldSession,
    map_id: u16,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    go_type: u8,
) {
    session.record_represented_gameobject_runtime_state_like_cpp(
        map_id, guid, entry, position, go_type,
    );
}
