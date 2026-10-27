// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read-only Player snapshots and spatial recipient queries over the directory.

use super::*;

fn player_visibility_snapshot_from_projection(
    guid: ObjectGuid,
    identity: PlayerDirectoryIdentityLikeCpp,
    position: Position,
    level: u8,
    vitals: crate::canonical_player_access::CanonicalPlayerVitalsLikeCpp,
    party_member_party_type: [u8; 2],
    presentation: crate::canonical_player_access::CanonicalPlayerPresentationLikeCpp,
    transport: Option<TransportInfo>,
) -> PlayerVisibilityCreateSnapshot {
    let (display_id, visible_items, _, zone_id, customizations) = presentation;
    PlayerVisibilityCreateSnapshot {
        guid,
        position,
        race: identity.race,
        class: identity.class,
        sex: identity.sex,
        level,
        display_id,
        zone_id,
        current_health: vitals.current_health,
        max_health: vitals.max_health,
        power_type: vitals.power_type,
        current_power: vitals.current_power,
        max_power: vitals.max_power,
        base_mana: vitals.base_mana,
        transport,
        visible_items: Arc::new(visible_items),
        customizations: Arc::new(
            customizations
                .into_iter()
                .map(
                    |(option_id, choice_id)| ChrCustomizationChoiceValuesUpdate {
                        option_id,
                        choice_id,
                    },
                )
                .collect(),
        ),
        party_member_party_type,
    }
}

impl PlayerRegistry {
    /// Return this registration's locale only while its generation still owns
    /// the expected canonical Player incarnation.
    #[must_use]
    pub fn current_player_locale(
        &self,
        registration: PlayerRegistration,
        expected_handle: wow_map::PlayerHandle,
    ) -> Option<String> {
        let entry = self.entries.get(&registration.guid)?;
        if entry.generation != registration.generation
            || entry.player_handle != Some(expected_handle)
        {
            return None;
        }
        Some(entry.identity.locale.clone())
    }

    /// Project the current directory endpoint for one map-selected aura
    /// recipient. This runs only after the producer has released MapManager.
    #[must_use]
    pub fn selected_aura_recipient(
        &self,
        guid: ObjectGuid,
        expected_handle: PlayerHandle,
        expected_map: wow_map::MapKey,
    ) -> Option<(PlayerRegistration, SharedClientVisibleGuidsLikeCpp)> {
        let entry = self.entries.get(&guid)?;
        if entry.player_handle != Some(expected_handle)
            || !entry.placement.is_in_world
            || u32::from(entry.placement.map_id) != expected_map.map_id
            || entry.placement.instance_id != expected_map.instance_id
        {
            return None;
        }
        Some((
            PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            entry.client_visible_guids_like_cpp.clone(),
        ))
    }

    /// Resolve a frozen Player packet receipt for direct realm delivery. The
    /// map residence revision is checked before the caller queues bytes, and
    /// the registration/HaveAtClient snapshot is rechecked after releasing
    /// MapManager so no directory access happens under the map guard.
    pub(crate) fn selected_packet_recipient_at_residence(
        &self,
        guid: ObjectGuid,
        expected_handle: PlayerHandle,
        expected_map: wow_map::MapKey,
        expected_residence_revision: u64,
        visible_source: Option<ObjectGuid>,
    ) -> Option<(PlayerRegistration, SharedClientVisibleGuidsLikeCpp)> {
        let manager = self.canonical_map_manager_like_cpp()?;
        if manager
            .lock()
            .ok()?
            .player_active_residence_revision_like_cpp(expected_handle)
            != Some((expected_map, expected_residence_revision))
        {
            return None;
        }

        let (registration, visibility) =
            self.selected_aura_recipient(guid, expected_handle, expected_map)?;
        if visible_source.is_some_and(|source| !visibility.contains(&source)) {
            return None;
        }
        Some((registration, visibility))
    }

    /// Snapshot the bounded facts used by runtime recipient selection.
    #[must_use]
    pub fn runtime_recipients(&self) -> Vec<PlayerRuntimeRecipient> {
        let guids: Vec<_> = self.entries.iter().map(|entry| *entry.key()).collect();
        guids
            .into_iter()
            .filter_map(|guid| self.runtime_recipient(guid))
            .collect()
    }

