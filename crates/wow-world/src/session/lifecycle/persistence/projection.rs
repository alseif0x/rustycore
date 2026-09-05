//! Coherent, SQLx-free projection of one admitted Player. No callback, await or delivery.
//! C++ Player::SaveToDB (Player.cpp:19323ff) reads the same Player for all row families.
use super::*;

pub(super) fn request(
    session: &WorldSession,
    player: &wow_entities::Player,
    snapshot: &PlayerSaveToDbSnapshotLikeCpp,
    now_unix_secs: i64,
) -> Option<PlayerCharacterSaveRequestLikeCpp> {
    let game = player.gameplay_state();
    let mut player_flags = player.data().player_flags;
    let visible_resting = if game.rest.location_initialized {
        if game.rest.rest_flag_mask != 0 {
            player_flags |= crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
        } else {
            player_flags &= !crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP;
        }
        game.rest.rest_flag_mask != 0
    } else {
        player_flags & crate::session::PLAYER_FLAGS_RESTING_LIKE_CPP != 0
    };
    let skills: std::collections::HashMap<_, _> = player
        .skill_records_like_cpp()
        .iter()
        .filter_map(crate::session::represented_player_skill_record_like_cpp)
        .map(|row| (row.skill_id, row))
        .collect();
    let guid_counter = snapshot.guid.counter() as u64;
    let powers = character_power_snapshot_values_like_cpp(&snapshot.powers);
    let (dungeon_difficulty, raid_difficulty, legacy_raid_difficulty) =
        player.difficulty_preferences_like_cpp();
    let character = PlayerCharacterSnapshotSaveLikeCpp {
        position: PlayerPositionSaveLikeCpp {
            x: snapshot.position.x,
            y: snapshot.position.y,
            z: snapshot.position.z,
            orientation: snapshot.position.orientation,
            map_id: snapshot.map_id,
            instance_id: snapshot.instance_id,
            zone_id: game.world_local.zone_id as u16,
        },
        level: snapshot.level,
        xp: snapshot.xp,
        money: snapshot.money,
        rest_state: game.rest.rest_state,
        player_flags: player_flags,
        rest_bonus: game.rest.rest_bonus,
        logout_time: now_unix_secs.max(0) as u64,
        is_logout_resting: visible_resting,
        health: snapshot.health,
        powers,
        talent_reset_cost: game.talents.reset_talents_cost,
        talent_reset_time: game.talents.reset_talents_time_secs,
        explored_zones: crate::session::explored_zones_db_string_from_blocks_like_cpp(
            player.explored_zones_blocks_like_cpp(),
        ),
        dungeon_difficulty,
        raid_difficulty,
        legacy_raid_difficulty,
    };

    let spell_runtime = Some(&game.spells);
    let spells = if let Some(spells) =
        (game.spells.rows_loaded && game.spells.rows_complete).then(|| {
            game.spells
                .rows
                .iter()
                .map(|(&id, row)| {
                    (
                        id,
                        crate::session::represented_player_spell_record_like_cpp(row),
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>()
        }) {
        Some(PlayerSpellSaveGroupLikeCpp::Complete {
            rows: spells
                .values()
                .map(|spell| PlayerSpellSaveLikeCpp {
                    spell_id: spell.spell_id,
                    active: spell.active,
                    disabled: spell.disabled,
                    dependent: spell.dependent,
                    favorite: spell.favorite,
                    state: match spell.state {
                        RepresentedPlayerSpellStateLikeCpp::Unchanged => {
                            PlayerSpellStateLikeCpp::Unchanged
                        }
                        RepresentedPlayerSpellStateLikeCpp::Changed => {
                            PlayerSpellStateLikeCpp::Changed
                        }
                        RepresentedPlayerSpellStateLikeCpp::New => PlayerSpellStateLikeCpp::New,
                        RepresentedPlayerSpellStateLikeCpp::Removed => {
                            PlayerSpellStateLikeCpp::Removed
                        }
                        RepresentedPlayerSpellStateLikeCpp::Temporary => {
                            PlayerSpellStateLikeCpp::Temporary
                        }
                    },
                })
                .collect(),
            fallback_rows_were_present: spell_runtime
                .as_ref()
                .is_some_and(|runtime| !runtime.fallback_rows.is_empty()),
        })
    } else if spell_runtime
        .as_ref()
        .is_some_and(|runtime| !runtime.fallback_rows.is_empty())
    {
        Some(PlayerSpellSaveGroupLikeCpp::Fallback {
            rows: spell_runtime
                .as_ref()
                .expect("non-empty fallback spell runtime")
                .fallback_rows
                .values()
                .map(|spell| PlayerFallbackSpellSaveLikeCpp {
                    spell_id: spell.spell_id,
                    active: spell.active,
                    dependent: spell.dependent,
                })
                .collect(),
        })
    } else {
        None
    };

    let skills = if player.skill_records_complete_like_cpp()
        && player
            .occupied_skill_slots_like_cpp()
            .is_some_and(|count| skills.len() == usize::from(count))
    {
        Some((player.non_durable_skill_tombstones_like_cpp(), &skills)).map(
            |(tombstones, records)| {
                records
                    .values()
                    .filter(|skill| {
                        skill.state != RepresentedPlayerSkillStateLikeCpp::Deleted
                            && !tombstones.contains(&skill.skill_id)
                    })
                    .map(|skill| PlayerSkillSaveLikeCpp {
                        skill_id: skill.skill_id,
                        value: skill.value,
                        max: skill.max,
                        profession_slot: skill.profession_slot,
                    })
                    .collect()
            },
        )
    } else {
        None
    };

    let talent_runtime = Some(&game.talents);
    let glyphs = if talent_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.glyphs_loaded)
    {
        Some(
            talent_runtime
                .as_ref()
                .expect("checked canonical glyph authority")
                .glyph_groups
                .iter()
                .enumerate()
                .flat_map(|(talent_group, glyphs)| {
                    glyphs
                        .iter()
                        .copied()
                        .enumerate()
                        .map(move |(glyph_slot, glyph_id)| PlayerGlyphSaveLikeCpp {
                            talent_group: talent_group as u8,
                            glyph_slot: glyph_slot as u8,
                            glyph_id,
                        })
                })
                .collect(),
        )
    } else {
        None
    };

    let talents = if talent_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.talents_loaded)
    {
        let mut rows = Vec::new();
        for (talent_group, talents) in talent_runtime
            .as_ref()
            .expect("checked canonical talent authority")
            .talent_groups
            .iter()
            .enumerate()
        {
            for (talent_id, rank) in talents {
                if session
                    .represented_talent_info_like_cpp(*talent_id, *rank)
                    .is_some()
                {
                    rows.push(PlayerTalentSaveLikeCpp {
                        talent_id: *talent_id,
                        rank: *rank,
                        talent_group: talent_group as u8,
                    });
                }
            }
        }
        Some(rows)
    } else {
        None
    };

    let spell_history = Some(&player.unit().subsystems().spells.history);
    let spell_cooldowns = if spell_history
        .as_ref()
        .is_some_and(|history| history.cooldowns_loaded)
    {
        Some(
            spell_history
                .as_ref()
                .expect("loaded history resolved above")
                .cooldowns
                .values()
                .map(|cooldown| PlayerSpellCooldownSaveLikeCpp {
                    spell_id: cooldown.spell_id,
                    item_id: cooldown.item_id,
                    cooldown_end_unix_secs: (cooldown.cooldown_end_ms / 1_000).min(i64::MAX as u64)
                        as i64,
                    category_id: cooldown.category_id,
                    category_end_unix_secs: (cooldown.category_end_ms / 1_000).min(i64::MAX as u64)
                        as i64,
                })
                .collect(),
        )
    } else {
        None
    };

    let spell_charges = if spell_history
        .as_ref()
        .is_some_and(|history| history.charges_loaded)
    {
        Some(
            spell_history
                .as_ref()
                .expect("loaded history resolved above")
                .charges
                .iter()
                .flat_map(|(&category_id, charges)| {
                    charges
                        .iter()
                        .map(move |charge| PlayerSpellChargeSaveLikeCpp {
                            category_id,
                            recharge_start_unix_secs: (charge.recharge_start_ms / 1_000)
                                .min(i64::MAX as u64)
                                as i64,
                            recharge_end_unix_secs: (charge.recharge_end_ms / 1_000)
                                .min(i64::MAX as u64)
                                as i64,
                        })
                })
                .collect(),
        )
    } else {
        None
    };

    let action_buttons = if let Some(action_buttons) = player
        .action_buttons_loaded_like_cpp()
        .then(|| player.action_buttons_snapshot_like_cpp())
    {
        let (spec, trait_config_id) = (game.talents.active_group, 0);
        Some(PlayerActionButtonsSaveLikeCpp {
            spec,
            trait_config_id,
            rows: action_buttons
                .iter()
                .copied()
                .enumerate()
                .filter_map(|(button, packed_action)| {
                    if packed_action == 0 {
                        return None;
                    }
                    Some(PlayerActionButtonSaveLikeCpp {
                        button: u8::try_from(button).ok()?,
                        packed_action,
                    })
                })
                .collect(),
        })
    } else {
        None
    };

    let equipment_sets = match Some((&game.equipment_sets, game.equipment_sets_loaded)) {
        Some((sets, true)) => Some(
            sets.values()
                .map(|equipment_set| PlayerEquipmentSetSaveLikeCpp {
                    set_guid: equipment_set.guid,
                    set_id: equipment_set.set_id,
                    set_type: match equipment_set.set_type {
                        RepresentedEquipmentSetTypeLikeCpp::Equipment => {
                            PlayerEquipmentSetTypeLikeCpp::Equipment
                        }
                        RepresentedEquipmentSetTypeLikeCpp::Transmog => {
                            PlayerEquipmentSetTypeLikeCpp::Transmog
                        }
                    },
                    state: match equipment_set.state {
                        RepresentedEquipmentSetUpdateStateLikeCpp::Unchanged => {
                            PlayerEquipmentSetStateLikeCpp::Unchanged
                        }
                        RepresentedEquipmentSetUpdateStateLikeCpp::Changed => {
                            PlayerEquipmentSetStateLikeCpp::Changed
                        }
                        RepresentedEquipmentSetUpdateStateLikeCpp::New => {
                            PlayerEquipmentSetStateLikeCpp::New
                        }
                        RepresentedEquipmentSetUpdateStateLikeCpp::Deleted => {
                            PlayerEquipmentSetStateLikeCpp::Deleted
                        }
                    },
                    name: equipment_set.set_name.clone(),
                    icon: equipment_set.set_icon.clone(),
                    ignore_mask: equipment_set.ignore_mask,
                    assigned_spec_index: equipment_set.assigned_spec_index,
                    pieces: equipment_set
                        .pieces
                        .iter()
                        .map(|guid| guid.counter() as u64)
                        .collect(),
                    appearances: equipment_set.appearances.to_vec(),
                    enchants: equipment_set.enchants,
                })
                .collect(),
        ),
        _ => None,
    };

    let void_storage = match Some((&game.void_storage_items, game.void_storage_loaded)) {
        Some((items, true)) => Some(
            items
                .iter()
                .enumerate()
                .map(|(slot, item)| PlayerVoidStorageSlotSaveLikeCpp {
                    slot: u8::try_from(slot).expect("void-storage slot fits u8"),
                    item: item.as_ref().map(|item| PlayerVoidStorageSaveLikeCpp {
                        item_id: item.item_id,
                        item_entry: item.item_entry,
                        creator_guid: item.creator_guid.counter() as u64,
                        fixed_scaling_level: item.fixed_scaling_level,
                        random_properties_id: item.random_properties_id,
                        random_properties_seed: item.random_properties_seed,
                        context: item.context,
                    }),
                })
                .collect(),
        ),
        _ => None,
    };

    // C++ `_SaveQuestStatus` only consumes entries present in `m_QuestStatusSave`; it does
    // not rewrite every loaded quest during Player::SaveToDB. Rust's quest mutation paths
    // already persist their changed quest directly, but there is no coherent dirty-set seam
    // yet. Rewriting every active quest here can delete objective rows that were not mapped
    // into represented state, so preserve them until that dirty tracking exists.

    let tutorials = if session.tutorials_changed_like_cpp {
        if session.tutorials_loaded_coherently_like_cpp {
            Some(PlayerTutorialsSaveLikeCpp {
                tutorials: session.tutorials_like_cpp,
                already_persisted: session.tutorials_loaded_from_db_like_cpp,
            })
        } else {
            None
        }
    } else {
        None
    };

    let instance_lock_times = session
        .represented_instance_reset_times_like_cpp
        .iter()
        .map(
            |(&instance_id, &release_time)| PlayerInstanceLockTimeSaveLikeCpp {
                instance_id,
                release_time,
            },
        )
        .collect();
    let (total_time, level_time) = session.current_played_time_values_like_cpp();
    let played_time = PlayerPlayedTimeSaveLikeCpp {
        total_time,
        level_time,
    };

    let reputations =
        crate::reputation::ReputationMgrLikeCpp::from_player_gameplay_state_like_cpp(game)
            .pending_save_rows_like_cpp()
            .into_iter()
            .map(
                |(faction_id, standing, flags)| PlayerReputationSaveLikeCpp {
                    faction_id,
                    standing,
                    flags,
                },
            )
            .collect();

    let cuf_profiles = match Some((&game.cuf_profiles, game.cuf_profiles_loaded)) {
        Some((profiles, true)) => Some(
            (0..wow_packet::packets::misc::MAX_CUF_PROFILES_LIKE_CPP)
                .map(|id| PlayerCufProfileSlotSaveLikeCpp {
                    profile_id: id as u8,
                    profile: profiles.get(id).and_then(Option::as_ref).map(|profile| {
                        PlayerCufProfileSaveLikeCpp {
                            profile_name: profile.profile_name.clone(),
                            frame_height: profile.frame_height,
                            frame_width: profile.frame_width,
                            sort_by: profile.sort_by,
                            health_text: profile.health_text,
                            bool_options: profile.bool_options,
                            top_point: profile.top_point,
                            bottom_point: profile.bottom_point,
                            left_point: profile.left_point,
                            top_offset: profile.top_offset,
                            bottom_offset: profile.bottom_offset,
                            left_offset: profile.left_offset,
                        }
                    }),
                })
                .collect(),
        ),
        _ => None,
    };

    Some(PlayerCharacterSaveRequestLikeCpp {
        player_guid: guid_counter,
        account_id: session.account_id,
        wall_clock_unix_secs: now_unix_secs,
        character,
        spells,
        skills,
        glyphs,
        talents,
        spell_cooldowns,
        spell_charges,
        action_buttons,
        equipment_sets,
        void_storage,
        tutorials,
        instance_lock_times,
        played_time,
        reputations,
        cuf_profiles,
    })
}
