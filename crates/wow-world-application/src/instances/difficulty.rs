// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use tracing::{debug, warn};
use wow_constants::shared::DifficultyFlags;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    DungeonDifficultySet, RaidDifficultySet, SetDifficultyId, SetDungeonDifficulty,
    SetRaidDifficulty, ToggleDifficulty,
};
use wow_persistence::{
    RepresentedGroupPersistenceModeLikeCpp, RepresentedGroupPersistenceOutcomeLikeCpp,
    RepresentedGroupPersistenceRequestLikeCpp,
};
use wow_social::group::GroupDifficultyKindLikeCpp;
use wow_world_instances::SessionDifficultyKindLikeCpp;

use crate::instances::{
    InstanceDifficultyHandlerCxLikeCpp, InstanceResetMethodLikeCpp,
};
use crate::instances::lock_operations::reset_locks_with_access_like_cpp;

impl InstanceDifficultyHandlerCxLikeCpp<'_> {
    pub fn apply_group_difficulty_like_cpp(
        &mut self,
        group_guid: u64,
        difficulty_id: u32,
        kind: GroupDifficultyKindLikeCpp,
    ) {
        apply_group_difficulty_like_cpp(self, group_guid, difficulty_id, kind);
    }
}

pub async fn handle_set_difficulty_id_like_cpp(
    cx: &mut InstanceDifficultyHandlerCxLikeCpp<'_>,
    mut pkt: wow_packet::WorldPacket,
) {
    let packet = match SetDifficultyId::read(&mut pkt) {
        Ok(packet) => packet,
        Err(error) => {
            warn!(
                account = cx.player.account_id_like_cpp(),
                "SetDifficultyId parse failed: {error}"
            );
            return;
        }
    };
    apply_represented_difficulty_change_like_cpp(cx, packet.difficulty_id).await;
}

pub async fn handle_toggle_difficulty_like_cpp(
    cx: &mut InstanceDifficultyHandlerCxLikeCpp<'_>,
    mut pkt: wow_packet::WorldPacket,
) {
    if let Err(error) = ToggleDifficulty::read(&mut pkt) {
        warn!(
            account = cx.player.account_id_like_cpp(),
            "ToggleDifficulty parse failed: {error}"
        );
        return;
    }

    let Some(difficulty_id) = cx
        .difficulty_store
        .and_then(|store| {
            cx.instances
                .represented_toggle_difficulty_target_with_access_like_cpp(
                    &cx.player,
                    store,
                )
        })
    else {
        debug!(
            account = cx.player.account_id_like_cpp(),
            "ToggleDifficulty has no represented toggle difficulty available"
        );
        return;
    };

    apply_represented_difficulty_change_like_cpp(cx, difficulty_id).await;
}

pub async fn handle_set_dungeon_difficulty_like_cpp(
    cx: &mut InstanceDifficultyHandlerCxLikeCpp<'_>,
    mut pkt: wow_packet::WorldPacket,
) {
    let packet = match SetDungeonDifficulty::read(&mut pkt) {
        Ok(packet) => packet,
        Err(error) => {
            warn!(
                account = cx.player.account_id_like_cpp(),
                "SetDungeonDifficulty parse failed: {error}"
            );
            return;
        }
    };
    apply_represented_difficulty_change_like_cpp(cx, packet.difficulty_id).await;
}

pub async fn handle_set_raid_difficulty_like_cpp(
    cx: &mut InstanceDifficultyHandlerCxLikeCpp<'_>,
    mut pkt: wow_packet::WorldPacket,
) {
    let packet = match SetRaidDifficulty::read(&mut pkt) {
        Ok(packet) => packet,
        Err(error) => {
            warn!(
                account = cx.player.account_id_like_cpp(),
                "SetRaidDifficulty parse failed: {error}"
            );
            return;
        }
    };

    let Some(difficulty_id) = cx.difficulty_store.and_then(|store| {
        cx.instances
            .represented_raid_difficulty_request_like_cpp(
                store,
                packet.difficulty_id,
                packet.legacy != 0,
            )
    }) else {
        return;
    };

    apply_represented_difficulty_change_like_cpp(cx, difficulty_id).await;
}