    /// Resolve one current runtime recipient without exposing directory storage.
    #[must_use]
    pub fn runtime_recipient(&self, guid: ObjectGuid) -> Option<PlayerRuntimeRecipient> {
        let (registration, placement, account_id, logging, visibility, transports) = {
            let entry = self.entries.get(&guid)?;
            (
                PlayerRegistration {
                    guid,
                    generation: entry.generation,
                },
                entry.placement,
                entry.identity.account_id,
                entry
                    .advanced_combat_logging_enabled_like_cpp
                    .load(Ordering::Relaxed),
                entry.client_visible_guids_like_cpp.clone(),
                entry.client_visible_transports_like_cpp.clone(),
            )
        };
        let (combat_reach, in_combat, liquid_status) =
            self.canonical_at(guid, placement.map_id, placement.instance_id, |p| {
                (
                    p.unit().world().combat_reach(),
                    p.unit()
                        .unit_flags_like_cpp()
                        .contains(UnitFlags::IN_COMBAT),
                    p.gameplay_state().liquid_status,
                )
            })?;
        Some(PlayerRuntimeRecipient {
            registration,
            guid,
            map_id: placement.map_id,
            instance_id: placement.instance_id,
            position: placement.position,
            combat_reach,
            liquid_status,
            is_in_world: placement.is_in_world,
            is_alive: placement.is_alive,
            account_id,
            in_combat,
            advanced_combat_logging: logging,
            committed_visibility: visibility,
            committed_visible_transports: transports,
        })
    }

    /// Resolve one connected chat/social identity by case-insensitive player
    /// name without exposing the directory iterator.
    #[must_use]
    pub fn social_recipient_by_name(
        &self,
        player_name: &str,
    ) -> Option<PlayerSocialRecipientSnapshot> {
        self.entries.iter().find_map(|entry| {
            entry
                .identity
                .player_name
                .eq_ignore_ascii_case(player_name)
                .then(|| self.social_snapshot(entry.key(), entry.value()))?
        })
    }

    /// Resolve one connected chat/social identity by GUID.
    #[must_use]
    pub fn social_recipient(&self, guid: ObjectGuid) -> Option<PlayerSocialRecipientSnapshot> {
        let entry = self.entries.get(&guid)?;
        self.social_snapshot(&guid, &entry)
    }

    fn social_snapshot(
        &self,
        guid: &ObjectGuid,
        entry: &PlayerRegistryEntry,
    ) -> Option<PlayerSocialRecipientSnapshot> {
        let (is_afk, is_dnd, is_game_master, dungeon_difficulty_id) = self.canonical_at(
            *guid,
            entry.placement.map_id,
            entry.placement.instance_id,
            |player| {
                (
                    player.has_player_flag(crate::session::PLAYER_FLAGS_AFK_LIKE_CPP),
                    player.has_player_flag(crate::session::PLAYER_FLAGS_DND_LIKE_CPP),
                    player.is_game_master_like_cpp(),
                    player.dungeon_difficulty_id_like_cpp(),
                )
            },
        )?;
        Some(PlayerSocialRecipientSnapshot {
            registration: PlayerRegistration {
                guid: *guid,
                generation: entry.generation,
            },
            guid: *guid,
            player_name: entry.identity.player_name.clone(),
            race: entry.identity.race,
            class: entry.identity.class,
            map_id: entry.placement.map_id,
            instance_id: entry.placement.instance_id,
            dungeon_difficulty_id,
            is_game_master,
            is_afk,
            is_dnd,
        })
    }

    #[must_use]
    pub fn social_auto_reply(&self, guid: ObjectGuid) -> Option<String> {
        let (map_id, instance_id) = {
            let entry = self.entries.get(&guid)?;
            (entry.placement.map_id, entry.placement.instance_id)
        };
        self.canonical_at(guid, map_id, instance_id, |player| {
            player
                .gameplay_state()
                .social
                .auto_reply_msg_like_cpp
                .clone()
        })
    }

