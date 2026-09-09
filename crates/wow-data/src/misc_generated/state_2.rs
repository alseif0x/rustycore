//! Miscellaneous DB2 readers state definitions, part 2 of 2.
//!
//! Separated from the misc_generated.rs root under #664. Behaviour is preserved.

use super::*;

impl ExpectedStatModStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ExpectedStatMod.db2", |id, idx, r| {
            ExpectedStatModEntry {
                id,
                creature_health_mod: f32_field(r, idx, 0),
                player_health_mod: f32_field(r, idx, 1),
                creature_auto_attack_dps_mod: f32_field(r, idx, 2),
                creature_armor_mod: f32_field(r, idx, 3),
                player_mana_mod: f32_field(r, idx, 4),
                player_primary_stat_mod: f32_field(r, idx, 5),
                player_secondary_stat_mod: f32_field(r, idx, 6),
                armor_constant_mod: f32_field(r, idx, 7),
                creature_spell_damage_mod: f32_field(r, idx, 8),
            }
        })
    }
}

impl GarrAbilityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrAbility.db2", |id, idx, r| {
            GarrAbilityEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                garr_ability_category_id: r.get_field_u8(idx, 3),
                garr_follower_type_id: r.get_field_u8(idx, 4),
                icon_file_data_id: r.get_field_i32(idx, 5),
                faction_change_garr_ability_id: r.get_field_u16(idx, 6),
                flags: r.get_field_u16(idx, 7),
            }
        })
    }
}

impl GarrBuildingStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrBuilding.db2", |id, idx, r| {
            GarrBuildingEntry {
                id,
                horde_name: r.get_field_string(idx, 0),
                alliance_name: r.get_field_string(idx, 1),
                description: r.get_field_string(idx, 2),
                tooltip: r.get_field_string(idx, 3),
                garr_type_id: r.get_field_u8(idx, 4),
                building_type: r.get_field_u8(idx, 5),
                horde_game_object_id: r.get_field_i32(idx, 6),
                alliance_game_object_id: r.get_field_i32(idx, 7),
                garr_site_id: r.get_field_u8(idx, 8),
                upgrade_level: r.get_field_u8(idx, 9),
                build_seconds: r.get_field_i32(idx, 10),
                currency_type_id: r.get_field_u16(idx, 11),
                currency_qty: r.get_field_i32(idx, 12),
                horde_ui_texture_kit_id: r.get_field_u16(idx, 13),
                alliance_ui_texture_kit_id: r.get_field_u16(idx, 14),
                icon_file_data_id: r.get_field_i32(idx, 15),
                alliance_scene_script_package_id: r.get_field_u16(idx, 16),
                horde_scene_script_package_id: r.get_field_u16(idx, 17),
                max_assignments: r.get_field_i32(idx, 18),
                shipment_capacity: r.get_field_u8(idx, 19),
                garr_ability_id: r.get_field_u16(idx, 20),
                bonus_garr_ability_id: r.get_field_u16(idx, 21),
                gold_cost: r.get_field_u16(idx, 22),
                flags: r.get_field_u8(idx, 23),
            }
        })
    }
}

impl GarrBuildingPlotInstStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "GarrBuildingPlotInst.db2",
            |id, idx, r| GarrBuildingPlotInstEntry {
                id,
                map_offset: f32_array::<2>(r, idx, 0),
                garr_building_id: r.get_relationship_id(idx).unwrap_or(0),
                garr_site_level_plot_inst_id: r.get_field_u16(idx, 3),
                ui_texture_atlas_member_id: r.get_field_u16(idx, 4),
            },
        )
    }
}

impl GarrClassSpecStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrClassSpec.db2", |id, idx, r| {
            GarrClassSpecEntry {
                id,
                class_spec: r.get_field_string(idx, 0),
                class_spec_male: r.get_field_string(idx, 1),
                class_spec_female: r.get_field_string(idx, 2),
                ui_texture_atlas_member_id: r.get_field_u16(idx, 4),
                garr_foll_item_set_id: r.get_field_u16(idx, 5),
                follower_class_limit: r.get_field_u8(idx, 6),
                flags: r.get_field_u8(idx, 7),
            }
        })
    }
}

