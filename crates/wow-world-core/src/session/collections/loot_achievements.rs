use crate::session::HubRef;
use crate::session::mailbox::{SendIfVisibleLikeCppCommand, SessionCommand};
use wow_core::{ObjectGuid, pack_session_wow_time};
use wow_entities::{PlayerAchievementCriteriaRecord, PlayerAchievementRecord};
use wow_packet::ServerPacket;
use wow_packet::packets::misc::{
    AchievementEarned, AchievementProgress, AllAchievementData, CriteriaUpdate, EarnedAchievement,
};
use wow_persistence::{LOOT_MONEY_ACHIEVEMENTS, LOOT_MONEY_CRITERION, LootMoneyCriteriaProgress};

impl HubRef<'_> {
    pub fn can_earn_loot_money_criteria(&self) -> bool {
        !self.core.account_state.cannot_earn_achievements
            && self.player_is_game_master_like_cpp() == Some(false)
    }

    pub fn loot_achievement_login_data(&self) -> AllAchievementData {
        let Some(guid) = self.core.player_guid() else {
            return AllAchievementData::default();
        };
        let offset = self.core.account_state.timezone_offset_minutes;
        self.core
            .with_owned_player_like_cpp(|player| {
                let state = player.gameplay_state();
                let earned = state
                    .achievements
                    .iter()
                    .filter_map(|row| {
                        if !LOOT_MONEY_ACHIEVEMENTS
                            .iter()
                            .any(|&(id, _)| id == row.achievement_id)
                        {
                            return None;
                        }
                        Some(EarnedAchievement {
                            id: row.achievement_id,
                            owner: guid,
                            packed_date: pack_session_wow_time(row.completed_at? as i64, offset),
                            realm: self.core.virtual_realm_address(),
                        })
                    })
                    .collect();
                let progress = state
                    .achievement_criteria
                    .iter()
                    .filter(|row| row.criteria_id == LOOT_MONEY_CRITERION)
                    .filter_map(|row| {
                        Some(AchievementProgress {
                            id: row.criteria_id,
                            player: guid,
                            quantity: row.counter,
                            packed_date: pack_session_wow_time(row.completed_at? as i64, offset),
                        })
                    })
                    .collect();
                AllAchievementData { earned, progress }
            })
            .unwrap_or_default()
    }

    /// Install absolute, committed values. A delayed payout may carry a lower
    /// counter but still contain a previously undelivered threshold award.
    pub fn apply_loot_money_criteria(&self, committed: &LootMoneyCriteriaProgress) {
        let Some(guid) = self.core.player_guid() else {
            return;
        };
        let Some((changed, earned)) = self.core.with_owned_player_mut_like_cpp(|player| {
            let state = player.gameplay_state_mut();
            let record = PlayerAchievementCriteriaRecord {
                criteria_id: LOOT_MONEY_CRITERION,
                counter: committed.counter,
                completed_at: Some(committed.date),
            };
            let changed = match state
                .achievement_criteria
                .iter_mut()
                .find(|row| row.criteria_id == LOOT_MONEY_CRITERION)
            {
                Some(old) if old.counter >= committed.counter => false,
                Some(old) => {
                    *old = record;
                    true
                }
                None => {
                    state.achievement_criteria.push(record);
                    true
                }
            };
            let mut earned = Vec::new();
            for &id in &committed.new_achievements {
                if let Some(date) = committed.achievement_date
                    && !state
                        .achievements
                        .iter()
                        .any(|row| row.achievement_id == id)
                {
                    state.achievements.push(PlayerAchievementRecord {
                        achievement_id: id,
                        completed_at: Some(date),
                    });
                    earned.push((id, date));
                }
            }
            (changed, earned)
        }) else {
            return;
        };
        let packed_date = pack_session_wow_time(
            committed.date as i64,
            self.core.account_state.timezone_offset_minutes,
        );
        if changed {
            self.core.send_packet(&CriteriaUpdate(AchievementProgress {
                id: LOOT_MONEY_CRITERION,
                player: guid,
                quantity: committed.counter,
                packed_date,
            }));
        }
        for (id, date) in earned {
            let packed_date =
                pack_session_wow_time(date as i64, self.core.account_state.timezone_offset_minutes);
            let award = EarnedAchievement {
                id,
                packed_date,
                owner: guid,
                realm: self.core.virtual_realm_address(),
            };
            self.core.send_packet(&AchievementEarned(award));
            self.broadcast_loot_achievement(guid, award, date);
        }
    }

    fn broadcast_loot_achievement(&self, guid: ObjectGuid, award: EarnedAchievement, date: u64) {
        let (Some(registry), Some(pos), Some(map)) = (
            self.core.player_registry(),
            self.player_position_like_cpp(),
            self.core.current_canonical_player_map_key_like_cpp(),
        ) else {
            return;
        };
        let map_id = self.core.player_map_id_like_cpp();
        let range = self.config.chat_listen_ranges_like_cpp.say;
        for target in
            registry.movement_recipients_within_range(guid, map_id, map.instance_id, pos, range)
        {
            let Some(offset) = registry.recipient_timezone(target) else {
                continue;
            };
            let packet = AchievementEarned(EarnedAchievement {
                packed_date: pack_session_wow_time(date as i64, offset),
                ..award
            });
            let _ = registry.try_send_current_command(
                target,
                SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                    queued_at: std::time::Instant::now(),
                    source_guid: guid,
                    map_id,
                    instance_id: map.instance_id,
                    packet_bytes: packet.to_bytes(),
                }),
            );
        }
    }
}