    /// Resolve connected presence facts for one Group member.
    #[must_use]
    pub fn group_presence(&self, guid: ObjectGuid) -> Option<PlayerGroupPresenceSnapshot> {
        let (registration, placement, account_id, recruiter_id, has_active_loot_rolls) = {
            let entry = self.entries.get(&guid)?;
            (
                PlayerRegistration {
                    guid,
                    generation: entry.generation,
                },
                entry.placement,
                entry.identity.account_id,
                entry.identity.recruiter_id,
                !entry.active_loot_rolls.is_empty(),
            )
        };
        let in_combat = self.canonical_at(guid, placement.map_id, placement.instance_id, |p| {
            p.unit()
                .unit_flags_like_cpp()
                .contains(UnitFlags::IN_COMBAT)
        })?;
        Some(PlayerGroupPresenceSnapshot {
            registration,
            guid,
            map_id: placement.map_id,
            instance_id: placement.instance_id,
            position: placement.position,
            is_in_world: placement.is_in_world,
            is_alive: placement.is_alive,
            level: placement.level,
            account_id,
            recruiter_id,
            in_combat,
            has_active_loot_rolls,
        })
    }

    /// Resolve connected Group members in the authoritative input order.
    #[must_use]
    pub fn group_presences_in_order(
        &self,
        member_guids: &[ObjectGuid],
    ) -> Vec<PlayerGroupPresenceSnapshot> {
        member_guids
            .iter()
            .filter_map(|guid| self.group_presence(*guid))
            .collect()
    }

    /// Resolve the connected projection used to build PartyUpdate payloads.
    #[must_use]
    pub fn party_member(&self, guid: ObjectGuid) -> Option<PlayerPartyMemberSnapshot> {
        let (registration, identity, placement) = {
            let entry = self.entries.get(&guid)?;
            (
                PlayerRegistration {
                    guid,
                    generation: entry.generation,
                },
                entry.identity.clone(),
                entry.placement,
            )
        };
        let state = self.party_state(guid, placement.map_id, placement.instance_id)?;
        let (in_vehicle, vehicle_seat, phase, auras, pet) =
            self.party_gameplay_projection(guid, placement.map_id, placement.instance_id)?;
        let vitals = state.vitals;
        Some(PlayerPartyMemberSnapshot {
            registration,
            guid,
            player_name: identity.player_name,
            race: identity.race,
            class: identity.class,
            position: placement.position,
            is_pvp: state.is_pvp,
            is_alive: vitals.is_alive,
            is_ghost: state.is_ghost,
            is_ffa_pvp: state.is_ffa_pvp,
            is_afk: state.is_afk,
            is_dnd: state.is_dnd,
            in_vehicle,
            power_type: vitals.power_type,
            current_health: vitals.current_health,
            max_health: vitals.max_health,
            current_power: vitals.current_power,
            max_power: vitals.max_power,
            level: placement.level,
            spec_id: state.spec_id,
            zone_id: state.zone_id,
            party_member_vehicle_seat: vehicle_seat,
            party_member_party_type: state.party_type,
            party_member_phase_states: phase,
            party_member_auras: auras,
            party_member_pet_stats: pet,
        })
    }

    /// Resolve connected PartyUpdate projections in authoritative Group order.
    #[must_use]
    pub fn party_members_in_order(
        &self,
        member_guids: &[ObjectGuid],
    ) -> Vec<PlayerPartyMemberSnapshot> {
        member_guids
            .iter()
            .filter_map(|guid| self.party_member(*guid))
            .collect()
    }

    /// Query the temporary connected-session achievement mirror through a
    /// bounded semantic operation.
    #[must_use]
    pub fn connected_player_has_achievement(&self, guid: ObjectGuid, achievement_id: u32) -> bool {
        let Some((map_id, instance_id)) = self
            .entries
            .get(&guid)
            .map(|entry| (entry.placement.map_id, entry.placement.instance_id))
        else {
            return false;
        };
        self.canonical_at(guid, map_id, instance_id, |player| {
            player
                .gameplay_state()
                .achievements
                .iter()
                .any(|record| record.achievement_id == achievement_id)
        })
        .unwrap_or(false)
    }