impl GarrFollowerStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrFollower.db2", |id, idx, r| {
            GarrFollowerEntry {
                id,
                horde_source_text: r.get_field_string(idx, 0),
                alliance_source_text: r.get_field_string(idx, 1),
                title_name: r.get_field_string(idx, 2),
                garr_type_id: r.get_field_u8(idx, 4),
                garr_follower_type_id: r.get_field_u8(idx, 5),
                horde_creature_id: r.get_field_i32(idx, 6),
                alliance_creature_id: r.get_field_i32(idx, 7),
                horde_garr_foll_race_id: r.get_field_u8(idx, 8),
                alliance_garr_foll_race_id: r.get_field_u8(idx, 9),
                horde_garr_class_spec_id: r.get_field_u8(idx, 10),
                alliance_garr_class_spec_id: r.get_field_u8(idx, 11),
                quality: r.get_field_u8(idx, 12),
                follower_level: r.get_field_u8(idx, 13),
                item_level_weapon: r.get_field_u16(idx, 14),
                item_level_armor: r.get_field_u16(idx, 15),
                horde_source_type_enum: r.get_field_i8(idx, 16),
                alliance_source_type_enum: r.get_field_i8(idx, 17),
                horde_icon_file_data_id: r.get_field_i32(idx, 18),
                alliance_icon_file_data_id: r.get_field_i32(idx, 19),
                horde_garr_foll_item_set_id: r.get_field_u16(idx, 20),
                alliance_garr_foll_item_set_id: r.get_field_u16(idx, 21),
                horde_ui_texture_kit_id: r.get_field_u16(idx, 22),
                alliance_ui_texture_kit_id: r.get_field_u16(idx, 23),
                vitality: r.get_field_u8(idx, 24),
                horde_flavor_garr_string_id: r.get_field_u8(idx, 25),
                alliance_flavor_garr_string_id: r.get_field_u8(idx, 26),
                horde_slotting_broadcast_text_id: r.get_field_u32(idx, 27),
                ally_slotting_broadcast_text_id: r.get_field_u32(idx, 28),
                chr_class_id: r.get_field_u8(idx, 29),
                flags: r.get_field_u8(idx, 30),
                gender: r.get_field_u8(idx, 31),
            }
        })
    }
}

impl GarrFollowerXAbilityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "GarrFollowerXAbility.db2",
            |id, idx, r| GarrFollowerXAbilityEntry {
                id,
                order_index: r.get_field_u8(idx, 0),
                faction_index: r.get_field_u8(idx, 1),
                garr_ability_id: r.get_field_u16(idx, 2),
                garr_follower_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl GarrMissionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrMission.db2", |id, idx, r| {
            GarrMissionEntry {
                id,
                name: r.get_field_string(idx, 0),
                location: r.get_field_string(idx, 1),
                description: r.get_field_string(idx, 2),
                map_pos: f32_array::<2>(r, idx, 3),
                world_pos: f32_array::<2>(r, idx, 4),
                garr_type_id: r.get_field_u8(idx, 6),
                garr_mission_type_id: r.get_field_u8(idx, 7),
                garr_follower_type_id: r.get_field_u8(idx, 8),
                max_followers: r.get_field_u8(idx, 9),
                mission_cost: r.get_field_u32(idx, 10),
                mission_cost_currency_types_id: r.get_field_u16(idx, 11),
                offered_garr_mission_texture_id: r.get_field_u8(idx, 12),
                ui_texture_kit_id: r.get_field_u16(idx, 13),
                env_garr_mechanic_id: r.get_field_u32(idx, 14),
                env_garr_mechanic_type_id: r.get_field_u8(idx, 15),
                player_condition_id: r.get_field_u32(idx, 16),
                target_level: r.get_field_i8(idx, 17),
                target_item_level: r.get_field_u16(idx, 18),
                mission_duration: r.get_field_i32(idx, 19),
                travel_duration: r.get_field_i32(idx, 20),
                offer_duration: r.get_field_u32(idx, 21),
                base_completion_chance: r.get_field_u8(idx, 22),
                base_follower_xp: r.get_field_u32(idx, 23),
                overmax_reward_pack_id: r.get_field_u32(idx, 24),
                follower_death_chance: r.get_field_u8(idx, 25),
                area_id: r.get_field_u32(idx, 26),
                flags: r.get_field_u32(idx, 27),
                garr_mission_set_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl GarrPlotStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrPlot.db2", |id, idx, r| {
            GarrPlotEntry {
                id,
                name: r.get_field_string(idx, 0),
                plot_type: r.get_field_u8(idx, 1),
                horde_construct_obj_id: r.get_field_i32(idx, 2),
                alliance_construct_obj_id: r.get_field_i32(idx, 3),
                flags: r.get_field_u8(idx, 4),
                ui_category_id: r.get_field_u8(idx, 5),
                upgrade_requirement: std::array::from_fn(|i| r.get_array_element(idx, 6, i, 32)),
            }
        })
    }
}

