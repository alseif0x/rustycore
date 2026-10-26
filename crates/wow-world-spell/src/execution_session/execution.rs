// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The represented spell execution entry point that drives effect application
//! (#1363 slice 9d), from the World `session/spell_effects/execution.rs`.

use super::*;

pub(crate) fn spell_go_original_cast_id_like_cpp(
    metadata: SpellCastMetadata,
    cast_id: ObjectGuid,
    server_triggered_resurrection_sickness: bool,
) -> ObjectGuid {
    if metadata.server_normal_cast_like_cpp || server_triggered_resurrection_sickness {
        metadata.original_cast_id
    } else {
        metadata.original_cast_id_or(cast_id)
    }
}

/// Represented aura duration for a creature target: the same bounded 30-second
/// convention the player self-cast chain uses until `SpellDuration.db2` is
/// consumed for every represented aura.
const REPRESENTED_CREATURE_AURA_DURATION_MS_LIKE_CPP: u32 = 30_000;

fn unit_is_in_world_on_map(map: &wow_map::Map, guid: ObjectGuid) -> bool {
    if guid.is_player() {
        map.get_typed_player(guid)
            .is_some_and(|player| player.unit().world().object().is_in_world())
    } else if guid.is_creature() {
        map.get_typed_creature(guid)
            .is_some_and(|creature| creature.unit().world().object().is_in_world())
    } else if guid.is_pet() {
        map.get_typed_pet(guid)
            .is_some_and(|pet| pet.creature().unit().world().object().is_in_world())
    } else {
        false
    }
}

/// C++ `Spell` resolves a distinct `originalCasterGUID` through the caster's
/// Map and discards it when the Unit is absent or out of world. Equal GUIDs
/// use `m_caster->ToUnit()` directly. Return `None` for an unresolved explicit
/// original so each existing aura path can keep its own caster fallback.
fn resolve_original_aura_caster_guid<H: SpellExecutionHostLikeCpp>(
    host: &H,
    caster_guid: ObjectGuid,
    original_caster_guid: Option<ObjectGuid>,
) -> Option<ObjectGuid> {
    let original_caster_guid = original_caster_guid?;
    if original_caster_guid == caster_guid {
        return Some(caster_guid);
    }

    let core = host.core();
    let player_guid = core.player_guid()?;
    let handle = core.player_handle_like_cpp?;
    if handle.guid() != player_guid || (caster_guid.is_player() && caster_guid != player_guid) {
        return None;
    }
    let manager = core.canonical_map_manager.as_ref()?.lock().ok()?;
    let key = match manager.player_residence_like_cpp(handle)? {
        wow_map::PlayerResidenceLikeCpp::Active(key) => key,
        wow_map::PlayerResidenceLikeCpp::Detached => return None,
    };
    let map = manager.find_map(key.map_id, key.instance_id)?.map();
    if !unit_is_in_world_on_map(map, caster_guid) {
        return None;
    }
    unit_is_in_world_on_map(map, original_caster_guid).then_some(original_caster_guid)
}

