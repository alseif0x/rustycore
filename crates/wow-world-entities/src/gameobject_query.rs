use wow_core::ObjectGuid;

use crate::WorldEntitiesState;

impl WorldEntitiesState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn record_represented_gameobject_faction_template_like_cpp(
        &mut self,
        guid: ObjectGuid,
        faction_template: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .faction_template = (faction_template != 0).then_some(faction_template);
    }

    pub fn restore_represented_gameobject_override_flags_like_cpp(&mut self, guid: ObjectGuid) {
        if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
            if let Some(flags) = state.gameobject_override_flags {
                state.gameobject_flags = flags;
            }
        }
    }
}
