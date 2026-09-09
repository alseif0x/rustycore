//! Values updates and packets published for represented creatures.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn send_initial_visible_packets_for_creature_like_cpp(
        &self,
        creature: &crate::map_manager::WorldCreature,
    ) {
        let aura_subsystem = &creature.creature.unit().subsystems().auras;
        if aura_subsystem.visible_auras.is_empty() {
            return;
        }

        let mut visible: Vec<_> = aura_subsystem.visible_auras.iter().collect();
        visible.sort_by_key(|(slot, _)| **slot);
        let auras = visible
            .into_iter()
            .map(|(slot, aura_ref)| {
                let active_flags = aura_subsystem
                    .applied_auras
                    .iter()
                    .filter(|applied| applied.aura_ref() == *aura_ref)
                    .fold(0u32, |mask, applied| mask | applied.effect_mask);
                let application = aura_subsystem.visible_aura_applications_like_cpp.get(slot);
                let flags = application.map_or(active_flags, |application| application.flags);
                let points = if flags & AFLAG_SCALABLE_LIKE_CPP != 0 {
                    application
                        .map(|application| {
                            application
                                .effect_amounts
                                .iter()
                                .filter(|effect| {
                                    effect.effect_index < u32::BITS as u8
                                        && active_flags & (1u32 << effect.effect_index) != 0
                                })
                                .map(|effect| effect.amount as f32)
                                .collect()
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };

                wow_packet::packets::misc::AuraInfoLikeCpp {
                    slot: *slot,
                    aura_data: Some(wow_packet::packets::misc::AuraDataInfoLikeCpp {
                        cast_id: ObjectGuid::create_world_object(
                            HighGuid::Cast,
                            3,
                            1,
                            self.player_map_id_like_cpp(),
                            0,
                            aura_ref.spell_id,
                            i64::from(*slot) + 1,
                        ),
                        spell_id: i32::try_from(aura_ref.spell_id).unwrap_or(i32::MAX),
                        flags: flags.min(u32::from(u16::MAX)) as u16,
                        active_flags,
                        caster_guid: aura_ref.caster_guid,
                        cast_level: creature.level().into(),
                        applications: 0,
                        duration_ms: None,
                        remaining_ms: None,
                        points,
                    }),
                }
            })
            .collect();

        self.send_packet(&wow_packet::packets::misc::AuraUpdate::full_for(
            creature.guid(),
            auras,
        ));
    }
    /// Route a creature-originated packet through the existing
    /// `MessageDistDeliverer`-style candidate and per-session visibility gates.
    /// The activating session receives its direct packet separately; this
    /// queues only nearby observers, matching `WorldObject::SendMessageToSet`.
    pub(crate) fn broadcast_creature_packet_to_visible_set_like_cpp(
        &self,
        source_guid: ObjectGuid,
        bytes: Vec<u8>,
    ) {
        self.broadcast_creature_packet_to_visible_set_and_connection_like_cpp(
            source_guid,
            bytes,
            false,
        );
    }
    pub(crate) fn broadcast_creature_packet_to_visible_set_realm_like_cpp(
        &self,
        source_guid: ObjectGuid,
        bytes: Vec<u8>,
    ) {
        self.broadcast_creature_packet_to_visible_set_and_connection_like_cpp(
            source_guid,
            bytes,
            true,
        );
    }
    /// Fan out a creature packet from an interaction snapshot that has
    /// already been validated against the canonical-or-legacy NPC authority.
    /// This keeps observer publication working during the transitional map
    /// split even when the source exists only in the legacy map manager.
    pub(crate) fn broadcast_creature_packet_from_position_to_visible_set_realm_like_cpp(
        &self,
        source_guid: ObjectGuid,
        source_position: Position,
        bytes: Vec<u8>,
    ) {
        self.broadcast_creature_packet_from_position_to_visible_set_and_connection_like_cpp(
            source_guid,
            source_position,
            bytes,
            true,
            true,
        );
    }
    fn broadcast_creature_packet_to_visible_set_and_connection_like_cpp(
        &self,
        source_guid: ObjectGuid,
        bytes: Vec<u8>,
        realm_connection: bool,
    ) {
        let Some(source) = self.canonical_creature_access_like_cpp(source_guid) else {
            return;
        };
        self.broadcast_creature_packet_from_position_to_visible_set_and_connection_like_cpp(
            source_guid,
            source.position,
            bytes,
            realm_connection,
            false,
        );
    }
    fn broadcast_creature_packet_from_position_to_visible_set_and_connection_like_cpp(
        &self,
        source_guid: ObjectGuid,
        source_position: Position,
        bytes: Vec<u8>,
        realm_connection: bool,
        allow_legacy_source_fallback: bool,
    ) {
        let Some(registry) = self.player_registry() else {
            return;
        };
        let player_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let range_sq =
            crate::map_manager::VISIBILITY_RADIUS * crate::map_manager::VISIBILITY_RADIUS;

        let candidates: Vec<_> = registry
            .runtime_recipients()
            .into_iter()
            .filter_map(|recipient| {
                if recipient.guid == player_guid
                    || !recipient.is_in_world
                    || recipient.map_id != map_id
                    || recipient.instance_id != instance_id
                {
                    return None;
                }
                let dx = recipient.position.x - source_position.x;
                let dy = recipient.position.y - source_position.y;
                if dx * dx + dy * dy > range_sq {
                    return None;
                }
                Some(recipient.registration)
            })
            .collect();

        for registration in candidates {
            let command = SendIfVisibleLikeCppCommand {
                queued_at: Instant::now(),
                source_guid,
                map_id,
                instance_id,
                packet_bytes: bytes.clone(),
            };
            let command = if realm_connection && allow_legacy_source_fallback {
                SessionCommand::SendRealmIfVisibleFromLegacySourceLikeCpp(command)
            } else if realm_connection {
                SessionCommand::SendRealmIfVisibleLikeCpp(command)
            } else {
                SessionCommand::SendIfVisibleLikeCpp(command)
            };
            let _ = registry.try_send_current_command(registration, command);
        }
    }
}
