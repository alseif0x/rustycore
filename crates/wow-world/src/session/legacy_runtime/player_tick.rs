//! Legacy player melee tick.
//!
//! Moved out of the Session root under #619. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

/// Resolve every player auto-attack once, from the map, for one frame.
///
/// #28 moves this transition off the session. Every logged-in session used to
/// run it on its own clock and write shared legacy creature state ungated, so
/// with N players there were N+1 concurrent writers of that state. Here it
/// resolves once, under the owner.
///
/// The phases are separated by which lock they need, and the separation is the
/// point: the owner gate reads through a `Copy` helper that holds no guard, the
/// collect phase touches canonical only, and the execute phase takes canonical
/// then legacy — the established order the canonical map loop already enforces
/// globally. No phase holds one manager while acquiring the other.
///
/// C++ anchor: `Player::Update` calls `DoMeleeAttackIfReady` before
/// `Map::Update` runs `ObjectUpdater`; the swing timer is consumed after the
/// range and facing checks, as it is here.
pub fn run_legacy_player_melee_tick_once_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    attackers: &[PlayerMeleeAttackerSnapshotLikeCpp],
    diff_ms: u32,
    phase_state: &mut PlayerMeleePhaseStateLikeCpp,
) -> LegacyPlayerMeleeTickOutcomeLikeCpp {
    let mut outcome = LegacyPlayerMeleeTickOutcomeLikeCpp::default();

    // Step 1 — the gate. Reading the owner through the shared helper returns a
    // `Copy` and releases immediately, so nothing below can be holding a legacy
    // guard while it reaches for canonical.
    if crate::map_manager::shared_runtime_tick_owner_like_cpp(legacy_map_manager)
        != RuntimeTickOwner::GlobalLegacy
    {
        outcome.skipped_owner_not_global = true;
        return outcome;
    }
    outcome.attackers_seen = attackers.len();

    let Some(canonical_map_manager) = canonical_map_manager else {
        return outcome;
    };

    // Step 2 — collect. Canonical only, once, never nested.
    let mut pending: Vec<PendingPlayerSwingLikeCpp> = Vec::new();
    {
        let Ok(mut manager) = canonical_map_manager.lock() else {
            return outcome;
        };
        let mut map_keys: Vec<(u16, u32)> = attackers
            .iter()
            .map(|attacker| (attacker.map_id, attacker.instance_id))
            .collect();
        map_keys.sort_unstable();
        map_keys.dedup();
        outcome.maps_seen = map_keys.len();

        for (map_id, instance_id) in map_keys {
            let accumulated = phase_state
                .revalidate_accumulated_ms
                .entry((map_id, instance_id))
                .or_insert(0);
            *accumulated = accumulated.saturating_add(diff_ms);
            let sweep_due = *accumulated >= PLAYER_MELEE_COMBAT_REF_REVALIDATE_INTERVAL_MS;
            if sweep_due {
                *accumulated = 0;
            }
            let Some(managed) = manager.find_map_mut(u32::from(map_id), instance_id) else {
                continue;
            };
            if sweep_due {
                let _ = managed.map_mut().revalidate_all_combat_refs_like_cpp();
                outcome.combat_ref_revalidations += 1;
            }
        }

        for attacker in attackers {
            let Some(managed) =
                manager.find_map_mut(u32::from(attacker.map_id), attacker.instance_id)
            else {
                outcome.attacker_unavailable += 1;
                continue;
            };
            let Some(player) = managed.map().get_typed_player(attacker.player_guid) else {
                outcome.attacker_unavailable += 1;
                continue;
            };
            let victim = player.unit().attacking();
            let has_combat = player.unit().subsystems().combat.has_combat();

            // The session used to reconcile its mirror on every combat tick,
            // including the branch that found no victim. Without this the
            // mirror stops being corrected, which is "stops resolving".
            if has_combat != attacker.in_combat_mirror {
                outcome.in_combat_reconciles += 1;
                outcome.commands.push(
                    crate::session::mailbox::ApplyPlayerMeleeResultLikeCppCommand {
                        attacker_guid: attacker.player_guid,
                        map_id: attacker.map_id,
                        instance_id: attacker.instance_id,
                        victim_guid: None,
                        swing_error_after: None,
                        combat_target_after: victim.is_none().then_some(None),
                        in_combat_after: Some(has_combat),
                        swings: Vec::new(),
                        target_level: 0,
                        victim_values_update: None,
                        killed_creature: None,
                    },
                );
            }

            let Some(victim_guid) = victim else {
                continue;
            };
            outcome.victims_resolved += 1;
            pending.push(PendingPlayerSwingLikeCpp {
                attacker: attacker.clone(),
                victim_guid,
            });
        }
    }

    outcome.swings_ready = pending.len();

    // Step 3 — execute. Canonical then legacy, the established order: a target
    // switch or a same-GUID respawn must not cross this commit
    // (`run_legacy_creature_melee_tick_once_like_cpp` takes them the same way and
    // says so). Every `continue` drops both guards.
    let mut canonical_syncs = Vec::new();
    for swing in pending {
        let attacker = &swing.attacker;
        let Ok(mut canonical_manager) = canonical_map_manager.lock() else {
            outcome.attacker_unavailable += 1;
            continue;
        };
        let Some(managed) =
            canonical_manager.find_map_mut(u32::from(attacker.map_id), attacker.instance_id)
        else {
            outcome.attacker_unavailable += 1;
            continue;
        };
        let map = managed.map_mut();

        // Re-read the attacker live: the collect phase released the lock, so a
        // logout, a death or a target switch may have landed since.
        let Some(player) = map.get_typed_player(attacker.player_guid) else {
            outcome.attacker_unavailable += 1;
            continue;
        };
        if !player.unit().is_alive() || player.unit().attacking() != Some(swing.victim_guid) {
            outcome.attacker_unavailable += 1;
            continue;
        }
        let attacker_unit_data = player.unit().data();
        let attacker_position = player.unit().world().position();
        let attacker_combat_reach = attacker_unit_data.combat_reach;

        // Resolve the victim from live state, canonical player first, then the
        // legacy creature. Geometry comes from whichever side owns it.
        let mut legacy_manager = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let victim_runtime = if let Some(creature) = legacy_manager.find_creature_mut(
            attacker.map_id,
            attacker.instance_id,
            swing.victim_guid,
        ) {
            if !creature.is_alive() {
                outcome.victim_not_alive += 1;
                continue;
            }
            let unit_data = creature.creature.unit().data();
            Some((
                true,
                creature.position(),
                unit_data.combat_reach,
                unit_data.bounding_radius,
            ))
        } else {
            map.get_typed_player(swing.victim_guid).map(|victim| {
                let unit_data = victim.unit().data();
                (
                    false,
                    victim.unit().world().position(),
                    unit_data.combat_reach,
                    unit_data.bounding_radius,
                )
            })
        };
        let Some((
            victim_is_creature,
            victim_position,
            victim_combat_reach,
            victim_bounding_radius,
        )) = victim_runtime
        else {
            outcome.victim_missing += 1;
            continue;
        };

        let in_melee_range = is_within_melee_range_like_cpp(
            attacker_position,
            attacker_combat_reach,
            victim_position,
            victim_combat_reach,
        ) && is_within_target_boundary_radius_like_cpp(
            attacker_position,
            attacker_combat_reach,
            victim_position,
            victim_combat_reach,
            victim_bounding_radius,
        );
        let facing_target =
            is_unit_facing_target_for_melee_like_cpp(attacker_position, victim_position);

        // The timer is consumed here, after range and facing, exactly where the
        // session consumed it.
        let Some(player) = map.get_typed_player_mut(attacker.player_guid) else {
            outcome.attacker_unavailable += 1;
            continue;
        };
        let swing_result = take_canonical_player_attack_swings_like_cpp(
            player,
            diff_ms,
            in_melee_range,
            facing_target,
            true,
        );
        let Some((damages, swing_error_update)) = swing_result else {
            continue;
        };

        let mut command = crate::session::mailbox::ApplyPlayerMeleeResultLikeCppCommand {
            attacker_guid: attacker.player_guid,
            map_id: attacker.map_id,
            instance_id: attacker.instance_id,
            victim_guid: Some(swing.victim_guid),
            swing_error_after: swing_error_update,
            combat_target_after: None,
            in_combat_after: None,
            swings: Vec::new(),
            target_level: 0,
            victim_values_update: None,
            killed_creature: None,
        };

        if victim_is_creature {
            let Some(creature) = legacy_manager.find_creature_mut(
                attacker.map_id,
                attacker.instance_id,
                swing.victim_guid,
            ) else {
                outcome.victim_missing += 1;
                outcome.commands.push(command);
                continue;
            };
            let expected_authority = creature.creature.loot_authority_like_cpp().clone();
            let expected_stamp = expected_authority.stamp_like_cpp();
            let Some(hit) = apply_player_melee_to_legacy_creature_like_cpp(
                creature,
                attacker.player_guid,
                &attacker.tap_group_guids,
                Some(&damages),
            ) else {
                outcome.commands.push(command);
                continue;
            };
            outcome.creature_hits += 1;
            command.target_level = hit.level;
            command.swings = hit
                .swings
                .iter()
                .map(|(damage, _killed, over_damage)| {
                    crate::session::mailbox::PlayerMeleeSwingLikeCpp {
                        damage: *damage,
                        over_damage: *over_damage,
                    }
                })
                .collect();
            command.victim_values_update = Some(hit.values_update);
            if hit.died {
                outcome.creature_kills += 1;
                command.killed_creature =
                    Some(crate::session::mailbox::PlayerMeleeCreatureKillLikeCpp {
                        creature_guid: swing.victim_guid,
                        creature_entry: hit.entry,
                        creature_level: hit.level,
                        move_stop: hit.move_stop,
                    });
                command.combat_target_after = Some(None);
                command.in_combat_after = Some(false);
            }
            canonical_syncs.push((
                attacker.map_id,
                attacker.instance_id,
                swing.victim_guid,
                creature.creature.clone(),
                expected_authority,
                expected_stamp,
            ));
        } else {
            drop(legacy_manager);
            let Some(victim) = map.get_typed_player_mut(swing.victim_guid) else {
                outcome.victim_missing += 1;
                outcome.commands.push(command);
                continue;
            };
            let Some((swings, target_level)) =
                apply_player_melee_to_canonical_player_like_cpp(victim, &damages)
            else {
                outcome.victim_not_alive += 1;
                outcome.commands.push(command);
                continue;
            };
            outcome.player_hits += 1;
            command.target_level = target_level;
            command.swings = swings
                .into_iter()
                .map(
                    |(damage, over_damage)| crate::session::mailbox::PlayerMeleeSwingLikeCpp {
                        damage,
                        over_damage,
                    },
                )
                .collect();
        }
        outcome.commands.push(command);
    }

    // Step 4 — mirror. Both guards are released; this takes canonical inside
    // `sync_canonical_creature_entity_on_map_like_cpp` and re-takes legacy for
    // the authority rebind, exactly as the lifecycle phase already does.
    for (map_id, instance_id, guid, creature, expected_authority, expected_stamp) in canonical_syncs
    {
        let authority = sync_canonical_creature_entity_on_map_like_cpp(
            canonical_map_manager,
            u32::from(map_id),
            instance_id,
            creature,
        );
        let Some(authority) = authority else {
            outcome.canonical_mirror_rejections += 1;
            continue;
        };
        let mut legacy = legacy_map_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(world_creature) = legacy.find_creature_mut(map_id, instance_id, guid) {
            let _ = world_creature
                .creature
                .rebind_loot_authority_if_current_like_cpp(
                    &expected_authority,
                    expected_stamp,
                    authority,
                );
        }
    }

    outcome
}
