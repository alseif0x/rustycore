//! The single complete creature generation/ownership algorithm for both routes.
use super::*;

impl WorldSession {
    pub(super) async fn ensure_creature_loot(
        &mut self,
        creature_guid: ObjectGuid,
        loot_owner_guid: ObjectGuid,
        level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
        allowed_looters: &[ObjectGuid],
        expected_loot_lifecycle_revision: u64,
        creature_owner: Option<&wow_map::manager::CreatureLootAccess<wow_map::manager::CreatureLootActorHandle>>,
        mut original: OriginalMeleeLoot<'_>,
    ) -> Result<(), MeleeLootError> {
        let authority = match original.as_ref() {
            Some((_, operation)) => Some(operation.authority().clone()),
            None => self.represented_owned_loot_authority_like_cpp(creature_guid),
        };
        if authority.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
            self.loot_table.remove(&creature_guid);
            return Ok(());
        }
        let mut retired_object_generation = None;
        if let Some(authority) = authority.as_ref() {
            #[cfg(test)]
            if original.is_none() && authority.is_pristine_like_cpp() && self.loot_table.contains_key(&creature_guid) {
                if !self
                    .represented_personal_loot_owners
                    .contains(&creature_guid)
                    && let Some(loot) = self.loot_table.get_mut(&creature_guid)
                {
                    prepare_represented_shared_creature_loot_generation_like_cpp(
                        loot,
                        allowed_looters,
                    );
                }
                if self
                    .sync_represented_creature_loot_to_canonical_like_cpp(
                        creature_guid,
                        loot_owner_guid,
                    )
                    .is_some()
                {
                    // Legacy packet fixtures pre-populate the former session
                    // cache. Install that value once into the typed object-owned
                    // authority instead of silently replacing it with generated
                    // empty loot. This branch does not exist in production.
                    return Ok(());
                }
            }
            let snapshot = self
                .player_guid()
                .and_then(|player_guid| authority.snapshot_for_player_like_cpp(player_guid))
                .or_else(|| authority.snapshot_for_player_like_cpp(loot_owner_guid));
            if let Some(snapshot) = snapshot {
                let cache_player = match snapshot.scope {
                    OwnedLootScope::Personal(player_guid) => player_guid,
                    OwnedLootScope::Shared => loot_owner_guid,
                };
                self.cache_represented_owned_loot_snapshot_like_cpp(
                    creature_guid,
                    cache_player,
                    snapshot,
                );
                return Ok(());
            }
            if !authority.is_retired_like_cpp() {
                self.loot_table.remove(&creature_guid);
                return Ok(());
            }
            retired_object_generation = Some(match original.as_ref() {
                Some((_, operation)) => operation.object_generation(),
                None => authority.generation_like_cpp(),
            });
            self.loot_table.remove(&creature_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&creature_guid);
        }

        let map_is_dungeon = self.current_map_dungeon_state_like_cpp();
        let connected_tappers =
            self.represented_connected_creature_tappers_like_cpp(allowed_looters);

        // C++ `Unit::Kill` has three distinct ownership shapes:
        // - overworld: one independently generated personal pool per tapper;
        // - dungeon encounter/boss: one independent, lockout-filtered pool per
        //   tapper (`GenerateDungeonEncounterPersonalLoot`);
        // - dungeon trash: exactly one personal pool, keyed by the group's
        //   selected looter (or the first tapper without a group).
        if map_is_dungeon == Some(false)
            || (map_is_dungeon == Some(true) && dungeon_encounter_id != 0)
        {
            let personal_tappers = connected_tappers
                .into_iter()
                .filter(|tapper| {
                    dungeon_encounter_id == 0
                        || self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
                            *tapper,
                            dungeon_encounter_id,
                        )
                })
                .collect::<Vec<_>>();
            if personal_tappers.is_empty() {
                self.loot_table.remove(&creature_guid);
                return Ok(());
            }

            let personal = match self
                .generate_creature_personal_loot(
                    creature_guid,
                    level,
                    entry,
                    loot_id,
                    gold_min,
                    gold_max,
                    dungeon_encounter_id,
                    &personal_tappers,
                    original.as_mut().map(|(tick, operation)| (*tick, &mut **operation)),
                )
                .await {
                Ok(generated) => generated,
                Err(error) if original.is_some() => return Err(error),
                Err(_) => return Ok(()),
            };
            let cache_player = self
                .player_guid()
                .filter(|player_guid| personal.contains_key(player_guid))
                .unwrap_or(personal_tappers[0]);

