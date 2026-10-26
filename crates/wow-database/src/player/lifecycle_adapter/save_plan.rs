//! Full-save operation ordering and the shared tutorials statement.
//! Private MariaDB implementation; no semantic port or transaction changes.

use crate::statements::StatementDef;

use super::save_steps::PlayerCharacterSaveStepLikeCpp;
use super::save_steps::player_character_save_statement_like_cpp;
use crate::params::PreparedStatement;
use crate::statements::CharStatements;
use wow_persistence::{
    PlayerCharacterSaveRequestLikeCpp, PlayerEquipmentSetStateLikeCpp,
    PlayerEquipmentSetTypeLikeCpp, PlayerLifecycleAuraFamilySave, PlayerLifecycleAuraSave,
    PlayerItemSaveLikeCpp, PlayerItemSaveStateLikeCpp, PlayerSpellSaveGroupLikeCpp,
    PlayerSpellStateLikeCpp,
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
        Step::CharacterDeathMetadata {
            at_login_flags: character.at_login_flags,
            death_expire_time: character.death_expire_time,
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
    if let Some(item_durability) = &request.item_durability {
        let mut rows = item_durability.clone();
        rows.sort_by_key(|row| row.item_guid);
        steps.extend(rows.into_iter().map(|row| Step::ItemDurability {
            player_guid: guid,
            item_guid: row.item_guid,
            durability: row.durability,
        }));
    }
    let item_statement_position = steps.len();

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
    if let Some(aura_spell) = &request.resurrection_sickness {
        steps.push(Step::DeleteLifecycleAuraEffectsBySpell {
            guid,
            spell_id: aura_spell.spell_id,
        });
        steps.push(Step::DeleteLifecycleAuraFamilyBySpell {
            guid,
            spell_id: aura_spell.spell_id,
        });
        let mut aura_families = aura_spell
            .families
            .iter()
            .filter(|family| family.spell_id == aura_spell.spell_id)
            .collect::<Vec<_>>();
        aura_families.sort_by_key(|family| (family.caster_guid_binary, family.item_guid_binary));
        for aura_family in aura_families {
            append_lifecycle_aura_family_insert_steps(&mut steps, guid, aura_family);
        }
    }
    if let Some(aura_families) = &request.ghost_wisp {
        for spell_id in [8_326, 20_584] {
            steps.push(Step::DeleteLifecycleAuraEffectsBySpell { guid, spell_id });
            steps.push(Step::DeleteLifecycleAuraFamilyBySpell { guid, spell_id });
        }
        let mut aura_families = aura_families
            .iter()
            .filter(|family| matches!(family.spell_id, 8_326 | 20_584))
            .collect::<Vec<_>>();
        aura_families.sort_by_key(|family| {
            (
                family.spell_id,
                family.caster_guid_binary,
                family.item_guid_binary,
            )
        });
        for aura_family in aura_families {
            append_lifecycle_aura_family_insert_steps(&mut steps, guid, aura_family);
        }
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
    let currency_statement_position = steps.len();
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
    let mut statements = Vec::new();
    for index in 0..=steps.len() {
        if index == item_statement_position
            && let Some(items) = &request.inventory_items
        {
            for item in items {
                append_player_item_save_statements_like_cpp(
                    &mut statements,
                    request.player_guid,
                    item,
                );
            }
        }
        if index == currency_statement_position
            && let Some(currencies) = &request.currencies
        {
            statements.extend(super::player_currency_save_statements_like_cpp(currencies));
        }
        if let Some(step) = steps.get(index) {
            statements.push(player_character_save_statement_like_cpp(step));
        }
    }
    statements
}

fn append_player_item_save_statements_like_cpp(
    statements: &mut Vec<PreparedStatement>,
    player_guid: u64,
    item: &PlayerItemSaveLikeCpp,
) {
    match item.state {
        PlayerItemSaveStateLikeCpp::New | PlayerItemSaveStateLikeCpp::Changed => {
            let mut link = PreparedStatement::for_statement(CharStatements::REP_CHAR_INVENTORY_ITEM);
            link.set_u64(0, player_guid);
            link.set_u64(1, item.bag_guid);
            link.set_u8(2, item.slot);
            link.set_u64(3, item.item_guid);
            statements.push(link);

            let kind = if item.state == PlayerItemSaveStateLikeCpp::New {
                CharStatements::REP_ITEM_INSTANCE
            } else {
                CharStatements::UPD_ITEM_INSTANCE
            };
            let mut item_statement = PreparedStatement::for_statement(kind);
            item_statement.set_u32(0, item.item_entry);
            item_statement.set_u64(1, item.owner_guid);
            item_statement.set_u64(2, item.creator_guid);
            item_statement.set_u64(3, item.gift_creator_guid);
            item_statement.set_u32(4, item.count);
            item_statement.set_u32(5, item.expiration);
            item_statement.set_string(6, &item.charges);
            item_statement.set_u32(7, item.dynamic_flags);
            item_statement.set_string(8, &item.enchantments);
            item_statement.set_u16(9, item.durability as u16);
            item_statement.set_u32(10, item.played_time);
            item_statement.set_string(11, &item.text);
            item_statement.set_u32(12, item.battle_pet_species_id);
            item_statement.set_u32(13, item.battle_pet_breed_data);
            item_statement.set_u32(14, item.battle_pet_level);
            item_statement.set_u32(15, item.battle_pet_display_id);
            item_statement.set_i32(16, item.random_properties_id);
            item_statement.set_i32(17, item.property_seed);
            item_statement.set_u8(18, item.context);
            item_statement.set_u64(19, item.item_guid);
            statements.push(item_statement);

            if item.state == PlayerItemSaveStateLikeCpp::Changed && item.wrapped {
                let mut gift = PreparedStatement::for_statement(CharStatements::UPD_GIFT_OWNER);
                gift.set_u64(0, item.owner_guid);
                gift.set_u64(1, item.item_guid);
                statements.push(gift);
            }

            let mut delete_gems =
                PreparedStatement::for_statement(CharStatements::DEL_ITEM_INSTANCE_GEMS);
            delete_gems.set_u64(0, item.item_guid);
            statements.push(delete_gems);
            if let Some(gems) = &item.gems {
                let mut insert_gems =
                    PreparedStatement::for_statement(CharStatements::INS_ITEM_INSTANCE_GEMS);
                insert_gems.set_u64(0, item.item_guid);
                for (index, gem) in gems.iter().enumerate() {
                    let offset = 1 + index * 3;
                    insert_gems.set_u32(offset, gem.item_id);
                    insert_gems.set_string(offset + 1, &gem.bonus_list_ids);
                    insert_gems.set_u8(offset + 2, gem.context);
                }
                statements.push(insert_gems);
            }

            let mut delete_transmog =
                PreparedStatement::for_statement(CharStatements::DEL_ITEM_INSTANCE_TRANSMOG);
            delete_transmog.set_u64(0, item.item_guid);
            statements.push(delete_transmog);
            if let Some(transmog) = item.transmog {
                let mut insert_transmog =
                    PreparedStatement::for_statement(CharStatements::INS_ITEM_INSTANCE_TRANSMOG);
                insert_transmog.set_u64(0, item.item_guid);
                for (index, value) in transmog.into_iter().enumerate() {
                    insert_transmog.set_u32(index + 1, value);
                }
                statements.push(insert_transmog);
            }
            return;
        }
        PlayerItemSaveStateLikeCpp::Removed => {
            let mut delete_link =
                PreparedStatement::for_statement(CharStatements::DEL_CHAR_INVENTORY_ITEM);
            delete_link.set_u64(0, player_guid);
            delete_link.set_u64(1, item.item_guid);
            statements.push(delete_link);

            let mut delete_item = PreparedStatement::for_statement(CharStatements::DEL_ITEM_INSTANCE);
            delete_item.set_u64(0, item.item_guid);
            statements.push(delete_item);

            for kind in [
                CharStatements::DEL_ITEM_INSTANCE_GEMS,
                CharStatements::DEL_ITEM_INSTANCE_TRANSMOG,
            ] {
                let mut delete = PreparedStatement::for_statement(kind);
                delete.set_u64(0, item.item_guid);
                statements.push(delete);
            }
            if item.wrapped {
                let mut delete_gift = PreparedStatement::for_statement(CharStatements::DEL_GIFT);
                delete_gift.set_u64(0, item.item_guid);
                statements.push(delete_gift);
            }
            return;
        }
    }
}

fn append_lifecycle_aura_family_insert_steps(
    steps: &mut Vec<PlayerCharacterSaveStepLikeCpp>,
    guid: u64,
    aura_family: &PlayerLifecycleAuraFamilySave,
) {
    use PlayerCharacterSaveStepLikeCpp as Step;

    let effect_mask = |row: &PlayerLifecycleAuraSave| {
        row.effects
            .iter()
            .fold(0_u32, |mask, effect| mask | (1_u32 << effect.index))
    };
    let mut auras = aura_family.auras.clone();
    auras.sort_by_key(|row| (row.difficulty, effect_mask(row)));
    for mut row in auras {
        let mask = effect_mask(&row);
        steps.push(Step::InsertLifecycleAura {
            guid,
            caster_guid_binary: aura_family.caster_guid_binary,
            item_guid_binary: aura_family.item_guid_binary,
            spell_id: aura_family.spell_id,
            effect_mask: mask,
            row: row.clone(),
        });
        row.effects.sort_by_key(|effect| effect.index);
        steps.extend(
            row.effects
                .into_iter()
                .map(|effect| Step::InsertLifecycleAuraEffect {
                    guid,
                    caster_guid_binary: aura_family.caster_guid_binary,
                    item_guid_binary: aura_family.item_guid_binary,
                    spell_id: aura_family.spell_id,
                    effect_mask: mask,
                    row: effect,
                }),
        );
    }
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
