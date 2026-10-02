// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot-related snapshots and recipient operations over the canonical directory.

use std::sync::Arc;

use super::*;
use crate::session::mailbox::ApplyLootMoneyLikeCppCommand;

impl PlayerRegistry {
    /// Snapshot only the presence facts used by loot and group-reward gates.
    #[must_use]
    pub fn loot_presence(&self, guid: ObjectGuid) -> Option<PlayerLootPresenceSnapshot> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerLootPresenceSnapshot {
            registration: PlayerRegistration {
                guid,
                generation: entry.generation,
            },
            guid,
            map_id: entry.placement.map_id,
            instance_id: entry.placement.instance_id,
            position: entry.placement.position,
            is_in_world: entry.placement.is_in_world,
        })
    }

    /// Resolve current in-world recipients in one exact map instance.
    #[must_use]
    pub fn same_map_loot_recipients(
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

    /// Resolve one current recipient only when it is in the requested map instance.
    #[must_use]
    pub fn loot_delivery_recipient(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Option<PlayerRegistration> {
        let entry = self.entries.get(&guid)?;
        (entry.placement.map_id == map_id && entry.placement.instance_id == instance_id).then_some(
            PlayerRegistration {
                guid,
                generation: entry.generation,
            },
        )
    }

    /// Resolve one current in-world recipient in the requested map instance.
    #[must_use]
    pub fn in_world_loot_delivery_recipient(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Option<PlayerRegistration> {
        let entry = self.entries.get(&guid)?;
        (entry.placement.is_in_world
            && entry.placement.map_id == map_id
            && entry.placement.instance_id == instance_id)
            .then_some(PlayerRegistration {
                guid,
                generation: entry.generation,
            })
    }

    /// Read one remote enchanting skill without exposing the Player mirror.
    #[must_use]
    pub fn loot_enchanting_skill(&self, guid: ObjectGuid) -> Option<u16> {
        let (map_id, instance_id) = {
            let entry = self.entries.get(&guid)?;
            (entry.placement.map_id, entry.placement.instance_id)
        };
        self.canonical_at(guid, map_id, instance_id, |player| {
            player
                .gameplay_state()
                .skills
                .iter()
                .find(|skill| {
                    skill.skill_line_id == u32::from(crate::session::SKILL_ENCHANTING_LIKE_CPP)
                })
                .map_or(0, |skill| skill.current_value)
        })
    }

    /// Read one connected peer's C++ `Player::GetPassOnGroupLoot()` state.
    #[must_use]
    pub fn loot_pass_on_group_loot(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> Option<bool> {
        let entry = self.entries.get(&guid)?;
        if entry.placement.map_id != map_id || entry.placement.instance_id != instance_id {
            return None;
        }
        drop(entry);
        self.canonical_at(guid, map_id, instance_id, |player| {
            player.gameplay_state().pass_on_group_loot
        })
    }

    /// Snapshot the exact remote facts used by represented loot conditions.
    #[must_use]
    pub fn loot_player_context(&self, guid: ObjectGuid) -> Option<PlayerLootContextSnapshot> {
        let (race, class, sex, placement) = {
            let entry = self.entries.get(&guid)?;
            (
                entry.identity.race,
                entry.identity.class,
                entry.identity.sex,
                entry.placement,
            )
        };
        self.canonical_at(guid, placement.map_id, placement.instance_id, |player| {
            let state = player.gameplay_state();
            PlayerLootContextSnapshot {
                race,
                class,
                sex,
                level: placement.level,
                known_spells: state
                    .spells
                    .known_spells_like_cpp()
                    .iter()
                    .copied()
                    .collect(),
                active_quest_statuses: state
                    .quests
                    .statuses_like_cpp()
                    .values()
                    .map(|status| (status.quest_id, status.status))
                    .collect(),
                active_quest_objective_counts: state
                    .quests
                    .objective_counts_by_quest_like_cpp()
                    .iter()
                    .cloned()
                    .collect(),
                rewarded_quests: state
                    .quests
                    .rewarded_quest_ids_like_cpp()
                    .iter()
                    .copied()
                    .collect(),
                inventory_item_counts: player.inventory_item_counts_like_cpp(),
            }
        })
    }

    /// Snapshot the exact remote facts used by C++ group reward calculations.
    #[must_use]
    pub fn group_reward_snapshot(&self, guid: ObjectGuid) -> Option<PlayerGroupRewardSnapshot> {
        let entry = self.entries.get(&guid)?;
        Some(PlayerGroupRewardSnapshot {
            level: entry.placement.level,
            map_id: entry.placement.map_id,
            position: entry.placement.position,
            is_alive: entry.placement.is_alive,
        })
    }

    /// Find the exact live loot-roll identity owned by another map peer.
    #[must_use]
    pub fn loot_roll_owner(
        &self,
        excluded_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
    ) -> Option<(PlayerRegistration, LootRollCommandIdentityLikeCpp)> {
        self.entries.iter().find_map(|entry| {
            let guid = *entry.key();
            let value = entry.value();
            if guid == excluded_guid
                || value.placement.map_id != map_id
                || value.placement.instance_id != instance_id
            {
                return None;
            }
            let identity = value
                .active_loot_rolls
                .iter()
                .find(|identity| identity.matches_key_like_cpp(loot_obj, loot_list_id))?
                .clone();
            Some((
                PlayerRegistration {
                    guid,
                    generation: entry.value().generation,
                },
                identity,
            ))
        })
    }

    /// Publish the current session's represented C++ `Player::m_lootRolls` identities.
    pub fn replace_loot_rolls_for_control_channel(
        &self,
        guid: ObjectGuid,
        command_tx: &flume::Sender<SessionCommand>,
        identities: Vec<LootRollCommandIdentityLikeCpp>,
    ) -> bool {
        let Some(mut entry) = self.entries.get_mut(&guid) else {
            return false;
        };
        if !entry.command_tx.same_channel(command_tx) {
            return false;
        }
        entry.active_loot_rolls = identities;
        true
    }

    /// Prepare a remote loot-money application without exposing its persistence tracker.
    #[must_use]
    pub fn prepare_loot_money_application(
        &self,
        input: PrepareLootMoneyApplicationLikeCpp,
    ) -> Option<PreparedLootMoneyApplicationLikeCpp> {
        let entry = self.entries.get(&input.recipient)?;
        Some(PreparedLootMoneyApplicationLikeCpp {
            registration: PlayerRegistration {
                guid: input.recipient,
                generation: entry.generation,
            },
            command: ApplyLootMoneyLikeCppCommand {
                recipient: input.recipient,
                loot_owner: input.loot_owner,
                loot_obj: input.loot_obj,
                amount: input.amount,
                durable_applied_amount: input.durable_applied_amount,
                durable_persistence_tracker: Arc::clone(&entry.durable_loot_money),
                sole_looter: input.sole_looter,
                authority: input.authority,
                authority_generation: input.authority_generation,
                authority_committed: input.authority_committed,
                send_coin_removed: input.send_coin_removed,
                applied: input.applied,
                published: input.published,
            },
        })
    }
}
