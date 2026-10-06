use super::LootState;
use wow_core::ObjectGuid;
use wow_packet::packets::loot::CreatureLoot;

impl LootState {
    pub fn represented_loot_money_for_player_like_cpp(
        &self,
        loot_guid: ObjectGuid,
        loot: &CreatureLoot,
        player_guid: ObjectGuid,
    ) -> u32 {
        if self.represented_personal_loot_owners.contains(&loot_guid) {
            return self
                .represented_personal_loot_money
                .get(&(loot_guid, player_guid))
                .copied()
                .unwrap_or(0);
        }

        loot.coins
    }
}
