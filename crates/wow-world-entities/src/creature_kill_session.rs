// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented creature death, kill hooks and the reputation they award at
//! the session boundary (#1363 slice 10b; moved from the World
//! `session/world_entities/creature_kill.rs`): C++ `KillRewarder`'s group
//! reputation rate, `Player::RewardReputation(Unit*, float)` per on-kill
//! faction, the deferred kill reward queue (loot, XP, reputation, quest kill
//! credit) and `Unit::Kill`'s `setDeathState(JUST_DIED)` completion. The
//! social, loot, XP and quest steps are host methods; the former
//! `#[cfg(test)]` event records run only when the World test fixtures are
//! present (`SessionHubHostLikeCpp::test_fixtures_ref_like_cpp`).

use std::sync::Arc;

use wow_constants::unit::Team;
use wow_core::ObjectGuid;
use wow_world_core::session::player_team_for_race_cpp;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedCreatureKillEventLikeCpp;
use crate::world_entities_host::WorldEntitiesHostLikeCpp;

pub fn represented_creature_kill_reputation_rate_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    player_guid: ObjectGuid,
    creature_guid: ObjectGuid,
) -> f32 {
    let (Some(group_guid), Some(group_registry)) = (
        host.resolved_group_guid_like_cpp(),
        host.core().directory.group_registry.as_ref(),
    ) else {
        return 1.0;
    };
    let Some(group) = group_registry.get(&group_guid) else {
        return 1.0;
    };
    if !group.members.contains(&player_guid) {
        return 1.0;
    }
    let group_members = group.members.clone();
    drop(group);

    let Some((player_level, player_map_id, player_position, _)) =
        host.represented_player_group_reward_state_like_cpp(player_guid)
    else {
        return 1.0;
    };

    if host
        .hub_ref()
        .catalogs
        .map_store()
        .and_then(|store| store.get(u32::from(player_map_id)))
        .is_some_and(|entry| entry.is_dungeon())
    {
        return 1.0;
    }

    let (reward_map_id, reward_position) = host
        .core()
        .read_world_creature_like_cpp(creature_guid, |creature| {
            (creature.map_id() as u16, creature.position())
        })
        .unwrap_or((player_map_id, player_position));

    let mut count = 0_u32;
    let mut sum_level = 0_u32;
    for member_guid in group_members {
        if member_guid == player_guid {
            count = count.saturating_add(1);
            sum_level = sum_level.saturating_add(u32::from(player_level));
            continue;
        }
        let Some((member_level, _member_map_id, _member_position, is_alive)) =
            host.represented_player_group_reward_state_like_cpp(member_guid)
        else {
            continue;
        };
        if is_alive
            && host.represented_player_at_group_reward_distance_like_cpp(
                member_guid,
                reward_map_id,
                reward_position,
            )
        {
            count = count.saturating_add(1);
            sum_level = sum_level.saturating_add(u32::from(member_level));
        }
    }

    if count == 0 || sum_level == 0 {
        return 1.0;
    }

    wow_entities::xp_in_group_rate_like_cpp(count, false) * f32::from(player_level)
        / sum_level as f32
}