    /// Resolve the bounded connected-player view required by inspect handlers.
    #[must_use]
    /// Resolve one inspect target's directory-owned identity and placement.
    ///
    /// Takes no canonical manager: the honor block moved to
    /// [`Self::inspect_honor_stats`] (#252), so the inspect and achievement
    /// opcodes — neither of which reads honor — no longer serialize with the
    /// canonical map update loop to populate a block they discard.
    pub fn inspect_snapshot(&self, guid: ObjectGuid) -> Option<PlayerInspectSnapshot> {
        let (identity, placement) = {
            let entry = self.entries.get(&guid)?;
            (entry.identity.clone(), entry.placement)
        };
        let presentation = self.canonical_at(
            guid,
            placement.map_id,
            placement.instance_id,
            canonical_player_presentation_like_cpp,
        )?;
        Some(PlayerInspectSnapshot {
            guid,
            map_id: placement.map_id,
            position: placement.position,
            faction_template_id: presentation.2,
            player_name: identity.player_name,
            race: identity.race,
            class: identity.class,
            sex: identity.sex,
            level: placement.level,
            visible_items: Arc::new(presentation.1),
        })
    }

    /// Read one target's honor block off its canonical `Player`.
    ///
    /// `None` means unknown, not zero, and a caller must not substitute a
    /// default. During a far teleport the canonical `Player` is removed from the
    /// old map before the destination world-port response lands, so a target
    /// mid-transfer has no resident owner. C++ `HandleInspectHonorStats` answers
    /// nothing when it cannot resolve the target; fabricating zeros would report
    /// a real honor level as lost rather than as unavailable.
    ///
    /// The registry guard is dropped before the canonical mutex is taken.
    pub fn inspect_honor_stats(
        &self,
        guid: ObjectGuid,
        canonical_map_manager: Option<&SharedCanonicalMapManager>,
    ) -> Option<HonorStatsLikeCpp> {
        let (map_id, instance_id) = {
            let entry = self.entries.get(&guid)?;
            (entry.placement.map_id, entry.placement.instance_id)
        };

        with_canonical_player_at_like_cpp(
            canonical_map_manager?,
            guid,
            u32::from(map_id),
            instance_id,
            canonical_player_honor_stats_like_cpp,
        )
    }

