use super::LootState;
use crate::RepresentedLootRollState;
use wow_core::ObjectGuid;
use wow_world_core::session::mailbox::LootRollCommandIdentityLikeCpp;

impl LootState {
    pub fn represented_loot_roll_like_cpp(
        &self,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
    ) -> Option<&RepresentedLootRollState> {
        self.represented_loot_rolls.get(&(loot_obj, loot_list_id))
    }

    pub fn represented_loot_roll_mut_like_cpp(
        &mut self,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
    ) -> Option<&mut RepresentedLootRollState> {
        self.represented_loot_rolls.get_mut(&(loot_obj, loot_list_id))
    }

    pub fn insert_represented_loot_roll_like_cpp(
        &mut self,
        roll: RepresentedLootRollState,
    ) -> Option<RepresentedLootRollState> {
        self.represented_loot_rolls
            .insert((roll.loot_obj, roll.loot_list_id), roll)
    }

    pub fn remove_represented_loot_roll_like_cpp(
        &mut self,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
    ) -> Option<RepresentedLootRollState> {
        self.represented_loot_rolls.remove(&(loot_obj, loot_list_id))
    }

    pub fn represented_loot_roll_keys_snapshot_like_cpp(&self) -> Vec<(ObjectGuid, u8)> {
        self.represented_loot_rolls.keys().copied().collect()
    }

    pub fn represented_loot_roll_command_identities_snapshot_like_cpp(
        &self,
    ) -> Vec<LootRollCommandIdentityLikeCpp> {
        self.represented_loot_rolls
            .values()
            .map(|state| state.command_identity.clone())
            .collect()
    }
}
