//! Complete close and lifecycle application for one loot owner.
use super::*;

impl WorldSession {
    pub(crate) async fn do_loot_release_all_like_cpp(&mut self, player_guid: ObjectGuid) {
        self.release_loot_views_operation(player_guid, LootOperationPolicy::Production).await;
    }

    pub(in crate::handlers::loot) async fn release_loot_views_operation(&mut self, player_guid: ObjectGuid, policy: LootOperationPolicy,
    ) {
        let mut active_owners: Vec<ObjectGuid> = self.loot_views.owners().copied().collect();
        if active_owners.is_empty() && !self.loot_views.primary_guid().is_empty() {
            active_owners.push(self.loot_views.primary_guid());
        }
        active_owners.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        for owner_guid in active_owners {
            self.release_loot_owner_operation(owner_guid, player_guid, policy)
                .await;
        }
    }

    pub(in crate::handlers::loot) async fn do_loot_release_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        self.release_loot_owner_operation(owner_guid, player_guid, LootOperationPolicy::Production).await
    }

    pub(in crate::handlers::loot) async fn release_loot_owner_operation(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        policy: LootOperationPolicy,
    ) -> bool {
        if !self.loot_views.contains_owner(&owner_guid)
            && !self.is_active_loot_guid(owner_guid)
        {
            return false;
        }

        let authoritative_release = if let Some(authority) =
            self.prepare_loot_authority_operation(owner_guid, player_guid, policy)
        {
            if !self
                .loot_views
                .authority(&owner_guid)
                .is_some_and(|opened| opened.shares_storage_like_cpp(&authority))
            {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            }
            let Some(active_generation) = self.loot_views.generation(&owner_guid)
            else {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            };
            let Some(close) =
                authority.close_viewer_if_generation_like_cpp(active_generation, player_guid)
            else {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            };
            Some(AuthoritativeLootReleaseLikeCpp {
                authority,
                selected_generation: active_generation,
                loot: close.snapshot.loot,
                whole_object_fully_looted: close.whole_object_fully_looted,
                whole_object_fully_skinned: close.whole_object_fully_skinned,
                object_generation: close.object_generation,
                lifecycle_revision: close.lifecycle_revision,
                require_no_viewers: false,
            })
        } else {
            None
        };

        if authoritative_release.is_none()
            && (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
            && !policy.permits_local_cache(false)
        {
            self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
            return true;
        }

        // C++ `Loot::isLooted()` requires both zero gold and zero remaining
        // player-visible item count.
        let Some(loot) = authoritative_release
            .as_ref()
            .map(|release| &release.loot)
            .or_else(|| self.loot_table.get(&owner_guid))
        else {
            return false;
        };
        let selected_pool_looted = loot_is_looted_like_cpp(loot);
        let represented_loot_type = loot.loot_type;
        let whole_object_fully_looted = if let Some(release) = authoritative_release.as_ref() {
            release.whole_object_fully_looted
        } else if owner_guid.is_game_object() {
            self.canonical_gameobject_fully_looted_after_represented_sync_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
            )
        } else if owner_guid.is_creature_or_vehicle() {
            self.canonical_creature_fully_looted_after_represented_sync_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
            )
        } else {
            selected_pool_looted
        };

        if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
            loot.remove_viewer(player_guid);
        }

        // Acknowledge the release to the client.
        let release = SLootRelease {
            loot_obj: owner_guid,
            owner: player_guid,
        };
        self.send_packet(&release);

        if owner_guid.is_game_object() {
            self.clear_active_loot_guid_if(owner_guid);
            if !self
                .represented_gameobject_can_autostore_loot_item_like_cpp(owner_guid, player_guid)
            {
                if authoritative_release.is_some() {
                    self.discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
                }
                return true;
            }
            self.apply_represented_gameobject_loot_release_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
                whole_object_fully_looted,
                authoritative_release.as_ref(),
            );
            let _ = self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(owner_guid);
            let go_type = self
                .represented_gameobject_use_states
                .get(&owner_guid)
                .and_then(|state| state.go_type)
                .map(u32::from);
            let selected_release_branch = selected_pool_looted
                || matches!(
                    go_type,
                    Some(GAMEOBJECT_TYPE_FISHING_NODE) | Some(GAMEOBJECT_TYPE_FISHING_HOLE)
                );
            if !selected_release_branch {
                if authoritative_release.is_some() {
                    self.discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
                }
                return true;
            }

            self.hide_represented_gameobject_for_player_after_loot_release_like_cpp(owner_guid);
            if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
                self.send_gathering_node_loot_release_dynamic_flags_update_like_cpp(owner_guid);
            }
            if authoritative_release.is_some() {
                self.discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            } else {
                self.loot_table.remove(&owner_guid);
            }
            return true;
        }

        if owner_guid.is_item()
            && matches!(
                represented_loot_type,
                LOOT_TYPE_PROSPECTING_LIKE_CPP | LOOT_TYPE_MILLING_LIKE_CPP
            )
        {
            // C++ always clears the generated Loot and consumes at most five
            // source items for prospecting/milling, even if the window closes
            // before every generated entry was taken.
            self.clear_active_loot_guid_if(owner_guid);
            self.loot_table.remove(&owner_guid);
            let _ = self.apply_inventory_item_object_updates_like_cpp(
                owner_guid,
                &[ItemObjectUpdateLikeCpp::SetLootGenerated(false)],
            );
            self.destroy_direct_item_count_after_loot_release_like_cpp(owner_guid, Some(5))
                .await;
            return true;
        }

        if owner_guid.is_item() && !selected_pool_looted {
            self.clear_active_loot_guid_if(owner_guid);
            let item_has_loot_flag = self
                .resolved_inventory_items_like_cpp()
                .and_then(|items| items.values().find(|item| item.guid == owner_guid).cloned())
                .and_then(|item| self.item_template_flags(item.entry_id))
                .map(|flags| flags.contains(wow_constants::ItemFlags::HAS_LOOT));
            if item_has_loot_flag == Some(false) {
                self.destroy_fully_looted_direct_item(owner_guid).await;
            }
            return true;
        }

        self.clear_active_loot_guid_if(owner_guid);
        let creature_owner = if owner_guid.is_creature_or_vehicle() {
            self.capture_creature_loot_owner(owner_guid)
        } else {
            wow_map::manager::CreatureLootAccess::NoActor
        };

        if !selected_pool_looted {
            let round_robin_released = if let Some(release) = authoritative_release.as_ref() {
                release
                    .authority
                    .clear_round_robin_if_generation_like_cpp(
                        release.selected_generation,
                        player_guid,
                    )
                    .is_some_and(|outcome| {
                        self.loot_table.insert(owner_guid, outcome.snapshot.loot);
                        outcome.cleared
                    })
            } else {
                self.loot_table.get_mut(&owner_guid).is_some_and(|loot| {
                    loot.release_round_robin(player_guid)
                })
            };
            if round_robin_released {
                self.represented_notify_loot_list_like_cpp(owner_guid);
            }
            if owner_guid.is_creature_or_vehicle() {
                let values_update = self.force_creature_loot_flags(&creature_owner, owner_guid);
                if let Some(values_update) = values_update.as_ref() {
                    self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                        owner_guid,
                        values_update,
                        authoritative_release
                            .as_ref()
                            .map(|release| &release.authority),
                    );
                }
            }
            if authoritative_release.is_some() {
                self.discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            }
            return true;
        }

        // Remove loot entry from memory once the represented loot is consumed.
        self.loot_table.remove(&owner_guid);

        if owner_guid.is_item() && selected_pool_looted {
            self.destroy_fully_looted_direct_item(owner_guid).await;
            return true;
        }

        if owner_guid.is_corpse() {
            self.remove_canonical_corpse_lootable_dynamic_flag_like_cpp(owner_guid);
            return true;
        }

        // C++ forces the viewer-dependent DynamicFlags field after every
        // creature release, including a selected personal pool that completed
        // while another pool remains.
        let forced_values_update = self.force_creature_loot_flags(&creature_owner, owner_guid);

        if !whole_object_fully_looted {
            if let Some(values_update) = forced_values_update.as_ref() {
                self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                    owner_guid,
                    values_update,
                    authoritative_release
                        .as_ref()
                        .map(|release| &release.authority),
                );
            }
            if authoritative_release.is_some() {
                self.discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            }
            return true;
        }

        let corpse_decay_looted_rate = self.loot_drop_rates_like_cpp().corpse_decay_looted;

        // Start corpse despawn timer if fully looted.
        let whole_object_fully_skinned = authoritative_release.as_ref().map_or(
            represented_loot_type == LOOT_TYPE_SKINNING_LIKE_CPP,
            |release| release.whole_object_fully_skinned,
        );
        let lifecycle_update = self.release_creature_loot_corpse(
            &creature_owner, owner_guid,
            authoritative_release.as_ref().map(|release| &release.authority),
            authoritative_release.as_ref().map_or(0, |release| release.object_generation),
            authoritative_release.as_ref().map_or(0, |release| release.lifecycle_revision),
            whole_object_fully_skinned, corpse_decay_looted_rate,
            wow_map::map_manager::CreatureLootReleasePhase::Normal,
        );

        if let Some((_, values_update)) = lifecycle_update.as_ref() {
            self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                owner_guid,
                values_update,
                authoritative_release
                    .as_ref()
                    .map(|release| &release.authority),
            );
        }
        let marked = lifecycle_update.and_then(|(marked, _)| marked);

        if let Some((entry, corpse_decay_secs)) = marked {
            info!(
                "Creature {:?} (entry {}) fully looted — despawning in {}s",
                owner_guid, entry, corpse_decay_secs
            );
        }

        if authoritative_release.is_some() {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                owner_guid,
                player_guid,
            );
        }

        true
    }
}