    /// Resolve in-world recipients inside one exact map-instance radius.
    #[must_use]
    pub fn movement_recipients_within_range(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        range: f32,
    ) -> Vec<PlayerRegistration> {
        let range_sq = range * range;
        self.entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let value = entry.value();
                if guid == excluded_guid
                    || !value.placement.is_in_world
                    || value.placement.map_id != map_id
                    || value.placement.instance_id != instance_id
                {
                    return None;
                }
                let dx = value.placement.position.x - source_position.x;
                let dy = value.placement.position.y - source_position.y;
                (dx * dx + dy * dy <= range_sq).then_some(PlayerRegistration {
                    guid,
                    generation: entry.value().generation,
                })
            })
            .collect()
    }

    /// Resolve every other in-world movement recipient in one map instance.
    #[must_use]
    pub fn same_map_movement_recipients(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Vec<PlayerRegistration> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let value = entry.value();
                (guid != excluded_guid
                    && value.placement.is_in_world
                    && value.placement.map_id == map_id
                    && value.placement.instance_id == instance_id)
                    .then_some(PlayerRegistration {
                        guid,
                        generation: entry.value().generation,
                    })
            })
            .collect()
    }

    /// Resolve spell-pull observers using the represented C++ combat-reach radius.
    #[must_use]
    pub fn spell_pull_recipients(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        source_combat_reach: f32,
        visibility_range: f32,
    ) -> Vec<PlayerRegistration> {
        let candidates: Vec<_> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let value = entry.value();
                if guid == excluded_guid
                    || !value.placement.is_in_world
                    || value.placement.map_id != map_id
                    || value.placement.instance_id != instance_id
                {
                    return None;
                }
                Some((guid, value.generation, value.placement))
            })
            .collect();
        candidates
            .into_iter()
            .filter_map(|(guid, generation, placement)| {
                let target_reach =
                    self.canonical_at(guid, placement.map_id, placement.instance_id, |p| {
                        p.unit().world().combat_reach()
                    })?;
                let dx = placement.position.x - source_position.x;
                let dy = placement.position.y - source_position.y;
                let reach = visibility_range + source_combat_reach.max(0.0) + target_reach.max(0.0);
                (dx * dx + dy * dy < reach * reach)
                    .then_some(PlayerRegistration { guid, generation })
            })
            .collect()
    }

    /// Snapshot fellow passengers needed by C++ `Map::SendInitSelf` CREATE blocks.
    #[must_use]
    pub fn fellow_transport_passengers(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        transport_guid: ObjectGuid,
    ) -> Vec<PlayerVisibilityCreateSnapshot> {
        let candidates: Vec<_> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let value = entry.value();
                if guid == excluded_guid
                    || !value.placement.is_in_world
                    || value.placement.map_id != map_id
                    || value.placement.instance_id != instance_id
                {
                    return None;
                }
                Some((guid, value.identity.clone(), value.placement))
            })
            .collect();

        candidates
            .into_iter()
            .filter_map(|(guid, identity, placement)| {
                let transport =
                    self.canonical_at(guid, placement.map_id, placement.instance_id, |player| {
                        player.gameplay_state().transport.clone()
                    })??;
                if transport.guid != transport_guid {
                    return None;
                }
                let (vitals, party_type, (display_id, visible_items, _, zone_id, customizations)) =
                    self.create_state(guid, placement.map_id, placement.instance_id)?;
                Some(PlayerVisibilityCreateSnapshot {
                    guid,
                    position: placement.position,
                    race: identity.race,
                    class: identity.class,
                    sex: identity.sex,
                    level: placement.level,
                    display_id,
                    zone_id,
                    current_health: vitals.current_health,
                    max_health: vitals.max_health,
                    power_type: vitals.power_type,
                    current_power: vitals.current_power,
                    max_power: vitals.max_power,
                    base_mana: vitals.base_mana,
                    transport: Some(TransportInfo {
                        guid: transport.guid,
                        x: transport.x,
                        y: transport.y,
                        z: transport.z,
                        o: transport.orientation,
                        seat: transport.seat,
                        time: transport.time,
                        prev_time: transport.prev_time,
                        vehicle_id: transport.vehicle_id,
                    }),
                    visible_items: Arc::new(visible_items),
                    customizations: Arc::new(
                        customizations
                            .into_iter()
                            .map(
                                |(option_id, choice_id)| ChrCustomizationChoiceValuesUpdate {
                                    option_id,
                                    choice_id,
                                },
                            )
                            .collect(),
                    ),
                    party_member_party_type: party_type,
                })
            })
            .collect()
    }

    /// Read one connected player's active status for an exact shared quest.
    #[must_use]
    pub fn quest_active_status(&self, guid: ObjectGuid, quest_id: u32) -> Option<Option<u8>> {
        let (map_id, instance_id) = {
            let entry = self.entries.get(&guid)?;
            (entry.placement.map_id, entry.placement.instance_id)
        };
        self.canonical_at(guid, map_id, instance_id, |player| {
            player
                .gameplay_state()
                .quests
                .statuses_like_cpp()
                .get(&quest_id)
                .map(|status| status.status)
        })
    }

    /// Snapshot the exact receiver facts used by represented quest sharing.
    /// The projection cannot grow outside the retirement ledger in issue #196.
    #[must_use]
    /// Resolve one quest-share receiver.
    ///
    /// Reputation standings are read off the receiver's canonical `Player`
    /// (#252) rather than mirrored. The registry guard is dropped before the
    /// canonical mutex is taken, so the two are never held together.
    pub fn quest_sharing_snapshot(
        &self,
        guid: ObjectGuid,
        canonical_map_manager: Option<&SharedCanonicalMapManager>,
    ) -> Option<PlayerQuestSharingSnapshot> {
        let (mut snapshot, map_id, instance_id) = {
            let entry = self.entries.get(&guid)?;
            (
                PlayerQuestSharingSnapshot {
                    registration: PlayerRegistration {
                        guid,
                        generation: entry.generation,
                    },
                    pending_quest_sharing: None,
                    is_alive: entry.placement.is_alive,
                    rewarded_quests: HashSet::new(),
                    active_quest_statuses: HashMap::new(),
                    df_quests: HashSet::new(),
                    daily_quests_completed: HashSet::new(),
                    level: entry.placement.level,
                    class: entry.identity.class,
                    race: entry.identity.race,
                    reputation_standings: None,
                    active_expansion: entry.identity.active_expansion,
                },
                entry.placement.map_id,
                entry.placement.instance_id,
            )
        };

        if let Some(manager) = canonical_map_manager {
            let gameplay = with_canonical_player_at_like_cpp(
                manager,
                guid,
                u32::from(map_id),
                instance_id,
                |player| player.gameplay_state().clone(),
            )?;
            snapshot.pending_quest_sharing = gameplay.quests.pending_share_like_cpp();
            snapshot.rewarded_quests = gameplay
                .quests
                .rewarded_quest_ids_like_cpp()
                .iter()
                .copied()
                .collect();
            snapshot.active_quest_statuses = gameplay
                .quests
                .statuses_like_cpp()
                .values()
                .map(|status| (status.quest_id, status.status))
                .collect();
            snapshot.df_quests = gameplay
                .quests
                .df_quest_ids_like_cpp()
                .iter()
                .copied()
                .collect();
            snapshot.daily_quests_completed = gameplay
                .quests
                .daily_quest_ids_like_cpp()
                .iter()
                .copied()
                .collect();
            snapshot.reputation_standings = with_canonical_player_at_like_cpp(
                manager,
                guid,
                u32::from(map_id),
                instance_id,
                canonical_player_reputation_standings_like_cpp,
            );
        }

        Some(snapshot)
    }

    /// Read one connected player's race for PvP quest-credit team comparison.
    #[must_use]
    pub fn quest_credit_race(&self, guid: ObjectGuid) -> Option<u8> {
        self.entries.get(&guid).map(|entry| entry.identity.race)
    }

    /// Snapshot one player target for represented vehicle interaction.
    #[must_use]
    pub fn vehicle_interaction_snapshot(
        &self,
        guid: ObjectGuid,
    ) -> Option<PlayerVehicleInteractionSnapshot> {
        let entry = self.entries.get(&guid)?;
        let placement = entry.placement;
        drop(entry);
        let has_vehicle_kit =
            self.canonical_at(guid, placement.map_id, placement.instance_id, |player| {
                player.gameplay_state().mount_vehicle_kit.is_some()
            })?;
        Some(PlayerVehicleInteractionSnapshot {
            map_id: placement.map_id,
            instance_id: placement.instance_id,
            position: placement.position,
            has_vehicle_kit,
        })
    }

    /// Snapshot only the live player facts the creature aggro phase needs.
    #[must_use]
    pub fn aggro_candidates(&self) -> Vec<PlayerAggroCandidateSnapshot> {
        let candidates: Vec<_> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let entry = entry.value();
                (entry.placement.is_in_world && entry.placement.is_alive)
                    .then_some((guid, entry.placement))
            })
            .collect();
        candidates
            .into_iter()
            .filter_map(|(guid, placement)| {
                let (
                    combat_reach,
                    ranks,
                    (
                        unit_flags,
                        unit_state,
                        is_game_master,
                        faction_template_id,
                        gray_level,
                        liquid_status,
                    ),
                ) = self.canonical_at(guid, placement.map_id, placement.instance_id, |player| {
                    (
                        player.unit().world().combat_reach(),
                        player
                            .reputation_like_cpp()
                            .forced_reactions_like_cpp()
                            .collect(),
                        canonical_player_aggro_unit_state_like_cpp(player),
                    )
                })?;
                Some(PlayerAggroCandidateSnapshot {
                    player_guid: guid,
                    map_id: placement.map_id,
                    instance_id: placement.instance_id,
                    position: placement.position,
                    combat_reach,
                    liquid_status,
                    level: placement.level,
                    gray_level,
                    unit_flags,
                    unit_state,
                    is_game_master,
                    faction_template_id,
                    forced_reputation_ranks: ranks,
                })
            })
            .collect()
    }

    /// Select player CREATE candidates by directory-owned presence and spatial facts.
    #[must_use]
    pub fn player_visibility_create_candidates(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        source_position: Position,
        source_combat_reach: f32,
        visibility_radius: f32,
    ) -> Vec<PlayerVisibilityCreateSnapshot> {
        let candidates: Vec<_> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let guid = *entry.key();
                let entry = entry.value();
                if guid == excluded_guid
                    || !entry.placement.is_in_world
                    || entry.placement.map_id != map_id
                    || entry.placement.instance_id != instance_id
                {
                    return None;
                }
                Some((guid, entry.identity.clone(), entry.placement))
            })
            .collect();

        candidates
            .into_iter()
            .filter_map(|(guid, identity, placement)| {
                let target_reach =
                    self.canonical_at(guid, placement.map_id, placement.instance_id, |p| {
                        p.unit().world().combat_reach()
                    })?;
                let dx = placement.position.x - source_position.x;
                let dy = placement.position.y - source_position.y;
                let reach =
                    visibility_radius + source_combat_reach.max(0.0) + target_reach.max(0.0);
                if dx * dx + dy * dy >= reach * reach {
                    return None;
                }
                let (vitals, party_type, presentation) =
                    self.create_state(guid, placement.map_id, placement.instance_id)?;
                let transport = self
                    .canonical_at(guid, placement.map_id, placement.instance_id, |player| {
                        player.gameplay_state().transport.clone()
                    })?
                    .map(|transport| TransportInfo {
                        guid: transport.guid,
                        x: transport.x,
                        y: transport.y,
                        z: transport.z,
                        o: transport.orientation,
                        seat: transport.seat,
                        time: transport.time,
                        prev_time: transport.prev_time,
                        vehicle_id: transport.vehicle_id,
                    });
                Some(player_visibility_snapshot_from_projection(
                    guid,
                    identity,
                    placement.position,
                    placement.level,
                    vitals,
                    party_type,
                    presentation,
                    transport,
                ))
            })
            .collect()
    }

    /// Project a selected Player only while the same session registration,
    /// canonical incarnation and active residence remain current.
    #[must_use]
    pub fn player_visibility_create_snapshot_for_candidate(
        &self,
        registration: PlayerRegistration,
        expected_handle: PlayerHandle,
        expected_map: wow_map::MapKey,
        expected_residence_revision: u64,
    ) -> Option<PlayerVisibilityCreateSnapshot> {
        if registration.guid() != expected_handle.guid() {
            return None;
        }
        let identity = {
            let entry = self.entries.get(&registration.guid())?;
            if entry.generation != registration.generation
                || entry.player_handle != Some(expected_handle)
                || !entry.placement.is_in_world
                || u32::from(entry.placement.map_id) != expected_map.map_id
                || entry.placement.instance_id != expected_map.instance_id
            {
                return None;
            }
            entry.identity.clone()
        };
        let manager = self.canonical_map_manager_like_cpp()?;
        let projection = {
            let manager = manager.lock().ok()?;
            if manager.player_active_residence_revision_like_cpp(expected_handle)
                != Some((expected_map, expected_residence_revision))
            {
                return None;
            }
            manager.with_player_like_cpp(expected_handle, |player| {
                let world = player.unit().world();
                if !world.object().is_in_world()
                    || world.map_id() != expected_map.map_id
                    || world.instance_id() != expected_map.instance_id
                {
                    return None;
                }
                let transport =
                    player
                        .gameplay_state()
                        .transport
                        .clone()
                        .map(|transport| TransportInfo {
                            guid: transport.guid,
                            x: transport.x,
                            y: transport.y,
                            z: transport.z,
                            o: transport.orientation,
                            seat: transport.seat,
                            time: transport.time,
                            prev_time: transport.prev_time,
                            vehicle_id: transport.vehicle_id,
                        });
                Some((
                    world.position(),
                    u8::try_from(player.unit().data().level).unwrap_or_default(),
                    canonical_player_vitals_like_cpp(player),
                    player.data().party_type,
                    canonical_player_presentation_like_cpp(player),
                    transport,
                ))
            })??
        };
        let entry = self.entries.get(&registration.guid())?;
        if entry.generation != registration.generation
            || entry.player_handle != Some(expected_handle)
            || !entry.placement.is_in_world
            || u32::from(entry.placement.map_id) != expected_map.map_id
            || entry.placement.instance_id != expected_map.instance_id
        {
            return None;
        }
        let (position, level, vitals, party_type, presentation, transport) = projection;
        Some(player_visibility_snapshot_from_projection(
            registration.guid(),
            identity,
            position,
            level,
            vitals,
            party_type,
            presentation,
            transport,
        ))
    }
}