impl GarrPlotBuildingStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrPlotBuilding.db2", |id, idx, r| {
            GarrPlotBuildingEntry {
                id,
                garr_plot_id: r.get_field_u8(idx, 0),
                garr_building_id: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl GarrPlotInstanceStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrPlotInstance.db2", |id, idx, r| {
            GarrPlotInstanceEntry {
                id,
                name: r.get_field_string(idx, 0),
                garr_plot_id: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl GarrSiteLevelStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrSiteLevel.db2", |id, idx, r| {
            GarrSiteLevelEntry {
                id,
                town_hall_ui_pos: f32_array::<2>(r, idx, 0),
                garr_site_id: r.get_field_u32(idx, 1),
                garr_level: r.get_field_u8(idx, 2),
                map_id: r.get_field_u16(idx, 3),
                upgrade_movie_id: r.get_field_u16(idx, 4),
                ui_texture_kit_id: r.get_field_u16(idx, 5),
                max_building_level: r.get_field_u8(idx, 6),
                upgrade_cost: r.get_field_u16(idx, 7),
                upgrade_gold_cost: r.get_field_u16(idx, 8),
            }
        })
    }
}

impl GarrSiteLevelPlotInstStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "GarrSiteLevelPlotInst.db2",
            |id, idx, r| GarrSiteLevelPlotInstEntry {
                id,
                ui_marker_pos: f32_array::<2>(r, idx, 0),
                garr_site_level_id: r.get_relationship_id(idx).unwrap_or(0),
                garr_plot_instance_id: r.get_field_u8(idx, 2),
                ui_marker_size: r.get_field_u8(idx, 3),
            },
        )
    }
}

impl GarrTalentTreeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GarrTalentTree.db2", |id, idx, r| {
            GarrTalentTreeEntry {
                id,
                name: r.get_field_string(idx, 0),
                garr_type_id: r.get_field_i32(idx, 1),
                class_id: r.get_field_i32(idx, 2),
                max_tiers: r.get_field_i8(idx, 3),
                ui_order: r.get_field_i8(idx, 4),
                flags: r.get_field_i8(idx, 5),
                ui_texture_kit_id: r.get_field_u16(idx, 6),
            }
        })
    }
}

impl GemPropertiesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GemProperties.db2", |id, idx, r| {
            GemPropertiesEntry {
                id,
                enchant_id: r.get_field_u16(idx, 0),
                gem_type: r.get_field_i32(idx, 1),
                min_item_level: r.get_field_u16(idx, 2),
            }
        })
    }
}

impl GossipNpcOptionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GossipNPCOption.db2", |id, idx, r| {
            GossipNpcOptionEntry {
                id,
                gossip_npc_option: r.get_field_i32(idx, 0),
                lfg_dungeons_id: r.get_field_i32(idx, 1),
                unk_341: std::array::from_fn(|i| r.get_field_i32(idx, i + 2)),
                gossip_option_id: r.get_field_i32(idx, 11),
            }
        })
    }
}

impl GuildPerkSpellsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GuildPerkSpells.db2", |id, idx, r| {
            GuildPerkSpellsEntry {
                id,
                spell_id: r.get_field_i32(idx, 0),
            }
        })
    }
}