async fn apply_represented_difficulty_change_like_cpp(
    cx: &mut InstanceDifficultyHandlerCxLikeCpp<'_>,
    difficulty_id: u32,
) {
    if let Some(reset_owner) = represented_set_difficulty_reset_owner_like_cpp(cx, difficulty_id) {
        let _ = reset_locks_with_access_like_cpp(
            &cx.locks,
            cx.lifecycle,
            &cx.packets,
            cx.map_store,
            cx.map_difficulty_store,
            &cx.player,
            reset_owner,
            InstanceResetMethodLikeCpp::OnChangeDifficulty,
        )
        .await;
    }

    let commands = represented_set_difficulty_id_like_cpp(cx, difficulty_id);
    if commands.is_empty() {
        return;
    }

    let Some(port) = cx.lifecycle.represented_group_persistence_port_like_cpp() else {
        return;
    };
    let outcome = port
        .persist_group_commands_like_cpp(RepresentedGroupPersistenceRequestLikeCpp {
            commands,
            mode: RepresentedGroupPersistenceModeLikeCpp::Atomic,
        })
        .await;
    if !matches!(
        outcome,
        RepresentedGroupPersistenceOutcomeLikeCpp::Applied { .. }
    ) {
        warn!(
            account = cx.player.account_id_like_cpp(),
            player_guid = ?cx.player.player_guid_like_cpp(),
            ?outcome,
            "failed to persist represented group difficulty change"
        );
    }
}

fn represented_set_difficulty_reset_owner_like_cpp(
    cx: &InstanceDifficultyHandlerCxLikeCpp<'_>,
    difficulty_id: u32,
) -> Option<wow_core::ObjectGuid> {
    let entry = cx.difficulty_store?.get(difficulty_id).copied()?;
    let flags = DifficultyFlags::from_bits_truncate(entry.flags);
    if !flags.contains(DifficultyFlags::CAN_SELECT) || current_map_instanceable_like_cpp(cx) {
        return None;
    }

    let player_guid = cx.player.player_guid_like_cpp()?;
    if let Some(group_guid) = resolved_group_guid_like_cpp(cx) {
        let group = cx.groups.group_snapshot_like_cpp(group_guid)?;
        if !group.is_leader_like_cpp(player_guid) || group.is_lfg_group_like_cpp() {
            return None;
        }

        let current = if entry.instance_type == wow_data::map::MAP_INSTANCE as u8 {
            group.dungeon_difficulty_id
        } else if entry.instance_type == wow_data::map::MAP_RAID as u8 {
            if flags.contains(DifficultyFlags::LEGACY) {
                group.legacy_raid_difficulty_id
            } else {
                group.raid_difficulty_id
            }
        } else {
            return None;
        };
        return (current != difficulty_id).then_some(group.leader_guid);
    }

    let (dungeon, raid, legacy_raid) = cx
        .instances
        .player_difficulty_preferences_with_access_like_cpp(&cx.player)?;
    let current = if entry.instance_type == wow_data::map::MAP_INSTANCE as u8 {
        dungeon
    } else if entry.instance_type == wow_data::map::MAP_RAID as u8 {
        if flags.contains(DifficultyFlags::LEGACY) {
            legacy_raid
        } else {
            raid
        }
    } else {
        return None;
    };
    (current != difficulty_id).then_some(player_guid)
}

fn represented_set_difficulty_id_like_cpp(
    cx: &mut InstanceDifficultyHandlerCxLikeCpp<'_>,
    difficulty_id: u32,
) -> Vec<wow_persistence::RepresentedGroupPersistenceCommandLikeCpp> {
    let Some((current_dungeon, current_raid, current_legacy_raid)) = cx
        .instances
        .player_difficulty_preferences_with_access_like_cpp(&cx.player)
    else {
        return Vec::new();
    };
    let Some(entry) = cx
        .difficulty_store
        .and_then(|store| store.get(difficulty_id))
        .copied()
    else {
        return Vec::new();
    };

    let flags = DifficultyFlags::from_bits_truncate(entry.flags);
    if !flags.contains(DifficultyFlags::CAN_SELECT) {
        return Vec::new();
    }
    if current_map_instanceable_like_cpp(cx) {
        return Vec::new();
    }

    if entry.instance_type == wow_data::map::MAP_INSTANCE as u8 {
        if let Some(command) = set_represented_group_difficulty_like_cpp(
            cx,
            difficulty_id,
            GroupDifficultyKindLikeCpp::Dungeon,
        ) {
            return vec![command];
        }
        if resolved_group_guid_like_cpp(cx).is_some() || difficulty_id == current_dungeon {
            return Vec::new();
        }
        if !cx.instances.set_player_difficulty_with_access_like_cpp(
            &cx.player,
            SessionDifficultyKindLikeCpp::Dungeon,
            difficulty_id,
        ) {
            return Vec::new();
        }
        cx.packets.send_packet(&DungeonDifficultySet {
            difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
        });
        Vec::new()
    } else if entry.instance_type == wow_data::map::MAP_RAID as u8 {
        let legacy = flags.contains(DifficultyFlags::LEGACY);
        let kind = if legacy {
            GroupDifficultyKindLikeCpp::LegacyRaid
        } else {
            GroupDifficultyKindLikeCpp::Raid
        };
        if let Some(command) = set_represented_group_difficulty_like_cpp(cx, difficulty_id, kind) {
            return vec![command];
        }
        if resolved_group_guid_like_cpp(cx).is_some() {
            return Vec::new();
        }
        let current = if legacy {
            current_legacy_raid
        } else {
            current_raid
        };
        if difficulty_id == current {
            return Vec::new();
        }
        if !cx.instances.set_player_difficulty_with_access_like_cpp(
            &cx.player,
            if legacy {
                SessionDifficultyKindLikeCpp::LegacyRaid
            } else {
                SessionDifficultyKindLikeCpp::Raid
            },
            difficulty_id,
        ) {
            return Vec::new();
        }
        cx.packets.send_packet(&RaidDifficultySet {
            difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
            legacy,
        });
        Vec::new()
    } else {
        Vec::new()
    }
}

