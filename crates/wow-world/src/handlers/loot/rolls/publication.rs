// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot-roll criterion updates and packet publication.

use super::*;

impl WorldSession {
    pub(super) fn update_represented_loot_roll_vote_criteria_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        roll_type: u8,
    ) {
        match roll_type {
            ROLL_VOTE_NEED_LIKE_CPP => {
                self.record_represented_roll_any_need_criteria_like_cpp(player_guid, 1)
            }
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                self.record_represented_roll_any_greed_criteria_like_cpp(player_guid, 1)
            }
            _ => {}
        }
    }

    pub(super) fn update_represented_loot_roll_winner_criteria_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        item_id: u32,
        winner_vote: RepresentedLootRollVote,
    ) {
        match winner_vote.vote {
            ROLL_VOTE_NEED_LIKE_CPP => self.record_represented_roll_need_criteria_like_cpp(
                player_guid,
                item_id,
                winner_vote.roll_number,
            ),
            ROLL_VOTE_DISENCHANT_LIKE_CPP => self.record_represented_disenchant_criteria_like_cpp(
                player_guid,
                DISENCHANT_LOOT_ROLL_CRITERIA_SPELL_LIKE_CPP,
            ),
            ROLL_VOTE_GREED_LIKE_CPP => self.record_represented_roll_greed_criteria_like_cpp(
                player_guid,
                item_id,
                winner_vote.roll_number,
            ),
            _ => {}
        }
    }

    pub(super) fn record_represented_roll_any_need_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _quantity: u32,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollAnyNeed {
                player_guid: _player_guid,
                quantity: _quantity,
            },
        );
    }

    pub(super) fn record_represented_roll_any_greed_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _quantity: u32,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollAnyGreed {
                player_guid: _player_guid,
                quantity: _quantity,
            },
        );
    }

    pub(super) fn record_represented_roll_need_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _item_id: u32,
        _roll_number: u8,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollNeed {
                player_guid: _player_guid,
                item_id: _item_id,
                roll_number: _roll_number,
            },
        );
    }

    pub(super) fn record_represented_roll_greed_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _item_id: u32,
        _roll_number: u8,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollGreed {
                player_guid: _player_guid,
                item_id: _item_id,
                roll_number: _roll_number,
            },
        );
    }

    pub(super) fn send_represented_loot_roll_final_values_like_cpp(
        &self,
        loot_obj: ObjectGuid,
        entry: &LootEntry,
        winner_guid: ObjectGuid,
        state: &RepresentedLootRollState,
        dungeon_encounter_id: i32,
    ) {
        for (player_guid, vote) in &state.voters {
            let (roll, roll_type) = match vote.vote {
                ROLL_VOTE_PASS_LIKE_CPP => continue,
                ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP | ROLL_VOTE_NOT_VALID_LIKE_CPP => {
                    (0, ROLL_VOTE_PASS_LIKE_CPP)
                }
                ROLL_VOTE_NEED_LIKE_CPP
                | ROLL_VOTE_GREED_LIKE_CPP
                | ROLL_VOTE_DISENCHANT_LIKE_CPP => (i32::from(vote.roll_number), vote.vote),
                _ => continue,
            };

            let ongoing = LootRollBroadcast {
                loot_obj,
                player: *player_guid,
                roll,
                roll_type,
                item: loot_roll_broadcast_item_like_cpp(
                    entry,
                    LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
                ),
                autopassed: false,
                off_spec: false,
                dungeon_encounter_id,
            };

            self.broadcast_represented_loot_roll_packet_to_voters_like_cpp(
                &ongoing,
                state,
                Some(winner_guid),
            );

            let allow = LootRollBroadcast {
                item: loot_roll_broadcast_item_like_cpp(entry, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
                ..ongoing
            };
            self.send_represented_loot_roll_packet_to_player_like_cpp(&allow, winner_guid);
        }
    }

    pub(super) fn send_represented_loot_roll_packet_to_player_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        target: ObjectGuid,
    ) {
        if self.player_guid() == Some(target) {
            self.send_packet(packet);
            return;
        }

        let Some(registry) = self.player_registry() else {
            return;
        };
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some(registration) =
            registry.loot_delivery_recipient(target, self.player_map_id_like_cpp(), instance_id)
        else {
            return;
        };

        let _ = registry.send_current_packet(registration, packet.to_bytes());
    }

    pub(super) fn broadcast_represented_loot_roll_packet_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        entry: &LootEntry,
        except: Option<ObjectGuid>,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let bytes = packet.to_bytes();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        for looter in &entry.allowed_looters {
            if Some(*looter) == except {
                continue;
            }

            if *looter == player_guid {
                self.send_packet(packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(registration) = registry.loot_delivery_recipient(
                *looter,
                self.player_map_id_like_cpp(),
                instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, bytes.clone());
        }
    }

    pub(super) fn broadcast_represented_loot_roll_packet_to_voters_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        state: &RepresentedLootRollState,
        except: Option<ObjectGuid>,
    ) {
        let bytes = packet.to_bytes();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        for (player_guid, vote) in &state.voters {
            if vote.vote == ROLL_VOTE_NOT_VALID_LIKE_CPP {
                continue;
            }
            if Some(*player_guid) == except {
                continue;
            }

            if self.player_guid() == Some(*player_guid) {
                self.send_packet(packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(registration) = registry.loot_delivery_recipient(
                *player_guid,
                self.player_map_id_like_cpp(),
                instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, bytes.clone());
        }
    }
}