impl HolidaysStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Holidays.db2", |id, idx, r| {
            HolidaysEntry {
                id,
                region: r.get_field_u16(idx, 1),
                looping: r.get_field_u8(idx, 2),
                holiday_name_id: r.get_field_u32(idx, 3),
                holiday_description_id: r.get_field_u32(idx, 4),
                priority: r.get_field_u8(idx, 5),
                calendar_filter_type: r.get_field_i8(idx, 6),
                flags: r.get_field_u8(idx, 7),
                world_state_expression_id: r.get_field_u32(idx, 8),
                duration: std::array::from_fn(|i| r.get_array_element(idx, 9, i, 16) as u16),
                date: std::array::from_fn(|i| r.get_array_element(idx, 10, i, 32)),
                calendar_flags: std::array::from_fn(|i| r.get_array_element(idx, 11, i, 8) as u8),
                texture_file_data_id: std::array::from_fn(|i| {
                    r.get_array_element(idx, 12, i, 32) as i32
                }),
            }
        })
    }
}

impl KeychainStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Keychain.db2", |id, idx, r| {
            KeychainEntry {
                id,
                key: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 8) as u8),
            }
        })
    }
}

impl KeystoneAffixStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "KeystoneAffix.db2", |id, idx, r| {
            KeystoneAffixEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                file_data_id: r.get_field_i32(idx, 3),
            }
        })
    }
}

impl LfgDungeonsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "LFGDungeons.db2", |id, idx, r| {
            LfgDungeonsEntry {
                id,
                name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                min_level: r.get_field_u8(idx, 2),
                max_level: r.get_field_u16(idx, 3),
                type_id: r.get_field_u8(idx, 4),
                subtype: r.get_field_u8(idx, 5),
                faction: r.get_field_i8(idx, 6),
                icon_texture_file_id: r.get_field_i32(idx, 7),
                rewards_bg_texture_file_id: r.get_field_i32(idx, 8),
                popup_bg_texture_file_id: r.get_field_i32(idx, 9),
                expansion_level: r.get_field_u8(idx, 10),
                map_id: r.get_field_i16(idx, 11),
                difficulty_id: r.get_field_u8(idx, 12),
                min_gear: f32_field(r, idx, 13),
                group_id: r.get_field_u8(idx, 14),
                order_index: r.get_field_u8(idx, 15),
                required_player_condition_id: r.get_field_u32(idx, 16),
                target_level: r.get_field_u8(idx, 17),
                target_level_min: r.get_field_u8(idx, 18),
                target_level_max: r.get_field_u16(idx, 19),
                random_id: r.get_field_u16(idx, 20),
                scenario_id: r.get_field_u16(idx, 21),
                final_encounter_id: r.get_field_u16(idx, 22),
                count_tank: r.get_field_u8(idx, 23),
                count_healer: r.get_field_u8(idx, 24),
                count_damage: r.get_field_u8(idx, 25),
                min_count_tank: r.get_field_u8(idx, 26),
                min_count_healer: r.get_field_u8(idx, 27),
                min_count_damage: r.get_field_u8(idx, 28),
                bonus_reputation_amount: r.get_field_u16(idx, 29),
                mentor_item_level: r.get_field_u16(idx, 30),
                mentor_char_level: r.get_field_u8(idx, 31),
                flags: std::array::from_fn(|i| r.get_array_element(idx, 32, i, 32) as i32),
            }
        })
    }

    pub fn get_by_map_and_difficulty_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: u8,
    ) -> Option<&LfgDungeonsEntry> {
        self.entries
            .values()
            .find(|entry| entry.map_id == map_id as i16 && entry.difficulty_id == difficulty_id)
    }
}

impl LanguageWordsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "LanguageWords.db2", |id, idx, r| {
            LanguageWordsEntry {
                id,
                word: r.get_field_string(idx, 0),
                language_id: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl LanguagesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Languages.db2", |id, idx, r| {
            LanguagesEntry {
                id,
                name: r.get_field_string(idx, 0),
                flags: r.get_field_i32(idx, 2),
                ui_texture_kit_id: r.get_field_i32(idx, 3),
                ui_texture_kit_element_count: r.get_field_i32(idx, 4),
                learning_curve_id: r.get_field_i32(idx, 5),
            }
        })
    }
}