            if let (Some(authority), Some(expected_generation)) =
                (authority.as_ref(), retired_object_generation)
            {
                if self.install_creature_generation_loot(
                    creature_owner,
                    creature_guid,
                    authority,
                    expected_generation,
                    expected_loot_lifecycle_revision,
                    None,
                    personal,
                    original.as_mut().map(|(tick, operation)| (*tick, &mut **operation)),
                )? {
                    let _ =
                        self.reconcile_creature_generation_cache(creature_guid, cache_player,
                            authority, original.is_some());
                } else {
                    self.loot_table.remove(&creature_guid);
                }
            } else if represented_local_loot_fixture_allowed_like_cpp()
                && let Some(pool) = personal.get(&cache_player).cloned()
            {
                self.loot_table.insert(creature_guid, pool);
            }
            return Ok(());
        }

        if map_is_dungeon == Some(true) {
            if connected_tappers.is_empty() {
                self.loot_table.remove(&creature_guid);
                return Ok(());
            }
            let selected_looter =
                self.represented_dungeon_trash_looter_like_cpp(&connected_tappers);
            let personal = match self
                .generate_creature_personal_loot(
                    creature_guid,
                    level,
                    entry,
                    loot_id,
                    gold_min,
                    gold_max,
                    0,
                    &[selected_looter],
                    original.as_mut().map(|(tick, operation)| (*tick, &mut **operation)),
                )
                .await {
                Ok(generated) => generated,
                Err(error) if original.is_some() => return Err(error),
                Err(_) => return Ok(()),
            };
            let has_loot = personal
                .get(&selected_looter)
                .is_some_and(|loot| !loot_is_looted_like_cpp(loot));

            if let (Some(authority), Some(expected_generation)) =
                (authority.as_ref(), retired_object_generation)
            {
                if self.install_creature_generation_loot(
                    creature_owner,
                    creature_guid,
                    authority,
                    expected_generation,
                    expected_loot_lifecycle_revision,
                    None,
                    personal,
                    original.as_mut().map(|(tick, operation)| (*tick, &mut **operation)),
                )? {
                    let _ = self
                        .reconcile_creature_generation_cache(creature_guid, selected_looter,
                            authority, original.is_some());
                    if has_loot {
                        self.advance_represented_dungeon_trash_looter_like_cpp(&connected_tappers);
                    }
                } else {
                    self.loot_table.remove(&creature_guid);
                }
            } else if represented_local_loot_fixture_allowed_like_cpp()
                && let Some(pool) = personal.get(&selected_looter).cloned()
            {
                self.loot_table.insert(creature_guid, pool);
            }
            return Ok(());
        }

        // Missing Map.db2 metadata is not proof of either overworld or
        // dungeon. Preserve the represented shared fallback for legacy test
        // fixtures, but still bind its async install to the exact death token.
        if !self.loot_table.contains_key(&creature_guid) {
            let mut loot = match self
                .generate_creature_loot(
                    creature_guid,
                    loot_owner_guid,
                    level,
                    entry,
                    loot_id,
                    gold_min,
                    gold_max,
                    dungeon_encounter_id,
                    original.as_mut().map(|(tick, operation)| (*tick, &mut **operation)),
                )
                .await {
                Ok(generated) => generated,
                Err(error) if original.is_some() => return Err(error),
                Err(_) => return Ok(()),
            };
            prepare_represented_shared_creature_loot_generation_like_cpp(
                &mut loot,
                allowed_looters,
            );
            if let (Some(authority), Some(expected_generation)) =
                (authority.as_ref(), retired_object_generation)
            {
                if self.install_creature_generation_loot(
                    creature_owner,
                    creature_guid,
                    authority,
                    expected_generation,
                    expected_loot_lifecycle_revision,
                    Some(loot),
                    HashMap::new(),
                    original.as_mut().map(|(tick, operation)| (*tick, &mut **operation)),
                )? {
                    let _ = self
                        .reconcile_creature_generation_cache(creature_guid, loot_owner_guid,
                            authority, original.is_some());
                } else {
                    self.loot_table.remove(&creature_guid);
                }
            } else if represented_local_loot_fixture_allowed_like_cpp() {
                self.loot_table.insert(creature_guid, loot);
            }
        }
        Ok(())
    }

}
