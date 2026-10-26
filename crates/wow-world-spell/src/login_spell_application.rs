// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The login spell application (#1363 slice 13c).
//!
//! C++ `Player::LoadFromDB` → `_LoadSpells`/`AddSpell` side effects the
//! continue-login coordinator replays: the known-spell and favourite
//! installation, the `spell_learn_spell` dependency closure with its
//! `SpellLearnSkill` pass, `CONFIG_START_ALL_SPELLS`, the parry/block
//! capabilities, the `_LoadSpells → AddMount` promotion, and the first-login
//! `AT_LOGIN_FIRST` clear and `PlayerInfo::castSpells`. Moved from the World
//! `session/spell_state/{spellbook,effects,cast,spell}.rs` and
//! `session/lifecycle_ops/operations.rs` verbatim except for the mechanical
//! `self` -> `host` rewrite: each host method is the same-named
//! `WorldSession` operation, called at the exact point the body called it
//! before the move.

use std::collections::HashSet;
use std::sync::Arc;

use tracing::{info, warn};

use wow_entities::SpellCastMetadata;
use wow_packet::packets::misc::AccountMount;
use wow_persistence::{
    AccountCollectionLoadOutcomeLikeCpp, AccountCollectionLoadRequestLikeCpp,
    AccountCollectionLoadedLikeCpp,
};
use wow_world_core::session::PlayerBootstrapCatalogsLikeCpp;
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;
use wow_world_inventory::BattlePetSessionHostLikeCpp;

use crate::{
    SpellEffectHostLikeCpp, SpellExecutionHostLikeCpp, SpellbookHostLikeCpp, TalentHostLikeCpp,
};

/// C++ `CAST_FLAG_PENDING` (`Spells/Spell.h:78`); `SendSpellStart` and `SendSpellGo` set it for a triggered cast that is not `m_fromClient`.
const CAST_FLAG_PENDING_LIKE_CPP: u32 = 0x0000_0001;