impl MailTemplateStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "MailTemplate.db2", |id, idx, r| {
            MailTemplateEntry {
                id,
                body: r.get_field_string(idx, 0),
            }
        })
    }
}

impl MovieStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Movie.db2", |id, idx, r| MovieEntry {
            id,
            volume: r.get_field_u8(idx, 0),
            key_id: r.get_field_u8(idx, 1),
            audio_file_data_id: r.get_field_u32(idx, 2),
            subtitle_file_data_id: r.get_field_u32(idx, 3),
        })
    }
}

impl MythicPlusSeasonStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "MythicPlusSeason.db2", |id, idx, r| {
            MythicPlusSeasonEntry {
                id,
                milestone_season: r.get_field_i32(idx, 1),
                expansion_level: r.get_field_i32(idx, 2),
                heroic_lfg_dungeon_min_gear: r.get_field_i32(idx, 3),
            }
        })
    }
}

impl NamesProfanityStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "NamesProfanity.db2", |id, idx, r| {
            NamesProfanityEntry {
                id,
                name: r.get_field_string(idx, 0),
                language: r.get_field_i8(idx, 1),
            }
        })
    }
}

impl NamesReservedStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "NamesReserved.db2", |id, idx, r| {
            NamesReservedEntry {
                id,
                name: r.get_field_string(idx, 0),
            }
        })
    }
}

impl NamesReservedLocaleStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "NamesReservedLocale.db2", |id, idx, r| {
            NamesReservedLocaleEntry {
                id,
                name: r.get_field_string(idx, 0),
                locale_mask: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl OverrideSpellDataStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "OverrideSpellData.db2", |id, idx, r| {
            OverrideSpellDataEntry {
                id,
                spells: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 32) as i32),
                player_action_bar_file_data_id: r.get_field_i32(idx, 1),
                flags: r.get_field_u8(idx, 2),
            }
        })
    }
}

impl PvpDifficultyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PVPDifficulty.db2", |id, idx, r| {
            PvpDifficultyEntry {
                id,
                range_index: r.get_field_u8(idx, 0),
                min_level: r.get_field_u8(idx, 1),
                max_level: r.get_field_u8(idx, 2),
                map_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl PvpItemStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PVPItem.db2", |id, idx, r| PvpItemEntry {
            id,
            item_id: r.get_field_i32(idx, 0),
            item_level_delta: r.get_field_u8(idx, 1),
        })
    }
}

impl PrestigeLevelInfoStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PrestigeLevelInfo.db2", |id, idx, r| {
            PrestigeLevelInfoEntry {
                id,
                name: r.get_field_string(idx, 0),
                prestige_level: r.get_field_i32(idx, 1),
                badge_texture_file_data_id: r.get_field_i32(idx, 2),
                flags: r.get_field_u8(idx, 3),
                awarded_achievement_id: r.get_field_i32(idx, 4),
            }
        })
    }
}

impl ScenarioStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Scenario.db2", |id, idx, r| {
            ScenarioEntry {
                id,
                name: r.get_field_string(idx, 0),
                area_table_id: r.get_field_u16(idx, 1),
                scenario_type: r.get_field_u8(idx, 2),
                flags: r.get_field_u8(idx, 3),
                ui_texture_kit_id: r.get_field_u32(idx, 4),
            }
        })
    }
}

impl SceneScriptStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SceneScript.db2", |id, idx, r| {
            SceneScriptEntry {
                id,
                first_scene_script_id: r.get_field_u16(idx, 0),
                next_scene_script_id: r.get_field_u16(idx, 1),
                unknown_915: r.get_field_i32(idx, 2),
            }
        })
    }
}

impl ServerMessagesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ServerMessages.db2", |id, idx, r| {
            ServerMessagesEntry {
                id,
                text: r.get_field_string(idx, 0),
            }
        })
    }
}

