//! Values updates and packets published for represented creatures.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

/// C++ `AuraApplication::BuildUpdatePacket` for one creature aura slot: the
/// application's `AFLAG` values and, when the aura is scalable, its per-effect
/// point amounts. The full aura update and the single-slot publication both go
/// through this builder so their payloads cannot diverge.
pub(super) fn represented_creature_aura_info_like_cpp(
    aura_subsystem: &wow_entities::AuraSubsystem,
    slot: u8,
    level: u8,
) -> wow_packet::packets::misc::AuraInfoLikeCpp {
    let captured = wow_map::CreatureAuraSlotFacts::capture(aura_subsystem, slot);
    creature_aura_info_from_facts(captured.as_ref(), slot, level)
}

fn creature_aura_info_from_facts(
    captured: Option<&wow_map::CreatureAuraSlotFacts>,
    slot: u8,
    level: u8,
) -> wow_packet::packets::misc::AuraInfoLikeCpp {
    let Some(captured) = captured else {
        return wow_packet::packets::misc::AuraInfoLikeCpp {
            slot,
            aura_data: None,
        };
    };
    let aura_ref = captured.aura_ref();
    let active_flags = captured.matching_effect_masks()
        .iter()
        .fold(0u32, |mask, effect_mask| mask | *effect_mask);
    let application = captured.application();
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
    let provenance = captured.provenance();

    wow_packet::packets::misc::AuraInfoLikeCpp {
        slot,
        aura_data: Some(wow_packet::packets::misc::AuraDataInfoLikeCpp {
            cast_id: provenance.cast_id,
            spell_id: i32::try_from(aura_ref.spell_id).unwrap_or(i32::MAX),
            spell_visual_id: provenance.spell_visual_id,
            flags: flags.min(u32::from(u16::MAX)) as u16,
            active_flags,
            caster_guid: aura_ref.caster_guid,
            cast_level: level.into(),
            applications: 0,
            duration_ms: None,
            remaining_ms: None,
            points,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_packet::ServerPacket;
    use wow_packet::packets::update::{UpdateBlock, UpdateObject};

    fn visibility_session() -> (
        WorldSession,
        crate::map_manager::SharedMapManager,
        flume::Receiver<Vec<u8>>,
    ) {
        let (_packet_tx, packet_rx) = flume::bounded(100);
        let (send_tx, send_rx) = flume::unbounded();
        let mut session = WorldSession::new(
            1, "VisibilityFacts".into(), 0, 2, 9, 54261,
            vec![0; 40], "esES".into(), packet_rx, send_tx,
        );
        let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
        let legacy = Arc::new(std::sync::RwLock::new(crate::map_manager::MapManager::new()));
        let guid = ObjectGuid::create_player(1, 96_101);
        let position = Position::new(10.0, 10.0, 0.0, 0.0);
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_manager(Arc::clone(&legacy));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            guid, "VisibilityFacts".into(), position, 571, 1, 1, 80, 0,
        ));
        let mut player = wow_entities::Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_map(571, 0).unwrap();
        player.unit_mut().world_mut().relocate(position);
        player.unit_mut().world_mut().object_mut().add_to_world();
        canonical.lock().unwrap().create_world_map(571, 0).map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
        assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
        (session, legacy, send_rx)
    }

    fn visibility_creature() -> crate::map_manager::WorldCreature {
        let guid = ObjectGuid::create_creature_like_cpp(1, 571, 9_601, 96_102);
        let mut creature = crate::map_manager::WorldCreature::new(
            guid, 9_601, Position::new(20.0, 20.0, 0.0, 0.0),
            150, 12, 1, 2, 0.0, 100, 35, 0, 0,
        );
        creature.creature.unit_mut().world_mut().set_map(571, 0).unwrap();
        creature.creature.unit_mut().world_mut().object_mut().add_to_world();
        creature.create_data.current_area_id = 47;
        // Live getters override the stored CREATE values in both APP loops.
        creature.create_data.health = 1;
        creature.create_data.max_health = 2;
        creature.create_data.level = 3;
        creature
    }

    fn insert_visibility_creature(
        manager: &crate::map_manager::SharedMapManager,
        creature: crate::map_manager::WorldCreature,
    ) {
        let position = creature.position();
        let (x, y) = crate::map_manager::world_to_grid_coords(position.x, position.y);
        manager.write().unwrap().add_creature(571, 0, x, y, creature);
    }

    fn live_create_block(creature: &crate::map_manager::WorldCreature, nearby: bool) -> UpdateBlock {
        let mut data = creature.create_data.clone();
        data.health = i64::from(creature.current_hp());
        data.max_health = i64::from(creature.max_hp());
        data.level = creature.level();
        data.npc_flags = creature.npc_flags_mask_like_cpp();
        if nearby {
            data.current_area_id = 0;
        }
        UpdateObject::create_creature_block_with_spline(
            data,
            &creature.position(),
            creature.active_move_spline_like_cpp()
                .and_then(crate::entity_update_bridge::create_object_spline_data_like_cpp),
        )
    }

    #[tokio::test]
    async fn nearby_create_from_facts_preserves_live_fields_and_have_at_client_gate() {
        let (mut session, manager, send_rx) = visibility_session();
        let mut creature = visibility_creature();
        creature.begin_move_spline_like_cpp(Position::new(24.0, 20.0, 0.0, 0.0)).unwrap();
        let guid = creature.guid();
        let expected = UpdateObject::create_creatures(vec![live_create_block(&creature, true)], 571)
            .to_bytes();
        insert_visibility_creature(&manager, creature);
        let position = Position::new(10.0, 10.0, 0.0, 0.0);
        session.send_nearby_creatures(571, &position, 0).await;
        assert_eq!(send_rx.try_recv().unwrap(), expected);
        assert!(session.client_visible_guids_like_cpp.contains(&guid));
        assert!(send_rx.try_recv().is_err());
        session.send_nearby_creatures(571, &position, 0).await;
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn visibility_refresh_publishes_create_then_sorted_captured_auras() {
        let (mut session, manager, send_rx) = visibility_session();
        let mut creature = visibility_creature();
        let guid = creature.guid();
        let auras = &mut creature.creature.unit_mut().subsystems_mut().auras;
        auras.visible_auras.insert(9, wow_entities::AuraRef::new(822, guid));
        auras.visible_auras.insert(2, wow_entities::AuraRef::new(823, guid));
        auras.applied_auras.push(wow_entities::AppliedAuraRef::new(822, guid, 4, 1));
        let expected_auras = wow_packet::packets::misc::AuraUpdate::full_for(guid, vec![
            represented_creature_aura_info_like_cpp(auras, 2, 12),
            represented_creature_aura_info_like_cpp(auras, 9, 12),
        ]).to_bytes();
        let expected_create = UpdateObject {
            map_id: 571, num_updates: 1,
            destroy_guids: Vec::new(), out_of_range_guids: Vec::new(),
            blocks: vec![live_create_block(&creature, false)],
        }.to_bytes();
        insert_visibility_creature(&manager, creature);
        session.update_visibility().await;
        assert_eq!(send_rx.try_recv().unwrap(), expected_create);
        assert_eq!(send_rx.try_recv().unwrap(), expected_auras);
        assert!(session.client_visible_guids_like_cpp.contains(&guid));
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn initial_aura_publication_keeps_the_create_observation_after_source_changes() {
        let (session, _manager, send_rx) = visibility_session();
        let mut creature = visibility_creature();
        let guid = creature.guid();
        let auras = &mut creature.creature.unit_mut().subsystems_mut().auras;
        auras.visible_auras.insert(7, wow_entities::AuraRef::new(u32::MAX, guid));
        auras.applied_auras.extend([
            wow_entities::AppliedAuraRef::new(u32::MAX, guid, 4, 1),
            wow_entities::AppliedAuraRef::new(u32::MAX, guid, 7, 0x8000_0000),
        ]);
        auras.visible_aura_applications_like_cpp.insert(7, wow_entities::VisibleAuraApplicationLikeCpp::new(
            u32::MAX, vec![
                wow_entities::VisibleAuraEffectAmountLikeCpp { effect_index: 31, amount: -17 },
                wow_entities::VisibleAuraEffectAmountLikeCpp { effect_index: 32, amount: 999 },
                wow_entities::VisibleAuraEffectAmountLikeCpp { effect_index: 1, amount: 999 },
                wow_entities::VisibleAuraEffectAmountLikeCpp { effect_index: 0, amount: 23 },
            ],
        ));
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 3, 1, 571, 0, 822, 41);
        auras.set_aura_cast_provenance_like_cpp(7, wow_entities::AuraCastProvenanceLikeCpp {
            cast_id, spell_visual_id: -7,
        });
        let expected = wow_packet::packets::misc::AuraUpdate::full_for(guid, vec![
            represented_creature_aura_info_like_cpp(auras, 7, 12),
        ]).to_bytes();
        let captured = creature.capture_visibility_candidate();
        creature.creature.unit_mut().set_level(80);
        creature.creature.unit_mut().subsystems_mut().auras.visible_auras.clear();
        creature.creature.unit_mut().subsystems_mut().auras.visible_aura_applications_like_cpp.clear();
        let slot = &captured.initial_auras().slots()[0];
        let data = creature_aura_info_from_facts(Some(slot), slot.slot(), captured.create().level())
            .aura_data.unwrap();
        assert_eq!(data.spell_id, i32::MAX);
        assert_eq!(data.flags, u16::MAX);
        assert_eq!(data.active_flags, 0x8000_0001);
        assert_eq!(data.points, vec![-17.0, 23.0]);
        assert_eq!(data.cast_id, cast_id);
        assert_eq!(data.spell_visual_id, -7);
        assert_eq!(data.cast_level, 12);
        session.send_initial_visible_packets_for_creature_like_cpp(captured.initial_auras());
        assert_eq!(send_rx.try_recv().unwrap(), expected);
        assert!(send_rx.try_recv().is_err());
        let empty = creature.capture_visibility_candidate();
        creature.creature.unit_mut().subsystems_mut().auras.visible_auras
            .insert(1, wow_entities::AuraRef::new(822, guid));
        session.send_initial_visible_packets_for_creature_like_cpp(empty.initial_auras());
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn nearby_facts_filter_phase_before_create_and_membership_publication() {
        let (mut session, manager, send_rx) = visibility_session();
        session.set_represented_player_phase_shift_like_cpp(PhaseShift::from_phases([10]));
        let mut creature = visibility_creature();
        let guid = creature.guid();
        *creature.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([20]);
        insert_visibility_creature(&manager, creature);
        session.send_nearby_creatures(571, &Position::new(10.0, 10.0, 0.0, 0.0), 0).await;
        assert!(!session.client_visible_guids_like_cpp.contains(&guid));
        assert!(send_rx.try_recv().is_err());
        // A request for another map is rejected before either source is read.
        session.send_nearby_creatures(530, &Position::new(10.0, 10.0, 0.0, 0.0), 0).await;
        assert!(!session.client_visible_guids_like_cpp.contains(&guid));
        assert!(send_rx.try_recv().is_err());
    }

    #[test]
    fn creature_aura_publication_uses_retained_base_provenance_like_cpp() {
        let creature_guid = ObjectGuid::create_creature_like_cpp(1, 571, 9_001, 7);
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 3, 1, 571, 0, 822, 41);
        let mut auras = wow_entities::AuraSubsystem::default();
        assert!(auras.add_self_cast_addon_aura_application_like_cpp(822, creature_guid, 1, 0x301,));
        let slot = *auras.visible_auras.keys().next().expect("visible slot");
        auras.set_aura_cast_provenance_like_cpp(
            slot,
            wow_entities::AuraCastProvenanceLikeCpp {
                cast_id,
                spell_visual_id: 7_822,
            },
        );

        let info = represented_creature_aura_info_like_cpp(&auras, slot, 80);
        let data = info.aura_data.expect("published aura data");
        assert_eq!(data.cast_id, cast_id);
        assert_eq!(data.spell_visual_id, 7_822);
    }

    #[test]
    fn creature_aura_publication_has_no_slot_derived_identity_fallback() {
        let creature_guid = ObjectGuid::create_creature_like_cpp(1, 571, 9_001, 7);
        let mut auras = wow_entities::AuraSubsystem::default();
        assert!(auras.add_self_cast_addon_aura_application_like_cpp(822, creature_guid, 1, 0,));
        let slot = *auras.visible_auras.keys().next().expect("visible slot");

        let info = represented_creature_aura_info_like_cpp(&auras, slot, 80);
        let data = info.aura_data.expect("published aura data");
        assert_eq!(data.cast_id, ObjectGuid::EMPTY);
        assert_eq!(data.spell_visual_id, 0);
    }
}

impl WorldSession {
    pub(crate) fn send_initial_visible_packets_for_creature_like_cpp(
        &self,
        creature: &wow_map::CreatureInitialAuraFacts,
    ) {
        if creature.slots().is_empty() {
            return;
        }

        let mut visible: Vec<_> = creature.slots().iter().collect();
        visible.sort_unstable_by_key(|captured| captured.slot());
        let level = creature.level();
        let auras = visible
            .into_iter()
            .map(|captured| creature_aura_info_from_facts(Some(captured), captured.slot(), level))
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
    /// C++ `WorldObject::SendMessageToSet(packet, true)` for a Player source.
    ///
    /// The owner session sends its own copy separately; this queues the same
    /// bytes for the nearby observers that already have the Player at client.
    /// The recipient range uses the represented Player's current position, and
    /// the source GUID is deliberately excluded, matching the C++ self-send
    /// split already used for creature publication.
    pub(crate) fn broadcast_player_packet_to_visible_set_realm_like_cpp(&self, bytes: Vec<u8>) {
        let (Some(source_guid), Some(source_position)) =
            (self.player_guid(), self.player_position_like_cpp())
        else {
            return;
        };
        self.broadcast_creature_packet_from_position_to_visible_set_and_connection_like_cpp(
            source_guid,
            source_position,
            bytes,
            true,
            false,
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
