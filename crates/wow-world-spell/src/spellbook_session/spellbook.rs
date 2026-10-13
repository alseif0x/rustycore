// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Known spells, learning and unlearning (C++ `Player::AddSpell`/`LearnSpell`/
//! `RemoveSpell`), moved from the World `session/spell_state/spellbook.rs`
//! (#1363 slice 9a).

use super::*;

pub fn spell_learn_skill_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    spell_id: u32,
) -> Option<&SpellLearnSkillNodeLikeCpp> {
    match host
        .hub_ref()
        .catalogs
        .spell_learn_skill_lookup_like_cpp(spell_id)
    {
        SpellLearnSkillLookupLikeCpp::Present(node) => Some(node),
        SpellLearnSkillLookupLikeCpp::CoveredWithoutNode
        | SpellLearnSkillLookupLikeCpp::Indeterminate(_)
        | SpellLearnSkillLookupLikeCpp::MissingCoverage => None,
    }
}
pub fn represented_spell_valid_for_learning_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> bool {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_spell_valid_for_learning_like_cpp(hub, spell_id)
}
pub fn represented_direct_learn_spell_triggers_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> Vec<i32> {
    let (state, hub) = host.split_spell_state_ref();
    state.represented_direct_learn_spell_triggers_like_cpp(hub, spell_id)
}
pub fn learn_account_mount_spells_like_cpp<H: SpellbookHostLikeCpp>(host: &mut H) -> usize {
    let Some(mut spell_ids) = host
        .hub_ref()
        .player_collection_state_snapshot_like_cpp()
        .map(|collections| {
            collections
                .mounts_like_cpp()
                .keys()
                .copied()
                .collect::<Vec<_>>()
        })
    else {
        return 0;
    };
    spell_ids.sort_unstable();
    spell_ids.dedup();

    let mut learned = 0usize;
    for spell_id in spell_ids {
        let Ok(spell_id_u32) = u32::try_from(spell_id) else {
            continue;
        };
        if !host
            .hub_ref()
            .catalogs
            .mount_store
            .as_ref()
            .is_none_or(|store| {
                store
                    .get_by_source_spell_id_like_cpp(spell_id_u32)
                    .is_some()
            })
        {
            continue;
        }
        let before = known_spells_like_cpp(host).len();
        learn_dependent_known_spell_like_cpp(host, spell_id);
        learned += usize::from(known_spells_like_cpp(host).len() != before);
    }
    learned
}
fn previous_spell_learn_skill_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
    mut prev_spell: u32,
) -> Option<SpellLearnSkillNodeLikeCpp> {
    let mut prev_skill = spell_learn_skill_like_cpp(host, prev_spell).copied();
    while prev_skill.is_none() && prev_spell != 0 {
        prev_spell = host
            .hub_ref()
            .catalogs
            .prev_spell_in_chain_like_cpp(prev_spell);
        let first_spell_id = host
            .hub_ref()
            .catalogs
            .first_spell_in_chain_like_cpp(prev_spell);
        prev_skill = spell_learn_skill_like_cpp(host, first_spell_id).copied();
    }
    prev_skill
}
fn downgrade_represented_spell_learn_skill_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    learned_skill: SpellLearnSkillNodeLikeCpp,
    current_spell_id: u32,
) {
    let prev_spell = host
        .hub_ref()
        .catalogs
        .prev_spell_in_chain_like_cpp(current_spell_id);
    if prev_spell == 0 {
        host.hub_mut()
            .set_represented_player_skill_like_cpp(learned_skill.skill, 0, 0, 0);
        return;
    }

    let Some(prev_skill) = previous_spell_learn_skill_like_cpp(host, prev_spell) else {
        host.hub_mut()
            .set_represented_player_skill_like_cpp(learned_skill.skill, 0, 0, 0);
        return;
    };

    let Some(mut skill_value) = host
        .hub_ref()
        .resolved_player_skill_value_like_cpp(prev_skill.skill)
    else {
        return;
    };
    let Some(mut skill_max_value) = host
        .hub_ref()
        .resolved_player_skill_max_value_like_cpp(prev_skill.skill)
    else {
        return;
    };
    let mut new_skill_max_value = prev_skill.maxvalue;

    if new_skill_max_value == 0 {
        if let (Some(skill_store), Some(skill_line_store), Some(skill_tiers_store)) = (
            host.hub_ref().catalogs.skill_store(),
            host.hub_ref().catalogs.skill_line_store(),
            host.hub_ref().catalogs.skill_tiers_store(),
        ) {
            if let Some(rc_info) = skill_store.skill_race_class_info_like_cpp(
                prev_skill.skill,
                host.hub_ref().player_race_like_cpp(),
                host.hub_ref().player_class_like_cpp(),
            ) {
                match skill_store.skill_range_type_like_cpp(
                    rc_info,
                    skill_line_store,
                    skill_tiers_store,
                ) {
                    SkillRangeTypeLikeCpp::Language => {
                        skill_value = 300;
                        new_skill_max_value = 300;
                    }
                    SkillRangeTypeLikeCpp::Level => {
                        new_skill_max_value = host.hub_ref().max_skill_value_for_level_like_cpp();
                    }
                    SkillRangeTypeLikeCpp::Mono => {
                        new_skill_max_value = 1;
                    }
                    SkillRangeTypeLikeCpp::Rank => {
                        if let Some(tier) =
                            u32::try_from(rc_info.skill_tier_id)
                                .ok()
                                .and_then(|skill_tier_id| {
                                    skill_tiers_store.get_skill_tier_like_cpp(skill_tier_id)
                                })
                        {
                            new_skill_max_value = tier
                                .get_value_for_tier_index_like_cpp(u32::from(
                                    prev_skill.step.saturating_sub(1),
                                ))
                                .try_into()
                                .unwrap_or(u16::MAX);
                        }
                    }
                    SkillRangeTypeLikeCpp::None => {}
                }

                if rc_info.flags & wow_data::SKILL_FLAG_ALWAYS_MAX_VALUE_LIKE_CPP != 0 {
                    skill_value = new_skill_max_value;
                }
            }
        }
    } else if skill_value > prev_skill.value {
        skill_value = prev_skill.value;
    }

    if skill_max_value > new_skill_max_value {
        skill_max_value = new_skill_max_value;
    }
    if skill_value > new_skill_max_value {
        skill_value = new_skill_max_value;
    }

    host.hub_mut().set_represented_player_skill_like_cpp(
        prev_skill.skill,
        prev_skill.step,
        skill_value,
        skill_max_value,
    );
}
pub fn learn_known_spell_like_cpp<H: SpellbookHostLikeCpp>(host: &mut H, spell_id: i32) {
    if !known_spells_like_cpp(host).contains(&spell_id) {
        host.core()
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    // This low-level helper does not run the complete C++ AddSpell closure
    // (ranks, dependencies, skills, traits and overrides). Retaining exact
    // acquisition authority after it would make those mirrors stale.
    invalidate_represented_player_spell_rows_like_cpp(host);
    let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.learn_known_spell_id_like_cpp(spell_id);
    });
}
pub fn learn_dependent_known_spell_like_cpp<H: SpellbookHostLikeCpp>(host: &mut H, spell_id: i32) {
    learn_known_spell_like_cpp(host, spell_id);
    let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.mark_known_spell_dependent_like_cpp(spell_id);
    });
}
pub fn remove_known_spell_like_cpp<H: SpellbookHostLikeCpp>(host: &mut H, spell_id: i32) {
    remove_known_spell_with_suppress_messaging_like_cpp(host, spell_id, false);
}
pub fn remove_known_spell_with_suppress_messaging_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    suppress_messaging: bool,
) {
    if known_spells_like_cpp(host).contains(&spell_id) {
        host.core()
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    let mut seen = HashSet::new();
    remove_known_spell_with_seen_like_cpp(host, spell_id, true, suppress_messaging, &mut seen);
}
fn remove_known_spell_with_seen_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    spell_id: i32,
    learn_low_rank: bool,
    suppress_messaging: bool,
    seen: &mut HashSet<i32>,
) {
    if !known_spells_like_cpp(host).contains(&spell_id) {
        return;
    }

    let preserve_complete = host
        .with_player_spell_runtime_like_cpp(|runtime| runtime.rows_complete_like_cpp())
        .unwrap_or(false);
    if !preserve_complete {
        invalidate_represented_player_spell_rows_like_cpp(host);
    }

    if !seen.insert(spell_id) {
        return;
    }

    if let Ok(current_spell_id) = u32::try_from(spell_id) {
        let next_spell_id = host
            .hub_ref()
            .catalogs
            .next_spell_in_chain_like_cpp(current_spell_id);
        if next_spell_id != 0 {
            if let Ok(next_known_spell_id) = i32::try_from(next_spell_id) {
                let next_spell_is_talent = host
                    .hub_ref()
                    .catalogs
                    .spell_custom_attributes_for_difficulty_like_cpp(next_spell_id, 0)
                    & wow_data::SPELL_ATTR0_CU_IS_TALENT_LIKE_CPP
                    != 0;
                if known_spells_like_cpp(host).contains(&next_known_spell_id)
                    && !next_spell_is_talent
                {
                    remove_known_spell_with_seen_like_cpp(
                        host,
                        next_known_spell_id,
                        false,
                        false,
                        seen,
                    );
                }
            }
        }

        let spells_requiring_removed: Vec<i32> = host
            .hub_ref()
            .catalogs
            .spells_requiring_spell_like_cpp(current_spell_id)
            .iter()
            .filter_map(|spell| i32::try_from(*spell).ok())
            .collect();
        for requiring_spell_id in spells_requiring_removed {
            if known_spells_like_cpp(host).contains(&requiring_spell_id) {
                remove_known_spell_with_seen_like_cpp(host, requiring_spell_id, true, false, seen);
            }
        }
    }

    let Some(was_dependent) = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        let forgotten = runtime.forget_known_spell_like_cpp(spell_id);
        let was_dependent = forgotten.was_dependent;
        if preserve_complete {
            if was_dependent {
                runtime.remove_row_like_cpp(spell_id);
            } else if let Some(row) = runtime.row_mut_like_cpp(spell_id) {
                row.active = false;
                row.disabled = false;
                row.dependent = false;
                row.favorite = false;
                row.state = wow_entities::PlayerSpellLoadState::Removed;
            }
        }
        was_dependent
    }) else {
        return;
    };

    let mut unlearned_spells_packet_like_cpp = None;

    if let Ok(current_spell_id) = u32::try_from(spell_id) {
        let mut prev_activate = false;

        if let Some(learned_skill) = spell_learn_skill_like_cpp(host, current_spell_id).copied() {
            downgrade_represented_spell_learn_skill_like_cpp(host, learned_skill, current_spell_id);
        }

        let learned_spells: Vec<SpellLearnSpellNodeLikeCpp> = host
            .hub_ref()
            .catalogs
            .spell_learn_spell_map_bounds_like_cpp(current_spell_id)
            .to_vec();
        for learned_spell in learned_spells {
            if let Ok(learned_spell_id) = i32::try_from(learned_spell.spell) {
                remove_known_spell_with_seen_like_cpp(host, learned_spell_id, true, false, seen);
                if learned_spell.overrides_spell != 0 {
                    if let Ok(overrides_spell_id) = i32::try_from(learned_spell.overrides_spell) {
                        remove_represented_override_spell_like_cpp(
                            host,
                            overrides_spell_id,
                            learned_spell_id,
                        );
                    }
                }
            }
        }

        if learn_low_rank {
            let prev_spell_id = host
                .hub_ref()
                .catalogs
                .prev_spell_in_chain_like_cpp(current_spell_id);
            if prev_spell_id != 0 {
                if let Ok(prev_known_spell_id) = i32::try_from(prev_spell_id) {
                    let current_spell_is_ranked = host
                        .hub_ref()
                        .catalogs
                        .spell_catalogs
                        .spell_chain_store()
                        .and_then(|store| store.spell_chain_node_like_cpp(current_spell_id))
                        .is_some();
                    if current_spell_is_ranked
                        && known_spells_like_cpp(host).contains(&prev_known_spell_id)
                    {
                        if was_dependent {
                            learn_dependent_known_spell_like_cpp(host, prev_known_spell_id);
                        } else {
                            learn_known_spell_like_cpp(host, prev_known_spell_id);
                            let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
                                runtime.set_dependent_like_cpp(prev_known_spell_id, false);
                            });
                        }
                        host.send_packet(&wow_packet::packets::trainer::SupercededSpells::single(
                            spell_id,
                            prev_known_spell_id,
                        ));
                        prev_activate = true;
                    }
                }
            }
        }

        if !prev_activate {
            unlearned_spells_packet_like_cpp = Some((current_spell_id, suppress_messaging));
        }
    }

    let trait_definition_id = host
        .mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.remove_override_spell_entry_like_cpp(spell_id);
            runtime.take_trait_definition_id_like_cpp(spell_id)
        })
        .flatten();
    if let Some(trait_definition_id) = trait_definition_id {
        if let Ok(trait_definition_id) = u32::try_from(trait_definition_id) {
            let override_spell_id = host
                .hub_ref()
                .catalogs
                .trait_definition_store()
                .and_then(|store| store.get(trait_definition_id))
                .map(|definition| definition.overrides_spell_id)
                .unwrap_or(0);
            if override_spell_id > 0 {
                remove_represented_override_spell_like_cpp(host, override_spell_id, spell_id);
            }
        }
    }
    cleanup_removed_spell_titan_grip_like_cpp(host, spell_id);
    {
        let (s, mut h) = host.split_spell_state_mut();
        s.cleanup_removed_spell_dual_wield_like_cpp(&mut h, spell_id)
    };
    if host.spell_state().offhand_check_at_spell_unlearn_like_cpp() {
        host.represented_auto_unequip_offhand_if_need_like_cpp(false);
    }

    if let Some((spell_id, suppress_messaging)) = unlearned_spells_packet_like_cpp {
        host.send_packet(&wow_packet::packets::trainer::UnlearnedSpells::single(
            spell_id,
            suppress_messaging,
        ));
    }
}
pub fn resolved_known_spells_like_cpp<H: SpellbookHostLikeCpp>(host: &H) -> Option<Vec<i32>> {
    host.with_player_spell_runtime_like_cpp(|runtime| runtime.known_spells_like_cpp().to_vec())
}
pub fn known_spells_like_cpp<H: SpellbookHostLikeCpp>(host: &H) -> Vec<i32> {
    resolved_known_spells_like_cpp(host).unwrap_or_default()
}
pub fn represented_dependent_known_spells_like_cpp<H: SpellbookHostLikeCpp>(
    host: &H,
) -> HashSet<i32> {
    host.with_player_spell_runtime_like_cpp(|runtime| {
        runtime
            .dependent_known_spells_like_cpp()
            .iter()
            .copied()
            .collect()
    })
    .unwrap_or_default()
}
