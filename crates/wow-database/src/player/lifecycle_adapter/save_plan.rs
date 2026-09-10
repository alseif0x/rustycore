//! Full-save operation ordering and the shared tutorials statement.
//! Private MariaDB implementation; no semantic port or transaction changes.

use crate::statements::StatementDef;

use super::save_steps::PlayerCharacterSaveStepLikeCpp;
use super::save_steps::player_character_save_statement_like_cpp;
use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use wow_persistence::{
    PlayerCharacterSaveRequestLikeCpp, PlayerEquipmentSetStateLikeCpp,
    PlayerEquipmentSetTypeLikeCpp, PlayerSpellSaveGroupLikeCpp, PlayerSpellStateLikeCpp,
};

pub(super) fn player_character_save_statements_like_cpp(
    request: &PlayerCharacterSaveRequestLikeCpp,
) -> Vec<PreparedStatement> {
    use PlayerCharacterSaveStepLikeCpp as Step;

    let guid = request.player_guid;
    let account_id = request.account_id;
    let character = &request.character;
    let mut steps = vec![
        Step::Position {
            x: character.position.x,
            y: character.position.y,
            z: character.position.z,
            orientation: character.position.orientation,
            map_id: character.position.map_id,
            instance_id: character.position.instance_id,
            zone_id: character.position.zone_id,
            guid,
        },
        Step::LevelXp {
            level: character.level,
            xp: character.xp,
            guid,
        },
        Step::Money {
            money: character.money,
            guid,
        },
        Step::RestState {
            rest_state: character.rest_state,
            player_flags: character.player_flags,
            rest_bonus: character.rest_bonus,
            logout_time: character.logout_time,
            is_logout_resting: character.is_logout_resting,
            guid,
        },
        Step::Health {
            health: character.health,
            guid,
        },
    ];
    if let Some(powers) = character.powers {
        steps.push(Step::Powers { powers, guid });
    }
    steps.extend([
        Step::TalentReset {
            reset_cost: character.talent_reset_cost,
            reset_time: character.talent_reset_time,
            guid,
        },
        Step::ExploredZones {
            explored_zones: character.explored_zones.clone(),
            guid,
        },
    ]);

    match &request.spells {
        Some(PlayerSpellSaveGroupLikeCpp::Complete { rows, .. }) => {
            let mut rows = rows.clone();
            rows.sort_by_key(|spell| spell.spell_id);
            for spell in rows {
                if matches!(
                    spell.state,
                    PlayerSpellStateLikeCpp::Removed | PlayerSpellStateLikeCpp::Changed
                ) {
                    steps.push(Step::DeleteSpell {
                        spell_id: spell.spell_id,
                        guid,
                    });
                }
                if matches!(
                    spell.state,
                    PlayerSpellStateLikeCpp::New | PlayerSpellStateLikeCpp::Changed
                ) {
                    if !spell.dependent {
                        steps.push(Step::InsertSpell {
                            guid,
                            spell_id: spell.spell_id,
                            active: spell.active,
                            disabled: spell.disabled,
                        });
                    }
                    steps.push(Step::DeleteFavoriteSpell {
                        guid,
                        spell_id: spell.spell_id,
                    });
                    if spell.favorite {
                        steps.push(Step::InsertFavoriteSpell {
                            guid,
                            spell_id: spell.spell_id,
                        });
                    }
                }
            }
        }
        Some(PlayerSpellSaveGroupLikeCpp::Fallback { rows }) => {
            let mut rows = rows.clone();
            rows.sort_by_key(|spell| spell.spell_id);
            for spell in rows {
                steps.push(if spell.dependent {
                    Step::DeleteSpell {
                        spell_id: spell.spell_id,
                        guid,
                    }
                } else {
                    Step::UpsertFallbackSpell {
                        guid,
                        spell_id: spell.spell_id,
                        active: spell.active,
                    }
                });
            }
        }
        None => {}
    }

    if let Some(skills) = &request.skills {
        steps.push(Step::DeleteSkills { guid });
        let mut skills = skills.clone();
        skills.sort_by_key(|skill| skill.skill_id);
        steps.extend(skills.into_iter().map(|skill| Step::InsertSkill {
            guid,
            skill_id: skill.skill_id,
            value: skill.value,
            max: skill.max,
            profession_slot: skill.profession_slot,
        }));
    }

    steps.push(Step::Difficulties {
        dungeon: character.dungeon_difficulty,
        raid: character.raid_difficulty,
        legacy_raid: character.legacy_raid_difficulty,
        guid,
    });

    if let Some(glyphs) = &request.glyphs {
        steps.push(Step::DeleteGlyphs { guid });
        steps.extend(glyphs.iter().map(|glyph| Step::InsertGlyph {
            guid,
            talent_group: glyph.talent_group,
            glyph_slot: glyph.glyph_slot,
            glyph_id: glyph.glyph_id,
        }));
    }
    if let Some(talents) = &request.talents {
        steps.push(Step::DeleteTalents { guid });
        steps.extend(talents.iter().map(|talent| Step::InsertTalent {
            guid,
            talent_id: talent.talent_id,
            rank: talent.rank,
            talent_group: talent.talent_group,
        }));
    }
    if let Some(cooldowns) = &request.spell_cooldowns {
        steps.push(Step::DeleteSpellCooldowns { guid });
        let mut cooldowns = cooldowns
            .iter()
            .copied()
            .filter(|cooldown| {
                cooldown.cooldown_end_unix_secs > request.wall_clock_unix_secs
                    || cooldown.category_end_unix_secs > request.wall_clock_unix_secs
            })
            .collect::<Vec<_>>();
        cooldowns.sort_by_key(|cooldown| cooldown.spell_id);
        steps.extend(
            cooldowns
                .into_iter()
                .map(|cooldown| Step::InsertSpellCooldown {
                    guid,
                    spell_id: cooldown.spell_id,
                    item_id: cooldown.item_id,
                    cooldown_end: cooldown.cooldown_end_unix_secs,
                    category_id: cooldown.category_id,
                    category_end: cooldown.category_end_unix_secs,
                }),
        );
    }
    if let Some(charges) = &request.spell_charges {
        steps.push(Step::DeleteSpellCharges { guid });
        steps.extend(
            charges
                .iter()
                .copied()
                .filter(|charge| charge.recharge_end_unix_secs > request.wall_clock_unix_secs)
                .map(|charge| Step::InsertSpellCharge {
                    guid,
                    category_id: charge.category_id,
                    recharge_start: charge.recharge_start_unix_secs,
                    recharge_end: charge.recharge_end_unix_secs,
                }),
        );
    }
    if let Some(actions) = &request.action_buttons {
        steps.push(Step::DeleteActions {
            guid,
            spec: actions.spec,
            trait_config_id: actions.trait_config_id,
        });
        steps.extend(actions.rows.iter().map(|button| Step::InsertAction {
            guid,
            spec: actions.spec,
            trait_config_id: actions.trait_config_id,
            button: button.button,
            action: button.packed_action & 0x00FF_FFFF,
            action_type: (button.packed_action >> 24) as u8,
        }));
    }
    if let Some(equipment_sets) = &request.equipment_sets {
        for row in equipment_sets {
            let step = match (row.state, row.set_type) {
                (PlayerEquipmentSetStateLikeCpp::Unchanged, _) => None,
                (
                    PlayerEquipmentSetStateLikeCpp::Deleted,
                    PlayerEquipmentSetTypeLikeCpp::Equipment,
                ) => Some(Step::DeleteEquipmentSet {
                    set_guid: row.set_guid,
                }),
                (
                    PlayerEquipmentSetStateLikeCpp::Deleted,
                    PlayerEquipmentSetTypeLikeCpp::Transmog,
                ) => Some(Step::DeleteTransmogOutfit {
                    set_guid: row.set_guid,
                }),
                (PlayerEquipmentSetStateLikeCpp::New, PlayerEquipmentSetTypeLikeCpp::Equipment) => {
                    Some(Step::InsertEquipmentSet {
                        player_guid: guid,
                        row: row.clone(),
                    })
                }
                (
                    PlayerEquipmentSetStateLikeCpp::Changed,
                    PlayerEquipmentSetTypeLikeCpp::Equipment,
                ) => Some(Step::UpdateEquipmentSet {
                    player_guid: guid,
                    row: row.clone(),
                }),
                (PlayerEquipmentSetStateLikeCpp::New, PlayerEquipmentSetTypeLikeCpp::Transmog) => {
                    Some(Step::InsertTransmogOutfit {
                        player_guid: guid,
                        row: row.clone(),
                    })
                }
                (
                    PlayerEquipmentSetStateLikeCpp::Changed,
                    PlayerEquipmentSetTypeLikeCpp::Transmog,
                ) => Some(Step::UpdateTransmogOutfit {
                    player_guid: guid,
                    row: row.clone(),
                }),
            };
            if let Some(step) = step {
                steps.push(step);
            }
        }
    }
    if let Some(slots) = &request.void_storage {
        steps.extend(slots.iter().map(|slot| match &slot.item {
            Some(row) => Step::ReplaceVoidStorageItem {
                player_guid: guid,
                slot: slot.slot,
                row: row.clone(),
            },
            None => Step::DeleteVoidStorageSlot {
                player_guid: guid,
                slot: slot.slot,
            },
        }));
    }
    if let Some(tutorials) = &request.tutorials {
        steps.push(if tutorials.already_persisted {
            Step::UpdateTutorials {
                account_id,
                tutorials: tutorials.tutorials,
            }
        } else {
            Step::InsertTutorials {
                account_id,
                tutorials: tutorials.tutorials,
            }
        });
    }
    if !request.instance_lock_times.is_empty() {
        steps.push(Step::DeleteInstanceLockTimes { account_id });
        steps.extend(
            request
                .instance_lock_times
                .iter()
                .map(|lock| Step::InsertInstanceLockTime {
                    account_id,
                    instance_id: lock.instance_id,
                    release_time: lock.release_time,
                }),
        );
    }
    steps.push(Step::PlayedTime {
        total_time: request.played_time.total_time,
        level_time: request.played_time.level_time,
        guid,
    });
    for reputation in &request.reputations {
        steps.push(Step::DeleteReputation {
            guid,
            faction_id: reputation.faction_id,
        });
        steps.push(Step::InsertReputation {
            guid,
            faction_id: reputation.faction_id,
            standing: reputation.standing,
            flags: reputation.flags,
        });
    }
    if let Some(profiles) = &request.cuf_profiles {
        steps.extend(profiles.iter().map(|slot| match &slot.profile {
            Some(row) => Step::ReplaceCufProfile {
                guid,
                profile_id: slot.profile_id,
                row: row.clone(),
            },
            None => Step::DeleteCufProfile {
                guid,
                profile_id: slot.profile_id,
            },
        }));
    }

    steps
        .iter()
        .map(player_character_save_statement_like_cpp)
        .collect()
}

/// Build the tutorials statement for one account.
///
/// Shared rather than duplicated: both the standalone SaveTutorialsData path
/// and the #286 Player full-save adapter append this same row, and two
/// independent copies of the column order would be free to drift.
pub fn build_tutorials_save_statement_like_cpp(
    account_id: u32,
    tutorials: &[u32],
    already_persisted: bool,
) -> PreparedStatement {
    let mut stmt = PreparedStatement::new(if already_persisted {
        CharStatements::UPD_TUTORIALS.sql()
    } else {
        CharStatements::INS_TUTORIALS.sql()
    });
    for (index, value) in tutorials.iter().copied().enumerate() {
        stmt.set_u32(index, value);
    }
    stmt.set_u32(tutorials.len(), account_id);
    stmt
}
