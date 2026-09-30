use super::*;

impl WorldSession {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(in crate::session) fn record_represented_tapper_pet_killed_unit_hooks_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) {
        if !self.character_lifecycle_fixture_mode() {
            return;
        }
        let (Some(player_guid), Some(pet_guid)) = (
            self.player_guid(),
            self.player_pet_guid_state_like_cpp().flatten(),
        ) else {
            return;
        };
        let tapper_has_current_player = self
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.tap_list().contains(&player_guid)
            })
            .unwrap_or(true);
        if !tapper_has_current_player {
            return;
        }
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi {
                tapper_guid: player_guid,
                pet_guid,
                victim_guid: creature_guid,
            },
        );
    }
}
