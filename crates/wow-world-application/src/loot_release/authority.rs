// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;
use std::collections::HashMap;
use wow_loot::rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp;

impl LootReleaseCxLikeCpp<'_> {
    pub(super) fn canonical_creature_fully_looted_after_represented_sync_like_cpp(
        &mut self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
        fallback_fully_looted: bool,
    ) -> bool {
        if self
            .sync_represented_creature_loot_to_canonical_like_cpp(guid, player_guid)
            .is_some()
        {
            return self
                .owner
                .canonical_creature_is_fully_looted_like_cpp(guid)
                .unwrap_or(fallback_fully_looted);
        }
        fallback_fully_looted
    }

    pub(super) fn canonical_gameobject_fully_looted_after_represented_sync_like_cpp(
        &mut self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
        fallback_fully_looted: bool,
    ) -> bool {
        if self
            .sync_represented_gameobject_loot_to_canonical_like_cpp(guid, player_guid)
            .is_some()
        {
            return self
                .owner
                .transitions_like_cpp()
                .canonical_gameobject_is_fully_looted_like_cpp(guid)
                .unwrap_or(fallback_fully_looted);
        }
        fallback_fully_looted
    }

    /// Typed counterpart of
    /// [`Self::represented_owned_loot_authority_like_cpp`]; it keeps
    /// reconciliation exhaustion distinct from absent loot (F6-7 R4).
    pub(super) fn represented_owned_loot_authority_outcome_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
        self.owner
            .represented_owned_loot_authority_outcome_like_cpp(guid)
    }

    /// Compatibility wrapper: `Absent` and `Unavailable` stay fail-closed
    /// `None`, so every consumer of this shape is unchanged. It delegates to
    /// the core compatibility body, which owns the only `Option` collapse.
    pub(super) fn represented_owned_loot_authority_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        self.owner.represented_owned_loot_authority_like_cpp(guid)
    }

    fn refresh_owned_loot_summary_like_cpp(&mut self, guid: ObjectGuid) {
        self.owner.refresh_owned_loot_summary_like_cpp(guid);
    }

    fn next_represented_loot_object_guid_like_cpp(
        &mut self,
        owner: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let canonical = self.owner.next_canonical_loot_object_guid_like_cpp(owner);
        if self.consumer_test {
            canonical.or_else(|| {
                (!owner.is_empty()).then(|| {
                    ObjectGuid::create_world_object(
                        wow_core::guid::HighGuid::LootObject,
                        0,
                        owner.realm_id(),
                        owner.map_id(),
                        0,
                        0,
                        owner.counter(),
                    )
                })
            })
        } else {
            canonical
        }
    }

    /// Typed counterpart of
    /// [`Self::prepare_owned_loot_authority_for_active_request_like_cpp`]. The
    /// first-generation bridge below is unchanged; only the caller can tell
    /// "no authority" from "the reconciliation did not converge".
    pub(super) fn prepare_owned_loot_authority_for_active_request_outcome_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        scope_player: ObjectGuid,
    ) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
        let authority = match self.represented_owned_loot_authority_outcome_like_cpp(owner_guid) {
            OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority) => authority,
            // Absent and Unavailable both leave no authority to install.
            other => return other,
        };
        let can_install_first_generation = self.consumer_test
            && authority.is_retired_like_cpp()
            && authority.generation_like_cpp() == 0
            && self.loot.cached_loot_contains_owner_like_cpp(owner_guid)
            && (self.loot.has_active_loot_view_owner_like_cpp(owner_guid)
                || self.loot.is_active_loot_guid(owner_guid));
        if !can_install_first_generation {
            return OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority);
        }

        if owner_guid.is_game_object() {
            let _ = self
                .sync_represented_gameobject_loot_to_canonical_like_cpp(owner_guid, scope_player);
        } else if owner_guid.is_creature_or_vehicle() {
            let _ =
                self.sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, scope_player);
        }

        let authority = match self.represented_owned_loot_authority_outcome_like_cpp(owner_guid) {
            OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority) => authority,
            other => return other,
        };
        if let Some(snapshot) = authority.snapshot_for_player_like_cpp(scope_player) {
            self.loot
                .ensure_active_loot_view_generation_like_cpp(owner_guid, snapshot.generation);
            self.loot
                .insert_active_loot_view_authority_if_absent_like_cpp(owner_guid, &authority);
        }
        OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority)
    }

    pub(super) fn sync_represented_gameobject_loot_to_canonical_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<()> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(gameobject_guid)
        else {
            return (self.consumer_test
                && self
                    .loot
                    .cached_loot_contains_owner_like_cpp(gameobject_guid))
            .then_some(());
        };
        let loot = self
            .loot
            .cached_loot_for_owner_like_cpp(gameobject_guid)?
            .clone();
        let is_personal = self.loot.is_personal_loot_owner_like_cpp(gameobject_guid);
        let (shared, personal) = self.represented_loot_authority_pools_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            is_personal,
        )?;
        let installed = authority
            .initialize_pristine_like_cpp(shared, personal)
            .installed();
        if !installed
            && authority
                .snapshot_for_player_like_cpp(player_guid)
                .is_none()
        {
            self.loot
                .remove_cached_loot_for_owner_like_cpp(gameobject_guid);
            self.loot
                .remove_cached_loot_generation_like_cpp(gameobject_guid);
            return None;
        }
        self.refresh_owned_loot_summary_like_cpp(gameobject_guid);
        let _ = self.reconcile_represented_loot_cache_like_cpp(gameobject_guid, player_guid);
        Some(())
    }

    pub(super) fn sync_represented_creature_loot_to_canonical_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        _player_guid: ObjectGuid,
    ) -> Option<()> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(creature_guid) else {
            return (self.consumer_test
                && self.loot.cached_loot_contains_owner_like_cpp(creature_guid))
            .then_some(());
        };
        let loot = self
            .loot
            .cached_loot_for_owner_like_cpp(creature_guid)?
            .clone();
        let is_personal = self.loot.is_personal_loot_owner_like_cpp(creature_guid);
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
            self.loot
                .remove_cached_loot_for_owner_like_cpp(creature_guid);
            self.loot
                .remove_cached_loot_generation_like_cpp(creature_guid);
            return None;
        }
        self.refresh_owned_loot_summary_like_cpp(creature_guid);
        let _ = self.reconcile_represented_loot_cache_like_cpp(creature_guid, _player_guid);
        Some(())
    }

    pub fn reconcile_represented_loot_cache_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) else {
            return false;
        };
        let Some(snapshot) = authority.snapshot_for_player_like_cpp(player_guid) else {
            self.loot
                .discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            return false;
        };
        self.loot
            .cache_represented_owned_loot_snapshot_like_cpp(owner_guid, player_guid, snapshot);
        true
    }

    pub fn represented_loot_authority_pools_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        personal: bool,
    ) -> Option<(Option<CreatureLoot>, HashMap<ObjectGuid, CreatureLoot>)> {
        if !personal {
            return Some((Some(loot), HashMap::new()));
        }

        let mut looters = loot.allowed_looters.clone();
        if looters.is_empty() && !player_guid.is_empty() {
            looters.push(player_guid);
        }
        looters.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        looters.dedup();

        let mut personal_loot = HashMap::new();
        for (index, looter) in looters.into_iter().enumerate() {
            let mut pool = loot.clone();
            if index != 0 {
                pool.loot_guid = self.next_represented_loot_object_guid_like_cpp(owner_guid)?;
            }
            pool.coins = self
                .loot
                .personal_loot_money_for_owner_and_player_like_cpp(owner_guid, looter)
                .copied()
                .unwrap_or(0);
            pool.allowed_looters = vec![looter];
            pool.players_looting.retain(|viewer| *viewer == looter);
            pool.items.retain(|entry| {
                entry.allowed_looters.is_empty() || entry.allowed_looters.contains(&looter)
            });
            for entry in &mut pool.items {
                entry.allowed_looters = vec![looter];
            }
            rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(&mut pool);
            personal_loot.insert(looter, pool);
        }

        Some((None, personal_loot))
    }
}