fn set_represented_group_difficulty_like_cpp(
    cx: &mut InstanceDifficultyHandlerCxLikeCpp<'_>,
    difficulty_id: u32,
    kind: GroupDifficultyKindLikeCpp,
) -> Option<wow_persistence::RepresentedGroupPersistenceCommandLikeCpp> {
    let group_guid = resolved_group_guid_like_cpp(cx)?;
    let player_guid = cx.player.player_guid_like_cpp()?;
    let outcome = cx
        .groups
        .set_difficulty_transition_like_cpp(group_guid, player_guid, difficulty_id, kind)?
        .ok()?;
    let persistence = outcome.persistence;
    let members = outcome.group.members;

    for member_guid in members {
        if member_guid == player_guid {
            apply_group_difficulty_like_cpp(cx, group_guid, difficulty_id, kind);
            continue;
        }
        // C++ `Group::SetDungeonDifficultyID` writes each connected member's
        // preference in member iteration order (#743).
        cx.groups.apply_member_transition_like_cpp(
            member_guid,
            group_guid,
            difficulty_id,
            kind,
        );
    }

    persistence
        .into_iter()
        .next()
        .map(wow_world_lifecycle::group_persistence_command_like_cpp)
}

fn apply_group_difficulty_like_cpp(
    cx: &mut InstanceDifficultyHandlerCxLikeCpp<'_>,
    group_guid: u64,
    difficulty_id: u32,
    kind: GroupDifficultyKindLikeCpp,
) {
    if resolved_group_guid_like_cpp(cx) != Some(group_guid) {
        return;
    }
    let session_kind = match kind {
        GroupDifficultyKindLikeCpp::Dungeon => SessionDifficultyKindLikeCpp::Dungeon,
        GroupDifficultyKindLikeCpp::Raid => SessionDifficultyKindLikeCpp::Raid,
        GroupDifficultyKindLikeCpp::LegacyRaid => SessionDifficultyKindLikeCpp::LegacyRaid,
    };
    if !cx.instances.set_player_difficulty_with_access_like_cpp(
        &cx.player,
        session_kind,
        difficulty_id,
    ) {
        return;
    }
    match kind {
        GroupDifficultyKindLikeCpp::Dungeon => {
            cx.packets.send_packet(&DungeonDifficultySet {
                difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
            });
        }
        GroupDifficultyKindLikeCpp::Raid => {
            cx.packets.send_packet(&RaidDifficultySet {
                difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
                legacy: false,
            });
        }
        GroupDifficultyKindLikeCpp::LegacyRaid => {
            cx.packets.send_packet(&RaidDifficultySet {
                difficulty_id: i32::try_from(difficulty_id).unwrap_or(i32::MAX),
                legacy: true,
            });
        }
    }
}

fn resolved_group_guid_like_cpp(cx: &InstanceDifficultyHandlerCxLikeCpp<'_>) -> Option<u64> {
    cx.social
        .resolved_group_guid_with_access_like_cpp(&cx.group_owner, cx.consumer_test)
}

fn current_map_instanceable_like_cpp(cx: &InstanceDifficultyHandlerCxLikeCpp<'_>) -> bool {
    let map_id = u32::from(cx.player.player_map_id_like_cpp());
    cx.map_store
        .and_then(|store| store.get(map_id))
        .is_some_and(|entry| {
            matches!(
                entry.instance_type,
                wow_data::map::MAP_INSTANCE
                    | wow_data::map::MAP_RAID
                    | wow_data::map::MAP_BATTLEGROUND
                    | wow_data::map::MAP_ARENA
                    | wow_data::map::MAP_SCENARIO
            )
        })
}
