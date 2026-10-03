use std::mem;

use wow_core::ObjectGuid;

use crate::{PendingCreatureKillRewardLikeCpp, PendingCreatureSpawn, WorldEntitiesState};

impl WorldEntitiesState {
    pub fn advance_creature_tick_like_cpp(&mut self) {
        self.creature_tick = self.creature_tick.wrapping_add(1);
    }

    pub fn creature_tick_like_cpp(&self) -> u32 {
        self.creature_tick
    }

    pub fn take_pending_creature_spawn_like_cpp(&mut self) -> Option<PendingCreatureSpawn> {
        self.pending_creature_spawn.take()
    }

    pub fn take_pending_creature_kill_loot_like_cpp(&mut self) -> Vec<ObjectGuid> {
        mem::take(&mut self.pending_creature_kill_loot_like_cpp)
    }

    pub fn take_pending_creature_kill_rewards_like_cpp(
        &mut self,
    ) -> Vec<PendingCreatureKillRewardLikeCpp> {
        mem::take(&mut self.pending_creature_kill_rewards_like_cpp)
    }
}