impl SoundKitStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SoundKit.db2", |id, idx, r| {
            SoundKitEntry {
                id,
                sound_type: r.get_field_u8(idx, 1),
                volume_float: f32_field(r, idx, 2),
                flags: r.get_field_u16(idx, 3),
                min_distance: f32_field(r, idx, 4),
                distance_cutoff: f32_field(r, idx, 5),
                eax_def: r.get_field_u8(idx, 6),
                sound_kit_advanced_id: r.get_field_u32(idx, 7),
                volume_variation_plus: f32_field(r, idx, 8),
                volume_variation_minus: f32_field(r, idx, 9),
                pitch_variation_plus: f32_field(r, idx, 10),
                pitch_variation_minus: f32_field(r, idx, 11),
                dialog_type: r.get_field_i8(idx, 12),
                pitch_adjust: f32_field(r, idx, 13),
                bus_overwrite_id: r.get_field_u16(idx, 14),
                max_instances: r.get_field_u8(idx, 15),
                sound_mix_group_id: r.get_field_u32(idx, 16),
            }
        })
    }
}

impl SpecSetMemberStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpecSetMember.db2", |id, idx, r| {
            SpecSetMemberEntry {
                id,
                chr_specialization_id: r.get_field_i32(idx, 0),
                spec_set_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl SpecializationSpellsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpecializationSpells.db2",
            |id, idx, r| SpecializationSpellsEntry {
                id,
                description: r.get_field_string(idx, 0),
                spec_id: r.get_field_u16(idx, 2),
                spell_id: r.get_field_i32(idx, 3),
                overrides_spell_id: r.get_field_i32(idx, 4),
                display_order: r.get_field_u8(idx, 5),
            },
        )
    }
}

impl SummonPropertiesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SummonProperties.db2", |id, idx, r| {
            SummonPropertiesEntry {
                id,
                control: r.get_field_i32(idx, 0),
                faction: r.get_field_i32(idx, 1),
                title: r.get_field_i32(idx, 2),
                slot: r.get_field_i32(idx, 3),
                flags: std::array::from_fn(|i| r.get_array_element(idx, 4, i, 32) as i32),
            }
        })
    }
}

impl TactKeyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TactKey.db2", |id, idx, r| TactKeyEntry {
            id,
            key: std::array::from_fn(|i| r.get_array_element(idx, 0, i, 8) as u8),
        })
    }
}

impl TotemCategoryStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "TotemCategory.db2", |id, idx, r| {
            TotemCategoryEntry {
                id,
                name: r.get_field_string(idx, 0),
                totem_category_type: r.get_field_u8(idx, 1),
                totem_category_mask: r.get_field_i32(idx, 2),
            }
        })
    }
}

pub(super) fn load_store<T, S>(
    data_dir: &str,
    locale: &str,
    file_name: &str,
    mut read: impl FnMut(u32, usize, &Wdc4Reader) -> T,
) -> Result<S>
where
    S: FromEntries<T>,
{
    let path = Path::new(data_dir).join("dbc").join(locale).join(file_name);
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;

    let mut entries = Vec::with_capacity(reader.total_count());
    for (id, idx) in reader.iter_records() {
        entries.push(read(id, idx, &reader));
    }

    let store = S::from_entries(entries);
    info!("Loaded {} rows from {}", store.len(), path.display());
    Ok(store)
}

pub(super) fn f32_field(reader: &Wdc4Reader, idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(idx, field))
}

pub(super) fn f32_array<const N: usize>(reader: &Wdc4Reader, idx: usize, field: usize) -> [f32; N] {
    std::array::from_fn(|i| f32::from_bits(reader.get_array_element(idx, field, i, 32)))
}

pub(super) trait FromEntries<T> {
    fn from_entries(entries: impl IntoIterator<Item = T>) -> Self;
    fn len(&self) -> usize;
}

impl FromEntries<PvpItemEntry> for PvpItemStore {
    fn from_entries(entries: impl IntoIterator<Item = PvpItemEntry>) -> Self {
        Self::from_entries(entries)
    }

    fn len(&self) -> usize {
        self.len()
    }
}