pub async fn execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp<
    H: SpellExecutionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    spell_id: i32,
    target_guid: ObjectGuid,
    cast_id: ObjectGuid,
    mut spell_visual: wow_packet::packets::spell::SpellCastVisual,
    target_data: SpellTargetData,
    mut metadata: SpellCastMetadata,
) -> Result<(), &'static str> {
    let player_guid = host.player_guid().ok_or("No player GUID")?;
    let caster_guid = metadata.caster_guid_override.unwrap_or(player_guid);
    let player_resurrection_sickness = target_guid == player_guid
        && caster_guid == player_guid
        && super::resurrection_sickness::is_resurrection_sickness_spell(host, spell_id);
    let server_triggered_resurrection_sickness = !metadata.from_client
        && metadata.trigger_cast_flags_like_cpp == wow_entities::TRIGGERED_FULL_MASK_LIKE_CPP
        && player_resurrection_sickness;
    let server_triggered_player_ghost_wisp = super::resurrection_auras::is_selected_self_cast(
        host,
        spell_id,
        target_guid,
        caster_guid,
        metadata.from_client,
        metadata.trigger_cast_flags_like_cpp == wow_entities::TRIGGERED_FULL_MASK_LIKE_CPP,
    );
    let selected_full_server_aura =
        server_triggered_resurrection_sickness || server_triggered_player_ghost_wisp;
    let selected_execution_effects =
        player_resurrection_sickness || server_triggered_player_ghost_wisp;
    if selected_execution_effects {
        spell_visual.spell_visual_id = super::selected_cast_visual::resolve(host, spell_id)?;
    }

    // Native normal Player casts execute the exact key admitted during
    // prepare and resolve each represented payload family through the
    // canonical difficulty chain. They do not borrow the legacy key-zero
    // payload. A full-mask
    // selected server aura may instead use a transient carrier after its
    // exact nonzero key is proven; its effects and attributes come from the
    // selected rows, and the carrier is never installed in SpellStore.
    let (legacy_spell_info, has_difficulty_zero_key, normal_spell_info) = {
        let spell_store = host
            .hub_ref()
            .catalogs
            .spell_store()
            .ok_or("Spell not found")?;
        let has_difficulty_zero_key = u32::try_from(spell_id)
            .ok()
            .is_some_and(|spell_id| spell_store.contains_spell_info_exact_like_cpp(spell_id, 0));
        let normal_spell_info = if metadata.server_normal_cast_like_cpp {
            let actual_difficulty = metadata
                .actual_spell_difficulty_id_like_cpp
                .ok_or("Normal Player cast lost selected SpellInfo difficulty")?;
            let difficulties = host
                .hub_ref()
                .catalogs
                .difficulty_store()
                .map(AsRef::as_ref);
            Some(
                spell_store
                    .spell_info_for_exact_key_like_cpp(spell_id, actual_difficulty, difficulties)
                    .map_err(|error| {
                        tracing::warn!(
                            spell_id,
                            actual_difficulty,
                            ?error,
                            "Normal Player cast SpellInfo payload unavailable"
                        );
                        "Normal Player cast SpellInfo payload unavailable"
                    })?
                    .ok_or("Normal Player cast exact SpellInfo key unavailable")?,
            )
        } else {
            None
        };
        (
            spell_store.get(spell_id).cloned(),
            has_difficulty_zero_key,
            normal_spell_info,
        )
    };
    let mut transient_selected_threat_plan = None;
    let mut spell_info = match normal_spell_info {
        Some(spell_info) => spell_info,
        None => match legacy_spell_info {
            Some(spell_info) => spell_info,
            None if selected_full_server_aura && !has_difficulty_zero_key => {
                let (spell_info, threat_plan) =
                    super::selected_effects::resolve_transient_full_aura(host, spell_id)?;
                transient_selected_threat_plan = Some(threat_plan);
                spell_info
            }
            None => return Err("Spell not found"),
        },
    };
    let selected_cast_threat_plan = if selected_execution_effects {
        match transient_selected_threat_plan {
            Some(threat_plan) => threat_plan,
            None => super::selected_effects::resolve(host, spell_id, &mut spell_info)?,
        }
    } else {
        super::selected_effects::SelectedCastThreatPlan::Legacy
    };
    let effect_type = spell_info.effect_type;
    let effect_base_points = spell_info.effect_base_points;

    info!(
        account = host.core().account_id,
        spell_id = spell_id,
        target = ?target_guid,
        effect_type = effect_type,
        "Executing spell effect"
    );

    let Some(represented_focus_object) = prepare_represented_spell_execution_like_cpp(
        host,
        &spell_info,
        &target_data,
        spell_id,
        cast_id,
        &spell_visual,
        metadata,
    )?
    else {
        return Ok(());
    };

    // C++ `Spell::SelectSpellTargets` resolves implicit destinations in
    // effect/target order before `Spell::SendSpellGo`; later selections
    // replace earlier ones through `SpellCastTargets::ModDst`.
    let ResolvedSpellEffectTargetsLikeCpp {
        target_data,
        spell_go_target_data,
        effect_target_data_like_cpp,
    } = resolve_spell_effect_target_data_like_cpp(
        host,
        spell_id,
        caster_guid,
        &spell_info,
        target_data,
        represented_focus_object,
    );

    let mut deferred_server_normal_cast_item_use = false;
    let mut take_server_normal_cast_item_before_effects = false;
    if metadata.server_normal_cast_like_cpp {
        match host.server_normal_pre_go_like_cpp(&spell_info, cast_id, &spell_visual, metadata) {
            Ok(plan) => {
                metadata = plan.metadata;
                deferred_server_normal_cast_item_use = plan.take_cast_item;
                take_server_normal_cast_item_before_effects = plan.take_cast_item_before_effects;
            }
            Err(failure) => {
                let (reason, fail_arg1, fail_arg2) = match failure {
                    super::ServerNormalPreGoFailureLikeCpp::MissingReagent(entry_id) => (
                        wow_constants::SpellCastResult::Reagents as i32,
                        entry_id as i32,
                        0,
                    ),
                    super::ServerNormalPreGoFailureLikeCpp::MissingCurrency(currency_id) => (
                        wow_constants::SpellCastResult::Reagents as i32,
                        -1,
                        i32::from(currency_id),
                    ),
                    super::ServerNormalPreGoFailureLikeCpp::Power => {
                        (wow_constants::SpellCastResult::DontReport as i32, 0, 0)
                    }
                    super::ServerNormalPreGoFailureLikeCpp::Unavailable(reason) => {
                        tracing::warn!(
                            spell_id,
                            reason,
                            "Normal Player pre-GO authority unavailable"
                        );
                        (
                            wow_constants::SpellCastResult::SpellUnavailable as i32,
                            0,
                            0,
                        )
                    }
                };
                if !matches!(failure, super::ServerNormalPreGoFailureLikeCpp::Power) {
                    host.send_packet(&wow_packet::packets::spell::CastFailed {
                        cast_id,
                        spell_id,
                        visual: spell_visual.clone(),
                        reason,
                        fail_arg1,
                        fail_arg2,
                    });
                }
                publish_player_cast_interrupted_frames_like_cpp(
                    host,
                    metadata,
                    cast_id,
                    spell_id,
                    &spell_visual,
                );
                return Ok(());
            }
        }
    }

    // Send SMSG_SPELL_GO
    use wow_packet::packets::spell::SpellGoPkt;

    let spell_visual_id = spell_visual.spell_visual_id;
    // C++ `SendSpellGo` samples the power that remains after the debit
    // above. Triggered consumers keep their own explicit metadata flags;
    // normal client defaults are never imposed on them.
    let go_phase = crate::PlayerCastPublicationPhaseLikeCpp::Go;
    let publishes_player_cast =
        metadata.client_cast_id.is_some() || metadata.server_normal_cast_like_cpp;
    let (cast_data, cast_flags) = if publishes_player_cast {
        player_cast_publication_like_cpp(host, &spell_info, &metadata, go_phase)
    } else {
        (Default::default(), metadata.cast_flags)
    };
    let send_spell_go =
        if server_triggered_resurrection_sickness || server_triggered_player_ghost_wisp {
            super::resurrection_sickness::should_send_resurrection_sickness_spell_go(
                host,
                spell_id,
                spell_visual.spell_visual_id,
            )?
        } else {
            true
        };
    if send_spell_go {
        let go_caster_guid = if metadata.server_normal_cast_like_cpp {
            metadata.cast_item_guid.unwrap_or(caster_guid)
        } else {
            caster_guid
        };
        let go_pkt = SpellGoPkt {
            cast_data,
            caster: go_caster_guid,
            cast_id,
            original_cast_id: spell_go_original_cast_id_like_cpp(
                metadata,
                cast_id,
                server_triggered_resurrection_sickness,
            ),
            spell_id,
            visual: spell_visual,
            cast_flags,
            cast_flags_ex: metadata.cast_flags_ex,
            cast_time_ms: wow_world_core::session::game_time_ms_like_cpp(),
            target: spell_go_target_data,
            hit_targets: vec![target_guid],
            miss_targets: Vec::new(),
        };
        if publishes_player_cast {
            publish_player_cast_frame_like_cpp(
                host,
                metadata,
                wow_packet::ServerPacket::to_bytes(&go_pkt),
            );
        } else {
            host.send_packet(&go_pkt);
        }
        if caster_guid != player_guid && caster_guid.is_any_type_creature() {
            host.broadcast_creature_packet_to_visible_set_like_cpp(
                caster_guid,
                wow_packet::ServerPacket::to_bytes(&go_pkt),
            );
        }
    }
    if take_server_normal_cast_item_before_effects && deferred_server_normal_cast_item_use {
        let _ = host.take_server_normal_cast_item_like_cpp(&spell_info, metadata);
    }
    // #1444 H3: TrinityCore `Spell::_cast` raises `OnPlayerSpellCast` for a
    // player caster (`Spell.cpp:3671`). Modules observe a cast that already
    // happened, so it is raised once SMSG_SPELL_GO is out and before the
    // effects apply; a creature or GameObject caster raises nothing.
    if caster_guid == player_guid && host.spell_cast_hook_wanted_like_cpp() {
        host.spell_cast_hook_like_cpp(&crate::SpellCastHookEventLikeCpp {
            guid: player_guid,
            spell_id: spell_id as u32,
            cast_id,
            target_guid,
            triggered: metadata.trigger_cast_flags_like_cpp != 0,
        });
    }
    apply_post_spell_go_target_effects_like_cpp(
        host,
        creature_spawn_catalogs,
        &spell_info,
        &target_data,
        &effect_target_data_like_cpp,
        spell_id,
        caster_guid,
        target_guid,
        spell_visual_id,
        effect_type,
        effect_base_points,
    )
    .await;

    let force_visibility_after_gameobject_summon = apply_spell_gameobject_summon_effects_like_cpp(
        host,
        spell_id,
        &spell_info,
        &target_data,
        &effect_target_data_like_cpp,
        represented_focus_object,
    );
    if force_visibility_after_gameobject_summon {
        host.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
            .await;
    }

    let direct_spell_effects_like_cpp: Vec<(u32, i32, u32, i32, i32, f32, f32)> =
        if spell_info.effects().is_empty() {
            vec![(effect_type, effect_base_points, 0, 0, 0, 0.0, 0.0)]
        } else {
            spell_info
                .effects()
                .iter()
                .filter(|effect| effect.effect != 0)
                .map(|effect| {
                    (
                        effect.effect,
                        effect.effect_base_points,
                        effect.effect_index,
                        effect.effect_misc_value_1,
                        effect.effect_trigger_spell,
                        effect.effect_bonus_coefficient_from_ap,
                        effect.effect_amplitude,
                    )
                })
                .collect()
        };
    // C++ owns one `Spell` object per cast, so `_executeLogEffects` starts
    // empty for it; the represented session reuses one accumulator.
    host.split_spell_state_mut()
        .0
        .clear_represented_spell_execute_log_effects_like_cpp();
    for (
        direct_effect_type,
        direct_effect_base_points,
        direct_effect_index,
        direct_effect_misc_value_1,
        direct_effect_trigger_spell,
        direct_effect_bonus_coefficient_from_ap,
        direct_effect_amplitude,
    ) in direct_spell_effects_like_cpp
    {
        let direct_effect_target_data = effect_target_data_like_cpp
            .iter()
            .find(|(effect_index, _)| *effect_index == direct_effect_index)
            .map(|(_, target_data)| target_data)
            .unwrap_or(&target_data);
        match direct_effect_type {
            x if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(x) => {}
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_INSTAKILL => {
                apply_instakill_like_cpp(host, item_guid_generator, spell_id, target_guid).await?;
            }
            x if x == super::resurrection_offer::SPELL_EFFECT_RESURRECT
                || x == super::resurrection_offer::SPELL_EFFECT_RESURRECT_NEW
                || x == super::resurrection_offer::SPELL_EFFECT_RESURRECT_WITH_AURA =>
            {
                if let Some(effect_info) = spell_info
                    .effects()
                    .iter()
                    .find(|effect| effect.effect_index == direct_effect_index)
                {
                    let hit_guid = if direct_effect_target_data.unit.is_empty() {
                        target_guid
                    } else {
                        direct_effect_target_data.unit
                    };
                    super::resurrection_offer::apply_resurrection_offer(
                        host,
                        creature_spawn_catalogs,
                        &spell_info,
                        effect_info,
                        caster_guid,
                        hit_guid,
                    );
                }
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL => {
                if let Ok(heal_amount) = u32::try_from(direct_effect_base_points) {
                    let heal_amount = represented_spell_healing_bonus_done_like_cpp(
                        host,
                        spell_id,
                        caster_guid,
                        target_guid,
                        spell_info.effect_bonus_coefficient,
                        direct_effect_bonus_coefficient_from_ap,
                        heal_amount,
                    );
                    apply_heal_from_caster_like_cpp(
                        host,
                        Some(spell_id),
                        caster_guid,
                        target_guid,
                        heal_amount,
                    )
                    .await?;
                } else {
                    debug!(
                        account = host.core().account_id,
                        spell_id,
                        effect_index = direct_effect_index,
                        effect_base_points = direct_effect_base_points,
                        "Skipping SPELL_EFFECT_HEAL because C++ EffectHeal returns when damage < 0"
                    );
                }
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL => {
                if let Ok(heal_amount) = u32::try_from(direct_effect_base_points) {
                    apply_heal_from_caster_like_cpp(
                        host,
                        Some(spell_id),
                        caster_guid,
                        target_guid,
                        heal_amount,
                    )
                    .await?;
                } else {
                    debug!(
                        account = host.core().account_id,
                        spell_id,
                        effect_index = direct_effect_index,
                        effect_base_points = direct_effect_base_points,
                        "Skipping SPELL_EFFECT_HEAL_MECHANICAL because C++ EffectHealMechanical returns when damage < 0"
                    );
                }
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH => {
                apply_heal_max_health_like_cpp(
                    host,
                    spell_id,
                    direct_effect_base_points,
                    caster_guid,
                    target_guid,
                )
                .await?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_PCT => {
                apply_heal_pct_like_cpp(
                    host,
                    spell_id,
                    direct_effect_base_points,
                    caster_guid,
                    target_guid,
                )
                .await?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS => {
                apply_add_extra_attacks_effect_like_cpp(
                    host,
                    x,
                    direct_effect_base_points,
                    target_guid,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE => {
                apply_inebriate_effect_like_cpp(host, direct_effect_base_points, target_guid);
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION => {
                host.apply_reputation_effect_like_cpp(
                    direct_effect_base_points,
                    direct_effect_misc_value_1,
                    target_guid,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN => {
                let drain_damage = power_drain_pre_scaled_damage_like_cpp(
                    host,
                    spell_id,
                    direct_effect_index,
                    caster_guid,
                    target_guid,
                    spell_info.effect_bonus_coefficient,
                    direct_effect_bonus_coefficient_from_ap,
                    direct_effect_base_points,
                );
                apply_power_drain_effect_like_cpp(
                    host,
                    item_guid_generator,
                    spell_id,
                    x,
                    drain_damage,
                    direct_effect_misc_value_1,
                    target_guid,
                    false,
                    direct_effect_amplitude,
                    cast_id,
                    spell_visual_id,
                )
                .await;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE => {
                apply_energize_effect_like_cpp(
                    host,
                    spell_id,
                    caster_guid,
                    direct_effect_base_points,
                    direct_effect_misc_value_1,
                    target_guid,
                    false,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE_PCT => {
                apply_energize_effect_like_cpp(
                    host,
                    spell_id,
                    caster_guid,
                    direct_effect_base_points,
                    direct_effect_misc_value_1,
                    target_guid,
                    true,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN => {
                let burn_damage = power_drain_pre_scaled_damage_like_cpp(
                    host,
                    spell_id,
                    direct_effect_index,
                    caster_guid,
                    target_guid,
                    spell_info.effect_bonus_coefficient,
                    direct_effect_bonus_coefficient_from_ap,
                    direct_effect_base_points,
                );
                apply_power_drain_effect_like_cpp(
                    host,
                    item_guid_generator,
                    spell_id,
                    x,
                    burn_damage,
                    direct_effect_misc_value_1,
                    target_guid,
                    true,
                    direct_effect_amplitude,
                    cast_id,
                    spell_visual_id,
                )
                .await;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEALTH_LEECH => {
                apply_health_leech_like_cpp(
                    host,
                    item_guid_generator,
                    spell_id,
                    direct_effect_base_points,
                    target_guid,
                )
                .await?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD => {
                apply_dual_wield_effect_like_cpp(host, target_guid)?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_TITAN_GRIP => {
                apply_titan_grip_effect_like_cpp(host, direct_effect_misc_value_1)?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY => {
                apply_parry_effect_like_cpp(host)?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK => {
                apply_block_effect_like_cpp(host)?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PROFICIENCY => {
                apply_proficiency_effect_like_cpp(host, spell_id)?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE => {
                host.apply_durability_damage_effect_like_cpp(
                    x,
                    direct_effect_base_points,
                    direct_effect_misc_value_1,
                    target_guid,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE_PCT => {
                host.apply_durability_damage_pct_effect_like_cpp(
                    direct_effect_base_points,
                    direct_effect_misc_value_1,
                    target_guid,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR => {
                apply_give_honor_effect_like_cpp(host, direct_effect_base_points, target_guid)?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT => {
                apply_threat_effect_like_cpp(host, direct_effect_base_points, target_guid)?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISTRACT => {
                apply_distract_effect_like_cpp(
                    host,
                    direct_effect_base_points,
                    target_guid,
                    direct_effect_target_data,
                )?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT => {
                apply_modify_threat_percent_effect_like_cpp(
                    host,
                    direct_effect_base_points,
                    target_guid,
                )?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK_ME => {
                apply_taunt_effect_like_cpp(
                    host,
                    spell_id,
                    target_guid,
                    true,
                    wow_entities::AuraCastProvenanceLikeCpp {
                        cast_id,
                        spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                        caster_level: None,
                    },
                )?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWN => {
                apply_modify_cooldown_effect_like_cpp(
                    host,
                    direct_effect_base_points,
                    direct_effect_trigger_spell,
                    target_guid,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_CHARGES => {
                apply_modify_spell_charges_effect_like_cpp(
                    host,
                    direct_effect_base_points,
                    direct_effect_misc_value_1,
                    target_guid,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SANCTUARY => {
                apply_sanctuary_effect_like_cpp(host, target_guid)?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT => {
                apply_self_resurrect_effect_like_cpp(
                    host,
                    direct_effect_base_points,
                    direct_effect_misc_value_1,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_STUCK => {
                apply_stuck_effect_like_cpp(host).await;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PLAY_MOVIE => {
                apply_play_movie_effect_like_cpp(host, target_guid, direct_effect_misc_value_1);
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DUEL => {
                host.apply_duel_effect_like_cpp(spell_id, direct_effect_misc_value_1, target_guid);
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISMISS_PET => {
                apply_dismiss_pet_effect_like_cpp(host, target_guid);
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_FORCE_DESELECT => {
                host.apply_force_deselect_effect_like_cpp();
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER => {
                apply_change_raid_marker_effect_like_cpp(
                    host,
                    direct_effect_base_points,
                    direct_effect_target_data,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL => {
                host.apply_learn_spell_effect_like_cpp(direct_effect_trigger_spell, target_guid)
                    .await;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET => {
                apply_learn_transmog_set_effect_like_cpp(
                    host,
                    target_guid,
                    direct_effect_misc_value_1,
                );
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM => {
                apply_upgrade_heirloom_effect_like_cpp(host, metadata);
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET => {
                wow_world_inventory::apply_uncage_battle_pet_effect_like_cpp(
                    host, spell_id, cast_id, metadata,
                )
                .await;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL => {
                wow_world_inventory::apply_grant_battle_pet_level_effect_like_cpp(
                    host,
                    metadata,
                    direct_effect_base_points as u16,
                )
                .await;
            }
            x if x
                == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE =>
            {
                wow_world_inventory::apply_grant_battle_pet_experience_effect_like_cpp(
                    host,
                    metadata,
                    direct_effect_base_points as u16,
                )
                .await;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PULL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_TRADE_SKILL => {}
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE => {
                host.apply_quest_complete_effect_like_cpp(
                    item_guid_generator,
                    target_guid,
                    direct_effect_misc_value_1,
                )
                .await?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT => {
                apply_kill_credit_effect_like_cpp(
                    host,
                    item_guid_generator,
                    target_guid,
                    direct_effect_misc_value_1,
                    false,
                )
                .await?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT2 => {
                apply_kill_credit_effect_like_cpp(
                    host,
                    item_guid_generator,
                    target_guid,
                    direct_effect_misc_value_1,
                    true,
                )
                .await?;
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE => {
                if let Ok(damage_amount) = u32::try_from(direct_effect_base_points) {
                    let damage_amount = represented_spell_damage_bonus_done_like_cpp(
                        host,
                        spell_id,
                        direct_effect_index,
                        caster_guid,
                        target_guid,
                        spell_info.effect_bonus_coefficient,
                        direct_effect_bonus_coefficient_from_ap,
                        damage_amount,
                    );
                    host.apply_damage_from_caster_like_cpp(
                        item_guid_generator,
                        Some(spell_id),
                        caster_guid,
                        target_guid,
                        damage_amount,
                        cast_id,
                        spell_visual_id,
                    )
                    .await?;
                } else {
                    debug!(
                        account = host.core().account_id,
                        spell_id,
                        effect_index = direct_effect_index,
                        effect_base_points = direct_effect_base_points,
                        "Skipping SPELL_EFFECT_SCHOOL_DAMAGE because C++ only applies positive m_damage"
                    );
                }
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE => {
                apply_effect_environmental_damage_like_cpp(
                    host,
                    &wow_data::SpellEffectInfo {
                        effect_index: direct_effect_index,
                        effect: direct_effect_type,
                        effect_base_points: direct_effect_base_points,
                        ..Default::default()
                    },
                    target_guid,
                );
            }
            _ => {}
        }
    }

    if !spell_info.effects().is_empty() {
        let apply_aura_rows_like_cpp = spell_info
            .effects()
            .iter()
            .filter(|effect| {
                effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            })
            .count();
        let generic_apply_aura_rows_like_cpp = spell_info
            .effects()
            .iter()
            .filter(|effect| {
                effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                    && !effect.is_mounted_aura_like_cpp()
                    && !effect.is_provide_spell_focus_aura_like_cpp()
                    && !effect.is_battle_pet_xp_pct_aura_like_cpp()
                    && effect.effect_aura
                        != wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE
                    && effect.effect_aura
                        != wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE
                    && effect.effect_aura
                        != wow_data::spell::aura_types::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION
                    && effect.effect_aura != wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT
            })
            .count();
        // C++ `Spell::EffectApplyAura` applies to `unitTarget`, not to the
        // caster. The represented player chain below is the host-cast path;
        // a creature target receives one canonical `AuraApplication`
        // carrying every applied effect slot.
        let original_aura_caster_guid =
            resolve_original_aura_caster_guid(host, caster_guid, metadata.original_caster_guid);
        if player_resurrection_sickness {
            super::resurrection_sickness::apply_resurrection_sickness_aura(
                host,
                spell_id,
                original_aura_caster_guid.unwrap_or(caster_guid),
                &spell_info,
                cast_id,
                spell_visual_id,
                metadata.resurrection_sickness_remaining_ms,
            )?;
        } else if server_triggered_player_ghost_wisp {
            super::resurrection_auras::apply(
                host,
                spell_id,
                original_aura_caster_guid.unwrap_or(player_guid),
                &spell_info,
                cast_id,
                spell_visual_id,
            )?;
        }
        let mut creature_aura_applied = false;
        for effect in spell_info.effects().iter().filter(|effect| {
            effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
        }) {
            // C++ `Spell::DoSpellEffectHit` creates the Aura with the resolved
            // original caster when present. Keep the spell's visible caster
            // above for `SMSG_SPELL_GO`; this scoped value reaches only Aura
            // application caster fields and C++ `Aura::GenerateKey` identity.
            let aura_caster_guid = original_aura_caster_guid.unwrap_or(player_guid);
            if player_resurrection_sickness || server_triggered_player_ghost_wisp {
                continue;
            }
            // The represented taunt path owns `SPELL_AURA_MOD_TAUNT` and
            // its `EffectTaunt` current-victim gate, so that aura keeps
            // flowing through the chain below instead of the generic
            // creature application.
            if target_guid != player_guid
                && target_guid.is_creature()
                && effect.effect_aura != wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT
            {
                if creature_aura_applied {
                    continue;
                }
                creature_aura_applied = true;
                let effect_mask = spell_info
                    .effects()
                    .iter()
                    .filter(|candidate| {
                        candidate.effect
                            == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                    })
                    .fold(0u32, |mask, candidate| {
                        mask | 1u32.checked_shl(candidate.effect_index).unwrap_or(0)
                    });
                host.apply_creature_aura_with_provenance_like_cpp(
                    spell_id,
                    original_aura_caster_guid.unwrap_or(caster_guid),
                    target_guid,
                    effect_mask,
                    REPRESENTED_CREATURE_AURA_DURATION_MS_LIKE_CPP,
                    wow_entities::AuraCastProvenanceLikeCpp {
                        cast_id,
                        spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                        caster_level: None,
                    },
                )?;
                continue;
            }
            if effect.is_mounted_aura_like_cpp() {
                apply_represented_mounted_aura_like_cpp(host, spell_id, aura_caster_guid, effect)?;
            } else if effect.is_provide_spell_focus_aura_like_cpp() {
                apply_represented_provide_spell_focus_aura_like_cpp(
                    host,
                    spell_id,
                    aura_caster_guid,
                    effect,
                )?;
            } else if effect.is_battle_pet_xp_pct_aura_like_cpp() {
                let (state, mut hub) = host.split_spell_state_mut();
                state.apply_represented_battle_pet_xp_pct_aura_like_cpp(
                    &mut hub,
                    spell_id,
                    aura_caster_guid,
                    effect,
                )?;
            } else if effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::ModDetectRange,
                    30_000,
                )?;
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::ModDetectedRange,
                    30_000,
                )?;
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
                    30_000,
                )?;
            } else if effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT {
                if !spell_info.effects().iter().any(|effect| {
                    effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK_ME
                }) {
                    if !target_guid.is_any_type_creature() {
                        return Err("Taunt aura requires a creature target");
                    }
                    apply_taunt_effect_like_cpp(
                        host,
                        spell_id,
                        target_guid,
                        false,
                        wow_entities::AuraCastProvenanceLikeCpp {
                            cast_id,
                            spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                            caster_level: None,
                        },
                    )?;
                }
            } else if effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MOD_SCALE {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::ModScale,
                    30_000,
                )?;
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_NO_CONTROL
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::ModSpeedNoControl,
                    30_000,
                )?;
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::Speed,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_run_speed_rate_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::SwimSpeed,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_swim_speed_rate_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::DecreaseSpeed,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_forward_speed_rates_like_cpp();
                host.hub_mut()
                    .recompute_represented_backward_speed_rates_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_forward_speed_rates_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_MINIMUM_SPEED
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::MinimumSpeed,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_run_speed_rate_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_MINIMUM_SPEED_RATE
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::MinimumSpeedRate,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_run_speed_rate_like_cpp();
            } else if effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_ALWAYS
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::SpeedAlways,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_run_speed_rate_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_NOT_STACK
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::SpeedNotStack,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_run_speed_rate_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::MountedSpeed,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_mounted_speed_rates_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::MountedSpeedAlways,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_mounted_speed_rates_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::MountedSpeedNotStack,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_mounted_speed_rates_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
                    30_000,
                )?;
                update_represented_flight_flags_for_flight_aura_like_cpp(host, true);
                host.hub_mut()
                    .recompute_represented_mounted_speed_rates_like_cpp();
            } else if effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_FLY {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::Fly,
                    30_000,
                )?;
                update_represented_flight_flags_for_flight_aura_like_cpp(host, true);
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::FlightSpeed,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_flight_speed_rate_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::VehicleFlightSpeed,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_flight_speed_rate_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::MountedFlightSpeedAlways,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_mounted_speed_rates_like_cpp();
            } else if effect.effect_aura
                == wow_data::spell::aura_types::SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK
            {
                host.apply_represented_aura_modifier_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    effect,
                    RepresentedAuraEffectLikeCpp::FlightSpeedNotStack,
                    30_000,
                )?;
                host.hub_mut()
                    .recompute_represented_mounted_speed_rates_like_cpp();
            } else if generic_apply_aura_rows_like_cpp == 1 && apply_aura_rows_like_cpp == 1 {
                host.apply_aura_with_effect_mask_and_provenance_like_cpp(
                    spell_id,
                    aura_caster_guid,
                    30000,
                    0x00000001,
                    1u32 << effect.effect_index,
                    wow_entities::AuraCastProvenanceLikeCpp {
                        cast_id,
                        spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                        caster_level: None,
                    },
                    true,
                )?;
            } else {
                debug!(
                    account = host.core().account_id,
                    spell_id,
                    effect_index = effect.effect_index,
                    "Skipping represented generic multi-effect aura grouping until C++ aura effect-mask application is ported"
                );
            }
        }
    }

    apply_primary_spell_effect_fallback_like_cpp(
        host,
        effect_type,
        &spell_info,
        spell_id,
        player_guid,
        cast_id,
        spell_visual_id,
    )?;

    if deferred_server_normal_cast_item_use && !take_server_normal_cast_item_before_effects {
        let _ = host.take_server_normal_cast_item_like_cpp(&spell_info, metadata);
    }

    complete_spell_execution_like_cpp(
        host,
        spell_id,
        caster_guid,
        target_guid,
        player_guid,
        spell_info,
        metadata,
        selected_full_server_aura,
        selected_cast_threat_plan,
    )?;

    Ok(())
}