pub fn set_known_spells_like_cpp<H: SpellbookHostLikeCpp>(host: &mut H, spells: Vec<i32>) {
    host.core()
        .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    crate::invalidate_represented_player_spell_rows_like_cpp(host);
    let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.replace_known_spell_ids_like_cpp(spells);
    });
    crate::learn_account_mount_spells_like_cpp(host);
}
pub fn set_represented_favorite_known_spells_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    favorite_spells: HashSet<i32>,
) {
    let preserve_complete = host
        .with_player_spell_runtime_like_cpp(|runtime| runtime.rows_complete_like_cpp())
        .unwrap_or(false);
    if !preserve_complete {
        crate::invalidate_represented_player_spell_rows_like_cpp(host);
    }
    let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
        runtime.replace_known_favorites_like_cpp(favorite_spells);
    });
}
pub fn apply_loaded_known_spell_dependencies_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    known_spells: &mut Vec<i32>,
) -> usize {
    let roots = known_spells.clone();
    apply_loaded_spell_dependencies_from_roots_like_cpp(host, &roots, known_spells)
}
/// Apply the non-aura combat-capability effects cast by C++ `Player::AddSpell`
/// while `_LoadSpells` reconstructs known passive spells.
///
/// `SPELL_EFFECT_PARRY` and `SPELL_EFFECT_BLOCK` set `m_canParry` /
/// `m_canBlock` before `Player::UpdateAllStats`, so the first login
/// projection must observe those flags as well.
pub fn apply_login_known_spell_combat_capabilities_like_cpp<H: SpellEffectHostLikeCpp>(
    host: &mut H,
    known_spells: &[i32],
) -> usize {
    if host.player_guid().is_none() {
        return 0;
    }
    let Some(spell_store) = host.hub_ref().catalogs.spell_store().cloned() else {
        return 0;
    };

    let before = host.core().canonical_player_parry_block_snapshot_like_cpp();
    for &spell_id in known_spells {
        if !spell_store.is_passive_like_cpp(spell_id) {
            continue;
        }
        let Some(spell_info) = spell_store.get(spell_id) else {
            continue;
        };

        if spell_info.has_effect_like_cpp(wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY) {
            let _ = crate::apply_parry_effect_like_cpp(host);
        }
        if spell_info.has_effect_like_cpp(wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK) {
            let _ = crate::apply_block_effect_like_cpp(host);
        }
    }

    let after = host.core().canonical_player_parry_block_snapshot_like_cpp();
    usize::from(!before.0 && after.0) + usize::from(!before.1 && after.1)
}
/// Represented C++ `Player::LearnCustomSpells` / `CONFIG_START_ALL_SPELLS`.
///
/// C++ calls `AddSpell` while the player is not in world, so the full port
/// still needs character_spell persistence semantics. This represented slice
/// mirrors the login spell snapshot: configured custom spells are included in
/// `INITIAL_SPELLS` without duplicating spells already loaded from DB/DBC.
pub fn apply_represented_start_all_spells_with_catalogs_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
    known_spells: &mut Vec<i32>,
) -> usize {
    if !player_bootstrap.start_all_spells {
        return 0;
    }

    let spells = player_bootstrap
        .custom_spells
        .custom_spells_like_cpp(
            host.hub_ref().player_race_like_cpp(),
            host.hub_ref().player_class_like_cpp(),
        )
        .to_vec();

    let mut applied = 0usize;
    for spell_id in spells {
        let Ok(spell_id) = i32::try_from(spell_id) else {
            continue;
        };
        if !known_spells.contains(&spell_id) {
            known_spells.push(spell_id);
            applied += 1;
        }
        let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
            runtime.set_dependent_like_cpp(spell_id, true);
        });
    }

    applied
}
pub fn apply_loaded_spell_dependency_skills_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    known_spells: &mut Vec<i32>,
    loaded_spell_side_effect_spells: &mut Vec<i32>,
) -> (usize, bool) {
    // C++ AddSpell applies SpellLearnSkill and recursively AddSpell-s every
    // non-auto spell_learn_spell target. Expanding the complete closure
    // first is final-state equivalent only if every reached target then
    // participates in the SpellLearnSkill pass before skill authority is
    // published.
    let dependent_spell_count = apply_loaded_known_spell_dependencies_like_cpp(host, known_spells);
    for &spell_id in known_spells.iter() {
        if !loaded_spell_side_effect_spells.contains(&spell_id) {
            loaded_spell_side_effect_spells.push(spell_id);
        }
    }
    let skills_complete = {
        let (s, mut h) = host.split_spell_state_mut();
        s.apply_loaded_spell_learn_skills_like_cpp(&mut h, loaded_spell_side_effect_spells)
    };
    (dependent_spell_count, skills_complete)
}
pub fn apply_loaded_spell_dependencies_from_roots_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    roots: &[i32],
    known_spells: &mut Vec<i32>,
) -> usize {
    let mut added = 0usize;
    let mut pending = roots.to_vec();
    let mut index = 0usize;
    while index < pending.len() {
        let spell_id = pending[index];
        index += 1;

        let Ok(spell_id_u32) = u32::try_from(spell_id) else {
            continue;
        };
        let learned_spells = host
            .hub_ref()
            .catalogs
            .spell_learn_spell_map_bounds_like_cpp(spell_id_u32)
            .to_vec();
        for learned_spell in learned_spells {
            let Ok(learned_spell_id) = i32::try_from(learned_spell.spell) else {
                continue;
            };
            if !learned_spell.auto_learned && learned_spell.active {
                let _ = host.mutate_player_spell_runtime_like_cpp(|runtime| {
                    runtime.set_dependent_like_cpp(learned_spell_id, true);
                    runtime.set_favorite_like_cpp(learned_spell_id, false);
                });
                if !known_spells.contains(&learned_spell_id) {
                    known_spells.push(learned_spell_id);
                    pending.push(learned_spell_id);
                    added += 1;
                }
            }

            // C++ AddSpell rebuilds this edge for every active dependency,
            // including auto-learned and already-known spells.
            if learned_spell.active && learned_spell.overrides_spell != 0 {
                if let Ok(overrides_spell_id) = i32::try_from(learned_spell.overrides_spell) {
                    crate::add_represented_override_spell_like_cpp(
                        host,
                        overrides_spell_id,
                        learned_spell_id,
                    );
                }
            }
        }
    }

    added
}
/// Represented C++ first-login `PlayerInfo::castSpells[GetCreateMode()]`.
///
/// C++ executes these casts after clearing `AT_LOGIN_FIRST` and before
/// `CONFIG_START_ALL_EXPLORED` / `CONFIG_START_ALL_REP`. This slice uses the
/// represented spell executor; full triggered-cast semantics still depend on
/// the remaining Spell runtime port.
pub async fn apply_represented_first_login_cast_spells_with_catalogs_like_cpp<
    H: SpellExecutionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
) -> usize {
    let Some(player_guid) = host.player_guid() else {
        return 0;
    };
    let Some(create_mode) = host.hub_ref().player_create_mode_like_cpp() else {
        return 0;
    };
    let spells = player_bootstrap
        .cast_spells
        .cast_spells_like_cpp(
            host.hub_ref().player_race_like_cpp(),
            host.hub_ref().player_class_like_cpp(),
            create_mode,
        )
        .to_vec();

    let mut cast_count = 0usize;
    for spell_id in spells {
        // C++ `CharacterHandler.cpp` casts the create-mode spells with
        // `CastSpell(pCurrChar, spellId, true)`, i.e. TRIGGERED_FULL_MASK:
        // the cast is triggered and ignores the global cooldown.
        if crate::execute_server_triggered_spell_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id as i32,
            player_guid,
            SpellCastMetadata {
                cast_flags: CAST_FLAG_PENDING_LIKE_CPP,
                trigger_cast_flags_like_cpp: wow_entities::TRIGGERED_FULL_MASK_LIKE_CPP,
                ..SpellCastMetadata::default()
            },
        )
        .await
        .is_ok()
        {
            cast_count += 1;
        }
    }

    cast_count
}
pub fn apply_represented_first_login_flag_if_needed_like_cpp<H: TalentHostLikeCpp>(
    host: &mut H,
) -> bool {
    const AT_LOGIN_FIRST_LIKE_CPP: u16 = 0x020;

    if !host
        .resolved_represented_at_login_flags_like_cpp()
        .is_some_and(|flags| (flags & AT_LOGIN_FIRST_LIKE_CPP) != 0)
    {
        return false;
    }

    host.remove_represented_at_login_flag_like_cpp(AT_LOGIN_FIRST_LIKE_CPP, false)
}
pub fn promote_loaded_character_mount_spells_like_cpp<H: BattlePetSessionHostLikeCpp>(
    host: &mut H,
    spells: &[i32],
) -> usize {
    if host.hub_ref().catalogs.mount_store.is_none() {
        return 0;
    }

    let mut added = 0usize;
    for &spell_id in spells {
        added += usize::from({
            let (state, mut hub) = host.split_lifecycle_mut();
            state.add_account_mount_with_faction_counterpart_like_cpp(&mut hub, spell_id, 0)
        });
    }
    added
}
pub async fn load_account_mounts_like_cpp<H: SpellbookHostLikeCpp + BattlePetSessionHostLikeCpp>(
    host: &mut H,
) -> bool {
    set_account_mounts_like_cpp(host, Vec::new());
    let Some(port) = host
        .lifecycle()
        .player_lifecycle_port_like_cpp()
        .map(Arc::clone)
    else {
        return false;
    };

    let bnet_account_id = host.core().battlenet_account_id();
    if bnet_account_id == 0 {
        warn!(
            account = host.core().account_id,
            "Skipping account mount load because the game account is not linked to a Battle.net account"
        );
        return false;
    }
    let rows = match port
        .load_account_collection_like_cpp(AccountCollectionLoadRequestLikeCpp::Mounts {
            bnet_account_id,
        })
        .await
    {
        AccountCollectionLoadOutcomeLikeCpp::Loaded(AccountCollectionLoadedLikeCpp::Mounts(
            rows,
        )) => rows,
        AccountCollectionLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                account = host.core().account_id,
                bnet_account = bnet_account_id,
                "Failed to load account mounts: {reason}"
            );
            return false;
        }
        AccountCollectionLoadOutcomeLikeCpp::Loaded(_) => {
            warn!(
                account = host.core().account_id,
                bnet_account = bnet_account_id,
                "Player lifecycle port returned the wrong account collection for mounts"
            );
            return false;
        }
    };

    if rows.is_empty() {
        info!(
            account = host.core().account_id,
            bnet_account = bnet_account_id,
            "Loaded 0 account mounts from battlenet_account_mounts"
        );
        return true;
    }

    let mut mounts = Vec::new();
    let mut skipped_invalid_spell_id = 0usize;
    let mut skipped_missing_mount_db2 = 0usize;
    for row in rows {
        let spell_id = row.mount_spell_id;
        if spell_id <= 0 {
            skipped_invalid_spell_id += 1;
            continue;
        }

        let has_mount = spell_id > 0
            && host.hub_ref().catalogs.mount_store().is_none_or(|store| {
                store
                    .get_by_source_spell_id_like_cpp(spell_id as u32)
                    .is_some()
            });
        if has_mount {
            mounts.push(AccountMount {
                spell_id,
                flags: row.flags,
            });
        } else {
            skipped_missing_mount_db2 += 1;
        }
    }

    info!(
        account = host.core().account_id,
        bnet_account = bnet_account_id,
        loaded = mounts.len(),
        skipped_invalid_spell_id,
        skipped_missing_mount_db2,
        "Loaded represented account mounts like C++ CollectionMgr"
    );
    set_account_mounts_like_cpp(host, mounts.clone());
    true
}
pub fn set_account_mounts_like_cpp<H: SpellbookHostLikeCpp + BattlePetSessionHostLikeCpp>(
    host: &mut H,
    mounts: Vec<AccountMount>,
) {
    let mounts = mounts
        .into_iter()
        .map(|mount| (mount.spell_id, mount.flags))
        .collect();
    let _ = host
        .hub_mut()
        .mutate_player_collection_state_like_cpp(|collections| {
            collections.replace_mounts_like_cpp(mounts);
        });
    {
        let (state, mut hub) = host.split_lifecycle_mut();
        state.expand_account_mount_faction_definitions_like_cpp(&mut hub)
    };
    crate::learn_account_mount_spells_like_cpp(host);
}