fn reward_creature_kill_reputation_branch_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
    faction_id: u32,
    reputation_value: i32,
    max_cap: u8,
    creature_level: u8,
    kill_reward_rate: f32,
) {
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = creature_guid;
    if faction_id == 0 {
        return;
    }

    let effective_faction_id = host
        .hub_ref()
        .represented_championing_faction_for_kill_like_cpp()
        .unwrap_or(faction_id);

    let faction_store = match host.hub_ref().catalogs.faction_store().map(Arc::clone) {
        Some(store) => store,
        None => return,
    };
    let Some(faction_entry) = faction_store.get(effective_faction_id).cloned() else {
        return;
    };

    let mut reputation = host.calculate_kill_reputation_gain_like_cpp(
        creature_level,
        reputation_value,
        effective_faction_id,
    );
    reputation = (reputation as f32 * kill_reward_rate) as i32;
    if reputation == 0 {
        return;
    }

    // Hoist the player identity: `self.player_*_like_cpp()` re-enters the
    // canonical manager lock held by the closure and would self-deadlock.
    let player_race = host.hub_ref().player_race_like_cpp();
    let player_class = host.hub_ref().player_class_like_cpp();
    let friendship_rep_reaction_store = host
        .hub_ref()
        .catalogs
        .friendship_rep_reaction_store
        .as_deref();
    let Some(current_rank) = host.hub_ref().with_reputation_mgr_like_cpp(|mgr| {
        mgr.rank_for_faction_entry_like_cpp(
            &faction_entry,
            friendship_rep_reaction_store,
            player_race,
            player_class,
        )
    }) else {
        return;
    };
    let spillover_only = current_rank.as_u8() > max_cap;

    let reputation_spillover_template_store = host
        .hub_ref()
        .catalogs
        .reputation_spillover_template_store()
        .map(Arc::clone);
    let friendship_rep_reaction_store = host
        .hub_ref()
        .catalogs
        .friendship_rep_reaction_store()
        .map(Arc::clone);
    let paragon_reputation_store = host
        .hub_ref()
        .catalogs
        .paragon_reputation_store()
        .map(Arc::clone);
    let currency_types_store = host
        .hub_ref()
        .catalogs
        .currency_types_store()
        .map(Arc::clone);
    let db_spillover_template = reputation_spillover_template_store
        .as_deref()
        .and_then(|store| store.get(effective_faction_id));
    let options = wow_progression::mgr::SetReputationOptionsLikeCpp {
        incremental: true,
        spillover_only,
        no_spillover: false,
        reputation_gain_rate: host.hub_ref().config.reputation_rates_like_cpp().gain,
        paragon_reward_quest_status_none_like_cpp: true,
        renown_current_level_like_cpp: 0,
        renown_currency_increased_cap_quantity_like_cpp: 0,
        player_race: host.hub_ref().player_race_like_cpp(),
        player_class: host.hub_ref().player_class_like_cpp(),
    };
    let Some((outcome, packet)) = host.hub_mut().mutate_reputation_mgr_like_cpp(|mgr| {
        let outcome = mgr.set_reputation_like_cpp(
            &faction_entry,
            reputation,
            options,
            &faction_store,
            db_spillover_template,
            friendship_rep_reaction_store.as_deref(),
            paragon_reputation_store.as_deref(),
            currency_types_store.as_deref(),
        );
        let packet = outcome
            .send_state_rep_list_id
            .map(|rep_list_id| mgr.set_faction_standing_packet_like_cpp(Some(rep_list_id)));
        (outcome, packet)
    }) else {
        return;
    };
    if let Some(packet) = packet {
        host.send_packet(&packet);
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = outcome;
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && outcome.applied {
        host.world_entities_mut()
            .record_represented_creature_kill_event_like_cpp(
                RepresentedCreatureKillEventLikeCpp::CreatureKillReputationAwarded {
                    creature_guid,
                    faction_id: effective_faction_id,
                    reputation,
                    spillover_only,
                },
            );
    }
}

pub fn reward_reputation_from_creature_kill_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    creature_entry: u32,
    creature_guid: ObjectGuid,
    creature_level: u8,
    kill_reward_rate: f32,
) {
    if host
        .core()
        .read_world_creature_like_cpp(creature_guid, |creature| {
            creature.is_reputation_gain_disabled()
        })
        .unwrap_or(false)
    {
        return;
    }

    let Some(rep) = host
        .hub_ref()
        .catalogs
        .creature_onkill_reputation_store()
        .and_then(|store| store.get(creature_entry))
        .copied()
    else {
        return;
    };
    let team = player_team_for_race_cpp(host.hub_ref().player_race_like_cpp());

    if rep.rep_faction_1 != 0 && (!rep.team_dependent || team == Team::Alliance) {
        reward_creature_kill_reputation_branch_like_cpp(
            host,
            creature_guid,
            rep.rep_faction_1,
            rep.rep_value_1,
            rep.reputation_max_cap_1,
            creature_level,
            kill_reward_rate,
        );
    }
    if rep.rep_faction_2 != 0 && (!rep.team_dependent || team == Team::Horde) {
        reward_creature_kill_reputation_branch_like_cpp(
            host,
            creature_guid,
            rep.rep_faction_2,
            rep.rep_value_2,
            rep.reputation_max_cap_2,
            creature_level,
            kill_reward_rate,
        );
    }
}

