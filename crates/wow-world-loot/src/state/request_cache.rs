use super::LootState;
use wow_core::ObjectGuid;

impl LootState {
    /// Drops only this session/player's packet-building mirror.
    pub fn discard_represented_personal_loot_cache_for_player_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        _player_guid: ObjectGuid,
    ) {
        self.loot_table.remove(&owner_guid);
        self.represented_loot_cache_generations_like_cpp
            .remove(&owner_guid);
        self.represented_personal_loot_money
            .retain(|(owner, _), _| *owner != owner_guid);
        self.represented_personal_loot_owners.remove(&owner_guid);
    }

    pub(crate) fn record_represented_disenchant_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _spell_id: u32,
    ) {
        #[cfg(any(test, feature = "test-fixtures"))]
        self.represented_loot_roll_criteria_events.push(
            crate::RepresentedLootRollCriteriaEvent::Disenchant {
                player_guid: _player_guid,
                spell_id: _spell_id,
            },
        );
    }
}
