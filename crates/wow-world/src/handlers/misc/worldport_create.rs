//! Self CREATE construction is separate from delivery. Existing partial payload
//! semantics are preserved; this is not yet a single coherent whole-Player snapshot.
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_packet::packets::update::{PlayerCombatStats, UpdateObject};

impl crate::session::WorldSession {
    pub(super) fn prepare_player_self_create_for_teleport_like_cpp(&self) -> Option<UpdateObject> {
        let Some(guid) = self.player_guid() else {
            return None;
        };
        let Some(pos) = self.player_position_like_cpp() else {
            return None;
        };
        let map_id = self.player_map_id_like_cpp();
        let Some((zone_id, _area_id)) = self.player_zone_area_like_cpp() else {
            return None;
        };
        let race = self.player_race_like_cpp();
        let class = self.player_class_like_cpp();
        let gender = self.player_gender_like_cpp();
        let level = self.player_level_like_cpp();
        let (Some(player_xp), Some(player_next_level_xp), Some(scaling_level_delta)) = (
            self.resolved_player_xp_like_cpp(),
            self.resolved_player_next_level_xp_like_cpp(),
            self.resolved_player_scaling_level_delta_like_cpp(),
        ) else {
            return None;
        };
        let Some(player_money) = self.resolved_player_money_like_cpp() else {
            return None;
        };

        // Equipped items drive the visible model; bag slots / item objects are not re-sent here.
        let mut visible_items = [(0i32, 0u16, 0u16); 19];
        let Some(inventory_items) = self.resolved_inventory_items_like_cpp() else {
            return None;
        };
        for (slot, item) in inventory_items {
            if (slot as usize) < 19 {
                visible_items[slot as usize] = (item.entry_id as i32, 0, 0);
            }
        }

        let Some((health, _, _)) = self.resolved_player_vitals_like_cpp() else {
            return None;
        };
        let health = health.max(1);
        let combat = PlayerCombatStats {
            health: i64::from(health),
            max_health: i64::from(health),
            ..PlayerCombatStats::default()
        };

        let quest_log = self.quest_log_create_entries_like_cpp();
        let account_toys = self.account_toy_active_player_rows_like_cpp();
        let account_heirlooms = self.account_heirloom_active_player_rows_like_cpp();
        let account_transmog = self.account_transmog_active_player_rows_like_cpp();
        let Some(trait_configs) = self.owned_trait_configs_for_create_like_cpp() else {
            return None;
        };
        // PlayerData::WriteCreate serializes current Player fields, not a new
        // login query (UpdateFields.cpp:1777,1822-1825). Missing owner is not empty.
        let Some(customizations) = self.owned_player_customizations_like_cpp() else {
            return None;
        };
        let player_customizations = customizations
            .into_iter()
            .map(
                |choice| wow_packet::packets::update::ChrCustomizationChoiceValuesUpdate {
                    option_id: choice.option_id,
                    choice_id: choice.choice_id,
                },
            )
            .collect();
        let party_type = self.party_member_party_type_like_cpp();
        let display_id = crate::handlers::character::default_display_id(race, gender);

        // Rebuild the active SkillInfo rows from the canonical login skill
        // records. This preserves persisted/default values across far
        // teleports instead of re-running LearnDefaultSkills with fabricated
        // level×5 ranks.
        let skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)> =
            if let (Some(skill_store), Some(skill_line_store), Some(skill_tiers_store)) = (
                self.skill_store(),
                self.skill_line_store(),
                self.skill_tiers_store(),
            ) {
                let Some(player_skill_records) = self.resolved_player_skill_records_like_cpp()
                else {
                    return None;
                };
                let mut skill_records: Vec<_> = player_skill_records.values().collect();
                skill_records.sort_by_key(|skill| skill.skill_id);
                skill_records
                    .into_iter()
                    .filter_map(|skill| {
                        skill_store.loaded_skill_info_like_cpp(
                            skill.skill_id,
                            race,
                            class,
                            level,
                            skill.value,
                            skill.max,
                            skill_line_store,
                            skill_tiers_store,
                        )
                    })
                    .map(|entry| {
                        (
                            entry.skill_id,
                            entry.step,
                            entry.rank,
                            entry.starting_rank,
                            entry.max_rank,
                            entry.temp_bonus,
                            entry.perm_bonus,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            };

        let mut player_pkt = UpdateObject::create_player_with_party_type(
            guid,
            race,
            class,
            gender,
            level,
            display_id,
            &pos,
            map_id,
            zone_id,
            true, // is_self -> ActivePlayer fields
            visible_items,
            [ObjectGuid::EMPTY; 141],
            combat,
            skill_info,
            player_money,
            quest_log,
            party_type,
        );
        let Some((player_flags, player_flags_ex)) =
            self.resolved_player_flags_for_create_like_cpp()
        else {
            return None;
        };
        player_pkt.set_player_flags_like_cpp(player_flags, player_flags_ex);
        player_pkt.set_player_xp_like_cpp(player_xp.min(i32::MAX as u32) as i32);
        player_pkt
            .set_player_next_level_xp_like_cpp(player_next_level_xp.min(i32::MAX as u32) as i32);
        player_pkt.set_player_max_level_like_cpp(self.player_active_max_level_like_cpp() as i32);
        player_pkt.set_player_scaling_level_delta_like_cpp(scaling_level_delta);
        let (Some(rest_threshold), Some(rest_state)) = (
            self.resolved_xp_rest_threshold_like_cpp(),
            self.resolved_xp_rest_state_like_cpp(),
        ) else {
            return None;
        };
        player_pkt.set_player_rest_info_like_cpp(0, rest_threshold, rest_state);
        player_pkt.set_player_account_guids_like_cpp(
            ObjectGuid::create_global(HighGuid::WowAccount, 0, self.account_id as i64),
            ObjectGuid::create_global(HighGuid::BNetAccount, 0, self.battlenet_account_id() as i64),
        );
        player_pkt.set_player_collection_dynamic_fields_like_cpp(
            account_toys,
            account_heirlooms,
            account_transmog,
            trait_configs,
        );
        let Some(action_buttons) = self.represented_action_buttons_snapshot_like_cpp() else {
            return None;
        };
        player_pkt.set_player_action_buttons_like_cpp(action_buttons);
        player_pkt.set_player_customizations_like_cpp(player_customizations);
        Some(player_pkt)
    }
}