pub async fn process_pending_creature_kills_with_generator_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) {
    let mut pending = host
        .world_entities_mut()
        .take_pending_creature_kill_loot_like_cpp();
    pending.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
    pending.dedup();
    for creature_guid in pending {
        host.ensure_represented_creature_kill_loot_like_cpp(creature_guid)
            .await;
    }

    let mut rewards = host
        .world_entities_mut()
        .take_pending_creature_kill_rewards_like_cpp();
    rewards.sort_by_key(|reward| {
        (
            reward.creature_guid.high_value(),
            reward.creature_guid.low_value(),
        )
    });
    rewards.dedup_by_key(|reward| reward.creature_guid);
    for reward in rewards {
        let can_give_experience = host
            .core()
            .read_world_creature_like_cpp(reward.creature_guid, |creature| {
                creature.can_give_experience_like_cpp()
            })
            .unwrap_or(false);
        let xp = can_give_experience
            .then(|| {
                let (s, h) = host.split_world_entities_ref();
                s.creature_kill_xp(h, reward.creature_level)
            })
            .unwrap_or(0);
        if xp > 0 {
            // The represented reward queue is still solo-session only.
            // Full C++ KillRewarder group fanout must pre-scale XP per
            // member before passing its separate `_groupRate` here.
            host.give_xp(xp, reward.creature_guid, 1.0).await;
        }
        let reputation_rate = represented_creature_kill_reputation_rate_like_cpp(
            host,
            reward.killer_guid,
            reward.creature_guid,
        );
        reward_reputation_from_creature_kill_like_cpp(
            host,
            reward.creature_entry,
            reward.creature_guid,
            reward.creature_level,
            reputation_rate,
        );
        host.on_creature_killed_with_generator_like_cpp(
            item_guid_generator,
            reward.creature_entry,
            reward.creature_guid,
        )
        .await;
        #[cfg(any(test, feature = "test-fixtures"))]
        if host.test_fixtures_ref_like_cpp().is_some() {
            record_represented_creature_kill_hooks_like_cpp(
                host,
                reward.killer_guid,
                reward.creature_guid,
            );
        }
        if let Some(values_update) =
            complete_represented_creature_death_state_after_kill_hooks_like_cpp(
                host,
                reward.killer_guid,
                reward.creature_guid,
            )
            && host
                .core()
                .client_visible_guids_like_cpp
                .contains(&reward.creature_guid)
            && let Some(update) = host.represented_unit_values_update_to_update_object_like_cpp(
                reward.creature_guid,
                host.core().player_map_id_like_cpp(),
                &values_update,
            )
        {
            host.send_packet(&update);
        }
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
pub fn record_represented_creature_kill_hooks_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    attacker_guid: ObjectGuid,
    creature_guid: ObjectGuid,
) {
    let reward_source = host
        .core()
        .read_world_creature_like_cpp(creature_guid, |creature| {
            (creature.map_id() as u16, creature.position())
        })
        .or_else(|| {
            host.hub_ref()
                .player_position_like_cpp()
                .map(|position| (host.core().player_map_id_like_cpp(), position))
        });
    let Some(reward_source) = reward_source else {
        return;
    };
    let mut tappers = host
        .core()
        .read_world_creature_like_cpp(creature_guid, |creature| creature.tap_list().to_vec())
        .unwrap_or_default();
    if tappers.is_empty() {
        tappers.push(attacker_guid);
    }
    let mut unique_tappers = Vec::with_capacity(tappers.len());
    for tapper in tappers {
        if !unique_tappers.contains(&tapper) {
            unique_tappers.push(tapper);
        }
    }

    host.world_entities_mut()
        .record_represented_creature_kill_event_like_cpp(
            RepresentedCreatureKillEventLikeCpp::KillerProc {
                attacker_guid,
                victim_guid: creature_guid,
            },
        );

    for tapper_guid in unique_tappers {
        if !host.represented_player_at_group_reward_distance_like_cpp(
            tapper_guid,
            reward_source.0,
            reward_source.1,
        ) {
            continue;
        }
        host.world_entities_mut()
            .record_represented_creature_kill_event_like_cpp(
                RepresentedCreatureKillEventLikeCpp::TapperTargetDiesProc {
                    tapper_guid,
                    victim_guid: creature_guid,
                },
            );
    }

    host.world_entities_mut()
        .record_represented_creature_kill_event_like_cpp(
            RepresentedCreatureKillEventLikeCpp::VictimDeathProc {
                victim_guid: creature_guid,
            },
        );
    host.world_entities_mut()
        .record_represented_creature_kill_event_like_cpp(
            RepresentedCreatureKillEventLikeCpp::DeliveredKillingBlowCriteria {
                player_guid: attacker_guid,
                victim_guid: creature_guid,
                quantity: 1,
            },
        );
}

pub fn complete_represented_creature_death_state_after_kill_hooks_like_cpp<
    H: WorldEntitiesHostLikeCpp,
>(
    host: &mut H,
    attacker_guid: ObjectGuid,
    creature_guid: ObjectGuid,
) -> Option<wow_entities::UnitValuesUpdate> {
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = attacker_guid;
    let lootable = host
        .cached_loot_for_owner_like_cpp(creature_guid)
        .is_some_and(|loot| loot.coins != 0 || loot.unlooted_count != 0);
    let can_skin = {
        let (s, mut h) = host.split_world_entities_mut();
        s.represented_creature_can_skin_after_death_state_like_cpp(&mut h, creature_guid)
    };
    // `Unit::Kill` runs `setDeathState(JUST_DIED)` once per kill. #1263
    // F6-8D3b-2: the canonical-first root no longer refuses a replay on a
    // stale copy, so the completion itself only proceeds when this call
    // performs the JUST_DIED transition (AI-dead, unit not yet dead).
    let values_update = host
        .core_mut()
        .mutate_world_creature(creature_guid, |creature| {
            if creature.ai_state() != wow_entities::CreatureAiState::Dead
                || creature.unit().is_dead()
            {
                return None;
            }
            creature.complete_death_state_after_kill_hooks_like_cpp();
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(lootable, can_skin);
            Some(creature.unit().values_update())
        })
        .flatten()?;
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() {
        host.world_entities_mut()
            .record_represented_creature_kill_event_like_cpp(
                RepresentedCreatureKillEventLikeCpp::DeathStateJustDied {
                    victim_guid: creature_guid,
                },
            );
        host.world_entities_mut()
            .record_represented_creature_kill_event_like_cpp(
                RepresentedCreatureKillEventLikeCpp::ZoneScriptUnitDeath {
                    unit_guid: creature_guid,
                },
            );
        record_represented_tapper_pet_killed_unit_hooks_like_cpp(host, creature_guid);
        host.world_entities_mut()
            .record_represented_creature_kill_event_like_cpp(
                RepresentedCreatureKillEventLikeCpp::LootFlagsApplied {
                    creature_guid,
                    lootable,
                    can_skin,
                    skinnable: can_skin,
                },
            );
        host.world_entities_mut()
            .record_represented_creature_kill_event_like_cpp(
                RepresentedCreatureKillEventLikeCpp::CreatureOnHealthDepletedAi {
                    creature_guid,
                    attacker_guid,
                    is_kill: true,
                },
            );
        host.world_entities_mut()
            .record_represented_creature_kill_event_like_cpp(
                RepresentedCreatureKillEventLikeCpp::CreatureJustDiedAi {
                    creature_guid,
                    killer_guid: attacker_guid,
                },
            );
        if attacker_guid.is_player() {
            host.world_entities_mut()
                .record_represented_creature_kill_event_like_cpp(
                    RepresentedCreatureKillEventLikeCpp::ScriptMgrOnCreatureKill {
                        killer_guid: attacker_guid,
                        creature_guid,
                    },
                );
        }
    }
    Some(values_update)
}

#[cfg(any(test, feature = "test-fixtures"))]
fn record_represented_tapper_pet_killed_unit_hooks_like_cpp<H: WorldEntitiesHostLikeCpp>(
    host: &mut H,
    creature_guid: ObjectGuid,
) {
    let (Some(player_guid), Some(pet_guid)) = (
        host.player_guid(),
        host.hub_ref().player_pet_guid_state_like_cpp().flatten(),
    ) else {
        return;
    };
    let tapper_has_current_player = host
        .core_mut()
        .mutate_world_creature(creature_guid, |creature| {
            creature.tap_list().contains(&player_guid)
        })
        .unwrap_or(true);
    if !tapper_has_current_player {
        return;
    }
    host.world_entities_mut()
        .record_represented_creature_kill_event_like_cpp(
            RepresentedCreatureKillEventLikeCpp::TapperPetKilledUnitAi {
                tapper_guid: player_guid,
                pet_guid,
                victim_guid: creature_guid,
            },
        );
}
