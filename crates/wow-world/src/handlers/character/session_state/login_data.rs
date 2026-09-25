// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Auxiliary persisted character data loaded during the login sequence.

use super::super::spell_rules::{
    spell_charge_entry_from_db_like_cpp, spell_history_entry_from_db_like_cpp,
    unix_now_secs_like_cpp,
};
use super::*;

impl WorldSession {
    pub(in crate::handlers::character) fn skill_rewarded_quest_fallback_allowed_like_cpp(
        &self,
        spell_id: i32,
    ) -> bool {
        let Ok(spell_id) = u32::try_from(spell_id) else {
            return false;
        };
        let misc = self.spell_catalogs.spell_misc_store();
        let Some(condition_id) = misc
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

    pub(in crate::handlers::character) fn skill_rewarded_spell_changes_for_login_like_cpp(
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
        let spell_levels_store = self.spell_catalogs.spell_levels_store().cloned();

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
                    let categories = self.spell_catalogs.spell_category_store();
                    let category_known_to_store =
                        categories.is_none_or(|store| store.get(row.category_id).is_some());
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
        node_entries: &wow_data::trait_tree::TraitNodeEntryStore,
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
        let normalized_configs = self.normalize_trait_configs_like_cpp(&configs);
        let effective_configs = normalized_configs.as_deref().unwrap_or(&configs);
        let trait_query_authority_complete_like_cpp = entries_complete_like_cpp
            && configs_complete_like_cpp
            && self.trait_authority_complete_like_cpp(effective_configs, node_entries, guid)
            && self.complete_represented_trait_config_authority_load_like_cpp(
                effective_configs.iter().map(|config| {
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
            let _ = self.retain_loaded_trait_configs_like_cpp(effective_configs);
            let exact_traits = self.trait_definition_store().map(|definitions| {
                let mut exact = BTreeMap::<i32, i32>::new();
                for entry in effective_configs
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
            trait_configs = effective_configs.len(),
            trait_entries = effective_configs
                .iter()
                .map(|config| config.entries.len())
                .sum::<usize>(),
            "Loaded character trait configs like C++"
        );
        effective_configs.to_vec()
    }
}
