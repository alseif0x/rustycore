//! Real APP generation on the first original occurrence; no driver activation.
use super::*;
use wow_core::guid::HighGuid;
use wow_map::manager::CreatureLootAccess;

impl WorldSession {
    /// The caller retains the original operation across all IO. Neither success
    /// nor failure advances its cursor, runs hooks, or disposes its ROOT slot.
    pub async fn consume_melee_loot(&mut self, tick: &MapObjectTickContinuation,
        prepared: PreparedMeleeKill)
        -> Result<PendingMeleeKills, (MeleeLootError, PendingMeleeKills)> {
        let mut operation = {
            let Some(manager) = self.canonical_map_manager.as_ref() else {
                return Err((MeleeLootError::ManagerUnavailable, prepared.into_pending()));
            };
            let Ok(manager) = manager.lock() else {
                return Err((MeleeLootError::ManagerUnavailable, prepared.into_pending()));
            };
            match manager.prepare_melee_loot(tick, prepared) {
                Ok(operation) => operation,
                Err(error) => return Err(error),
            }
        };
        let guid = operation.target_guid();
        let source = operation.source();
        let Some(loot_owner_guid) = source.tappers().first().copied() else {
            return Ok(operation.into_pending());
        };
        // Copy only the immutable tapper input list, not the Actor or its state.
        // This permits the original operation's private token validation during
        // GUID allocation while the generation algorithm borrows these inputs.
        let tappers = source.tappers().to_vec();
        let (level, entry, loot_id, gold_min, gold_max, encounter, lifetime) = (
            source.level(), source.entry(), source.loot_id(), source.gold_min(),
            source.gold_max(), source.dungeon_encounter_id(), source.loot_lifecycle_revision(),
        );
        // Preserve the existing pre-ensure scope-player reads even though this
        // route uses the captured authority rather than compatibility cache sync.
        let _loot_scope_player_guid = if self.current_map_dungeon_state_like_cpp() == Some(false) {
            let connected = self.represented_connected_creature_tappers_like_cpp(&tappers);
            self.player_guid().filter(|player| connected.contains(player))
                .or_else(|| connected.first().copied()).unwrap_or(loot_owner_guid)
        } else { loot_owner_guid };
        let result = self.ensure_creature_loot(guid, loot_owner_guid, level, entry,
            loot_id, gold_min, gold_max, encounter, &tappers, lifetime,
            None, Some((tick, &mut operation))).await;
        match result {
            Ok(()) => Ok(operation.into_pending()),
            Err(error) => Err((error, operation.into_pending())),
        }
    }

    pub(super) fn next_creature_generation_guid(&mut self, owner: ObjectGuid,
        original: OriginalMeleeLoot<'_>) -> Result<ObjectGuid, MeleeLootError> {
        let Some((tick, operation)) = original else {
            return self.next_represented_loot_object_guid_like_cpp(owner)
                .ok_or(MeleeLootError::GuidUnavailable);
        };
        let manager = self.canonical_map_manager.as_ref()
            .ok_or(MeleeLootError::ManagerUnavailable)?;
        let (key, counter) = manager.lock().map_err(|_| MeleeLootError::ManagerUnavailable)?
            .next_melee_loot_counter(tick, operation)?;
        let map_id = u16::try_from(key.map_id).map_err(|_| MeleeLootError::GuidEncoding)?;
        Ok(ObjectGuid::create_world_object(HighGuid::LootObject, 0,
            self.realm_id(), map_id, 0, 0, counter))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn install_creature_generation_loot(&mut self,
        owner: Option<&CreatureLootAccess<wow_map::manager::CreatureLootActorHandle>>,
        guid: ObjectGuid, authority: &OwnedLootAuthority, generation: u64, lifetime: u64,
        shared: Option<CreatureLoot>, personal: HashMap<ObjectGuid, CreatureLoot>,
        original: OriginalMeleeLoot<'_>) -> Result<bool, MeleeLootError> {
        let Some((tick, operation)) = original else {
            return Ok(self.install_creature_loot_for_owner(
                owner.expect("compatibility generation retains its observed owner"), guid, authority,
                generation, lifetime, shared, personal));
        };
        let result = (|| {
            let manager = self.canonical_map_manager.as_ref()
                .ok_or(MeleeLootError::ManagerUnavailable)?;
            manager.lock().map_err(|_| MeleeLootError::ManagerUnavailable)?
                .install_melee_loot(tick, operation, shared, personal)
        })();
        match result {
            Ok(()) => Ok(true),
            Err(error) => {
                self.loot_table.remove(&guid);
                Err(error)
            }
        }
    }

    pub(super) fn reconcile_creature_generation_cache(&mut self, guid: ObjectGuid,
        player: ObjectGuid, authority: &OwnedLootAuthority, original: bool) -> bool {
        if !original {
            return self.reconcile_represented_loot_cache_like_cpp(guid, player);
        }
        let Some(snapshot) = authority.snapshot_for_player_like_cpp(player) else {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(guid, player);
            return false;
        };
        self.cache_represented_owned_loot_snapshot_like_cpp(guid, player, snapshot);
        true
    }
}
