//! Represented health and power changes and their regeneration.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Test fixtures created before #578 may inject a typed Player directly
    /// into a synthetic MapManager without installing its owner handle. Keep
    /// that compatibility outside production; an existing stale handle never
    /// falls back to GUID lookup.
    pub(in crate::session) fn with_owned_player_mut_for_power_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.mutate_canonical_player_like_cpp(f);
        }
        self.with_owned_player_mut_like_cpp(f)
    }
    pub(crate) fn sync_canonical_player_primary_power_like_cpp(
        &mut self,
        power_type: PowerType,
        current: i32,
        max: i32,
        base_mana: i32,
    ) -> bool {
        let synced = self
            .with_owned_player_mut_for_power_like_cpp(|player| {
                for raw_power in 0..=25 {
                    player.set_power_index(power_type_from_u8_like_cpp(raw_power), None);
                }
                player.set_power_index(power_type, Some(0));
                player.unit_mut().set_display_power(power_type);
                player.unit_mut().set_create_mana_like_cpp(base_mana.max(0));
                player.unit_mut().set_max_power(power_type, max.max(0));
                player.unit_mut().set_power(power_type, current.max(0));
            })
            .is_some();
        #[cfg(test)]
        if synced || self.player_handle_like_cpp.is_none() {
            self.represented_player_base_mana_like_cpp = base_mana.max(0);
            self.set_represented_player_power_slot_like_cpp(0, current, Some(max));
        }
        synced
    }
    pub(crate) fn sync_canonical_player_primary_power_max_like_cpp(
        &mut self,
        power_type: PowerType,
        max: i32,
        base_mana: i32,
    ) -> Option<(i32, i32)> {
        let result = self.with_owned_player_mut_for_power_like_cpp(|player| {
            if player.unit().get_power_index(power_type).is_none() {
                player.set_power_index(power_type, Some(0));
            }
            player.unit_mut().set_display_power(power_type);
            player.unit_mut().set_create_mana_like_cpp(base_mana.max(0));
            // C++ `Unit::SetMaxPower` updates max and clamps current if needed.
            player.unit_mut().set_max_power(power_type, max.max(0));
            (
                player.unit().get_power(power_type),
                player.unit().get_max_power(power_type),
            )
        });
        #[cfg(test)]
        if let Some((current, max)) = result.or_else(|| {
            (self.player_handle_like_cpp.is_none()).then_some((
                self.represented_player_powers_like_cpp[0].unwrap_or(0),
                max.max(0),
            ))
        }) {
            self.represented_player_base_mana_like_cpp = base_mana.max(0);
            self.set_represented_player_power_slot_like_cpp(0, current, Some(max));
        }
        result
    }
    pub(crate) fn sync_canonical_player_max_health_like_cpp(
        &mut self,
        max_health: u32,
    ) -> Option<(u32, u32)> {
        let max_health = max_health.max(1);
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            // C++ `Unit::SetMaxHealth` updates max and clamps current only if needed.
            player.unit_mut().set_max_health(u64::from(max_health));
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
            )
        });
        #[cfg(test)]
        let result = canonical.or_else(|| {
            if self.player_handle_like_cpp.is_some() {
                return None;
            }
            self.mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().set_max_health(u64::from(max_health));
                (
                    player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                    player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
                )
            })
            .or_else(|| Some((self.player_health_like_cpp.min(max_health), max_health)))
        });
        #[cfg(not(test))]
        let result = canonical;
        #[cfg(test)]
        if let Some((current, max)) = result {
            self.player_health_like_cpp = current;
            self.player_max_health_like_cpp = max;
            self.player_alive_like_cpp = current > 0;
        }
        result
    }
    pub(crate) fn sync_canonical_player_health_like_cpp(
        &mut self,
        health: u32,
        max_health: u32,
    ) -> Option<(u32, u32)> {
        let max_health = max_health.max(1);
        let health = health.min(max_health);
        let canonical = self.with_owned_player_mut_like_cpp(|player| {
            if health == 0 {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Corpse);
            } else if matches!(
                player.unit().death_state(),
                wow_constants::DeathState::JustDied | wow_constants::DeathState::Corpse
            ) {
                player
                    .unit_mut()
                    .set_death_state(wow_constants::DeathState::Alive);
            }
            player.unit_mut().set_max_health(u64::from(max_health));
            player.unit_mut().set_health(u64::from(health));
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
            )
        });
        #[cfg(test)]
        let result = canonical.or_else(|| {
            if self.player_handle_like_cpp.is_some() {
                return None;
            }
            self.mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().set_death_state(if health == 0 {
                    wow_constants::DeathState::Corpse
                } else {
                    wow_constants::DeathState::Alive
                });
                player.unit_mut().set_max_health(u64::from(max_health));
                player.unit_mut().set_health(u64::from(health));
                (
                    player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                    player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
                )
            })
        });
        #[cfg(not(test))]
        let result = canonical;
        #[cfg(test)]
        {
            let (current, max) = result.unwrap_or((health, max_health));
            self.player_health_like_cpp = current;
            self.player_max_health_like_cpp = max;
            self.player_alive_like_cpp = current > 0;
        }
        result
    }
    pub(crate) fn canonical_player_power_snapshot_like_cpp(
        &self,
        power_type: PowerType,
    ) -> Option<(i32, i32)> {
        self.canonical_player_snapshot_like_cpp(|player| {
            (
                player.unit().get_power(power_type),
                player.unit().get_max_power(power_type),
            )
        })
    }
    pub(crate) fn canonical_player_health_snapshot_like_cpp(&self) -> Option<(u32, u32)> {
        self.canonical_player_snapshot_like_cpp(|player| {
            (
                player.unit().data().health.min(u64::from(u32::MAX)) as u32,
                player.unit().data().max_health.min(u64::from(u32::MAX)) as u32,
            )
        })
    }
    pub(in crate::session) fn send_player_health_values_update_like_cpp(
        &self,
        guid: ObjectGuid,
        health: u64,
    ) {
        let mut mask = UpdateMask::new(UNIT_DATA_HEALTH_BIT + 1);
        mask.set(UNIT_DATA_HEALTH_BIT);
        let update = wow_entities::PlayerValuesUpdate {
            changed_object_type_mask: 0,
            object_data: None,
            unit_data: Some(UnitDataUpdate {
                mask,
                values: UnitDataValues {
                    health,
                    ..Default::default()
                },
            }),
            player_data: None,
            active_player_data: None,
        };

        if let Some(packet) =
            player_values_update_to_update_object(guid, self.player_map_id_like_cpp(), &update)
        {
            self.send_packet(&packet);
        }
    }
    pub(in crate::session) fn send_player_health_update_like_cpp(
        &self,
        guid: ObjectGuid,
        health: u64,
    ) {
        self.send_packet(&wow_packet::packets::combat::HealthUpdate {
            guid,
            health: health.min(i64::MAX as u64) as i64,
        });
    }
    #[cfg(test)]
    pub fn set_power_type_store(&mut self, store: Arc<PowerTypeStore>) {
        self.power_type_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn set_represented_player_power_slot_like_cpp(
        &mut self,
        slot: usize,
        current: i32,
        max: Option<i32>,
    ) {
        if slot >= MAX_POWERS_PER_CLASS {
            return;
        }
        self.represented_player_powers_like_cpp[slot] = Some(current.max(0));
        if let Some(max) = max {
            self.represented_player_max_powers_like_cpp[slot] = Some(max.max(0));
        }
    }
    pub(crate) fn represented_player_power_values_like_cpp(
        &self,
    ) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
        let canonical = self.resolved_player_power_values_like_cpp();
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return character_power_snapshot_values_like_cpp(
                &self.represented_player_powers_like_cpp,
            );
        }
        canonical
    }
    fn resolved_player_power_values_like_cpp(&self) -> Option<[i32; MAX_POWERS_PER_CLASS]> {
        let canonical = self.with_owned_player_like_cpp(|player| player.unit().data().power);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return self.mutate_canonical_player_like_cpp(|player| player.unit().data().power);
        }
        canonical
    }
    pub(in crate::session) fn resolved_player_power_snapshot_like_cpp(
        &self,
    ) -> Option<CharacterPowerSnapshotLikeCpp> {
        let canonical = self
            .resolved_player_power_values_like_cpp()
            .map(loaded_character_power_snapshot_like_cpp);
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_player_powers_like_cpp);
        }
        canonical
    }
    pub(crate) fn set_player_health_like_cpp(&mut self, health: u32, max_health: u32) {
        let _ = self.sync_canonical_player_health_like_cpp(health, max_health);
        self.sync_player_registry_state_like_cpp();
    }
    #[cfg(test)]
    pub(crate) fn player_health_like_cpp(&self) -> u32 {
        self.resolved_player_vitals_like_cpp().unwrap().0
    }
    #[cfg(test)]
    pub(crate) fn player_max_health_like_cpp(&self) -> u32 {
        self.resolved_player_vitals_like_cpp().unwrap().1
    }
}
