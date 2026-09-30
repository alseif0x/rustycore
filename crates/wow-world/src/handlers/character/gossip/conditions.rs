// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

impl WorldSession {
    pub(super) fn gossip_conditions_meet_like_cpp(
        &mut self,
        condition_store: &ConditionEntriesByTypeStore,
        source_type: ConditionSourceType,
        source_group: u32,
        source_entry: i32,
        npc_guid: ObjectGuid,
    ) -> bool {
        let Some(conditions) = condition_store
            .conditions_for_like_cpp(source_type, ConditionId::new(source_group, source_entry, 0))
        else {
            return true;
        };

        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            warn!(
                "Gossip condition check failed closed: missing player object for {:?}",
                source_type
            );
            return false;
        };
        let Some((source_object, source_unit_snapshot)) =
            self.build_condition_creature_object_like_cpp(npc_guid)
        else {
            warn!(
                "Gossip condition check failed closed: missing source object for {:?}",
                source_type
            );
            return false;
        };

        let Some(player_unit_snapshot) = self.condition_player_unit_snapshot_like_cpp() else {
            return false;
        };
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let player_condition_store = self.player_condition_store().cloned();
        let Some(player_condition_context) = self.represented_player_condition_context_like_cpp()
        else {
            return false;
        };

        let mut source_info = wow_conditions::ConditionSourceInfo::from_targets(
            Some(&player_object),
            Some(&source_object),
            None,
        );
        source_info.set_unit_target_snapshot(0, player_unit_snapshot);
        source_info.set_player_target_snapshot(0, player_snapshot);
        source_info.set_unit_target_snapshot(1, source_unit_snapshot);
        if let Some(store) = player_condition_store.as_ref() {
            source_info.set_player_condition_store(store.as_ref());
            if let Some(context) = player_condition_context.as_context(self) {
                source_info.set_player_condition_context(0, context);
            }
        }

        wow_conditions::is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions.as_slice(),
            condition_store,
            |condition, source_info| match wow_conditions::condition_meets_basic_like_cpp(
                condition,
                source_info,
                |current_area, required_area| current_area == required_area,
            ) {
                wow_conditions::ConditionMeetResult::Evaluated(value) => value,
                wow_conditions::ConditionMeetResult::Unsupported => {
                    warn!(
                        "Gossip condition check failed closed: unsupported {:?} for {:?} {}:{}",
                        condition.condition_type, source_type, source_group, source_entry
                    );
                    false
                }
            },
        )
    }

    pub(super) fn gossip_menu_text_conditions_meet_like_cpp(
        &mut self,
        condition_store: &ConditionEntriesByTypeStore,
        menu_id: u32,
        text_id: u32,
        npc_guid: ObjectGuid,
    ) -> bool {
        if condition_store
            .conditions_for_like_cpp(
                ConditionSourceType::GossipMenu,
                ConditionId::new(menu_id, text_id as i32, 0),
            )
            .is_some()
        {
            return self.gossip_conditions_meet_like_cpp(
                condition_store,
                ConditionSourceType::GossipMenu,
                menu_id,
                text_id as i32,
                npc_guid,
            );
        }

        self.gossip_conditions_meet_like_cpp(
            condition_store,
            ConditionSourceType::GossipMenu,
            menu_id,
            0,
            npc_guid,
        )
    }
}
