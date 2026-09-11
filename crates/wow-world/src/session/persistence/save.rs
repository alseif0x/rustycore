//! Represented Session-side save operations.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn current_player_save_to_db_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerSaveToDbSnapshotLikeCpp> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.fixture_player_save_to_db_snapshot_like_cpp();
        }
        let guid = self.player_guid()?;
        let handle = self.player_handle_like_cpp?;
        if handle.guid() != guid {
            return None;
        }
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let residence = manager.player_residence_like_cpp(handle)?;
        // C++ Player.cpp:19480-19514 reads one Player and selects a save-only
        // teleport destination. Resolve every mutable input under this same guard.
        // The existing residence-specific health projection remains explicit
        // compatibility debt; map, instance and level come from the Player.
        manager.with_player_like_cpp(handle, |player| {
            self.player_save_header_from_owner_like_cpp(player, residence)
        })
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_player_save_to_db_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerSaveToDbSnapshotLikeCpp> {
        let guid = self.player_guid()?;
        // C++ saves through this session's exact `Player*`. Resolve the raw
        // power array through the generation-checked owner before any spatial
        // lookup so a replacement with the same GUID cannot be persisted by a
        // stale session incarnation.
        let powers = self.resolved_player_power_snapshot_like_cpp()?;
        let xp = self.resolved_player_xp_like_cpp()?;
        let money = self.resolved_player_money_like_cpp()?;
        let pending_teleport_destination = self.pending_teleport_save_destination_like_cpp();
        if let Some(manager) = self.canonical_map_manager.as_ref()
            && let Ok(manager) = manager.lock()
        {
            let mut snapshot = None;
            manager.do_for_all_maps(|managed| {
                if snapshot.is_some() {
                    return;
                }
                let Some(player) = managed.map().get_typed_player(guid) else {
                    return;
                };
                // C++ has one live Player object, and Player::SaveToDB reads a
                // coherent snapshot from that object. Accepted movement now
                // relocates this canonical Player before persistence, so do not
                // recursively resolve a Session mirror while MapManager is held.
                let (map_id, instance_id, position) =
                    if let Some((map_id, position)) = pending_teleport_destination {
                        (map_id, 0, position)
                    } else {
                        (
                            self.player_map_id_like_cpp(),
                            managed.instance_id(),
                            player.unit().world().position(),
                        )
                    };

                let canonical_max_health = player
                    .unit()
                    .data()
                    .max_health
                    .max(1)
                    .min(u64::from(u32::MAX)) as u32;
                let canonical_health = player.unit().data().health.min(u64::from(u32::MAX)) as u32;
                let health = if player.unit().is_alive() && canonical_health > 0 {
                    canonical_health
                } else {
                    0
                };

                snapshot = Some(PlayerSaveToDbSnapshotLikeCpp {
                    guid,
                    map_id,
                    instance_id,
                    position,
                    level: self.player_level_like_cpp(),
                    xp,
                    money,
                    health,
                    max_health: canonical_max_health,
                    powers,
                });
            });
            if snapshot.is_some() {
                return snapshot;
            }
        }

        let (map_id, instance_id, position) =
            if let Some((map_id, position)) = pending_teleport_destination {
                (map_id, 0, position)
            } else {
                (
                    self.player_map_id_like_cpp(),
                    self.current_canonical_player_map_key_like_cpp()
                        .map(|key| key.instance_id)
                        .unwrap_or(0),
                    self.player_position_like_cpp()?,
                )
            };

        let (health, max_health, _) = self.resolved_player_vitals_like_cpp()?;
        Some(PlayerSaveToDbSnapshotLikeCpp {
            guid,
            map_id,
            instance_id,
            position,
            level: self.player_level_like_cpp(),
            xp,
            money,
            health,
            max_health,
            powers,
        })
    }
    /// C++ `Player::_SaveCurrency` plan for changed/new currency rows.
    /// Gameplay owns filtering and state transitions; the persistence adapter
    /// owns statement identity, bind order, and transaction execution.
    pub(crate) fn plan_player_currency_save_like_cpp(
        &self,
        character_guid: u64,
        currencies: &mut HashMap<u32, PlayerCurrency>,
    ) -> wow_persistence::PlayerCurrencySaveRequestLikeCpp {
        let mut rows = Vec::new();
        let Some(store) = self.currency_types_store.as_ref() else {
            return wow_persistence::PlayerCurrencySaveRequestLikeCpp {
                player_guid: character_guid,
                rows,
            };
        };
        for (&currency_id, currency) in currencies.iter_mut() {
            if !store.has_record(currency_id) {
                continue;
            }
            let Ok(currency_db_id) = u16::try_from(currency_id) else {
                continue;
            };

            match currency.state {
                PlayerCurrencyState::New => {
                    rows.push(wow_persistence::PlayerCurrencySaveRowLikeCpp {
                        kind: wow_persistence::PlayerCurrencySaveKindLikeCpp::New,
                        currency_id: currency_db_id,
                        quantity: currency.quantity,
                        weekly_quantity: currency.weekly_quantity,
                        tracked_quantity: currency.tracked_quantity,
                        increased_cap_quantity: currency.increased_cap_quantity,
                        earned_quantity: currency.earned_quantity,
                        flags: currency.flags,
                    });
                    currency.state = PlayerCurrencyState::Unchanged;
                }
                PlayerCurrencyState::Changed => {
                    rows.push(wow_persistence::PlayerCurrencySaveRowLikeCpp {
                        kind: wow_persistence::PlayerCurrencySaveKindLikeCpp::Changed,
                        currency_id: currency_db_id,
                        quantity: currency.quantity,
                        weekly_quantity: currency.weekly_quantity,
                        tracked_quantity: currency.tracked_quantity,
                        increased_cap_quantity: currency.increased_cap_quantity,
                        earned_quantity: currency.earned_quantity,
                        flags: currency.flags,
                    });
                    currency.state = PlayerCurrencyState::Unchanged;
                }
                PlayerCurrencyState::Unchanged | PlayerCurrencyState::Removed => {}
            }
        }
        wow_persistence::PlayerCurrencySaveRequestLikeCpp {
            player_guid: character_guid,
            rows,
        }
    }
    pub(crate) async fn persist_standalone_player_currency_save_like_cpp(
        &mut self,
        character_guid: u64,
        pre_save_snapshot: HashMap<u32, PlayerCurrency>,
    ) -> Result<(), wow_persistence::PersistenceOutcomeLikeCpp> {
        let Some(port) = self.player_lifecycle_port_like_cpp().map(Arc::clone) else {
            return Ok(());
        };
        let Some(mut currencies) = self.player_currencies_like_cpp() else {
            return Err(wow_persistence::PersistenceOutcomeLikeCpp::Failed {
                reason: "canonical Player currency owner is unavailable".to_string(),
            });
        };
        let request = self.plan_player_currency_save_like_cpp(character_guid, &mut currencies);
        if !self.set_player_currencies_like_cpp(currencies) {
            return Err(wow_persistence::PersistenceOutcomeLikeCpp::Failed {
                reason: "canonical Player currency owner became unavailable".to_string(),
            });
        }
        let outcome = port.persist_currency_save_like_cpp(request).await;
        if matches!(
            outcome,
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. }
        ) {
            Ok(())
        } else {
            self.set_player_currencies_like_cpp(pre_save_snapshot);
            Err(outcome)
        }
    }
    /// C++ `CollectionMgr::SaveAccountHeirlooms`.
    pub(crate) fn account_heirloom_save_rows_like_cpp(
        &self,
    ) -> Option<Vec<AccountHeirloomSaveRowLikeCpp>> {
        let bnet_account_id = self.battlenet_account_id();
        Some(
            self.player_collection_state_snapshot_like_cpp()?
                .heirlooms
                .into_iter()
                .map(|(item_id, data)| AccountHeirloomSaveRowLikeCpp {
                    bnet_account_id,
                    item_id,
                    flags: data.flags,
                })
                .collect(),
        )
    }
    /// C++ `CollectionMgr::SaveAccountToys`.
    pub(crate) fn account_toy_save_rows_like_cpp(&self) -> Option<Vec<AccountToySaveRowLikeCpp>> {
        let bnet_account_id = self.battlenet_account_id();
        Some(
            self.player_collection_state_snapshot_like_cpp()?
                .toys
                .into_iter()
                .map(|(item_id, flags)| AccountToySaveRowLikeCpp {
                    bnet_account_id,
                    item_id,
                    is_favorite: (flags & TOY_FLAG_FAVORITE_LIKE_CPP) != 0,
                    has_fanfare: (flags & TOY_FLAG_HAS_FANFARE_LIKE_CPP) != 0,
                })
                .collect(),
        )
    }
    pub fn set_player_save_interval_ms_like_cpp(&mut self, interval_ms: u32) {
        self.player_save_interval_ms_like_cpp = interval_ms;
        self.reset_player_save_timer_like_cpp();
    }
    pub(in crate::session) fn reset_player_save_timer_like_cpp(&mut self) {
        self.next_player_save_ms_like_cpp = self.player_save_interval_ms_like_cpp;
        self.pending_periodic_player_save_like_cpp = false;
    }
    pub(in crate::session) fn update_player_save_timer_like_cpp(&mut self, diff_ms: u32) {
        if self.player_save_interval_ms_like_cpp == 0 || self.next_player_save_ms_like_cpp == 0 {
            return;
        }

        if diff_ms >= self.next_player_save_ms_like_cpp {
            self.next_player_save_ms_like_cpp = 0;
            self.pending_periodic_player_save_like_cpp = true;
        } else {
            self.next_player_save_ms_like_cpp -= diff_ms;
        }
    }
    pub(in crate::session) fn resolved_player_flags_for_rest_state_save_like_cpp(
        &self,
    ) -> Option<u32> {
        let resolve = |mut player_flags: u32, rest: &wow_entities::PlayerRestState| {
            if rest.location_initialized {
                if rest.rest_flag_mask != 0 {
                    player_flags |= PLAYER_FLAGS_RESTING_LIKE_CPP;
                } else {
                    player_flags &= !PLAYER_FLAGS_RESTING_LIKE_CPP;
                }
            }
            player_flags
        };
        let canonical = self.with_owned_player_for_rest_like_cpp(|player| {
            resolve(player.data().player_flags, player.rest_state_like_cpp())
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(resolve(
                self.represented_loaded_player_flags_like_cpp.unwrap_or(0),
                &self.player_rest_state_snapshot_like_cpp()?,
            ));
        }
        canonical
    }
    #[cfg(test)]
    pub(in crate::session) fn represented_player_flags_for_rest_state_save_like_cpp(&self) -> u32 {
        self.resolved_player_flags_for_rest_state_save_like_cpp()
            .unwrap_or_else(|| self.represented_loaded_player_flags_like_cpp.unwrap_or(0))
    }
    pub(in crate::session) async fn process_pending_periodic_player_save_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        if !self.pending_periodic_player_save_like_cpp || self.state != SessionState::LoggedIn {
            return;
        }
        if self.pending_teleport_save_destination_like_cpp().is_some() {
            return;
        }

        self.save_current_player_to_db_with_generator_like_cpp(item_guid_generator)
            .await;
    }
    #[cfg(test)]
    pub(in crate::session) async fn process_pending_periodic_player_save_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        self.process_pending_periodic_player_save_with_generator_like_cpp(generators.item.as_ref())
            .await;
    }
    /// C++ `CollectionMgr::SaveAccountMounts`.
    pub(crate) fn account_mount_save_rows_like_cpp(
        &self,
    ) -> Option<Vec<AccountMountSaveRowLikeCpp>> {
        let bnet_account_id = self.battlenet_account_id();
        let mut rows = self
            .player_collection_state_snapshot_like_cpp()?
            .mounts
            .into_iter()
            .filter_map(|(spell_id, flags)| {
                Some(AccountMountSaveRowLikeCpp {
                    bnet_account_id,
                    mount_spell_id: u32::try_from(spell_id).ok()?,
                    flags,
                })
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.mount_spell_id);
        Some(rows)
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_mark_player_skills_saved_like_cpp(&mut self) {
        let Some(mut records) = self.resolved_player_skill_records_like_cpp() else {
            return;
        };
        let Some(mut tombstones) = self.resolved_player_skill_non_durable_tombstones_like_cpp()
        else {
            return;
        };
        for skill in records.values_mut() {
            if skill.state == RepresentedPlayerSkillStateLikeCpp::Deleted {
                tombstones.insert(skill.skill_id);
            }
            skill.state = RepresentedPlayerSkillStateLikeCpp::Unchanged;
        }
        let occupied = self.complete_player_skill_occupied_slots_like_cpp();
        let _ = self.replace_player_skill_runtime_exact_like_cpp(
            records,
            true,
            occupied.is_some(),
            occupied,
            tombstones,
        );
        self.sync_player_registry_state_like_cpp();
    }
    pub(in crate::session) fn has_complete_player_skill_save_authority_like_cpp(&self) -> bool {
        self.complete_player_skill_records_like_cpp()
            .zip(self.complete_player_skill_occupied_slots_like_cpp())
            .is_some_and(|(skills, occupied_slots)| skills.len() == usize::from(occupied_slots))
    }
    pub(crate) fn represented_save_cuf_profiles_like_cpp(
        &mut self,
        profiles: Vec<wow_packet::packets::misc::CufProfile>,
    ) -> bool {
        if profiles.len() > wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP {
            return false;
        }

        #[cfg(test)]
        let fixture_profiles = profiles.clone();
        let profiles = profiles
            .into_iter()
            .map(player_cuf_profile_from_packet_like_cpp)
            .collect::<Vec<_>>();
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            // C++ `WorldSession::HandleSaveCUFProfiles` saves the sent slots
            // and then empties the rest (MiscHandler.cpp:1115-1119).
            let sent = profiles.len();
            for (slot, profile) in profiles.into_iter().enumerate() {
                player.save_cuf_profile_like_cpp(slot, Some(profile));
            }
            for slot in sent..wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP {
                player.save_cuf_profile_like_cpp(slot, None);
            }
        });
        if canonical.is_some() {
            return true;
        }

        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.cuf_profiles_like_cpp =
                vec![None; wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP];
            for (slot, profile) in fixture_profiles.into_iter().enumerate() {
                self.cuf_profiles_like_cpp[slot] = Some(profile);
            }
            return true;
        }
        false
    }
}
