use super::*;

impl WorldSession {
    pub(in crate::session) fn validate_represented_pet_action_bar_like_cpp(
        &self,
        charm_info: &mut wow_entities::CharmInfoState,
    ) {
        let Some(spell_store) = self.spell_store() else {
            return;
        };

        for button in &mut charm_info.action_bar {
            // C++ `UNIT_ACTION_BUTTON_TYPE` drops the low bit after `MAKE_UNIT_ACTION_BUTTON`
            // stores `ActiveStates << 23`; recover the just-loaded type to apply the
            // intended `LoadPetActionBar` validation without changing the packed wire shape.
            let action_type = ((*button >> 23) & 0xFF) as u8;
            if !matches!(
                action_type,
                wow_entities::ACT_DISABLED_LIKE_CPP
                    | wow_entities::ACT_ENABLED_LIKE_CPP
                    | wow_entities::ACT_PASSIVE_LIKE_CPP
            ) {
                continue;
            }

            let action = wow_entities::unit_action_button_action_like_cpp(*button);
            if spell_store
                .get(i32::try_from(action).unwrap_or(i32::MAX))
                .is_none()
            {
                *button = wow_entities::make_unit_action_button_like_cpp(
                    0,
                    wow_entities::ACT_PASSIVE_LIKE_CPP,
                );
                continue;
            }

            if self
                .spell_catalogs
                .spell_misc_store()
                .is_some_and(|store| !store.is_autocastable_like_cpp(action))
            {
                *button = wow_entities::make_unit_action_button_like_cpp(
                    action,
                    wow_entities::ACT_PASSIVE_LIKE_CPP,
                );
            }
        }
    }
}
