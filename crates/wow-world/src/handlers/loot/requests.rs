// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot window open/close requests and the represented loot cache.

use super::*;
use wow_entities::ItemObjectUpdateLikeCpp;
use wow_packet::ClientPacket;

mod context;
mod item_storage;
#[cfg(test)]
#[path = "requests/test_support.rs"]
mod test_support;

impl WorldSession {
    pub(super) async fn represented_loot_response_for_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        ae_looting: bool,
    ) -> Option<LootResponse> {
        let creature = self.represented_creature_loot_state_like_cpp(owner_guid)?;
        if !creature.tappers.is_empty() && !creature.tappers.contains(&player_guid) {
            return None;
        }
        // `Player::isAllowedToLoot` reads `Creature::GetLootForPlayer`; the
        // client request is not a generation trigger. A retired/missing
        // authority therefore means there is no loot response.
        if !self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid) {
            self.loot_table.remove(&owner_guid);
            return None;
        }

        let loot = self.loot_table.get(&owner_guid)?;
        if !self.represented_loot_can_be_opened_by_player_like_cpp(owner_guid, loot, player_guid) {
            return None;
        }

        Some(LootResponse {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: self.represented_loot_money_for_player_like_cpp(owner_guid, loot, player_guid),
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting,
        })
    }

    pub(super) fn represented_on_loot_opened_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        mut response: LootResponse,
    ) {
        let authority = self
            .prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid)
            .filter(|authority| {
                authority
                    .snapshot_for_player_like_cpp(player_guid)
                    .is_some()
            });
        let authoritative_open = if let Some(authority) = authority.as_ref() {
            match authority.try_open_view_with_snapshot_like_cpp(
                player_guid,
                |snapshot, outcome| {
                    // Enqueue the response while the authority mutex still
                    // excludes item/money commit. Any later commit therefore
                    // observes this viewer and its removal packet is ordered
                    // after the response on this session's send queue.
                    response.loot_obj = snapshot.loot.loot_guid;
                    response.acquire_reason =
                        loot_type_for_client_like_cpp(snapshot.loot.loot_type);
                    response.loot_method = snapshot.loot.loot_method;
                    response.coins = snapshot.loot.coins;
                    response.items =
                        represented_loot_response_items_like_cpp(&snapshot.loot, player_guid);

                    // `flume::Sender::send` may wait indefinitely while this
                    // lock is held. Reject a saturated/disconnected socket
                    // queue immediately; the authority method rolls back its
                    // tentative viewer and first-open mutations before unlock.
                    if !self.try_send_packet(&response) {
                        return None;
                    }

                    // Session mirrors become observable only after the client
                    // response was accepted by its ordered send queue.
                    self.loot_table.insert(owner_guid, snapshot.loot.clone());
                    self.represented_loot_cache_generations_like_cpp
                        .insert(owner_guid, snapshot.generation);
                    self.active_loot_view_generations_like_cpp
                        .insert(owner_guid, outcome.generation);
                    self.active_loot_view_authorities_like_cpp
                        .insert(owner_guid, authority.clone());
                    Some(())
                },
            ) {
                Ok((outcome, ())) => Some(outcome),
                Err(LootClaimError::ResponseEnqueueFailed) => {
                    // Do not attempt a blocking release on the same saturated
                    // queue. The client never observed this view, so dropping
                    // every local mirror is the closed state.
                    self.discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
                    self.clear_active_loot_guid_if(owner_guid);
                    return;
                }
                Err(_) => None,
            }
        } else {
            None
        };
        if authoritative_open.is_none() {
            if (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
                && !represented_local_loot_fixture_allowed_like_cpp()
            {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return;
            }
            self.send_packet(&response);
            self.ensure_represented_player_looting_like_cpp(owner_guid, player_guid);
        } else if let Some(authority) = authority.as_ref() {
            if !self
                .active_loot_view_authorities_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| opened.shares_storage_like_cpp(authority))
            {
                self.active_loot_view_authorities_like_cpp
                    .insert(owner_guid, authority.clone());
            }
        }

        self.represented_notify_loot_list_like_cpp(owner_guid);

        let first_open = match authoritative_open {
            Some(outcome) => outcome.first_viewer,
            None => match self.loot_table.get_mut(&owner_guid) {
                Some(loot) if !loot.looted_by_player => {
                    loot.looted_by_player = true;
                    true
                }
                _ => false,
            },
        };
        if !first_open {
            return;
        }

        let loot_method = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| loot.loot_method)
            .unwrap_or_default();
        match loot_method {
            LOOT_METHOD_GROUP_LIKE_CPP | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP => {
                self.represented_start_group_loot_rolls_on_first_open_like_cpp(
                    item_valuation,
                    owner_guid,
                    player_guid,
                );
            }
            LOOT_METHOD_MASTER_LIKE_CPP => {
                if let Some(packet) =
                    self.represented_master_loot_candidate_list_like_cpp(owner_guid, player_guid)
                {
                    self.send_packet(&packet);
                }
            }
            _ => {}
        }
    }

    /// True only while a request still belongs to the exact object lifetime
    /// whose loot window this session opened.
    pub(super) fn represented_active_loot_generation_matches_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        authority: &OwnedLootAuthority,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let current_generation = authority
            .snapshot_for_player_like_cpp(player_guid)
            .map(|snapshot| snapshot.generation);
        self.active_loot_view_authorities_like_cpp
            .get(&owner_guid)
            .is_some_and(|opened| opened.shares_storage_like_cpp(authority))
            && self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| Some(*opened) == current_generation)
    }

    pub(super) fn ensure_represented_player_looting_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        if let Some(loot) = self.loot_table.get_mut(&owner_guid)
            && !loot.players_looting.contains(&player_guid)
        {
            loot.players_looting.push(player_guid);
        }
    }

    pub(super) fn represented_player_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<ObjectGuid> {
        self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
            player_guid,
            dungeon_encounter_id,
        )
        .then_some(player_guid)
    }

    pub(super) fn represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> bool {
        if let Some(locked) =
            self.player_is_locked_to_dungeon_encounter_like_cpp(player_guid, dungeon_encounter_id)
        {
            return !locked;
        }
        #[cfg(test)]
        {
            return !self
                .represented_locked_dungeon_encounters
                .contains(&(player_guid, dungeon_encounter_id));
        }
        #[cfg(not(test))]
        {
            // C++ has a valid Player, DungeonEncounter row and InstanceLockMgr
            // here. Missing authority is indeterminate and must not grant loot.
            false
        }
    }

    pub(super) fn active_loot_owner_for_loot_object_like_cpp(
        &self,
        loot_object: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let active_owners: Vec<ObjectGuid> = if self.active_loot_view_owners.is_empty() {
            vec![self.active_loot_guid]
        } else {
            self.active_loot_view_owners.iter().copied().collect()
        };

        active_owners.into_iter().find(|owner_guid| {
            !owner_guid.is_empty()
                && self
                    .loot_table
                    .get(owner_guid)
                    .is_some_and(|loot| loot.loot_guid == loot_object)
        })
    }

    pub(super) fn canonical_map_object_position_for_loot_like_cpp(
        &self,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
    ) -> Option<wow_core::Position> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?.map();
        map.map_object_by_kind(guid, allowed)
            .map(|object| object.position())
    }

    pub(super) fn represented_spell_max_range_like_cpp(&self, spell_id: i32) -> Option<f32> {
        let spell_store = self.spell_store()?;
        let spell_misc_store = self.spell_catalogs.spell_misc_store()?;
        let spell_range_store = self.spell_catalogs.spell_range_store()?;
        spell_store.get(spell_id)?;
        let spell_id = u32::try_from(spell_id).ok()?;
        let range_index = spell_misc_store.get(spell_id)?.range_index;
        let range = spell_range_store.get(u32::from(range_index))?;
        Some(range.range_max[1].max(range.range_max[0]))
    }

    /// Mirrors the observable side of C++ `Loot::~Loot`: once the exact
    /// object-owned allocation behind an open view is retired, detached, or
    /// replaced, the next session tick releases that stale client window.
    /// Each session owns its socket, so global object destruction is fanned
    /// out cooperatively without holding a map lock across network work.
    pub(crate) fn close_retired_active_loot_windows_like_cpp(&mut self, player_guid: ObjectGuid) {
        let mut stale_owners = self
            .active_loot_view_authorities_like_cpp
            .iter()
            .filter_map(|(owner_guid, authority)| {
                let generation = self
                    .active_loot_view_generations_like_cpp
                    .get(owner_guid)
                    .copied();
                let still_open = generation.is_some_and(|generation| {
                    authority
                        .snapshot_for_player_like_cpp(player_guid)
                        .is_some_and(|snapshot| snapshot.generation == generation)
                });
                (!still_open).then_some(*owner_guid)
            })
            .collect::<Vec<_>>();
        stale_owners.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));

        for owner_guid in stale_owners {
            self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
        }
    }

    pub(super) fn close_stale_active_loot_view_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        self.discard_represented_personal_loot_cache_for_player_like_cpp(owner_guid, player_guid);
        self.send_packet(&SLootRelease {
            loot_obj: owner_guid,
            owner: player_guid,
        });
        self.clear_active_loot_guid_if(owner_guid);
    }

    /// CMSG_LOOT_RELEASE — player closes the loot window.
    ///
    /// C++ `WorldSession::DoLootRelease` creature branch:
    /// `loot->isLooted() && creature->IsFullyLooted()` removes the lootable
    /// dynamic flag and calls `Creature::AllLootRemovedFromCorpse` for a corpse.
    pub async fn handle_loot_release(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootRelease::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad LootRelease: {e}");
                return;
            }
        };

        debug!(account = self.account_id, unit = ?req.unit, "CMSG_LOOT_RELEASE");

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        self.do_loot_release_owner_like_cpp(req.unit, player_guid)
            .await;
    }
}
