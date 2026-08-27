// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Remaining per-session character state and its helpers.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use wow_database::SqlResult;

use super::*;

impl WorldSession {
    pub(super) fn skill_rewarded_quest_fallback_allowed_like_cpp(&self, spell_id: i32) -> bool {
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return false;
        };
        let Some(condition_id) = self
            .spell_misc_store()
            .and_then(|store| store.entry_for_spell_difficulty_like_cpp(spell_id, 0))
            .map(|misc| misc.show_future_spell_player_condition_id)
            .filter(|condition_id| *condition_id > 0)
            .and_then(|condition_id| u32::try_from(condition_id).ok())
        else {
            // C++ `SpellInfo::MeetsFutureSpellPlayerCondition` returns false
            // when ShowFutureSpellPlayerConditionID is zero.
            return false;
        };

        self.represented_meets_player_condition_id_like_cpp(condition_id)
    }

    pub(super) fn skill_rewarded_spell_changes_for_login_like_cpp(
        &self,
        skill_id: u16,
        skill_value: u16,
        race: u8,
        class: u8,
        level: u8,
    ) -> wow_data::SkillRewardedSpellChangesLikeCpp {
        let Some(skill_store) = self.skill_store() else {
            return wow_data::SkillRewardedSpellChangesLikeCpp::default();
        };
        let spell_store = self.spell_store().cloned();
        let spell_levels_store = self.spell_levels_store().cloned();

        skill_store.skill_rewarded_spell_changes_like_cpp(
            skill_id,
            skill_value,
            race,
            class,
            level,
            |spell_id| {
                let spell_id_u32 = u32::try_from(spell_id).ok()?;
                spell_store.as_ref()?.get(spell_id)?;
                spell_levels_store
                    .as_ref()
                    .and_then(|store| store.entry_for_spell_difficulty_like_cpp(spell_id_u32, 0))
                    .map(|spell| {
                        (
                            u32::try_from(spell.base_level).unwrap_or(0),
                            u32::try_from(spell.spell_level).unwrap_or(0),
                        )
                    })
                    .or(Some((0, 0)))
            },
            |spell_id| self.skill_rewarded_quest_fallback_allowed_like_cpp(spell_id),
        )
    }

    fn creature_addon_create_fields_like_cpp(
        addon: Option<&CreatureAddonLifecycleRecordLikeCpp>,
    ) -> CreatureAddonCreateFieldsLikeCpp {
        let Some(addon) = addon else {
            // C++ Creature::UpdateEntry calls SetSheath(SHEATH_STATE_MELEE)
            // when no addon row exists; addon rows then own the exact value.
            return CreatureAddonCreateFieldsLikeCpp {
                stand_state: UnitStandStateType::Stand as u8,
                sheathe_state: SheathState::Melee as u8,
                ..CreatureAddonCreateFieldsLikeCpp::default()
            };
        };

        CreatureAddonCreateFieldsLikeCpp {
            has_addon: true,
            mount_display_id: addon.mount_display_id as i32,
            stand_state: addon.stand_state as u8,
            vis_flags: addon.vis_flags,
            anim_tier: addon.anim_tier,
            sheathe_state: addon.sheath_state as u8,
            pvp_flags: addon.pvp_flags.bits(),
            emote_state: addon.emote as i32,
            ai_anim_kit_id: addon.ai_anim_kit_id,
            movement_anim_kit_id: addon.movement_anim_kit_id,
            melee_anim_kit_id: addon.melee_anim_kit_id,
        }
    }

    /// Fallback: skip ConnectTo and trigger direct login on the realm socket.
    ///
    /// Used when no session manager is configured or all ConnectTo retries fail.
    /// Sets a flag so that `process_pending` will call `handle_continue_player_login`.
    pub(super) fn fallback_direct_login(&mut self) {
        // player_loading is already set — create a dummy oneshot that fires immediately
        let (tx, rx) = tokio::sync::oneshot::channel();
        let link = wow_network::session_mgr::InstanceLink {
            send_tx: self.send_tx().clone(),
            send_write_fence_like_cpp: None,
            pkt_rx: None, // None = keep using realm socket's packet_rx
        };
        let _ = tx.send(link);
        self.set_instance_link_rx(Some(rx));
        info!(
            "Fallback: direct login scheduled for account {}",
            self.account_id
        );
    }

    /// Resolve C++ `Player::LoadFromDB`'s persisted transport passenger state
    /// against the currently materialized MO-transport path.
    pub(super) async fn resolve_persisted_transport_login_like_cpp(
        &self,
        guid_low: u64,
        saved_map_id: u16,
        offset: Position,
    ) -> Option<PersistedTransportLoginLikeCpp> {
        let world_db = self.world_db().map(Arc::clone)?;
        let query = format!(
            "SELECT t.guid, t.entry, t.phaseUseFlags, t.phaseid, t.phasegroup, \
             gt.displayId, gt.size, gt.Data0, gt.Data1, gt.Data2, gt.Data8, \
             COALESCE(goo.flags, gta.flags, 0), COALESCE(goo.faction, gta.faction, 0) \
             FROM transports t \
             JOIN gameobject_template gt ON gt.entry = t.entry \
             LEFT JOIN gameobject_template_addon gta ON gta.entry = t.entry \
             LEFT JOIN gameobject_overrides goo ON goo.spawnId = t.guid \
             WHERE gt.type = 15 AND t.guid = {guid_low} \
             LIMIT 1"
        );
        let result = world_db.direct_query(&query).await.ok()?;
        if result.is_empty() {
            return None;
        }
        let transport_create = map_transport_create_from_row_like_cpp(&result);

        let data_dir = self.mmap_runtime_config_like_cpp().data_dir.clone();
        let taxi_path_nodes = TaxiPathNodeStore::load(&data_dir, &self.locale).ok()?;
        let nodes: Vec<TaxiPathNodeEntry> = taxi_path_nodes
            .entries()
            .filter(|node| node.path_id == transport_create.taxi_path_id)
            .cloned()
            .collect();
        // TransportMgr creates one same-GUID transport object for every map in
        // the template route. C++ first asks the character's saved map for that
        // object before it may follow GetExpectedMapId() to the current leg.
        if !transport_route_contains_saved_map_like_cpp(
            nodes.iter().map(|node| node.continent_id),
            saved_map_id,
        ) {
            return None;
        }
        let transport_position = transport_position_for_login_like_cpp(
            &nodes,
            transport_create.move_speed,
            transport_create.accel_rate,
            Self::game_time_ms_like_cpp(),
        )?;
        let guid = ObjectGuid::create_transport(HighGuid::Transport, guid_low as i64);
        validate_persisted_transport_login_like_cpp(
            guid,
            offset,
            transport_position,
            transport_create,
        )
    }

    /// C++ `Map::SendInitSelf` appends other passengers on the player's
    /// current transport after the player's own CREATE block, but only when
    /// `HaveAtClient(passenger)` was already true.
    fn init_self_fellow_transport_passenger_blocks_like_cpp(
        &self,
        map_id: u16,
        transport_guid: ObjectGuid,
    ) -> Vec<UpdateBlock> {
        let (Some(player_guid), Some(registry)) = (self.player_guid(), self.player_registry())
        else {
            return Vec::new();
        };
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut passengers: Vec<_> = registry
            .fellow_transport_passengers(player_guid, map_id, instance_id, transport_guid)
            .into_iter()
            .filter(|passenger| self.client_visible_guids_like_cpp.contains(&passenger.guid))
            .collect();
        passengers.sort_by_key(|passenger| passenger.guid);
        passengers
            .into_iter()
            .filter_map(|passenger| {
                player_visibility_create_update_from_snapshot_like_cpp(&passenger, map_id)
                    .blocks
                    .pop()
            })
            .collect()
    }

    /// Plan C++ `Map::SendInitSelf`'s current transport plus
    /// `Map::SendInitTransports`' remaining map transports from one stable
    /// path-time snapshot.
    async fn plan_init_transports_like_cpp(
        &mut self,
        map_id: u16,
        persisted_transport: Option<PersistedTransportLoginLikeCpp>,
    ) -> Box<InitTransportsPlanLikeCpp> {
        self.client_visible_transports_like_cpp.clear();
        let mut plan = Box::new(InitTransportsPlanLikeCpp::default());
        let now_ms = Self::game_time_ms_like_cpp();
        if let Some(snapshot) = persisted_transport {
            if snapshot.map_id == map_id {
                plan.own_transport = Some((
                    snapshot.guid,
                    map_transport_create_block_like_cpp(
                        snapshot.transport_create,
                        snapshot.transport_position,
                        now_ms,
                    ),
                ));
                plan.considered += 1;
            } else {
                // A validated attachment and the selected login map must be
                // one snapshot. Fail closed instead of sending a player whose
                // nested transport reference has no preceding CREATE block.
                self.set_player_transport_info_like_cpp(None);
            }
        }
        let Some(world_db) = self.world_db().map(Arc::clone) else {
            return plan;
        };

        let data_dir = self.mmap_runtime_config_like_cpp().data_dir.clone();
        let locale = self.locale.clone();
        let taxi_path_nodes = match TaxiPathNodeStore::load(&data_dir, &locale) {
            Ok(store) => store,
            Err(error) => {
                warn!(
                    map_id,
                    data_dir,
                    locale,
                    %error,
                    "RUST_LOGIN send_init_transports skipped: TaxiPathNode.db2 load failed"
                );
                return plan;
            }
        };

        let mut nodes_by_path: HashMap<u16, Vec<TaxiPathNodeEntry>> = HashMap::new();
        for node in taxi_path_nodes.entries() {
            nodes_by_path
                .entry(node.path_id)
                .or_default()
                .push(node.clone());
        }

        let mut result = match world_db
            .direct_query(
                "SELECT t.guid, t.entry, t.phaseUseFlags, t.phaseid, t.phasegroup, \
                 gt.displayId, gt.size, gt.Data0, gt.Data1, gt.Data2, gt.Data8, \
                 COALESCE(goo.flags, gta.flags, 0), COALESCE(goo.faction, gta.faction, 0) \
                 FROM transports t \
                 JOIN gameobject_template gt ON gt.entry = t.entry \
                 LEFT JOIN gameobject_template_addon gta ON gta.entry = t.entry \
                 LEFT JOIN gameobject_overrides goo ON goo.spawnId = t.guid \
                 WHERE gt.type = 15 \
                 ORDER BY t.guid",
            )
            .await
        {
            Ok(result) => result,
            Err(error) => {
                warn!(
                    map_id,
                    %error,
                    "RUST_LOGIN send_init_transports skipped: DB query failed"
                );
                return plan;
            }
        };

        if result.is_empty() {
            return plan;
        }

        let mut transports = Vec::new();
        loop {
            transports.push(map_transport_create_from_row_like_cpp(&result));

            if !result.next_row() {
                break;
            }
        }

        let player_transport_guid = self.player_transport_guid_like_cpp();

        for transport in transports {
            let transport_guid =
                ObjectGuid::create_transport(HighGuid::Transport, transport.guid_low as i64);
            if player_transport_guid == Some(transport_guid) {
                // The validated own transport was materialized above without a
                // second fallible query/path load.
                continue;
            }
            let Some(nodes) = nodes_by_path.get(&transport.taxi_path_id) else {
                plan.skipped_missing_path += 1;
                continue;
            };
            let Some(path_position) = transport_position_for_login_like_cpp(
                nodes,
                transport.move_speed,
                transport.accel_rate,
                now_ms,
            ) else {
                plan.skipped_missing_path += 1;
                continue;
            };

            if path_position.map_id != map_id {
                plan.skipped_other_map += 1;
                continue;
            }

            let (target_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
                map_id,
                transport.phase_use_flags,
                transport.phase_id,
                transport.phase_group_id,
                -1,
            );
            if !self.should_send_init_transport_like_cpp(transport_guid, &target_phase_shift) {
                plan.skipped_phase += 1;
                continue;
            }

            plan.considered += 1;
            plan.other_blocks.push(map_transport_create_block_like_cpp(
                transport,
                path_position,
                now_ms,
            ));
            plan.other_visible_guids.push(transport_guid);
        }

        info!(
            map_id,
            own_transport = plan.own_transport.is_some(),
            other_blocks = plan.other_blocks.len(),
            considered = plan.considered,
            skipped_other_map = plan.skipped_other_map,
            skipped_missing_path = plan.skipped_missing_path,
            skipped_phase = plan.skipped_phase,
            "RUST_LOGIN send_init_transports plan"
        );

        plan
    }

    /// C++ `Map::SendInitTransports`: after `SendInitSelf`, send map
    /// transports other than the player's current transport.
    fn send_init_transports_like_cpp(&mut self, map_id: u16, plan: Box<InitTransportsPlanLikeCpp>) {
        if plan.other_blocks.is_empty() {
            return;
        }

        let InitTransportsPlanLikeCpp {
            other_blocks,
            other_visible_guids,
            ..
        } = *plan;
        let update = UpdateObject::create_world_objects(other_blocks, map_id);
        if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
            for line in update.debug_create_summary_like_cpp() {
                info!("RUST_UPDATEOBJECT init_transports {line}");
            }
        }
        self.send_packet(&update);
        self.client_visible_transports_like_cpp
            .extend(other_visible_guids);
    }

    pub(super) fn materialize_creature_spawn_row_like_cpp(
        &mut self,
        map_id: u16,
        row: &SqlResult,
        viewer_position: &Position,
        visibility_range: f32,
    ) -> Option<MaterializedCreatureSpawnLikeCpp> {
        // C++ ObjectMgr::LoadCreatures materializes CreatureData once, then
        // Creature::LoadFromDB consumes that data for create/update fields.
        // Rust still queries lazily by visibility; keep a single row reader so
        // login and movement refresh cannot drift from each other.
        let spawn_guid: u64 = row
            .try_read::<i64>(0)
            .map(|value| value as u64)
            .or_else(|| row.try_read::<u64>(0))
            .unwrap_or(0);
        let entry: u32 = row.try_read(1).unwrap_or(0);
        let pos_x: f32 = row.try_read(2).unwrap_or(0.0);
        let pos_y: f32 = row.try_read(3).unwrap_or(0.0);
        let pos_z: f32 = row.try_read(4).unwrap_or(0.0);
        let orientation: f32 = row.try_read(5).unwrap_or(0.0);
        if !is_within_2d_visibility_range_like_cpp(viewer_position, pos_x, pos_y, visibility_range)
        {
            return None;
        }
        let spawn_difficulties: String = row
            .try_read::<Option<String>>(CREATURE_SPAWN_DIFFICULTIES_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<String>(CREATURE_SPAWN_DIFFICULTIES_COLUMN))
            .unwrap_or_default();
        if !spawn_difficulties_contains_spawn_mode_like_cpp(
            &spawn_difficulties,
            self.current_map_difficulty_id_like_cpp(),
        ) {
            return None;
        }

        let cur_health: u32 = row.try_read(6).unwrap_or(100);
        let cur_mana: u32 = row.try_read(7).unwrap_or(0);
        let model_id: u32 = row.try_read(8).unwrap_or(0);
        let min_level: u8 = row.try_read::<Option<u8>>(9).flatten().unwrap_or(1);
        let faction: i32 = row.try_read::<u16>(11).unwrap_or(35) as i32;
        let template_npc_flags: u64 = row
            .try_read::<i64>(12)
            .map(|value| value as u64)
            .or_else(|| row.try_read::<u64>(12))
            .unwrap_or(0);
        let template_unit_flags: u32 = row.try_read(13).unwrap_or(0);
        let template_unit_flags2: u32 = row.try_read(14).unwrap_or(0);
        let template_unit_flags3: u32 = row.try_read(15).unwrap_or(0);
        let speed_walk: f32 =
            normalize_creature_template_speed_walk_like_cpp(row.try_read(16).unwrap_or(1.0));
        let speed_run: f32 =
            normalize_creature_template_speed_run_like_cpp(row.try_read(17).unwrap_or(1.14286));
        let scale: f32 = row.try_read(18).unwrap_or(1.0);
        let unit_class: u8 = row.try_read(19).unwrap_or(1);
        let flags_extra: u32 = row.try_read(20).unwrap_or(0);
        let (npc_flags, unit_flags, unit_flags2, unit_flags3) = choose_creature_flags_like_cpp(
            template_npc_flags,
            template_unit_flags,
            template_unit_flags2,
            template_unit_flags3,
            optional_u64_column_like_cpp(row, CREATURE_SPAWN_NPC_FLAGS_OVERRIDE_COLUMN),
            optional_u32_column_like_cpp(row, CREATURE_SPAWN_UNIT_FLAGS_OVERRIDE_COLUMN),
            optional_u32_column_like_cpp(row, CREATURE_SPAWN_UNIT_FLAGS2_OVERRIDE_COLUMN),
            optional_u32_column_like_cpp(row, CREATURE_SPAWN_UNIT_FLAGS3_OVERRIDE_COLUMN),
            flags_extra,
        );
        let classification: u32 = row
            .try_read::<u32>(CREATURE_SPAWN_CLASSIFICATION_COLUMN)
            .unwrap_or(0);
        let regen_health: bool = row
            .try_read::<u8>(CREATURE_SPAWN_REGEN_HEALTH_COLUMN)
            .map(|value| value != 0)
            .or_else(|| {
                row.try_read::<i8>(CREATURE_SPAWN_REGEN_HEALTH_COLUMN)
                    .map(|value| value != 0)
            })
            .unwrap_or(true);
        let base_attack_time: u32 = row.try_read(21).unwrap_or(2000);
        let template_display_id: u32 = row.try_read::<Option<u32>>(23).flatten().unwrap_or(0);
        let template_display_scale: f32 = row
            .try_read::<Option<f32>>(CREATURE_SPAWN_DISPLAY_SCALE_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<f32>(CREATURE_SPAWN_DISPLAY_SCALE_COLUMN))
            .unwrap_or(1.0);
        let loot_id: u32 = row.try_read::<Option<u32>>(24).flatten().unwrap_or(0);
        let skin_loot_id: u32 = row.try_read::<Option<u32>>(25).flatten().unwrap_or(0);
        let gold_min: u32 = row.try_read::<Option<u32>>(26).flatten().unwrap_or(0);
        let gold_max: u32 = row.try_read::<Option<u32>>(27).flatten().unwrap_or(0);
        let respawn_delay_secs: u32 = row
            .try_read::<Option<u32>>(CREATURE_SPAWN_RESPAWN_DELAY_SECS_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<u32>(CREATURE_SPAWN_RESPAWN_DELAY_SECS_COLUMN))
            .unwrap_or(wow_entities::DEFAULT_RESPAWN_DELAY_SECS);
        let script_name: String = row
            .try_read::<Option<String>>(CREATURE_SPAWN_SCRIPT_NAME_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<String>(CREATURE_SPAWN_SCRIPT_NAME_COLUMN))
            .unwrap_or_default();
        let string_id: Option<String> = row
            .try_read::<Option<String>>(CREATURE_SPAWN_STRING_ID_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<String>(CREATURE_SPAWN_STRING_ID_COLUMN))
            .filter(|value| !value.is_empty());
        let vehicle_id: u32 = row
            .try_read::<Option<u32>>(CREATURE_SPAWN_VEHICLE_ID_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<u32>(CREATURE_SPAWN_VEHICLE_ID_COLUMN))
            .unwrap_or(0);
        let phase_use_flags: u8 = row
            .try_read::<u8>(28)
            .or_else(|| row.try_read::<i16>(28).map(|value| value.max(0) as u8))
            .unwrap_or(0);
        let phase_id: u16 = row
            .try_read::<u16>(29)
            .or_else(|| row.try_read::<i32>(29).map(|value| value.max(0) as u16))
            .unwrap_or(0);
        let phase_group_id: u32 = row
            .try_read::<u32>(30)
            .or_else(|| row.try_read::<i32>(30).map(|value| value.max(0) as u32))
            .unwrap_or(0);
        let terrain_swap_map: i32 = row.try_read(31).unwrap_or(-1);
        let ground_movement_type: u8 = row
            .try_read::<Option<u8>>(32)
            .flatten()
            .or_else(|| row.try_read::<u8>(32))
            .or_else(|| row.try_read::<i16>(32).map(|value| value.max(0) as u8))
            .unwrap_or(wow_constants::CreatureGroundMovementType::Run as u8);
        let swim_allowed: bool = row
            .try_read::<Option<u8>>(33)
            .flatten()
            .or_else(|| row.try_read::<u8>(33))
            .or_else(|| row.try_read::<i16>(33).map(|value| value.max(0) as u8))
            .unwrap_or(1)
            != 0;
        let flight_movement_type: u8 = row
            .try_read::<Option<u8>>(34)
            .flatten()
            .or_else(|| row.try_read::<u8>(34))
            .or_else(|| row.try_read::<i16>(34).map(|value| value.max(0) as u8))
            .unwrap_or(0);
        let rooted: bool = row
            .try_read::<Option<u8>>(CREATURE_SPAWN_ROOTED_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<u8>(CREATURE_SPAWN_ROOTED_COLUMN))
            .or_else(|| {
                row.try_read::<i16>(CREATURE_SPAWN_ROOTED_COLUMN)
                    .map(|value| value.max(0) as u8)
            })
            .unwrap_or(0)
            != 0;
        let chase_movement_type: u8 = row
            .try_read::<Option<u8>>(CREATURE_SPAWN_CHASE_MOVEMENT_TYPE_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<u8>(CREATURE_SPAWN_CHASE_MOVEMENT_TYPE_COLUMN))
            .or_else(|| {
                row.try_read::<i16>(CREATURE_SPAWN_CHASE_MOVEMENT_TYPE_COLUMN)
                    .map(|value| value.max(0) as u8)
            })
            .map(normalize_creature_chase_movement_type_like_cpp)
            .unwrap_or(wow_constants::CreatureChaseMovementType::Run as u8);
        let random_movement_type: u8 = row
            .try_read::<Option<u8>>(CREATURE_SPAWN_RANDOM_MOVEMENT_TYPE_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<u8>(CREATURE_SPAWN_RANDOM_MOVEMENT_TYPE_COLUMN))
            .or_else(|| {
                row.try_read::<i16>(CREATURE_SPAWN_RANDOM_MOVEMENT_TYPE_COLUMN)
                    .map(|value| value.max(0) as u8)
            })
            .map(normalize_creature_random_movement_type_like_cpp)
            .unwrap_or(CreatureRandomMovementType::Walk as u8);
        let interaction_pause_timer_ms: u32 = row
            .try_read::<Option<u32>>(CREATURE_SPAWN_INTERACTION_PAUSE_TIMER_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<u32>(CREATURE_SPAWN_INTERACTION_PAUSE_TIMER_COLUMN))
            .unwrap_or(wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP);
        let wander_distance: f32 = row
            .try_read::<Option<f32>>(CREATURE_SPAWN_WANDER_DISTANCE_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<f32>(CREATURE_SPAWN_WANDER_DISTANCE_COLUMN))
            .unwrap_or(0.0)
            .max(0.0);
        let default_movement_type = row
            .try_read::<Option<u8>>(CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<u8>(CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN))
            .or_else(|| {
                row.try_read::<i16>(CREATURE_SPAWN_EFFECTIVE_MOVEMENT_TYPE_COLUMN)
                    .map(|value| value.max(0) as u8)
            })
            .map(|movement_type| {
                creature_movement_generator_type_from_db_like_cpp(movement_type, wander_distance)
            })
            .unwrap_or(MovementGeneratorType::Idle);
        let wander_distance =
            normalized_creature_wander_distance_like_cpp(default_movement_type, wander_distance);
        let waypoint_path_id: u32 = row
            .try_read::<Option<u32>>(CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN)
            .flatten()
            .or_else(|| row.try_read::<u32>(CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN))
            .or_else(|| {
                row.try_read::<i64>(CREATURE_SPAWN_WAYPOINT_PATH_ID_COLUMN)
                    .map(|value| value.max(0) as u32)
            })
            .unwrap_or(0);

        let Some(display_selection) = self.choose_creature_display_like_cpp(
            entry,
            model_id,
            flags_extra,
            template_display_id,
            template_display_scale,
        ) else {
            return None;
        };
        let display_id = display_selection.display_id;
        let Some(model_scalars) = self.creature_create_model_scalars_like_cpp(
            display_id,
            scale,
            display_selection.display_scale,
        ) else {
            warn!(
                "Skipping creature entry={} spawn={} display={} because creature_model_info is missing, matching C++ CreateFromProto failure",
                entry, spawn_guid, display_id
            );
            return None;
        };

        let (target_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
            map_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
        );
        if !self.can_see_phase_shift_like_cpp(&target_phase_shift) {
            return None;
        }

        let creature_stats = self.creature_create_stats_like_cpp(
            entry,
            min_level,
            unit_class,
            classification,
            regen_health,
            cur_health,
            cur_mana,
        );
        let addon = self
            .creature_addon_store_like_cpp()
            .and_then(|store| store.get_for_creature_like_cpp(spawn_guid, entry));
        let addon_fields = Self::creature_addon_create_fields_like_cpp(addon.as_ref());
        let equipment_fields = self.creature_virtual_items_from_row_like_cpp(entry, row);

        let guid = if vehicle_id != 0 {
            ObjectGuid::create_vehicle_like_cpp(self.realm_id(), map_id, entry, spawn_guid as i64)
        } else {
            ObjectGuid::create_creature_like_cpp(self.realm_id(), map_id, entry, spawn_guid as i64)
        };
        let movement_flags = creature_create_movement_flags_like_cpp(ground_movement_type, rooted);
        let position = creature_create_position_after_hover_offset_like_cpp(
            Position::new(pos_x, pos_y, pos_z, orientation),
            movement_flags,
            model_scalars.hover_height,
        );
        let create_data = CreatureCreateData {
            guid,
            entry,
            display_id,
            native_display_id: display_id,
            display_scale: model_scalars.display_scale,
            native_x_display_scale: model_scalars.native_x_display_scale,
            bounding_radius: model_scalars.bounding_radius,
            combat_reach: model_scalars.combat_reach,
            health: creature_stats.health,
            max_health: creature_stats.max_health,
            level: min_level,
            faction_template: faction,
            npc_flags,
            unit_flags,
            unit_flags2,
            unit_flags3,
            aura_state: crate::map_manager::WorldCreature::health_aura_state_like_cpp(
                creature_stats.health.max(0) as u64,
                creature_stats.max_health.max(0) as u64,
                creature_stats.health > 0,
            ),
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale,
            unit_class,
            display_power: creature_stats.power_type as u8,
            power: {
                let mut power = [0; 10];
                power[0] = creature_stats.power;
                power
            },
            max_power: {
                let mut max_power = [0; 10];
                max_power[0] = creature_stats.max_power;
                max_power
            },
            base_mana: creature_stats.base_mana,
            virtual_items: equipment_fields.virtual_items,
            base_attack_time,
            ranged_attack_time: row.try_read(22).unwrap_or(base_attack_time),
            movement_flags,
            vehicle_id,
            play_hover_anim: false,
            hover_height: model_scalars.hover_height,
            mount_display_id: addon_fields.mount_display_id,
            stand_state: addon_fields.stand_state,
            vis_flags: addon_fields.vis_flags,
            anim_tier: addon_fields.anim_tier,
            emote_state: addon_fields.emote_state,
            sheathe_state: addon_fields.sheathe_state,
            pvp_flags: addon_fields.pvp_flags,
            current_area_id: 0,
            speed_walk_rate: speed_walk,
            speed_run_rate: speed_run,
            ai_anim_kit_id: addon_fields.ai_anim_kit_id,
            movement_anim_kit_id: addon_fields.movement_anim_kit_id,
            melee_anim_kit_id: addon_fields.melee_anim_kit_id,
        };

        let aggro_radius =
            self.creature_aggro_radius_for_faction_template_like_cpp(faction.max(0) as u32, 15.0);
        let min_damage = (min_level as u32).saturating_sub(1) * 3 + 5;
        let max_damage = min_damage + min_damage / 2;

        Some(MaterializedCreatureSpawnLikeCpp {
            guid,
            position,
            create_data,
            min_damage,
            max_damage,
            aggro_radius,
            loot_id,
            skin_loot_id,
            gold_min,
            gold_max,
            respawn_delay_secs,
            selected_equipment_id: equipment_fields.selected_equipment_id,
            original_equipment_id: equipment_fields.original_equipment_id,
            script_name,
            string_id,
            addon,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
            flags_extra,
            ground_movement_type,
            swim_allowed,
            flight_movement_type,
            rooted,
            chase_movement_type,
            random_movement_type,
            interaction_pause_timer_ms,
            wander_distance,
            default_movement_type,
            waypoint_path_id,
        })
    }

    pub(super) fn register_materialized_creature_spawn_like_cpp(
        &mut self,
        map_id: u16,
        spawn: &MaterializedCreatureSpawnLikeCpp,
    ) {
        self.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
            map_id,
            spawn.position,
            spawn.create_data.clone(),
            spawn.min_damage,
            spawn.max_damage,
            spawn.aggro_radius,
            spawn.loot_id,
            spawn.skin_loot_id,
            spawn.gold_min,
            spawn.gold_max,
            spawn.respawn_delay_secs,
            spawn.selected_equipment_id,
            spawn.original_equipment_id,
            spawn.script_name.clone(),
            spawn.string_id.clone(),
            spawn.addon.clone(),
            None,
            0,
            spawn.phase_use_flags,
            spawn.phase_id,
            spawn.phase_group_id,
            spawn.terrain_swap_map,
            spawn.flags_extra,
            spawn.ground_movement_type,
            spawn.swim_allowed,
            spawn.flight_movement_type,
            spawn.rooted,
            spawn.chase_movement_type,
            spawn.random_movement_type,
            spawn.interaction_pause_timer_ms,
            spawn.wander_distance,
            spawn.default_movement_type,
            spawn.waypoint_path_id,
        );
    }

    /// Handle CMSG_PING — respond with Pong containing the serial.
    pub async fn handle_ping(&mut self, ping: wow_packet::packets::auth::Ping) {
        trace!(
            "Ping: serial={}, latency={}ms for account {}",
            ping.serial, ping.latency, self.account_id
        );
        self.send_packet(&wow_packet::packets::auth::Pong {
            serial: ping.serial,
        });
    }

    pub(crate) fn build_condition_player_object_like_cpp(&self) -> Option<WorldObject> {
        let mut player = WorldObject::new(
            false,
            TypeId::Player,
            TypeMask::OBJECT | TypeMask::UNIT | TypeMask::PLAYER,
        );
        player.object_mut().create(self.player_guid()?);
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let _ = player.set_map(u32::from(self.player_map_id_like_cpp()), instance_id);
        let (zone_id, area_id) = self.player_zone_area_like_cpp();
        player.set_zone_and_area(zone_id, area_id);
        if let Some(position) = self.player_position_like_cpp() {
            player.relocate(position);
        }
        Some(player)
    }

    pub(crate) fn build_condition_creature_object_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
    ) -> Option<(WorldObject, crate::conditions::ConditionUnitSnapshot)> {
        self.mutate_world_creature(npc_guid, |creature| {
            let mut source =
                WorldObject::new(false, TypeId::Unit, TypeMask::OBJECT | TypeMask::UNIT);
            source.object_mut().create(creature.guid());
            source.object_mut().set_entry(creature.entry());
            let _ = source.set_map(creature.map_id(), creature.instance_id());
            source.relocate(creature.position());
            *source.phase_shift_mut() = creature.phase_shift().clone();
            let snapshot = crate::conditions::ConditionUnitSnapshot {
                level: u32::from(creature.level()),
                health: u64::from(creature.current_hp()),
                max_health: u64::from(creature.max_hp()),
                class_mask: 0,
                race: 0,
                creature_type: None,
                is_alive: creature.is_alive(),
                is_charmed: false,
                in_water: false,
                unit_state: 0,
                stand_state: UnitStandStateType::Stand as u32,
            };
            (source, snapshot)
        })
    }

    pub(crate) fn condition_player_unit_snapshot_like_cpp(
        &self,
    ) -> crate::conditions::ConditionUnitSnapshot {
        crate::conditions::ConditionUnitSnapshot {
            level: u32::from(self.player_level_like_cpp()),
            health: u64::from(self.player_health_like_cpp()),
            max_health: u64::from(self.player_max_health_like_cpp()),
            class_mask: player_class_mask(self.player_class_like_cpp()),
            race: self.player_race_like_cpp(),
            creature_type: None,
            is_alive: self.player_is_alive_like_cpp(),
            is_charmed: false,
            in_water: false,
            unit_state: 0,
            stand_state: UnitStandStateType::Stand as u32,
        }
    }

    pub(crate) fn condition_player_snapshot_like_cpp(
        &self,
    ) -> crate::conditions::ConditionPlayerSnapshot {
        crate::conditions::ConditionPlayerSnapshot {
            team: player_team_for_race_cpp(self.player_race_like_cpp()) as u32,
            native_gender: u32::from(self.player_gender_like_cpp()),
            drunken_state: 0,
            can_be_game_master: false,
            is_game_master: false,
            pet_type: None,
            is_in_flight: self.is_in_taxi_flight_like_cpp(),
        }
    }

    /// Direct interaction for NPCs without gossip menus (banker, auctioneer, etc.).
    pub(super) async fn handle_npc_direct_interaction(&mut self, hello: Hello, npc_flags: u32) {
        use wow_packet::packets::misc::{AuctionHelloResponse, NpcInteractionOpenResult};

        // This is Rust's shortcut for C++ `PrepareGossipMenu` followed by a
        // built-in gossip option. C++ `SendGossipMenu` first replaces the
        // complete InteractionData with this validated source, even when the
        // service subsequently emits a dedicated packet.
        self.set_player_interaction_source_like_cpp(hello.unit);

        // This shortcut represents selecting one of C++'s built-in gossip
        // options immediately after publishing it. `HandleGossipSelectOptionOpcode`
        // removes fake death after source validation and before dispatching the
        // service. Keep the empty-menu fallback out of that transition.
        if npc_has_direct_interaction_like_cpp(npc_flags) {
            self.remove_represented_feign_death_if_needed_like_cpp();
        }

        if npc_flags & DIRECT_VENDOR_MASK_LIKE_CPP != 0 {
            self.handle_list_inventory(hello).await;
        } else if npc_flags & DIRECT_TRAINER_MASK_LIKE_CPP != 0 {
            self.handle_trainer_list(hello).await;
        } else if npc_flags & DIRECT_AUCTIONEER_LIKE_CPP != 0 {
            self.send_packet(&AuctionHelloResponse::open(hello.unit));
        } else if npc_flags & DIRECT_BANKER_LIKE_CPP != 0 {
            self.send_show_bank_like_cpp(hello.unit);
        } else if npc_flags & DIRECT_FLIGHT_MASTER_LIKE_CPP != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 6));
        } else if npc_flags & DIRECT_TABARD_DESIGNER_LIKE_CPP != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 14));
        } else if npc_flags & DIRECT_STABLE_MASTER_LIKE_CPP != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 22));
        } else if npc_flags & DIRECT_GUILD_BANKER_LIKE_CPP != 0 {
            self.send_packet(&NpcInteractionOpenResult::new(hello.unit, 10));
        } else {
            self.send_packet(&GossipMessage::empty(hello.unit, 0, 1));
        }
    }

    /// CMSG_AUCTION_HELLO_REQUEST — player talks to an auctioneer.
    /// C++ refs: `HandleAuctionHelloOpcode` / `SendAuctionHello`
    /// (`Handlers/AuctionHouseHandler.cpp:192-205,995-1007`).
    pub async fn handle_auction_hello_request(&mut self, mut pkt: wow_packet::WorldPacket) {
        use wow_packet::packets::misc::AuctionHelloResponse;
        let guid = pkt
            .read_packed_guid()
            .unwrap_or(wow_core::ObjectGuid::EMPTY);
        info!(
            "AuctionHelloRequest from {:?} account {}",
            guid, self.account_id
        );
        self.send_packet(&AuctionHelloResponse::open(guid));
    }

    /// CMSG_BINDER_ACTIVATE — player sets hearthstone at innkeeper.
    /// C++ refs: `WorldSession::HandleBinderActivateOpcode` /
    /// `WorldSession::SendBindPoint` (`Handlers/NPCHandler.cpp:373-402`).
    pub async fn handle_binder_activate(&mut self, hello: Hello) {
        info!(
            "BinderActivate {:?} account {}",
            hello.unit, self.account_id
        );
        if !self.player_is_strictly_in_world_like_cpp() || !self.player_is_alive_like_cpp() {
            return;
        }
        let Some(_innkeeper) = self.represented_npc_can_interact_with_like_cpp(
            hello.unit,
            NPCFlags1::INNKEEPER.bits(),
            0,
        ) else {
            debug!(
                innkeeper_guid = ?hello.unit,
                account = self.account_id,
                "BinderActivate rejected: NPC missing, out of range, dead, or lacks INNKEEPER flag"
            );
            return;
        };
        // C++ HandleBinderActivateOpcode removes feign death before
        // SendBindPoint performs its instanceable-map rejection.
        self.remove_represented_feign_death_if_needed_like_cpp();
        if self.player_current_map_instanceable_like_cpp() {
            debug!(
                innkeeper_guid = ?hello.unit,
                map_id = self.player_map_id_like_cpp(),
                "BinderActivate rejected: current map is instanceable like C++ SendBindPoint"
            );
            return;
        }

        // C++ SendBindPoint calls innkeeper->CastSpell(player, 3286, true).
        // Route the triggered creature cast through the represented spell
        // pipeline so SpellGo, EffectBind, persistence, and bind packets keep
        // their C++ ordering and caster identity.
        const BIND_SPELL_ID_LIKE_CPP: i32 = 3286;
        const CAST_FLAG_PENDING_LIKE_CPP: u32 = 0x0000_0001;
        const CAST_FLAG_UNKNOWN_9_LIKE_CPP: u32 = 0x0000_0100;
        const CAST_FLAG_NO_GCD_LIKE_CPP: u32 = 0x0004_0000;
        const BIND_SPELL_GO_CAST_FLAGS_LIKE_CPP: u32 =
            CAST_FLAG_UNKNOWN_9_LIKE_CPP | CAST_FLAG_PENDING_LIKE_CPP | CAST_FLAG_NO_GCD_LIKE_CPP;
        if let Some(player_guid) = self.player_guid() {
            let cast_id = self.next_represented_spell_cast_guid_like_cpp(BIND_SPELL_ID_LIKE_CPP);
            if let Err(error) = self
                .execute_spell_with_visual_and_target_data_with_metadata(
                    BIND_SPELL_ID_LIKE_CPP,
                    player_guid,
                    cast_id,
                    SpellCastVisual::default(),
                    SpellTargetData {
                        flags: 0x2,
                        unit: player_guid,
                        item: ObjectGuid::EMPTY,
                        ..SpellTargetData::default()
                    },
                    SpellCastMetadata {
                        caster_guid_override: Some(hello.unit),
                        // C++ Spell::SendSpellGo starts with UNKNOWN_9, adds
                        // PENDING for this non-client triggered cast, and
                        // adds NO_GCD because spell 3286 has no
                        // StartRecoveryTime row.
                        cast_flags: BIND_SPELL_GO_CAST_FLAGS_LIKE_CPP,
                        ..SpellCastMetadata::default()
                    },
                )
                .await
            {
                warn!(
                    innkeeper_guid = ?hello.unit,
                    account = self.account_id,
                    error,
                    "BinderActivate bind spell failed"
                );
            }
        }
        // C++ closes gossip after attempting the triggered cast, even if the
        // spell execution itself cannot complete.
        self.send_close_gossip_like_cpp();
    }

    /// Shared C++ area-spirit-healer checks: creature exists, has the area
    /// spirit-healer flag, and is within MAX_AREA_SPIRIT_HEALER_RANGE.
    pub(super) fn represented_area_spirit_healer_access_like_cpp(
        &self,
        healer_guid: ObjectGuid,
    ) -> Option<crate::session::RepresentedCreatureAccessLikeCpp> {
        let access = self.canonical_creature_access_like_cpp(healer_guid)?;
        if (access.npc_flags & NPCFlags1::AREA_SPIRIT_HEALER.bits()) == 0 {
            return None;
        }

        let player_position = self.player_position_like_cpp()?;
        access
            .position
            .is_within_dist(&player_position, MAX_AREA_SPIRIT_HEALER_RANGE_LIKE_CPP)
            .then_some(access)
    }

    /// CMSG_AREA_SPIRIT_HEALER_QUEUE — select an area spirit healer for resurrection.
    /// C++ ref: `WorldSession::HandleAreaSpiritHealerQueueOpcode`.
    pub async fn handle_area_spirit_healer_queue(&mut self, mut pkt: wow_packet::WorldPacket) {
        let queue = match AreaSpiritHealerQueue::read(&mut pkt) {
            Ok(queue) => queue,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "AreaSpiritHealerQueue parse failed: {error}"
                );
                return;
            }
        };

        if self
            .represented_area_spirit_healer_access_like_cpp(queue.healer_guid)
            .is_none()
        {
            debug!(
                account = self.account_id,
                healer = ?queue.healer_guid,
                "AreaSpiritHealerQueue ignored without represented area spirit healer"
            );
            return;
        }

        // C++ also casts SPELL_WAITING_FOR_RESURRECT; deferred until the
        // player spell/aura runtime owns battleground spirit resurrection.
        self.set_area_spirit_healer_guid_like_cpp(queue.healer_guid);
    }

    /// CMSG_SPIRIT_HEALER_ACTIVATE — ghost uses spirit healer.
    /// C++ ref: `WorldSession::HandleSpiritHealerActivate`.
    pub async fn handle_spirit_healer_activate(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match SpiritHealerActivate::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "SpiritHealerActivate parse failed: {error}"
                );
                return;
            }
        };

        let Some(_healer) = self.represented_npc_can_interact_with_like_cpp(
            request.healer,
            NPCFlags1::SPIRIT_HEALER.bits(),
            0,
        ) else {
            debug!(
                account = self.account_id,
                healer = ?request.healer,
                "SpiritHealerActivate ignored without represented spirit healer"
            );
            return;
        };

        // C++ continues into SendSpiritResurrect here: resurrect 50%, durability
        // loss, corpse-bones spawn, and possible graveyard teleport. That player
        // corpse/death runtime is not represented in this handler yet.
        debug!(
            account = self.account_id,
            healer = ?request.healer,
            "SpiritHealerActivate validated; resurrection runtime pending"
        );
    }

    /// CMSG_REQUEST_STABLED_PETS — player opens stable master UI.
    /// C++ ref: `WorldSession::HandleRequestStabledPets`.
    pub async fn handle_request_stabled_pets(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match RequestStabledPets::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "RequestStabledPets parse failed: {error}"
                );
                return;
            }
        };

        // C++ returns before sending anything when CheckStableMaster fails.
        // The live stable-master validation and Player::SetStableMaster update
        // fields are not ported here yet, so preserve that observable branch.
        debug!(
            account = self.account_id,
            stable_master = ?request.stable_master,
            "RequestStabledPets ignored without represented stable-master runtime"
        );
    }

    pub(super) fn collect_quest_giver_status_multiple_like_cpp(
        &self,
        guids: impl IntoIterator<Item = ObjectGuid>,
    ) -> Vec<(ObjectGuid, u64)> {
        let mut statuses = Vec::new();

        for guid in guids {
            if guid.is_any_type_creature() {
                let Some(access) = self.canonical_creature_access_like_cpp(guid) else {
                    continue;
                };
                if (access.npc_flags & NPCFlags1::QUEST_GIVER.bits()) == 0 {
                    continue;
                }

                let status = self.get_represented_quest_giver_status_like_cpp(
                    RepresentedQuestGiverStatusSourceLikeCpp::Creature {
                        entry: access.entry,
                    },
                );
                statuses.push((guid, status));
                continue;
            }

            if guid.is_game_object() {
                let Some(access) = self.canonical_gameobject_access_like_cpp(guid) else {
                    continue;
                };
                let Some(state) = self.represented_gameobject_use_states.get(&guid) else {
                    continue;
                };
                if state.go_type.map(u32::from) != Some(GAMEOBJECT_TYPE_QUESTGIVER) {
                    continue;
                }

                let status = self.get_represented_quest_giver_status_like_cpp(
                    RepresentedQuestGiverStatusSourceLikeCpp::GameObject {
                        entry: access.entry,
                    },
                );
                statuses.push((guid, status));
            }
        }

        statuses
    }

    /// Send SMSG_QUEST_GIVER_STATUS for a single NPC.
    #[allow(dead_code)]
    fn send_quest_giver_status(&self, guid: ObjectGuid, status: u32) {
        use wow_constants::ServerOpcodes;
        let mut pkt = wow_packet::WorldPacket::new_server(ServerOpcodes::QuestGiverStatus);
        pkt.write_packed_guid(&guid);
        pkt.write_uint32(status);
        self.send_raw_packet(&pkt.into_data());
    }

    // ── Item equip/swap handlers ─────────────────────────────────────

    pub(super) fn represented_player_gear_stats_like_cpp(
        &self,
        include_represented_item_bonuses: bool,
    ) -> RepresentedPlayerGearStatsLikeCpp {
        let mut gear = RepresentedPlayerGearStatsLikeCpp::default();
        if let Some(item_stats_store) = self.item_stats_store() {
            for (&slot, inventory_item) in self.inventory_items_like_cpp() {
                if slot >= 19 {
                    continue;
                }
                let Some(entry) = item_stats_store.get(inventory_item.entry_id) else {
                    continue;
                };
                let base_stats = entry.base_stat_bonuses();
                for (target, amount) in gear.stats.iter_mut().zip(base_stats) {
                    *target = target.saturating_add(amount);
                }
                gear.attack_power = gear.attack_power.saturating_add(entry.attack_power_bonus());
                gear.ranged_attack_power = gear
                    .ranged_attack_power
                    .saturating_add(entry.ranged_attack_power_bonus());
                gear.health = gear.health.saturating_add(entry.health_bonus());
                gear.mana = gear.mana.saturating_add(entry.mana_bonus());
                for (target, amount) in gear
                    .combat_ratings
                    .iter_mut()
                    .zip(entry.combat_rating_bonuses())
                {
                    *target = target.saturating_add(amount);
                }
                gear.spell_power = gear.spell_power.saturating_add(entry.spell_power_bonus());
                gear.armor = gear.armor.saturating_add(entry.armor);
            }
        }

        if include_represented_item_bonuses {
            let bonuses = self.represented_item_bonus_state_like_cpp();
            for (target, amount) in gear.stats.iter_mut().zip(bonuses.stats_base) {
                *target = target.saturating_add(amount);
            }
            gear.attack_power = gear.attack_power.saturating_add(bonuses.attack_power_total);
            gear.ranged_attack_power = gear
                .ranged_attack_power
                .saturating_add(bonuses.ranged_attack_power_total);
            gear.health = gear.health.saturating_add(bonuses.health_base);
            gear.mana = gear.mana.saturating_add(bonuses.mana_base);
            for (target, amount) in gear.combat_ratings.iter_mut().zip(bonuses.combat_ratings) {
                *target = target.saturating_add(amount);
            }
            gear.spell_power = gear.spell_power.saturating_add(bonuses.spell_power_bonus);
            gear.armor = gear
                .armor
                .saturating_add(bonuses.armor_base)
                .saturating_add(bonuses.armor_total)
                .saturating_add(bonuses.resistances_base[0]);
            gear.mana_regen_bonus = bonuses.mana_regen_bonus;
            gear.shield_block_base_mod = bonuses.shield_block_base_mod;
            gear.shield_block_value = bonuses.shield_block_value;
        }

        gear
    }

    pub(super) fn player_stat_system_projection_like_cpp(
        &self,
        race: u8,
        class: u8,
        level: u8,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> Option<PlayerStatSystemProjectionLikeCpp> {
        let base = *self.player_stats()?.get(race, class, level)?;
        let (attack_power_per_strength, attack_power_per_agility, ranged_attack_power_per_agility) =
            self.player_class_attack_power_coefficients_like_cpp(class)?;
        let rating_bonuses = std::array::from_fn(|index| {
            gear.combat_ratings[index] as f32
                * self.combat_rating_multiplier_like_cpp(level, index as u32)
        });
        let (can_parry, can_block) = self.canonical_player_parry_block_snapshot_like_cpp();

        Some(calculate_player_stat_system_like_cpp(
            PlayerStatSystemInputLikeCpp {
                base,
                class,
                level,
                attack_power_per_strength,
                attack_power_per_agility,
                ranged_attack_power_per_agility,
                stat_total_multipliers: self.represented_total_stat_multipliers_like_cpp(),
                stat_buff_total_multipliers: self
                    .represented_total_stat_buff_multipliers_like_cpp(),
                gear_stats: gear.stats,
                gear_health: gear.health,
                gear_mana: gear.mana,
                gear_armor: gear.armor,
                gear_attack_power: gear.attack_power,
                gear_ranged_attack_power: gear.ranged_attack_power,
                rating_bonuses,
                can_parry,
                can_block,
            },
        ))
    }

    /// Recalculate all stats from base + gear.
    ///
    /// C++ `Player::UpdateAllStats` updates max power but preserves current power,
    /// clamping only when the max drops below current (`Unit::SetMaxPower`).
    pub(super) fn player_stat_changes_like_cpp(
        &mut self,
    ) -> Option<(ObjectGuid, PlayerStatChanges)> {
        self.player_stat_changes_with_represented_item_bonuses_like_cpp(false)
    }

    pub(crate) fn level_up_stat_deltas_like_cpp(&self, new_level: u8) -> Option<(i32, [i32; 5])> {
        let store = self.player_stats()?;
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let old = store.get(race, class, self.player_level_like_cpp())?;
        let new = store.get(race, class, new_level)?;
        let old_stats = old.primary_stats_like_cpp();
        let new_stats = new.primary_stats_like_cpp();
        Some((
            i32::try_from(new.base_mana)
                .unwrap_or(i32::MAX)
                .saturating_sub(i32::try_from(old.base_mana).unwrap_or(i32::MAX)),
            std::array::from_fn(|index| {
                i32::from(new_stats[index]).saturating_sub(i32::from(old_stats[index]))
            }),
        ))
    }

    /// Recalculate all stats from base + gear and send a VALUES update to the client.
    ///
    /// Called after equip/desequip changes to gear slots (0-18).
    pub(crate) fn send_stat_update(&mut self) {
        let Some((player_guid, changes)) = self.player_stat_changes_like_cpp() else {
            return;
        };

        let update =
            UpdateObject::player_stat_update(player_guid, self.player_map_id_like_cpp(), changes);
        self.send_packet(&update);
    }

    /// Recalculate stats for C++ `HandleModTotalPercentStat`.
    ///
    /// Ability auras that select stamina preserve the pre-change health
    /// percentage after max health is recalculated. Other total-stat auras use
    /// ordinary `SetMaxHealth` clamping.
    pub(crate) fn send_total_stat_percentage_update_like_cpp(&mut self, preserve_health_pct: bool) {
        let (health_before, max_health_before) = self
            .canonical_player_health_snapshot_like_cpp()
            .unwrap_or_else(|| {
                (
                    self.player_health_like_cpp(),
                    self.player_max_health_like_cpp(),
                )
            });
        let max_health_before = max_health_before.max(1);
        let zero_health = health_before == 0;
        let Some((player_guid, mut changes)) =
            self.player_stat_changes_with_represented_item_bonuses_like_cpp(true)
        else {
            return;
        };

        if preserve_health_pct {
            let max_health_after = max_health_u32_like_cpp(changes.max_health);
            let health_pct = health_before as f32 * 100.0 / max_health_before as f32;
            let restored = (max_health_after as f32 * health_pct / 100.0) as u32;
            let restored = restored.max(if zero_health { 0 } else { 1 });
            let _ = self.sync_canonical_player_health_like_cpp(restored, max_health_after);
            changes.health = i64::from(restored);
        }

        let update =
            UpdateObject::player_stat_update(player_guid, self.player_map_id_like_cpp(), changes);
        self.send_packet(&update);
    }

    /// C++ `Player::GiveLevel` refills health and powers carrying
    /// `PowerTypeFlags::SetToMaxOnLevelUp` after `UpdateAllStats`.
    ///
    /// The 3.4.3 data path represented here has mana as the refillable
    /// primary power; rage, energy and runic power preserve their current
    /// values.
    pub(crate) fn send_level_up_stat_update_like_cpp(&mut self) {
        let Some((player_guid, mut changes)) = self.player_stat_changes_like_cpp() else {
            return;
        };

        let max_health = max_health_u32_like_cpp(changes.max_health);
        let _ = self.sync_canonical_player_health_like_cpp(max_health, max_health);
        changes.health = i64::from(max_health);

        if primary_power_type_for_class_like_cpp(self.player_class_like_cpp()) == PowerType::Mana {
            changes.power0 = changes.max_power0;
            let _ = self.sync_canonical_player_primary_power_like_cpp(
                PowerType::Mana,
                changes.power0,
                changes.max_power0,
                changes.base_mana,
            );
            self.set_represented_player_power_slot_like_cpp(
                0,
                changes.power0,
                Some(changes.max_power0),
            );
        }

        let update =
            UpdateObject::player_stat_update(player_guid, self.player_map_id_like_cpp(), changes);
        self.send_packet(&update);
    }

    /// Update the realmcharacters count in the login database.
    ///
    /// Counts how many characters this account has on the character DB, then
    /// upserts the count into `realmcharacters` in the login DB.
    pub(crate) async fn update_realm_characters(&self) {
        let port = match self.player_lifecycle_port_like_cpp().map(Arc::clone) {
            Some(port) => port,
            None => return,
        };
        let request = wow_persistence::PlayerRealmCharacterCountRefreshRequestLikeCpp {
            account_id: self.account_id,
            realm_id: self.realm_id() as u32,
        };
        match port.refresh_realm_character_count_like_cpp(request).await {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {
                debug!(
                    "Updated realmcharacters: account={} realm={}",
                    self.account_id,
                    self.realm_id()
                );
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!("Failed to update realmcharacters: {reason}");
            }
        }
    }

    pub(crate) async fn load_character_spell_history_packets_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> (Vec<SpellHistoryEntry>, Vec<SpellChargeEntry>) {
        let now = unix_now_secs_like_cpp();
        let guid_counter = guid.counter() as u64;
        let port = self.player_lifecycle_port_like_cpp().map(Arc::clone);
        let mut history_entries = Vec::new();
        self.reset_represented_character_spell_cooldowns_like_cpp();

        let cooldown_outcome = match port.as_ref() {
            Some(port) => {
                port.load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCooldowns {
                        player_guid: guid_counter,
                    },
                )
                .await
            }
            None => wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "Player lifecycle port unavailable".to_owned(),
            },
        };
        match cooldown_outcome {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::SpellCooldowns(rows),
            ) => {
                for row in rows {
                    let spell_known_to_store = self.spell_store().is_none_or(|store| {
                        i32::try_from(row.spell_id)
                            .ok()
                            .is_some_and(|id| store.get(id).is_some())
                    });
                    if spell_known_to_store {
                        if let Some(entry) = spell_history_entry_from_db_like_cpp(
                            row.spell_id,
                            row.item_id,
                            row.cooldown_end,
                            row.category_id,
                            row.category_end,
                            now,
                        ) {
                            self.record_loaded_character_spell_cooldown_like_cpp(
                                row.spell_id,
                                row.item_id,
                                row.cooldown_end,
                                row.category_id,
                                row.category_end,
                            );
                            history_entries.push(entry);
                        }
                    }
                }
                self.mark_represented_character_spell_cooldowns_loaded_like_cpp();
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load spell cooldowns for {:?}: {reason}", guid);
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => warn!(
                "Failed to load spell cooldowns for {:?}: lifecycle port returned mismatched rows",
                guid
            ),
        }

        let mut charges_by_category = BTreeMap::<u32, (i64, u8)>::new();
        self.reset_represented_character_spell_charges_like_cpp();
        let charges_outcome = match port {
            Some(port) => {
                port.load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::SpellCharges {
                        player_guid: guid_counter,
                    },
                )
                .await
            }
            None => wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "Player lifecycle port unavailable".to_owned(),
            },
        };
        match charges_outcome {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::SpellCharges(rows),
            ) => {
                for row in rows {
                    let category_known_to_store = self
                        .spell_category_store()
                        .is_none_or(|store| store.get(row.category_id).is_some());
                    if category_known_to_store && row.recharge_end > now {
                        self.record_loaded_character_spell_charge_like_cpp(
                            row.category_id,
                            row.recharge_start,
                            row.recharge_end,
                        );
                        charges_by_category
                            .entry(row.category_id)
                            .and_modify(|(first_recharge_end, consumed_charges)| {
                                *first_recharge_end = (*first_recharge_end).min(row.recharge_end);
                                *consumed_charges = consumed_charges.saturating_add(1);
                            })
                            .or_insert((row.recharge_end, 1));
                    }
                }
                self.mark_represented_character_spell_charges_loaded_like_cpp();
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load spell charges for {:?}: {reason}", guid);
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => warn!(
                "Failed to load spell charges for {:?}: lifecycle port returned mismatched rows",
                guid
            ),
        }

        let charge_entries = charges_by_category
            .into_iter()
            .filter_map(|(category_id, (first_recharge_end, consumed_charges))| {
                spell_charge_entry_from_db_like_cpp(
                    category_id,
                    first_recharge_end,
                    consumed_charges,
                    now,
                )
            })
            .collect();

        (history_entries, charge_entries)
    }

    /// C++ `Player::_LoadTraits`: `CHAR_SEL_CHAR_TRAIT_CONFIGS` +
    /// `CHAR_SEL_CHAR_TRAIT_ENTRIES`, serialized in ActivePlayerData::TraitConfigs.
    pub(crate) async fn load_active_player_trait_configs_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Vec<TraitConfigCreateData> {
        self.begin_represented_trait_config_authority_load_like_cpp();
        let port = self.player_lifecycle_port_like_cpp().cloned();
        let player_guid = guid.counter() as u64;

        let mut entries_by_config = BTreeMap::<i32, Vec<TraitEntryCreateData>>::new();
        let mut entries_complete_like_cpp = false;
        let mut entries_empty_like_cpp = false;
        let entries_outcome = match port.as_ref() {
            Some(port) => {
                port.load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitEntries {
                        player_guid,
                    },
                )
                .await
            }
            None => wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "Player lifecycle port unavailable".to_owned(),
            },
        };
        match entries_outcome {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::TraitEntries(rows),
            ) => {
                entries_complete_like_cpp = true;
                entries_empty_like_cpp = rows.is_empty();
                for row in rows {
                    match (
                        row.trait_config_id,
                        row.trait_node_id,
                        row.trait_node_entry_id,
                        row.rank,
                        row.granted_ranks,
                    ) {
                        (
                            Some(trait_config_id),
                            Some(trait_node_id),
                            Some(trait_node_entry_id),
                            Some(rank),
                            Some(granted_ranks),
                        ) => {
                            entries_by_config.entry(trait_config_id).or_default().push(
                                TraitEntryCreateData {
                                    trait_node_id,
                                    trait_node_entry_id,
                                    rank,
                                    granted_ranks,
                                },
                            );
                        }
                        _ => {
                            entries_complete_like_cpp = false;
                            warn!(
                                player_guid = guid.counter(),
                                "Keeping trait-entry authority incomplete: malformed row"
                            );
                        }
                    }
                }
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    player_guid = guid.counter(),
                    "Failed to load character trait entries: {reason}"
                );
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => warn!(
                player_guid = guid.counter(),
                "Failed to load character trait entries: lifecycle port returned mismatched rows"
            ),
        }

        let mut configs_complete_like_cpp = false;
        let configs_outcome = match port {
            Some(port) => {
                port.load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::TraitConfigs {
                        player_guid,
                    },
                )
                .await
            }
            None => wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "Player lifecycle port unavailable".to_owned(),
            },
        };
        let configs = match configs_outcome {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::TraitConfigs(rows),
            ) => {
                configs_complete_like_cpp = true;
                let mut configs = Vec::new();
                for row in rows {
                    let id = row.id;
                    let config_type = row.config_type;
                    let chr_specialization_id = row.chr_specialization_id;
                    let combat_config_flags = row.combat_config_flags;
                    let local_identifier = row.local_identifier;
                    let skill_line_id = row.skill_line_id;
                    let trait_system_id = row.trait_system_id;
                    let name = row.name;
                    let type_columns_complete = match config_type {
                        Some(1) => {
                            chr_specialization_id.is_some()
                                && combat_config_flags.is_some()
                                && local_identifier.is_some()
                        }
                        Some(2) => skill_line_id.is_some(),
                        Some(3) => trait_system_id.is_some(),
                        Some(_) => true,
                        None => false,
                    };
                    match (id, config_type, name, type_columns_complete) {
                        (Some(id), Some(config_type), Some(name), true) => {
                            configs.push(TraitConfigCreateData {
                                id,
                                config_type,
                                chr_specialization_id: chr_specialization_id.unwrap_or(0),
                                combat_config_flags: combat_config_flags.unwrap_or(0),
                                local_identifier: local_identifier.unwrap_or(0),
                                skill_line_id: skill_line_id.unwrap_or(0),
                                trait_system_id: trait_system_id.unwrap_or(0),
                                name,
                                entries: entries_by_config.remove(&id).unwrap_or_default(),
                            });
                        }
                        _ => {
                            configs_complete_like_cpp = false;
                            warn!(
                                player_guid = guid.counter(),
                                "Keeping trait-config authority incomplete: malformed row"
                            );
                        }
                    }
                }
                configs
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    player_guid = guid.counter(),
                    "Failed to load character trait configs: {reason}"
                );
                Vec::new()
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    player_guid = guid.counter(),
                    "Failed to load character trait configs: lifecycle port returned mismatched rows"
                );
                Vec::new()
            }
        };

        let trait_query_authority_complete_like_cpp = entries_complete_like_cpp
            && configs_complete_like_cpp
            && self.complete_represented_trait_config_authority_load_like_cpp(
                configs.iter().map(|config| {
                    (
                        config.id,
                        config.config_type,
                        config.chr_specialization_id,
                        config.combat_config_flags,
                    )
                }),
                entries_empty_like_cpp,
            );

        if trait_query_authority_complete_like_cpp {
            let exact_traits = self
                .trait_node_entry_store()
                .zip(self.trait_definition_store())
                .map(|(node_entries, definitions)| {
                    let mut exact = BTreeMap::<i32, i32>::new();
                    for entry in configs
                        .iter()
                        .flat_map(|config| config.entries.iter())
                        .filter(|entry| entry.rank > 0 || entry.granted_ranks > 0)
                    {
                        let Some(node_entry) = u32::try_from(entry.trait_node_entry_id)
                            .ok()
                            .and_then(|id| node_entries.get(id))
                        else {
                            return None;
                        };
                        let trait_definition_id = node_entry.trait_definition_id;
                        let Some(definition) = u32::try_from(trait_definition_id)
                            .ok()
                            .and_then(|id| definitions.get(id))
                        else {
                            return None;
                        };
                        if definition.spell_id <= 0 {
                            continue;
                        }
                        if exact
                            .insert(definition.spell_id, trait_definition_id)
                            .is_some_and(|previous| previous != trait_definition_id)
                        {
                            return None;
                        }
                    }
                    Some(exact.into_iter().collect::<Vec<_>>())
                });
            if let Some(Some(exact_traits)) = exact_traits {
                if !self.set_complete_represented_spell_trait_definition_ids_like_cpp(exact_traits)
                {
                    warn!(
                        player_guid = guid.counter(),
                        "Could not authorize represented trait spell ownership"
                    );
                }
            } else {
                warn!(
                    player_guid = guid.counter(),
                    "Keeping represented trait spell ownership incomplete: missing DB2 stores"
                );
            }
        }

        info!(
            player_guid = guid.counter(),
            trait_configs = configs.len(),
            trait_entries = configs
                .iter()
                .map(|config| config.entries.len())
                .sum::<usize>(),
            "Loaded character trait configs like C++"
        );
        configs
    }

    /// C++ `Player::SendInitialPacketsBeforeAddToMap` (Player.cpp:23479-23590): the init
    /// packets sent before the player is added to the map, ending with `SetMovedUnit`
    /// (SMSG_MOVE_SET_ACTIVE_MOVER). Shared by login and far teleport
    /// (#NEXT.R8.ENTITIES.1229). Most data is read from self; the per-character items that
    /// the caller already has on hand (known/favorite spells, spell history/charges, action
    /// buttons, account mounts) plus the destination guid/position/map/zone are passed in.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn send_initial_packets_before_add_to_map(
        &mut self,
        guid: ObjectGuid,
        _position: &Position,
        _map_id: i32,
        _zone_id: i32,
        homebind: CharacterLoginLocationLikeCpp,
        known_spells: Vec<i32>,
        favorite_spells: Vec<i32>,
        spell_history_entries: Vec<SpellHistoryEntry>,
        spell_charge_entries: Vec<SpellChargeEntry>,
        action_buttons: [i64; 180],
        account_mounts: Vec<AccountMount>,
        updateobject_trace_enabled: bool,
    ) -> bool {
        if updateobject_trace_enabled {
            info!(guid = ?guid, "RUST_LOGIN before_initial_packets_before_add");
        }

        // C++ `Player::SendInitialPacketsBeforeAddToMap` resets m_movementCounter to 0 for a
        // non-seamless add (login / far teleport; Player.cpp:23483) before any control packets.
        self.reset_movement_counter_like_cpp();

        // 6. TimeSyncRequest (critical — client needs time sync)
        //    Also initializes the periodic timer (5s first, then 10s).
        self.reset_time_sync_like_cpp();
        self.send_time_sync();

        // 7. ContactList — C++ `GetSocial()->SendSocialList(this, SOCIAL_FLAG_ALL)`.
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            return false;
        }
        self.send_contact_list_like_cpp(7).await;

        // 8. BindPointUpdate — C++ `Player::SendBindPointUpdate` always uses
        // `m_homebind`/`m_homebindAreaId`, independently of the current login
        // location selected by `UpdatePositionData`.
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            return false;
        }
        self.send_packet(&login_bind_point_update_like_cpp(homebind));

        // 9. UpdateTalentData — C++ `Player::SendTalentsInfoData`.
        self.send_packet(&self.represented_update_talent_data_packet_like_cpp());

        // 10. SendKnownSpells — populated from character_spell table
        info!("Sending {} known spells for {:?}", known_spells.len(), guid);
        self.send_packet(&SendKnownSpells {
            initial_login: true,
            known_spells,
            favorite_spells,
        });

        // 11. SendUnlearnSpells (empty)
        self.send_packet(&SendUnlearnSpells);

        // 12. SendSpellHistory — C++ `SpellHistory::WritePacket`.
        self.send_packet(&SendSpellHistory {
            entries: spell_history_entries,
        });

        // 13. SendSpellCharges — C++ `SpellHistory::WritePacket`.
        self.send_packet(&SendSpellCharges {
            entries: spell_charge_entries,
        });

        // 14. ActiveGlyphs — full update; bindable spell mapping is still pending.
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            return false;
        }
        self.send_packet_realm(&self.represented_active_glyphs_packet_like_cpp());

        // 15. UpdateActionButtons — populated from character_action table
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            return false;
        }
        self.send_packet(&UpdateActionButtons {
            buttons: action_buttons,
            reason: 0, // Initialization
        });

        // 16. InitializeFactions (1000 factions, all neutral)
        let initialize_factions = self
            .reputation_mgr_like_cpp_mut()
            .initialize_factions_packet_like_cpp();
        self.send_packet(&initialize_factions);

        // 17. SetupCurrency (empty)
        self.send_packet(&SetupCurrency::empty());

        // 18. LoadEquipmentSet
        self.send_packet(&self.represented_load_equipment_set_packet_like_cpp());

        // 19. AllAchievementData — C++ `AchievementMgr::SendAllData`.
        // `QuestObjectiveCriteriaMgr::SendAllData` does not emit
        // `AllAccountCriteria` in the traced 3.4.3 login when there is no
        // progress; do not synthesize an empty packet here.
        self.send_packet(&AllAchievementData);

        // 20. LoginSetTimeSpeed
        self.send_packet(&LoginSetTimeSpeed::now());

        // 21. WorldServerInfo
        self.send_packet(&WorldServerInfo::default_open_world());

        // 22. SetFlatSpellModifier + SetPctSpellModifier.
        //      C++ `Player::SendInitialPacketsBeforeAddToMap` calls
        //      `Player::SendSpellModifiers()` immediately after
        //      `WorldServerInfo` (`Player.cpp:23562-23563`). Fresh characters
        //      have empty modifier maps, but C++ still sends the packets.
        self.send_raw_packet(&SetSpellModifier::flat_empty().to_bytes());
        self.send_raw_packet(&SetSpellModifier::pct_empty().to_bytes());

        // 23. AccountMountUpdate
        self.send_packet(&AccountMountUpdate::full(account_mounts));

        // 24. AccountToyUpdate
        self.send_account_toys_like_cpp();

        // 25. AccountHeirloomUpdate
        self.send_account_heirlooms_like_cpp();

        // 26. AccountTransmogUpdate favorite appearances
        self.send_favorite_appearances_like_cpp();

        // 27. InitialSetup (expansion level)
        self.send_packet(&InitialSetup::wotlk());

        // C++ `Player::SendInitialPacketsBeforeAddToMap` ends with
        // `SetMovedUnit(this)`. `Unit::SetMovedUnit` updates server-side mover
        // state and sends `SMSG_MOVE_SET_ACTIVE_MOVER` to bind client input to
        // the player before the create block.
        self.set_player_moved_unit_guid_like_cpp(guid);
        self.send_packet(&MoveSetActiveMover { mover_guid: guid });
        if updateobject_trace_enabled {
            info!(guid = ?guid, "RUST_LOGIN after_initial_packets_before_add");
        }
        true
    }

    /// C++ `Player::SendInitialPacketsAfterAddToMap` (Player.cpp:23592-23685): the packets
    /// sent after the player is added to the map — the post-add phase shift, visibility
    /// mirror, `UpdateZone` -> SMSG_INIT_WORLD_STATES (resolved for the destination map),
    /// CUF profiles, auras and the `PhasingHandler::OnMapChange` phase shift. Shared by login
    /// and far teleport (#NEXT.R8.ENTITIES.1229). Reads all data from self; the destination
    /// guid/position/map are passed in.
    pub(crate) async fn send_initial_packets_after_add_to_map(
        &mut self,
        guid: ObjectGuid,
        position: &Position,
        map_id: i32,
        updateobject_trace_enabled: bool,
    ) {
        if updateobject_trace_enabled {
            info!(guid = ?guid, "RUST_LOGIN before_initial_packets_after_add");
        }

        // C++ Map::AddPlayerToMap sends the phase shift after
        // UpdateObjectVisibility(false), before post-add world-state packets.
        self.send_packet(&PhaseShiftChange::default_for(guid));

        // C++ `HandlePlayerLogin` calls `ObjectAccessor::AddObject` after
        // `Map::AddPlayerToMap` returns and before
        // `Player::SendInitialPacketsAfterAddToMap`
        // (`CharacterHandler.cpp:1241-1262`).
        self.register_in_player_registry();
        self.sync_object_accessor_player();

        // C++ `HandlePlayerLogin` calls `ObjectAccessor::AddObject`, then
        // `Player::SendInitialPacketsAfterAddToMap`; that method starts with
        // `UpdateVisibilityForPlayer()`. Rust must force the same rebuild here
        // because Map::AddPlayerToMap just cleared the client-visible GUID cache;
        // after logout/relogin at the same position the normal movement-distance
        // throttle can otherwise leave the client with no visible creatures.
        self.sync_current_player_session_visibility_detection_like_cpp();
        self.force_update_visibility_like_cpp().await;
        if updateobject_trace_enabled {
            info!(
                guid = ?guid,
                count = self.client_visible_guids_like_cpp.len(),
                "RUST_LOGIN after_initial_update_visibility_for_player"
            );
        }

        let terrain_area_authority_complete = terrain_grid_area_id_for_position_like_cpp(
            &self.mmap_runtime_config_like_cpp().data_dir,
            map_id as u32,
            position.x,
            position.y,
        )
        .is_ok_and(|area_id| area_id.is_some_and(|area_id| area_id != 0));

        match zone_and_area_for_position_like_cpp(
            &self.mmap_runtime_config_like_cpp().data_dir,
            map_id as u32,
            position.x,
            position.y,
            self.area_table_store().map(|store| store.as_ref()),
            |map_id| {
                self.map_store()
                    .as_deref()
                    .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
                    .unwrap_or(0)
            },
        ) {
            Ok((resolved_zone_id, resolved_area_id)) => {
                self.update_zone_represented_without_rest_update_packet_like_cpp(
                    resolved_zone_id,
                    resolved_area_id,
                );
                self.set_player_zone_area_authority_complete_like_cpp(
                    terrain_area_authority_complete
                        && resolved_zone_id != 0
                        && resolved_area_id != 0,
                );
                info!(
                    map_id,
                    x = position.x,
                    y = position.y,
                    zone_id = resolved_zone_id,
                    area_id = resolved_area_id,
                    "Resolved player zone/area like C++ terrain before InitWorldStates"
                );
            }
            Err(error) => {
                warn!(
                    map_id,
                    x = position.x,
                    y = position.y,
                    %error,
                    "failed to resolve C++ terrain zone/area before InitWorldStates; using DB-seeded zone/area"
                );
                let (seeded_zone_id, seeded_area_id) = self.player_zone_area_like_cpp();
                self.update_zone_represented_without_rest_update_packet_like_cpp(
                    seeded_zone_id,
                    seeded_area_id,
                );
                self.set_player_zone_area_authority_complete_like_cpp(false);
            }
        }
        let rest_flag_update_dirty = self.take_deferred_rest_flag_update_dirty_like_cpp();

        // 27. InitWorldStates — C++ `Player::SendInitWorldStates` delegates to
        // `WorldStateMgr::FillInitialWorldStates`: realm values first, then map
        // values filtered by AreaIDs.
        let (represented_zone_id, represented_area_id) = self.player_zone_area_like_cpp();
        let world_states = self
            .load_initial_world_states_for_login_like_cpp(map_id, represented_area_id)
            .await;
        self.send_packet(&InitWorldStates::with_world_states(
            map_id,
            represented_zone_id as i32,
            represented_area_id as i32,
            world_states,
        ));

        // 28. LoadCufProfiles — C++ sends this immediately after InitWorldStates.
        // Keeping the CUF profile application at that exact point in the login burst is
        // client-significant: the later phase refresh must not overtake it.
        self.send_packet(&self.represented_load_cuf_profiles_packet_like_cpp());
        // C++ `Player::SendInitialPacketsAfterAddToMap` calls
        // `SendAurasForTarget(this)` after movement aura state setup.
        self.send_initial_player_auras_like_cpp();
        // C++ calls PhasingHandler::OnMapChange(this) only after CUF profiles, the
        // login-effect/movement-aura work and SendAurasForTarget (Player.cpp:23600-23672).
        // This re-sends SMSG_PHASE_SHIFT_CHANGE (the second phase-shift of login,
        // byte-identical to the AddToMap one). #NEXT.R8.ENTITIES.1228.
        self.send_packet(&PhaseShiftChange::default_for(guid));
        // C++ RestMgr only dirties PLAYER_FLAGS_RESTING during UpdateZone; the
        // map object-update owner flushes that field after post-add packets.
        if rest_flag_update_dirty {
            self.send_represented_resting_player_flag_update_like_cpp();
        }
        if updateobject_trace_enabled {
            info!(guid = ?guid, "RUST_LOGIN after_initial_packets_after_add");
        }
    }

    /// Retry the final C++ `Player::LoadFromDB` recovery location after the
    /// saved map cannot be selected. C++ first tries go-back/map-entrance
    /// triggers; Rust's current MapEntry/instance-template stores do not expose
    /// enough data to select those faithfully, so this implements the final
    /// mandatory homebind retry and reports whether relocation succeeded.
    pub(super) fn resolved_homebind_area_id_like_cpp(
        &self,
        map_id: u32,
        position: Position,
    ) -> u32 {
        let map_area_id = self
            .map_store()
            .as_deref()
            .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
            .unwrap_or(0);
        zone_and_area_for_position_like_cpp(
            &self.mmap_runtime_config_like_cpp().data_dir,
            map_id,
            position.x,
            position.y,
            self.area_table_store().map(|store| store.as_ref()),
            |map_id| {
                self.map_store()
                    .as_deref()
                    .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
                    .unwrap_or(0)
            },
        )
        .ok()
        .map(|(_, area_id)| area_id)
        .filter(|area_id| *area_id != 0)
        .unwrap_or(map_area_id)
    }

    pub(super) async fn delete_invalid_character_homebind_like_cpp(&self, guid: ObjectGuid) {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            return;
        };
        match port
            .persist_homebind_like_cpp(
                wow_persistence::PlayerHomebindPersistenceRequestLikeCpp::DeleteInvalid {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => warn!(
                player_guid = guid.counter(),
                "failed to delete invalid character homebind like C++ Player::_LoadHomeBind: {reason}"
            ),
        }
    }

    pub(super) fn seed_login_location_zone_area_like_cpp(
        &mut self,
        zone_id: &mut i32,
        location: CharacterLoginLocationLikeCpp,
    ) {
        let resolved = login_location_zone_area_like_cpp(location, |map_id, position| {
            zone_and_area_for_position_like_cpp(
                &self.mmap_runtime_config_like_cpp().data_dir,
                map_id,
                position.x,
                position.y,
                self.area_table_store().map(|store| store.as_ref()),
                |map_id| {
                    self.map_store()
                        .as_deref()
                        .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
                        .unwrap_or(0)
                },
            )
        });
        let fallback_area_id = location.bind_area_id.unwrap_or_else(|| {
            self.map_store()
                .as_deref()
                .map(|store| u32::from(store.area_table_id_like_cpp(location.map_id)))
                .unwrap_or(0)
        });
        let (fallback_zone_id, fallback_area_id) = zone_and_area_from_area_id_like_cpp(
            fallback_area_id,
            self.area_table_store().map(Arc::as_ref),
        );

        match resolved {
            Ok((resolved_zone_id, resolved_area_id)) if resolved_area_id != 0 => {
                *zone_id = i32::try_from(resolved_zone_id)
                    .expect("resolved login zone ID must fit the packet field");
                self.set_player_zone_area_like_cpp(resolved_zone_id, resolved_area_id);
            }
            Ok(_) => {
                *zone_id = i32::try_from(fallback_zone_id)
                    .expect("fallback login zone ID must fit the packet field");
                self.set_player_zone_area_like_cpp(fallback_zone_id, fallback_area_id);
                warn!(
                    map_id = location.map_id,
                    x = location.position.x,
                    y = location.position.y,
                    fallback_zone_id,
                    fallback_area_id,
                    "terrain returned no fallback login area for C++ UpdatePositionData"
                );
            }
            Err(error) => {
                *zone_id = i32::try_from(fallback_zone_id)
                    .expect("fallback login zone ID must fit the packet field");
                self.set_player_zone_area_like_cpp(fallback_zone_id, fallback_area_id);
                warn!(
                    map_id = location.map_id,
                    x = location.position.x,
                    y = location.position.y,
                    %error,
                    fallback_zone_id,
                    fallback_area_id,
                    "failed to refresh fallback login zone/area like C++ UpdatePositionData"
                );
            }
        }
    }

    pub(super) fn retry_login_at_homebind_like_cpp(
        &mut self,
        map_id: &mut i32,
        zone_id: &mut i32,
        position: &mut Position,
        homebind: CharacterLoginLocationLikeCpp,
    ) -> bool {
        if self.current_canonical_player_map_key_like_cpp().is_some() {
            return false;
        }
        if !usable_character_homebind_like_cpp(
            homebind,
            self.map_store().map(Arc::as_ref),
            self.expansion,
        ) {
            return false;
        }
        let homebind_map_id =
            u16::try_from(homebind.map_id).expect("validated character login homebind map ID");

        *map_id = i32::from(homebind_map_id);
        *position = homebind.position;
        self.seed_login_location_zone_area_like_cpp(zone_id, homebind);
        self.set_player_map_position_like_cpp(homebind_map_id, homebind.position);
        let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        self.current_canonical_player_map_key_like_cpp().is_some()
    }

    /// A failed cross-socket ordering fence means the successful-login burst
    /// cannot be completed coherently. C++ loses the socket and destroys the
    /// partially loaded `Player`; mirror that lifetime boundary immediately
    /// so the process-wide character claim cannot outlive this failed login.
    pub(super) fn abort_partial_login_sequence_like_cpp(&mut self) {
        self.cleanup_shared_runtime_state();
        self.set_player_guid(None);
        self.kick("WorldSession::HandlePlayerLogin login packet sequence failed");
    }

    /// Send the player login packet sequence to the client.
    ///
    /// Follows the C++ login phases:
    /// HandlePlayerLogin → SendInitialPacketsBeforeAddToMap → AddToMap →
    /// SendInitialPacketsAfterAddToMap.
    ///
    /// AuthResponse, SetTimeZone, FeatureSystemStatusGlueScreen,
    /// AccountDataTimes(global), and TutorialFlags are first sent during
    /// session init. C++ intentionally resends the account-data times,
    /// tutorials, and time-zone packets during `HandlePlayerLogin`.
    pub(super) async fn send_login_sequence(
        &mut self,
        guid: ObjectGuid,
        race: u8,
        class: u8,
        sex: u8,
        level: u8,
        display_id: u32,
        position: &Position,
        map_id: i32,
        zone_id: i32,
        homebind: CharacterLoginLocationLikeCpp,
        persisted_transport_login: Option<PersistedTransportLoginLikeCpp>,
        visible_items: [(i32, u16, u16); 19],
        inv_slots: [ObjectGuid; 141],
        item_creates: Vec<wow_packet::packets::update::ItemCreateData>,
        combat: PlayerCombatStats,
        current_power0: i32,
        base_mana: i32,
        known_spells: Vec<i32>,
        favorite_spells: Vec<i32>,
        spell_history_entries: Vec<SpellHistoryEntry>,
        spell_charge_entries: Vec<SpellChargeEntry>,
        action_buttons: [i64; 180],
        skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
        account_mounts: Vec<AccountMount>,
    ) -> bool {
        let updateobject_trace_enabled = std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some();
        let authoritative_grid_map_key = self
            .current_canonical_player_map_key_like_cpp()
            .filter(|key| u32::try_from(map_id).ok() == Some(key.map_id));
        let grid_instance_id = authoritative_grid_map_key
            .map(|key| key.instance_id)
            .unwrap_or(0);
        // Rust's loaded-grid bridge also validates that the canonical map is
        // usable. C++ finishes all fallible `Player::LoadFromDB` map selection
        // before emitting successful-login packets, so run this bridge before
        // Phase 1 and retain its outcome for the Map::AddPlayerToMap trace.
        let grid_load_outcome = self.ensure_player_grid_loaded_like_cpp(
            map_id as u16,
            authoritative_grid_map_key.map(|key| key.instance_id),
            *position,
        );
        if !self.continue_login_after_grid_load_like_cpp(
            guid,
            map_id,
            grid_instance_id,
            grid_load_outcome,
        ) {
            return false;
        }
        let corpse_load_outcome = self
            .load_map_corpse_data_like_cpp(map_id as u16, grid_instance_id)
            .await;
        if corpse_load_outcome.rows_seen != 0
            || corpse_load_outcome.already_loaded
            || corpse_load_outcome.invalid_type_rows != 0
            || corpse_load_outcome.invalid_race_rows != 0
            || corpse_load_outcome.invalid_position_rows != 0
            || corpse_load_outcome.add_to_map_errors != 0
        {
            info!(
                map_id,
                instance_id = grid_instance_id,
                already_loaded = corpse_load_outcome.already_loaded,
                rows_seen = corpse_load_outcome.rows_seen,
                corpses_added = corpse_load_outcome.corpses_added,
                invalid_type_rows = corpse_load_outcome.invalid_type_rows,
                invalid_race_rows = corpse_load_outcome.invalid_race_rows,
                invalid_position_rows = corpse_load_outcome.invalid_position_rows,
                add_to_map_errors = corpse_load_outcome.add_to_map_errors,
                "Loaded canonical map corpse data like C++ Map::LoadCorpseData"
            );
        }

        // ── Phase 1: HandlePlayerLogin packets ──
        let motd =
            wow_config::get_value_default::<String>("Motd", DEFAULT_MOTD_LIKE_CPP.to_string());
        let account_mount_login_partials = self.account_mount_login_partial_rows_like_cpp();
        if !self
            .send_handle_player_login_packets_like_cpp(
                guid,
                position,
                map_id,
                &account_mount_login_partials,
                &motd,
            )
            .await
        {
            return false;
        }

        // ── Phase 2: SendInitialPacketsBeforeAddToMap ──
        if !self
            .send_initial_packets_before_add_to_map(
                guid,
                position,
                map_id,
                zone_id,
                homebind,
                known_spells,
                favorite_spells,
                spell_history_entries,
                spell_charge_entries,
                action_buttons,
                account_mounts,
                updateobject_trace_enabled,
            )
            .await
        {
            return false;
        }

        // ── C++ Map::AddPlayerToMap ──
        if updateobject_trace_enabled {
            info!(
                guid = ?guid,
                map_id,
                instance_id = grid_instance_id,
                init_player = 1u8,
                player_x = position.x,
                player_y = position.y,
                player_z = position.z,
                player_o = position.orientation,
                "RUST_LOGIN map_add start"
            );
        }

        // C++ `Map::AddPlayerToMap` performs EnsureGridLoadedForActiveObject
        // before AddToWorld/SendInitSelf (`Map.cpp:443-470`). The Rust bridge
        // was preflighted before Phase 1 above because it also contains
        // fallible map validation; report the retained result at this
        // equivalent map-add point.
        if let Some(outcome) = grid_load_outcome {
            info!(
                map_id,
                instance_id = grid_instance_id,
                map_unavailable = outcome.map_unavailable,
                map_created = outcome.map_created,
                grid_loaded_now = outcome.grid_loaded_now,
                metadata_entries = outcome.metadata_entries,
                skipped_already_loaded = outcome.skipped_already_loaded,
                skipped_should_not_spawn = outcome.skipped_should_not_spawn,
                skipped_difficulty_mismatch = outcome.skipped_difficulty_mismatch,
                stale_index_entries = outcome.stale_index_entries,
                creature_records_added = outcome.creature_records_added,
                gameobject_records_added = outcome.gameobject_records_added,
                area_trigger_records_added = outcome.area_trigger_records_added,
                pre_add_records_added = outcome.pre_add_records_added,
                add_to_map_errors = outcome.add_to_map_errors,
                load_record_missing = outcome.load_record_missing,
                creature_load_record_missing = outcome.creature_load_record_missing,
                gameobject_load_record_missing = outcome.gameobject_load_record_missing,
                area_trigger_load_record_missing = outcome.area_trigger_load_record_missing,
                legacy_creature_mirrors = outcome.legacy_creature_mirrors,
                "RUST_LOGIN grid_load"
            );
        }
        if updateobject_trace_enabled {
            info!(guid = ?guid, "RUST_LOGIN map_add after_ensure_grid");
        }

        info!(
            guid = ?guid,
            map_id,
            "RUST_LOGIN map_add before_add_to_world"
        );
        let attached_controller = self.ensure_login_player_controller_like_cpp(
            guid,
            self.player_name_like_cpp()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("Player{}", guid.counter())),
            *position,
            map_id as u16,
            race,
            class,
            level,
            sex,
        );
        if attached_controller {
            let _ = self.ensure_canonical_world_map_for_current_player_like_cpp();
        }
        self.sync_canonical_player_health_like_cpp(
            combat.health.max(0).min(u32::MAX as i64) as u32,
            combat.max_health.max(1).min(u32::MAX as i64) as u32,
        );
        let primary_power_type = primary_power_type_for_class_like_cpp(class);
        let primary_max_power = primary_max_power_for_class_like_cpp(class, combat.max_mana);
        let primary_base_mana = if primary_power_type == PowerType::Mana {
            base_mana
        } else {
            0
        };
        self.sync_canonical_player_primary_power_like_cpp(
            primary_power_type,
            current_power0,
            primary_max_power,
            primary_base_mana,
        );
        if std::env::var_os("RUSTYCORE_SPELL_POWER_TRACE").is_some() {
            info!(
                guid = ?guid,
                class,
                power_type = ?primary_power_type,
                current_power0,
                max_power0 = primary_max_power,
                base_mana = primary_base_mana,
                "RUST_LOGIN_POWER_SYNC"
            );
        }
        self.login_time = Some(std::time::Instant::now());
        self.suppress_creature_movement_queued_at_or_before_like_cpp = None;
        // Clear per-session loot/combat state as part of the Rust AddToWorld
        // equivalent, before C++ would build `Map::SendInitSelf`.
        self.loot_table.clear();
        self.set_active_loot_guid(ObjectGuid::EMPTY);
        self.combat_target = None;
        self.in_combat = false;
        info!(
            guid = ?guid,
            map_id,
            "RUST_LOGIN map_add after_add_to_world"
        );
        let mut init_transports_plan = self
            .plan_init_transports_like_cpp(map_id as u16, persisted_transport_login)
            .await;

        // C++ `Map::SendInitSelf` — current transport + items + player in a
        // single packet. The transport precedes the player's nested
        // MovementInfo::TransportInfo reference; items precede InvSlots.
        {
            // Build quest log for the UpdateObject (25 slots max).
            // C++ Player::BuildValuesCreate sends quest log fields in the
            // self-view player create block.
            // StateFlags: 0=None, 1=Complete (QuestSlotStateMask)
            let quest_log: Vec<(u32, u32, i64, [u16; 24])> =
                self.quest_log_create_entries_like_cpp();

            let account_toys = self.account_toy_active_player_rows_like_cpp();
            let account_heirlooms = self.account_heirloom_active_player_rows_like_cpp();
            let account_transmog = self.account_transmog_active_player_rows_like_cpp();
            let trait_configs = self.load_active_player_trait_configs_like_cpp(guid).await;
            let player_customizations = self.load_player_customizations_like_cpp(guid).await;
            self.set_loaded_player_customizations_like_cpp(player_customizations.clone());
            info!(
                toys = account_toys.len(),
                heirlooms = account_heirlooms.len(),
                transmog = account_transmog.len(),
                trait_configs = trait_configs.len(),
                customizations = player_customizations.len(),
                "Building player CREATE collection dynamic fields"
            );

            let mut player_pkt = UpdateObject::create_player_with_party_type(
                guid,
                race,
                class,
                sex,
                level,
                display_id,
                position,
                map_id as u16,
                zone_id as u32,
                true,
                visible_items,
                inv_slots,
                combat,
                skill_info,
                self.player_gold_like_cpp(),
                quest_log,
                self.party_member_party_type_like_cpp(),
            );
            let (player_flags, player_flags_ex) =
                self.represented_player_flags_for_create_like_cpp();
            player_pkt.set_player_flags_like_cpp(player_flags, player_flags_ex);
            player_pkt.set_player_current_power0_like_cpp(current_power0);
            player_pkt.set_player_xp_like_cpp(self.player_xp_like_cpp() as i32);
            player_pkt
                .set_player_next_level_xp_like_cpp(self.player_next_level_xp_like_cpp() as i32);
            player_pkt
                .set_player_max_level_like_cpp(self.player_active_max_level_like_cpp() as i32);
            player_pkt.set_player_scaling_level_delta_like_cpp(
                self.player_scaling_level_delta_like_cpp(),
            );
            player_pkt.set_player_rest_info_like_cpp(
                0,
                self.represented_xp_rest_threshold_like_cpp(),
                self.represented_xp_rest_state_like_cpp(),
            );
            if std::env::var_os("RUSTYCORE_SPELL_POWER_TRACE").is_some() {
                info!(
                    guid = ?guid,
                    current_power0,
                    max_power0 = primary_max_power,
                    base_mana = primary_base_mana,
                    "RUST_LOGIN_POWER_CREATE"
                );
            }
            player_pkt.set_player_account_guids_like_cpp(
                ObjectGuid::create_global(HighGuid::WowAccount, 0, self.account_id as i64),
                ObjectGuid::create_global(
                    HighGuid::BNetAccount,
                    0,
                    self.battlenet_account_id() as i64,
                ),
            );
            player_pkt.set_player_collection_dynamic_fields_like_cpp(
                account_toys,
                account_heirlooms,
                account_transmog,
                trait_configs,
            );
            player_pkt.set_player_action_buttons_like_cpp(
                self.represented_action_buttons_snapshot_like_cpp(),
            );
            player_pkt.set_player_customizations_like_cpp(player_customizations);

            if let (Some((transport_guid, _)), Some(transport_position)) = (
                init_transports_plan.own_transport.as_ref(),
                self.player_transport_position_like_cpp(),
            ) {
                player_pkt.set_player_movement_transport_like_cpp(TransportInfo {
                    guid: *transport_guid,
                    x: transport_position.x,
                    y: transport_position.y,
                    z: transport_position.z,
                    o: transport_position.orientation,
                    seat: -1,
                    time: 0,
                    prev_time: None,
                    vehicle_id: None,
                });
            }

            let fellow_passenger_blocks = init_transports_plan
                .own_transport
                .as_ref()
                .map(|(transport_guid, _)| {
                    self.init_self_fellow_transport_passenger_blocks_like_cpp(
                        map_id as u16,
                        *transport_guid,
                    )
                })
                .unwrap_or_default();
            if !item_creates.is_empty()
                || init_transports_plan.own_transport.is_some()
                || !fellow_passenger_blocks.is_empty()
            {
                info!(
                    items = item_creates.len(),
                    own_transport = init_transports_plan.own_transport.is_some(),
                    fellow_passengers = fellow_passenger_blocks.len(),
                    "Sending C++ Map::SendInitSelf CREATE blocks"
                );
                if let Some(transport_guid) = compose_init_self_create_blocks_like_cpp(
                    &mut player_pkt,
                    item_creates,
                    init_transports_plan.own_transport.take(),
                    fellow_passenger_blocks,
                ) {
                    self.client_visible_transports_like_cpp
                        .insert(transport_guid);
                }
            }

            if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
                for line in player_pkt.debug_create_summary_like_cpp() {
                    info!("RUST_UPDATEOBJECT login_self {line}");
                }
            }
            self.send_packet(&player_pkt);
        }
        if updateobject_trace_enabled {
            info!(guid = ?guid, "RUST_LOGIN map_add after_send_init_self");
        }
        // C++ Map::AddPlayerToMap sends transports immediately after
        // SendInitSelf, before clearing the normal visible GUID cache.
        self.send_init_transports_like_cpp(map_id as u16, init_transports_plan);
        if updateobject_trace_enabled {
            info!(guid = ?guid, "RUST_LOGIN map_add after_send_init_transports");
        }

        // C++ Map::AddPlayerToMap clears the visible GUID cache immediately
        // after SendInitSelf and SendInitTransports.
        if updateobject_trace_enabled {
            info!(
                guid = ?guid,
                count = self.client_visible_guids_like_cpp.len(),
                "RUST_LOGIN map_add before_clear_client_guids"
            );
        }
        self.client_visible_guids_like_cpp.clear();
        // C++ clears m_clientGUIDs here, then Player::SendInitialPacketsAfterAddToMap
        // starts with UpdateVisibilityForPlayer. Do not let Rust's movement-distance
        // throttle reuse the previous login/logout position after the clear.
        self.last_visibility_pos = None;
        if updateobject_trace_enabled {
            info!(
                guid = ?guid,
                count = self.client_visible_guids_like_cpp.len(),
                "RUST_LOGIN map_add after_clear_client_guids"
            );
        }

        // C++ `Map::AddPlayerToMap` clears `m_clientGUIDs`, then calls
        // `Player::UpdateObjectVisibility(false)`. In the traced 3.4.3 login
        // this only schedules `NOTIFY_VISIBILITY_CHANGED`; it does not emit
        // the nearby object create batch in this map-add phase.
        if updateobject_trace_enabled {
            info!(
                guid = ?guid,
                count = self.client_visible_guids_like_cpp.len(),
                "RUST_LOGIN map_add after_update_object_visibility"
            );
        }

        // ── Phase 4: SendInitialPacketsAfterAddToMap ──
        self.send_initial_packets_after_add_to_map(
            guid,
            position,
            map_id,
            updateobject_trace_enabled,
        )
        .await;

        // C++ does not deliver SMSG_ON_MONSTER_MOVE inside the initial
        // enter-world packet burst. Rust fan-out commands are queued from a
        // sessionless world tick, so remember the burst boundary and drop only
        // movement commands that were queued at or before it.
        self.suppress_creature_movement_queued_at_or_before_like_cpp =
            Some(std::time::Instant::now());

        // Rust keeps the session status flip after the initial after-add packet
        // subset so the network loop cannot process normal movement/gameplay
        // opcodes while the login stream is still being emitted. The represented
        // player itself was already installed above at the AddToWorld point.
        self.set_state(crate::session::SessionState::LoggedIn);

        // 31. Existing nearby sessions run the same C++-style visibility diff
        //     so their client GUID caches gain this player symmetrically.
        self.notify_other_players_visibility_changed_like_cpp();

        // 32. Send full stat VALUES update so all character panel tabs
        //     (Melee, Ranged, Spell, Defense) display correct values on login.
        //     C++ has already applied loaded item enchantments at this point;
        //     merge their represented modifiers into this absolute snapshot.
        //     A second bonus-only packet would write its default fields as
        //     zero and corrupt unrelated client-visible stats.
        self.send_login_stat_update_with_represented_item_bonuses_like_cpp();

        info!(
            "Login sequence complete for {:?} (38 packets including broadcasts)",
            guid
        );
        true
    }

    /// C++ `Player::LoadFromDB` first attempts go-back/homebind relocation and
    /// never substitutes an arbitrary instance when authoritative map
    /// selection fails. The caller has already tried the represented BG entry
    /// point/homebind recovery; if that also produced no canonical map, fail
    /// closed: tear down the partially loaded player while its GUID is still
    /// present, then disconnect without `CharacterLoginFailed`, matching C++'s
    /// final `LoadFromDB == false` cleanup path.
    pub(super) fn continue_login_after_grid_load_like_cpp(
        &mut self,
        guid: ObjectGuid,
        map_id: i32,
        instance_id: u32,
        outcome: Option<crate::session::PlayerGridLoadOutcomeLikeCpp>,
    ) -> bool {
        if outcome.is_some_and(|outcome| !outcome.map_unavailable) {
            return true;
        }

        let reason = if outcome.is_some() {
            "authoritative canonical map unavailable"
        } else {
            "loaded-grid resolver unavailable"
        };
        warn!(
            guid = ?guid,
            map_id,
            instance_id,
            reason,
            "RUST_LOGIN map_add aborted"
        );
        self.cleanup_shared_runtime_state();
        self.set_player_guid(None);
        self.kick("WorldSession::HandlePlayerLogin authoritative map resolution failed");
        false
    }

    async fn load_initial_world_states_for_login_like_cpp(
        &self,
        map_id: i32,
        player_area_id: u32,
    ) -> Vec<(i32, i32)> {
        let Some(port) = self.player_lifecycle_port_like_cpp() else {
            warn!("InitWorldStates: missing Player lifecycle persistence port");
            return Vec::new();
        };
        let loaded = port.load_initial_world_states_like_cpp().await;

        let area_store = self.area_table_store().map(Arc::as_ref);
        let map_store = self.map_store().map(Arc::as_ref);

        let mut templates = Vec::new();
        match loaded.templates {
            PlayerInitialWorldStateRowsLikeCpp::Loaded(rows) => {
                for row in rows {
                    let map_ids_csv = row.map_ids_csv;
                    let area_ids_csv = row.area_ids_csv;
                    let map_ids =
                        parse_login_world_state_map_ids_like_cpp(&map_ids_csv, |map_id| {
                            u32::try_from(map_id).ok().is_some_and(|map_id| {
                                map_store.is_some_and(|store| store.get(map_id).is_some())
                            })
                        });
                    if !map_ids_csv.is_empty() && map_ids.is_empty() {
                        continue;
                    }

                    let area_ids = parse_login_world_state_area_ids_like_cpp(
                        &area_ids_csv,
                        &map_ids,
                        area_store,
                    );
                    if !area_ids_csv.is_empty() && !map_ids.is_empty() && area_ids.is_empty() {
                        continue;
                    }

                    templates.push(LoginWorldStateTemplateLikeCpp {
                        id: row.id,
                        default_value: row.default_value,
                        map_ids,
                        area_ids,
                    });
                }
            }
            PlayerInitialWorldStateRowsLikeCpp::Failed { reason } => {
                warn!(
                    reason,
                    "InitWorldStates: failed to load C++ world_state templates"
                );
            }
        }

        let saved_values = match loaded.saved_values {
            PlayerInitialWorldStateRowsLikeCpp::Loaded(rows) => {
                rows.into_iter().map(|row| (row.id, row.value)).collect()
            }
            PlayerInitialWorldStateRowsLikeCpp::Failed { reason } => {
                warn!(
                    reason,
                    "InitWorldStates: failed to load C++ world_state_value overlay"
                );
                Vec::new()
            }
        };

        let mut states = build_initial_world_states_like_cpp(
            templates,
            saved_values,
            map_id,
            player_area_id,
            area_store,
        );
        // C++ World.cpp:1363-1364 / 2300-2301: the WorldStateMgr seeds realm-wide PvP-season
        // world states that FillInitialWorldStates always includes. CONFIG_ARENA_SEASON_ID
        // defaults to 32, CONFIG_ARENA_SEASON_IN_PROGRESS to false. #NEXT.R8.ENTITIES.1232.
        apply_pvp_season_world_states_like_cpp(
            &mut states,
            wow_config::get_value_default::<i32>("Arena.ArenaSeason.ID", 32),
            wow_config::get_value_default::<i32>("Arena.ArenaSeason.InProgress", 0) != 0,
        );
        info!(
            map_id,
            player_area_id,
            count = states.len(),
            "InitWorldStates loaded like C++"
        );
        states
    }

    #[cfg(test)]
    pub(super) async fn test_load_initial_world_states_for_login_like_cpp(
        &self,
        map_id: i32,
        player_area_id: u32,
    ) -> Vec<(i32, i32)> {
        self.load_initial_world_states_for_login_like_cpp(map_id, player_area_id)
            .await
    }

    // ── ShowTradeSkill ───────────────────────────────────────────────────────

    /// C++ `HandleShowTradeSkill(WorldPackets::Null&)` only logs the request.
    pub async fn handle_show_trade_skill(&mut self) {
        if let Some(player_guid) = self.player_guid() {
            debug!("ShowTradeSkill from {:?}", player_guid);
        } else {
            debug!("ShowTradeSkill from account {}", self.account_id);
        }
    }
}
