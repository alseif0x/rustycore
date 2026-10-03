use wow_core::ObjectGuid;
use wow_loot::LootStoreKind;
use wow_world_core::session::{HubMut, HubRef};

use crate::{PendingCreatureKillRewardLikeCpp, WorldEntitiesState};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedCreatureKillEventLikeCpp;

impl WorldEntitiesState {
    /// Queue one creature kill for the loot and reward phases, deduplicated.
    ///
    /// Both writers of this queue — `run_combat_tick` and the delivery handler
    /// for a map-owned resolution (#28) — must enqueue identically, so the
    /// dedup lives here instead of being written twice. The queues stay private
    /// to the session; a handler asks for the transition, not for the fields.
    pub fn queue_pending_creature_kill_like_cpp(
        &mut self,
        killer_guid: ObjectGuid,
        creature_guid: ObjectGuid,
        creature_entry: u32,
        creature_level: u8,
    ) {
        if !self
            .pending_creature_kill_loot_like_cpp
            .contains(&creature_guid)
        {
            self.pending_creature_kill_loot_like_cpp.push(creature_guid);
        }
        if !self
            .pending_creature_kill_rewards_like_cpp
            .iter()
            .any(|reward| reward.creature_guid == creature_guid)
        {
            self.pending_creature_kill_rewards_like_cpp
                .push(PendingCreatureKillRewardLikeCpp {
                    killer_guid,
                    creature_guid,
                    creature_entry,
                    creature_level,
                });
        }
    }

    /// XP reward for killing a creature.
    /// C++ `Trinity::XP::Gain` / `Trinity::XP::BaseGain`.
    pub fn creature_kill_xp(&self, hub: HubRef<'_>, mob_level: u8) -> u32 {
        let pl = hub.player_level_like_cpp() as i32;
        let ml = mob_level as i32;

        // nBaseExp by content level (WotLK = 71-80 content)
        let n_base_exp: i32 = if pl >= 71 {
            580
        } else if pl >= 61 {
            235
        } else {
            45
        };

        // Gray level check
        let gray = hub.gray_level(pl as u8) as i32;
        if ml <= gray {
            return 0;
        }

        let base_gain = if ml >= pl {
            let diff = (ml - pl).min(4);
            ((pl * 5 + n_base_exp) * (20 + diff) / 10 + 1) / 2
        } else {
            let zd = hub.zero_difference(pl as u8) as i32;
            (pl * 5 + n_base_exp) * (zd + ml - pl) / zd
        };

        base_gain.max(0) as u32
    }

    fn represented_creature_can_skin_after_death_state_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        creature_guid: ObjectGuid,
    ) -> bool {
        let skin_loot_id = hub
            .core
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership().skin_loot_id
            })
            .unwrap_or(0);
        if skin_loot_id == 0 {
            return false;
        }
        hub.catalogs
            .loot_stores
            .as_ref()
            .and_then(|stores| stores.get(&LootStoreKind::Skinning))
            .is_some_and(|store| store.collect_loot_ids_like_cpp().contains(&skin_loot_id))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_creature_kill_events_like_cpp(
        &self,
    ) -> &[RepresentedCreatureKillEventLikeCpp] {
        &self.represented_creature_kill_events_like_cpp
    }
}
