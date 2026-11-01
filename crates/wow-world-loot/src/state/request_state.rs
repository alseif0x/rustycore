use super::LootState;
use wow_core::ObjectGuid;
use wow_entities::AccessorObjectKind;
use wow_loot::OwnedLootAuthority;
use wow_packet::packets::loot::SLootRelease;
use wow_world_core::session::{HubMut, HubRef};

impl LootState {
    /// True only while a request still belongs to the exact object lifetime
    /// whose loot window this session opened.
    pub fn represented_active_loot_generation_matches_like_cpp(
        &self,
        hub: HubRef<'_>,
        owner_guid: ObjectGuid,
        authority: &OwnedLootAuthority,
    ) -> bool {
        let Some(player_guid) = hub.core.player_guid() else {
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

    pub fn ensure_represented_player_looting_like_cpp(
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

    pub fn active_loot_owner_for_loot_object_like_cpp(
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

    pub fn canonical_map_object_position_for_loot_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
    ) -> Option<wow_core::Position> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let manager = hub.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?.map();
        map.map_object_by_kind(guid, allowed)
            .map(|object| object.position())
    }

    /// Mirrors the observable side of C++ `Loot::~Loot`: once the exact
    /// object-owned allocation behind an open view is retired, detached, or
    /// replaced, the next session tick releases that stale client window.
    /// Each session owns its socket, so global object destruction is fanned
    /// out cooperatively without holding a map lock across network work.
    pub fn close_retired_active_loot_windows_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        player_guid: ObjectGuid,
    ) {
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
            self.close_stale_active_loot_view_like_cpp(hub, owner_guid, player_guid);
        }
    }

    pub fn close_stale_active_loot_view_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        self.discard_represented_personal_loot_cache_for_player_like_cpp(owner_guid, player_guid);
        hub.core.send_packet(&SLootRelease {
            loot_obj: owner_guid,
            owner: player_guid,
        });
        self.clear_active_loot_guid_if(owner_guid);
    }
}
