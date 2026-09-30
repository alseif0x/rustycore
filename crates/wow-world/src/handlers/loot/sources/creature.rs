// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature loot generation, persistence observations, and source snapshots.

use super::*;
#[path = "creature/generation.rs"]
mod generation;
#[path = "creature/ensure.rs"]
mod ensure;
#[path = "creature/melee.rs"]
mod melee;

use wow_map::manager::{MapObjectTickContinuation, MeleeLootError, PendingMeleeKills,
    PreparedMeleeKill, PreparedMeleeLoot};
// One specific original operation; no context, authority mirror or token lease.
type OriginalMeleeLoot<'a> = Option<(&'a MapObjectTickContinuation, &'a mut PreparedMeleeLoot)>;

impl WorldSession {
    pub(in crate::handlers::loot) fn sync_represented_creature_loot_to_canonical_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        _player_guid: ObjectGuid,
    ) -> Option<()> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(creature_guid) else {
            return (represented_local_loot_fixture_allowed_like_cpp()
                && self.loot_table.contains_key(&creature_guid))
            .then_some(());
        };
        let loot = self.loot_table.get(&creature_guid)?.clone();
        let is_personal = self
            .represented_personal_loot_owners
            .contains(&creature_guid);
        let (shared, personal) = self.represented_loot_authority_pools_like_cpp(
            creature_guid,
            _player_guid,
            loot,
            is_personal,
        )?;
        let installed = authority
            .initialize_pristine_like_cpp(shared, personal)
            .installed();
        if !installed
            && authority
                .snapshot_for_player_like_cpp(_player_guid)
                .is_none()
        {
            self.loot_table.remove(&creature_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&creature_guid);
            return None;
        }
        self.refresh_owned_loot_summary_like_cpp(creature_guid);
        let _ = self.reconcile_represented_loot_cache_like_cpp(creature_guid, _player_guid);
        Some(())
    }

    pub(in crate::handlers::loot) fn canonical_creature_fully_looted_after_represented_sync_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        player_guid: ObjectGuid,
        fallback_fully_looted: bool,
    ) -> bool {
        let owner = self.capture_creature_loot_owner(creature_guid);
        let fallback_fully_looted = match &owner {
            wow_map::manager::CreatureLootAccess::Rejected(_) => return false,
            wow_map::manager::CreatureLootAccess::Ready(_) => false,
            wow_map::manager::CreatureLootAccess::NoActor => fallback_fully_looted,
        };
        if self
            .sync_represented_creature_loot_to_canonical_like_cpp(creature_guid, player_guid)
            .is_some()
        {
            return self
                .creature_loot_fully_consumed(&owner, creature_guid)
                .unwrap_or(fallback_fully_looted);
        }

        fallback_fully_looted
    }

    pub(in crate::handlers::loot) async fn represented_ae_loot_creature_targets_like_cpp(
        &mut self,
        main_loot_target: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(player_position) = self.player_position_like_cpp() else {
            return Vec::new();
        };

        let mut candidates: Vec<ObjectGuid> = self
            .world_creature_guids()
            .into_iter()
            .filter(|guid| {
                if *guid == main_loot_target || !guid.is_creature_or_vehicle() {
                    return false;
                }
                self.represented_creature_loot_state_like_cpp(*guid)
                    .is_some_and(|creature| {
                        !creature.is_alive()
                            && player_position.is_within_dist(&creature.position(), 30.0)
                    })
            })
            .collect();
        candidates.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        let mut result = Vec::new();
        for owner_guid in candidates {
            let Some(creature) = self.represented_creature_loot_state_like_cpp(owner_guid) else {
                continue;
            };
            if !creature.tappers().is_empty() && !creature.tappers().contains(&player_guid) {
                continue;
            }
            // C++ `CMSG_LOOT_UNIT` only reads the Loot created by
            // `Unit::Kill`; it never regenerates a corpse pool. Reconcile the
            // active object-owned generation and fail closed if kill-time
            // generation is absent or the corpse lifetime was retired.
            if !self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid) {
                self.loot_table.remove(&owner_guid);
                continue;
            }

            if self.loot_table.get(&owner_guid).is_some_and(|loot| {
                self.represented_loot_can_be_opened_by_player_like_cpp(
                    owner_guid,
                    loot,
                    player_guid,
                )
            }) {
                result.push(owner_guid);
            }
        }

        result
    }

    pub(crate) async fn ensure_represented_creature_kill_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) {
        let (creature_owner, creature) = self.observe_creature_loot_owner(creature_guid);
        let Some(creature) = creature else {
            return;
        };
        let Some(loot_owner_guid) = creature.tappers().first().copied() else {
            return;
        };
        let loot_scope_player_guid = if self.current_map_dungeon_state_like_cpp() == Some(false) {
            let connected_tappers =
                self.represented_connected_creature_tappers_like_cpp(creature.tappers());
            self.player_guid()
                .filter(|player_guid| connected_tappers.contains(player_guid))
                .or_else(|| connected_tappers.first().copied())
                .unwrap_or(loot_owner_guid)
        } else {
            loot_owner_guid
        };

        let _ = self.ensure_creature_loot(
            creature_guid,
            loot_owner_guid,
            creature.level(),
            creature.entry(),
            creature.loot_id(),
            creature.gold_min(),
            creature.gold_max(),
            creature.dungeon_encounter_id(),
            creature.tappers(),
            creature.loot_lifecycle_revision(),
            Some(&creature_owner),
            None,
        )
        .await;
        if self
            .sync_represented_creature_loot_to_canonical_like_cpp(
                creature_guid,
                loot_scope_player_guid,
            )
            .is_none()
        {
            self.loot_table.remove(&creature_guid);
        }
    }

    /// Install kill-time pools only while the exact creature death lifetime
    /// observed before async template generation is still current. C++ runs
    /// `Unit::Kill` and loot creation on one map thread; this lock-scoped CAS
    /// is the Rust equivalent and prevents corpse-removal/respawn ABA.
    pub(in crate::handlers::loot) fn install_represented_creature_kill_loot_if_current_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        expected_authority: &OwnedLootAuthority,
        expected_object_generation: u64,
        expected_loot_lifecycle_revision: u64,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> bool {
        let owner = self.capture_creature_loot_owner(creature_guid);
        self.install_creature_loot_for_owner(&owner, creature_guid, expected_authority,
            expected_object_generation, expected_loot_lifecycle_revision, shared, personal)
    }

    pub(in crate::handlers::loot) fn represented_creature_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureLootStateLikeCpp> {
        self.observe_creature_loot_owner(guid).1
    }

    pub(in crate::handlers::loot) fn represented_creature_position_for_loot_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<wow_core::Position> {
        match self.capture_creature_loot_owner(guid) {
            wow_map::manager::CreatureLootAccess::Rejected(_) => return None,
            wow_map::manager::CreatureLootAccess::Ready(handle) => {
                let mut manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
                return match manager.observe_idle_creature_loot_source(&handle) {
                    wow_map::manager::CreatureLootAccess::Ready(source) => Some(source.position()),
                    _ => None,
                };
            }
            wow_map::manager::CreatureLootAccess::NoActor => {}
        }
        if let Some(position) = self
            .canonical_map_object_position_for_loot_like_cpp(guid, &[AccessorObjectKind::Creature])
        {
            return Some(position);
        }

        self.represented_creature_loot_state_like_cpp(guid)
            .map(|creature| creature.position())
    }
}
