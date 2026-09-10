//! Represented effects that act on the session player.
//!
//! Moved out of the Session root under #621. Behaviour is preserved.

use super::*;

impl WorldSession {
    pub(crate) fn loaded_player_visible_items_for_create_like_cpp(
        &self,
    ) -> Option<[(i32, u16, u16); 19]> {
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        for (slot, item) in self.resolved_inventory_items_like_cpp()? {
            if (slot as usize) < 19 {
                visible_items[slot as usize] = self
                    .resolved_inventory_item_object_like_cpp(item.guid)
                    .map(|item_object| {
                        self.loaded_inventory_item_visible_fields_like_cpp(&item_object)
                    })
                    .unwrap_or((item.entry_id as i32, 0u16, 0u16));
            }
        }
        Some(visible_items)
    }
    /// Ask nearby sessions to run the same visibility diff after this player enters.
    ///
    /// C++ adds the new Player to the map before `UpdateVisibilityForPlayer`, so
    /// other players discover it through their normal `m_clientGUIDs` path. Queue
    /// the existing full visibility command instead of sending a raw CREATE that
    /// cannot update the receiver's client-visible GUID set.
    pub(crate) fn notify_other_players_visibility_changed_like_cpp(&self) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(registry) = &self.player_registry else {
            return;
        };
        let Some(pos) = self.player_position_like_cpp() else {
            return;
        };
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let visibility_range = self.player_map_visibility_range_like_cpp(map_id);
        let target_combat_reach = self.represented_visibility_source_combat_reach_like_cpp();
        let mut broadcast_count = 0;

        for recipient in registry.runtime_recipients() {
            if recipient.guid == guid
                || !recipient.is_in_world
                || recipient.map_id != map_id
                || recipient.instance_id != instance_id
            {
                continue;
            }
            if !crate::session_rules::visibility_distance_allows_like_cpp(
                &recipient.position,
                recipient.combat_reach,
                &pos,
                target_combat_reach,
                visibility_range,
            ) {
                continue;
            }

            match registry.request_current_visibility_refresh(
                recipient.registration,
                map_id,
                instance_id,
            ) {
                Ok(()) => {
                    broadcast_count += 1;
                }
                Err(_) => {}
            }
        }

        if broadcast_count > 0 {
            info!(
                "Queued visibility refresh for {:?} on {} nearby players on map {}",
                guid, broadcast_count, map_id
            );
        }
    }
    pub(crate) fn player_current_map_instanceable_like_cpp(&self) -> bool {
        let map_id = u32::from(self.player_map_id_like_cpp());
        self.map_store()
            .and_then(|store| store.get(map_id))
            .is_some_and(|entry| entry.instance_type != wow_data::map::MAP_COMMON)
    }
    pub(in crate::session) fn persist_player_homebind_like_cpp(
        &mut self,
        homebind: RepresentedHomebindLikeCpp,
    ) {
        let (Some(player_guid), Some(port)) = (
            self.player_guid(),
            self.player_lifecycle_port_like_cpp().map(Arc::clone),
        ) else {
            return;
        };
        let guid_counter = player_guid.counter() as u64;
        let request =
            crate::session_rules::player_homebind_update_request_like_cpp(homebind, guid_counter);
        // C++ Player::SetHomebind queues CharacterDatabase.Execute(stmt) on
        // the ordered database worker and immediately sends the bind packets.
        // Send it to one FIFO worker so SQL latency stays off the packet path
        // without allowing an older bind to finish after a newer one.
        let persistence_tx = self
            .homebind_persistence_tx_like_cpp
            .get_or_insert_with(|| {
                let (tx, mut rx) =
                    tokio::sync::mpsc::unbounded_channel::<HomebindPersistenceJobLikeCpp>();
                tokio::spawn(async move {
                    while let Some(job) = rx.recv().await {
                        match job.port.persist_homebind_like_cpp(job.request).await {
                            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
                            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                                warn!(
                                    player_guid = job.guid_counter,
                                    "failed to update represented player homebind: {reason}"
                                );
                            }
                        }
                    }
                });
                tx
            });
        if persistence_tx
            .send(HomebindPersistenceJobLikeCpp {
                port,
                request,
                guid_counter,
            })
            .is_err()
        {
            warn!(
                player_guid = guid_counter,
                "represented homebind database worker stopped"
            );
        }
    }
    pub(in crate::session) fn add_honor_xp_to_current_player_like_cpp(
        &mut self,
        xp: i32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let xp = u32::try_from(xp).ok()?;
        let player_level = self.player_level_like_cpp();
        self.mutate_canonical_player_like_cpp(|player| {
            player
                .add_honor_xp_like_cpp(xp, player_level)
                .then(|| player.values_update(true))
        })
        .flatten()
    }
    pub(in crate::session) fn apply_self_resurrect_effect_like_cpp(
        &mut self,
        damage: i32,
        misc_value: i32,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.player_is_in_world_for_registry_like_cpp() {
            return;
        }

        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                if player.unit().is_alive() {
                    return None;
                }
                let max_health = player
                    .unit()
                    .data()
                    .max_health
                    .clamp(1, u64::from(u32::MAX)) as u32;
                let (health, mana) = if damage < 0 {
                    (damage.saturating_abs() as u32, misc_value.max(0))
                } else {
                    let pct = damage.max(0);
                    (
                        max_health
                            .saturating_mul(u32::try_from(pct).unwrap_or(u32::MAX))
                            .saturating_div(100),
                        player
                            .get_max_power(PowerType::Mana)
                            .max(0)
                            .saturating_mul(pct)
                            / 100,
                    )
                };
                let health = health.min(max_health);
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Alive);
                player.unit_mut().set_health(u64::from(health));
                player.unit_mut().set_power(PowerType::Mana, mana);
                player.unit_mut().set_power(PowerType::Rage, 0);
                let max_energy = player.get_max_power(PowerType::Energy);
                player.unit_mut().set_power(PowerType::Energy, max_energy);
                player.unit_mut().set_power(PowerType::Focus, 0);
                Some((health, Some(player.values_update(true))))
            })
            .flatten();
        #[cfg(test)]
        let outcome = canonical.or_else(|| {
            if self.player_handle_like_cpp.is_some() || self.player_alive_like_cpp {
                return None;
            }
            let max_health = self.player_max_health_like_cpp.max(1);
            let health = if damage < 0 {
                damage.saturating_abs() as u32
            } else {
                max_health
                    .saturating_mul(u32::try_from(damage.max(0)).unwrap_or(u32::MAX))
                    .saturating_div(100)
            }
            .min(max_health);
            self.player_health_like_cpp = health;
            self.player_alive_like_cpp = true;
            Some((health, None))
        });
        #[cfg(not(test))]
        let outcome = canonical;
        let Some((health, values_update)) = outcome else {
            return;
        };
        self.sync_player_registry_state_like_cpp();
        if let Some(values_update) = values_update {
            self.send_player_values_update_like_cpp(&values_update);
        } else {
            self.send_player_health_values_update_like_cpp(player_guid, u64::from(health));
        }
    }
}
