use super::*;

impl WorldSession {
    pub(super) fn represented_pet_position_like_cpp(&self, pet_guid: ObjectGuid) -> Option<Position> {
        let map_id = u32::from(self.player_map_id_like_cpp());
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let managed = manager.find_map(map_id, instance_id)?;
        managed.map().with_world_object_by_kinds_like_cpp(
            pet_guid,
            &[AccessorObjectKind::Pet],
            |object| object.position(),
        )
    }
    pub(super) fn send_represented_pet_spline_speed_like_cpp(
        &self,
        pet_guid: ObjectGuid,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let Some(opcode) =
            crate::session::creature_movement_spline_speed_opcode_like_cpp(move_type)
        else {
            return;
        };
        let packet_bytes = wow_packet::packets::movement::MoveSplineSetSpeed {
            opcode,
            mover_guid: pet_guid,
            speed: PLAYER_BASE_MOVE_SPEED_LIKE_CPP[move_type.index()] * rate,
        }
        .to_bytes();
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        if self.client_visible_guids_like_cpp.contains(&pet_guid)
            && self.send_tx().send(packet_bytes.clone()).is_err()
        {
            warn!("Send channel closed for account {}", self.account_id);
        }

        let (Some(player_guid), Some(registry)) = (self.player_guid(), self.player_registry())
        else {
            return;
        };
        let Some(source_position) = self
            .represented_pet_position_like_cpp(pet_guid)
            .or_else(|| self.player_position_like_cpp())
        else {
            return;
        };
        for registration in registry.movement_recipients_within_range(
            player_guid,
            map_id,
            instance_id,
            source_position,
            crate::map_manager::VISIBILITY_RADIUS,
        ) {
            let _ = registry.try_send_current_command(
                registration,
                SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                    queued_at: Instant::now(),
                    source_guid: pet_guid,
                    map_id,
                    instance_id,
                    packet_bytes: packet_bytes.clone(),
                }),
            );
        }
    }
    pub(in crate::session) fn propagate_represented_player_speed_to_pet_like_cpp(
        &mut self,
        move_type: UnitMoveTypeLikeCpp,
        rate: f32,
    ) {
        let Some(pet_guid) = self.player_pet_guid_state_like_cpp().flatten() else {
            return;
        };
        if self.resolved_in_combat_like_cpp() != Some(false) {
            return;
        }

        let rate = rate.max(0.01);
        let index = move_type.index();
        let canonical_changed = self.with_canonical_pet_mut_like_cpp(pet_guid, |pet| {
            let unit = pet.creature_mut().unit_mut();
            if unit.speed_rate_at_like_cpp(index) == Some(rate) {
                return false;
            }
            unit.set_speed_rate_at_like_cpp(index, rate)
        });
        let changed = match canonical_changed {
            Some(changed) => changed,
            None => {
                #[cfg(test)]
                {
                    if self.player_handle_like_cpp.is_none() {
                        if self.represented_pet_movement_speed_rates_like_cpp[index] == rate {
                            return;
                        }
                        self.represented_pet_movement_speed_rates_like_cpp[index] = rate;
                        true
                    } else {
                        false
                    }
                }
                #[cfg(not(test))]
                {
                    false
                }
            }
        };
        if !changed {
            return;
        }
        #[cfg(test)]
        {
            self.represented_pet_speed_propagations_like_cpp = self
                .represented_pet_speed_propagations_like_cpp
                .saturating_add(1);
        }
        self.send_represented_pet_spline_speed_like_cpp(pet_guid, move_type, rate);
    }
    #[cfg(test)]
    pub(crate) fn represented_pet_movement_speed_rate_like_cpp(
        &self,
        move_type: UnitMoveTypeLikeCpp,
    ) -> f32 {
        let canonical = self
            .player_pet_guid_state_like_cpp()
            .flatten()
            .and_then(|pet_guid| {
                self.with_canonical_pet_like_cpp(pet_guid, |pet| {
                    pet.creature()
                        .unit()
                        .speed_rate_at_like_cpp(move_type.index())
                })
                .flatten()
            });
        canonical.unwrap_or(self.represented_pet_movement_speed_rates_like_cpp[move_type.index()])
    }
    #[cfg(test)]
    pub(crate) fn represented_pet_speed_propagations_like_cpp(&self) -> u32 {
        self.represented_pet_speed_propagations_like_cpp
    }
}
