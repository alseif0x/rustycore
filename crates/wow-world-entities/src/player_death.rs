// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The Player death → release lifecycle (#43 slice 1).
//!
//! C++ source of truth (`Entities/Player/Player.cpp`): `Player::KillPlayer`
//! (`:4406`), `Player::BuildPlayerRepop` (`:4262`), `Player::CreateCorpse`
//! (`:4449`), `Player::SpawnCorpseBones` (`:4509`), `Player::RepopAtGraveyard`
//! (`:4743`), `Player::GetCorpseReclaimDelay`/`UpdateCorpseReclaimDelay`/
//! `CalculateCorpseReclaimDelay`/`SendCorpseReclaimDelay` (`:25001-25088`);
//! `ObjectMgr::GetClosestGraveyard` (`Globals/ObjectMgr.cpp:6843`).
//!
//! The canonical Player owns `_corpseLocation` and `m_deathExpireTime`
//! ([`wow_entities::PlayerResurrectionStateLikeCpp`]); the Map owns the corpse
//! object ([`wow_map::Map::add_corpse_to_map_like_cpp`]); the corpse rows go
//! through [`wow_persistence::MapCorpsePersistencePortLikeCpp`] after the Map
//! guard is released. No Map guard crosses an await.
//!
//! Native JUST_DIED cleanup belongs to Application's admitted death operation;
//! repop delegates RemovePet to `CorpseHostLikeCpp::remove_pet_for_repop_like_cpp`.
//! Wider controlled-unit/pet effects, ongoing mirror-timer progression and
//! drowning, battleground/battlefield graveyard selectors and `OnPlayerRepop`
//! remain follow-ups. The unconditional mirror-stop packets are represented.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use tracing::warn;
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::{GraveyardLookupContextLikeCpp, GraveyardStore};
use wow_entities::{Corpse, CorpseType};
use wow_packet::packets::misc::{CorpseReclaimDelay, DeathReleaseLoc};
use wow_world_core::session::DeathPolicyLikeCpp;
use wow_world_core::session::connection_identity::unix_now;

use crate::player_resurrection::PLAYER_FLAGS_IS_OUT_OF_BOUNDS;
use crate::{
    CorpseHostLikeCpp, CreatureSpawnCatalogsLikeCpp, TELE_REVIVE_AT_TELEPORT_LIKE_CPP,
    TELE_TO_NONE_LIKE_CPP,
};

/// C++ spell 8326 (`SPELL_AURA_GHOST`), cast by `BuildPlayerRepop`.
pub const SPELL_GHOST_LIKE_CPP: i32 = 8326;
/// C++ Wisp Spirit racial (`HasSpell(20585)`) and its ghost aura 20584.
pub const SPELL_WISP_SPIRIT_RACIAL_LIKE_CPP: i32 = 20585;
pub const SPELL_WISP_SPIRIT_AURA_LIKE_CPP: i32 = 20584;
/// C++ `CORPSE_RECLAIM_RADIUS` (`Corpse.h:38`).
pub const CORPSE_RECLAIM_RADIUS_LIKE_CPP: f32 = 39.0;
/// C++ `SpellAuraInterruptFlags::Release` (`SpellDefines.h`, bit 0x00080000).
pub const SPELL_AURA_INTERRUPT_FLAG_RELEASE_LIKE_CPP: u32 = 0x0008_0000;

/// C++ `DEATH_EXPIRE_STEP` (`Player.cpp:139`).
const DEATH_EXPIRE_STEP_SECS: i64 = 5 * 60;
/// C++ `MAX_DEATH_COUNT` (`Player.cpp:140`).
const MAX_DEATH_COUNT: i64 = 3;
/// C++ `copseReclaimDelay` (`Player.cpp:147`).
const CORPSE_RECLAIM_DELAY_SECS: [u32; MAX_DEATH_COUNT as usize] = [30, 60, 120];

/// C++ `PLAYER_EXTRA_PVP_DEATH` (`Player.h:522`).
const PLAYER_EXTRA_PVP_DEATH_LIKE_CPP: u32 = 0x0100;
/// C++ `CorpseFlags` (`Corpse.h:40-50`).
const CORPSE_FLAG_PVP_LIKE_CPP: u32 = 0x04;
const CORPSE_FLAG_SKINNABLE_LIKE_CPP: u32 = 0x20;
const CORPSE_FLAG_FFA_PVP_LIKE_CPP: u32 = 0x40;
/// C++ `AreaFlags::NoGhostOnRelease` (`DBCEnums.h:123`).
const AREA_FLAG_NO_GHOST_ON_RELEASE_LIKE_CPP: u32 = 0x0000_1000;
/// C++ `MAP_ARENA` instance type.
const MAP_ARENA_LIKE_CPP: i8 = 4;

/// The process catalogs the release path borrows: the triggered ghost casts
/// need the item-GUID generator and creature catalogs of the spell executor,
/// the graveyard search needs `ObjectMgr::GraveyardStore`.
#[derive(Clone, Copy)]
pub struct CorpseLifecycleCatalogsLikeCpp<'a> {
    pub item_guid_generator: &'a ObjectGuidGenerator,
    pub creature_spawns: &'a CreatureSpawnCatalogsLikeCpp,
    pub graveyards: &'a GraveyardStore,
}

/// Catalog inputs consumed by resurrection recovery and its triggered casts.
/// Graveyard data is borrowed only by recovery phases that perform the
/// post-save corpse or ghost graveyard search.
#[derive(Clone, Copy)]
pub struct ResurrectionOperationCatalogs<'a> {
    pub item_guid_generator: &'a ObjectGuidGenerator,
    pub creature_spawns: &'a CreatureSpawnCatalogsLikeCpp,
    pub graveyards: Option<&'a GraveyardStore>,
}

impl<'a> CorpseLifecycleCatalogsLikeCpp<'a> {
    pub fn resurrection(self) -> ResurrectionOperationCatalogs<'a> {
        ResurrectionOperationCatalogs {
            item_guid_generator: self.item_guid_generator,
            creature_spawns: self.creature_spawns,
            graveyards: Some(self.graveyards),
        }
    }
}

// ── Pure C++ rules ──────────────────────────────────────────────────────────

/// C++ `Player::GetCorpseReclaimDelay(pvp)` in seconds.
pub fn corpse_reclaim_delay_secs_like_cpp(
    pvp: bool,
    death_expire_time: i64,
    now: i64,
    policy: DeathPolicyLikeCpp,
) -> u32 {
    if pvp {
        if !policy.corpse_reclaim_delay_pvp {
            return CORPSE_RECLAIM_DELAY_SECS[0];
        }
    } else if !policy.corpse_reclaim_delay_pve {
        return 0;
    }

    // 0..2 full period; should be ceil(x)-1 but not floor(x).
    let count = if now < death_expire_time - 1 {
        (death_expire_time - 1 - now) / DEATH_EXPIRE_STEP_SECS
    } else {
        0
    };
    CORPSE_RECLAIM_DELAY_SECS[count.clamp(0, MAX_DEATH_COUNT - 1) as usize]
}

/// C++ `Player::UpdateCorpseReclaimDelay`: the new `m_deathExpireTime`.
pub fn updated_death_expire_time_like_cpp(
    pvp: bool,
    death_expire_time: i64,
    now: i64,
    policy: DeathPolicyLikeCpp,
) -> i64 {
    if (pvp && !policy.corpse_reclaim_delay_pvp) || (!pvp && !policy.corpse_reclaim_delay_pve) {
        return death_expire_time;
    }

    if now < death_expire_time {
        // full and partly periods 1..3
        let count = (death_expire_time - now) / DEATH_EXPIRE_STEP_SECS + 1;
        if count < MAX_DEATH_COUNT {
            now + (count + 1) * DEATH_EXPIRE_STEP_SECS
        } else {
            now + MAX_DEATH_COUNT * DEATH_EXPIRE_STEP_SECS
        }
    } else {
        now + DEATH_EXPIRE_STEP_SECS
    }
}

/// C++ `Player::CreateCorpse` corpse flags.
pub fn corpse_flags_like_cpp(pvp_flags: u8, in_battleground: bool, in_arena: bool) -> u32 {
    let mut flags = 0;
    if pvp_flags & wow_constants::unit::UnitPvpFlags::PVP.bits() != 0 {
        flags |= CORPSE_FLAG_PVP_LIKE_CPP;
    }
    if in_battleground && !in_arena {
        flags |= CORPSE_FLAG_SKINNABLE_LIKE_CPP;
    }
    if pvp_flags & wow_constants::unit::UnitPvpFlags::FFA_PVP.bits() != 0 {
        flags |= CORPSE_FLAG_FFA_PVP_LIKE_CPP;
    }
    flags
}

/// C++ `Corpse::SaveToDB` `itemCache`: every item followed by one space.
pub fn corpse_item_cache_like_cpp(items: &[u32]) -> String {
    let mut cache = String::with_capacity(items.len() * 4);
    for item in items {
        cache.push_str(&item.to_string());
        cache.push(' ');
    }
    cache
}

// ── Player-state reads ──────────────────────────────────────────────────────

fn player_pvp_death_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> bool {
    host.core()
        .with_owned_player_like_cpp(|player| {
            player.extra_flags() & PLAYER_EXTRA_PVP_DEATH_LIKE_CPP != 0
        })
        .unwrap_or(false)
}

pub(crate) fn player_death_expire_time_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> i64 {
    host.core()
        .with_owned_player_like_cpp(|player| player.resurrection_state_like_cpp().death_expire_time)
        .unwrap_or(0)
}

/// C++ `Player::GetCorpseLocation`.
pub fn player_corpse_location_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> Option<(u32, Position)> {
    host.core()
        .with_owned_player_like_cpp(|player| player.resurrection_state_like_cpp().corpse_location)
        .flatten()
}

fn set_player_corpse_location_like_cpp<H: CorpseHostLikeCpp>(
    host: &H,
    location: Option<(u32, Position)>,
) {
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player.resurrection_state_mut_like_cpp().corpse_location = location;
    });
}

fn current_map_entry_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> Option<wow_data::MapEntry> {
    let map_id = u32::from(host.core().player_map_id_like_cpp());
    host.hub_ref()
        .catalogs
        .maps
        .store
        .as_ref()
        .and_then(|store| store.get(map_id).copied())
}

fn in_arena_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> bool {
    host.hub_ref().player_in_represented_battleground_like_cpp()
        && current_map_entry_like_cpp(host)
            .is_some_and(|entry| entry.instance_type == MAP_ARENA_LIKE_CPP)
}

/// C++ `Player::InArena`.
pub fn player_in_arena_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> bool {
    in_arena_like_cpp(host)
}

/// C++ `Player::CalculateCorpseReclaimDelay(false)` in milliseconds: the
/// corpse type decides PvP when a corpse exists, else `PLAYER_EXTRA_PVP_DEATH`.
pub(crate) fn calculate_corpse_reclaim_delay_ms_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> u32 {
    let pvp = player_corpse_type_like_cpp(host)
        .map(|corpse_type| corpse_type == CorpseType::ResurrectablePvp)
        .unwrap_or_else(|| player_pvp_death_like_cpp(host));
    corpse_reclaim_delay_secs_like_cpp(
        pvp,
        player_death_expire_time_like_cpp(host),
        unix_now(),
        host.hub_ref().config.death_policy_like_cpp,
    ) * 1_000
}

pub(crate) fn with_current_map_like_cpp<H: CorpseHostLikeCpp, R>(
    host: &H,
    f: impl FnOnce(&mut wow_map::Map<wow_map::NoopTerrainGridLoader, wow_map::NoopGridLifecycle>) -> R,
) -> Option<R> {
    let key = host.core().current_canonical_player_map_key_like_cpp()?;
    let manager = Arc::clone(host.core().canonical_map_manager.as_ref()?);
    let mut manager = manager.lock().ok()?;
    let managed = manager.find_map_mut(key.map_id, key.instance_id)?;
    Some(f(managed.map_mut()))
}

/// C++ `Player::GetCorpse()->GetType()` on the Player's current map.
fn player_corpse_type_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> Option<CorpseType> {
    let owner = host.player_guid()?;
    with_current_map_like_cpp(host, |map| {
        let guid = map.corpse_guid_by_player_like_cpp(owner)?;
        map.get_typed_corpse(guid).map(Corpse::corpse_type)
    })
    .flatten()
}

/// C++ `Player::GetCorpse()`: the corpse GUID, ghost time and type.
pub fn player_corpse_like_cpp<H: CorpseHostLikeCpp>(
    host: &H,
) -> Option<(ObjectGuid, i64, CorpseType)> {
    let owner = host.player_guid()?;
    with_current_map_like_cpp(host, |map| {
        let guid = map.corpse_guid_by_player_like_cpp(owner)?;
        map.get_typed_corpse(guid)
            .map(|corpse| (guid, corpse.ghost_time(), corpse.corpse_type()))
    })
    .flatten()
}

/// C++ `Player::SendCorpseReclaimDelay`.
fn send_corpse_reclaim_delay_like_cpp<H: CorpseHostLikeCpp>(host: &H, delay_ms: u32) {
    host.send_packet(&CorpseReclaimDelay {
        remaining: delay_ms,
    });
}

// ── KillPlayer ──────────────────────────────────────────────────────────────

/// C++ `Player::KillPlayer` as `HandleRepopRequest` calls it for a Player
/// still in `JUST_DIED`: `setDeathState(CORPSE)`, clear dynamic flags,
/// `UpdateCorpseReclaimDelay`, and send the reclaim delay. The Rust death
/// bookkeeping is updated here; rooting is published here too. The six-minute
/// `m_deathTimer` countdown advances separately in PlayerUpdate, after SaveTimer.
/// Forced visibility follows the reclaim-delay packet; falling and ongoing
/// mirror-timer progression remain outside this operation.
pub async fn kill_player_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    creature_spawns: &CreatureSpawnCatalogsLikeCpp,
) -> bool {
    let Some(owner) = crate::recovery_fences::owner(host) else {
        return false;
    };
    let policy = host.hub_ref().config.death_policy_like_cpp;
    let now = unix_now();
    let pvp = player_pvp_death_like_cpp(host);
    // C++ `Player::KillPlayer` roots before changing JUST_DIED to CORPSE.
    host.hub_mut().set_player_rooted_like_cpp(true);
    stop_mirror_timers_like_cpp(host);
    let changed = host.core().with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Corpse);
    });
    if changed.is_none()
        || host.finish_player_non_alive_state_like_cpp().is_err()
        || !crate::recovery_fences::owner_is_current(host, owner)
    {
        return false;
    }
    let changed = host.core().with_owned_player_mut_like_cpp(|player| {
        let object = player.unit_mut().world_mut().object_mut();
        object.replace_all_dynamic_flags(0);
        object.force_dynamic_flags_update_like_cpp();
    });
    if changed.is_none() {
        return false;
    }

    // C++ `KillPlayer` Set/RemovePlayerLocalFlag(PLAYER_LOCAL_FLAG_RELEASE_TIMER)
    // after clearing dynamic flags and before arming m_deathTimer (`Player.cpp:4417`).
    let release_timer = current_map_entry_like_cpp(host)
        .is_some_and(|entry| !entry.is_instanceable_like_cpp())
        && !host
            .hub_ref()
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_PREVENT_RESURRECTION,
            )
            .is_some_and(|effects| !effects.is_empty());
    let changed = host.core().with_owned_player_mut_like_cpp(|player| {
        if release_timer {
            player.set_player_local_flags_like_cpp(crate::PLAYER_LOCAL_FLAG_RELEASE_TIMER);
        } else {
            player.remove_player_local_flags_like_cpp(crate::PLAYER_LOCAL_FLAG_RELEASE_TIMER);
        }
    });
    if changed.is_none() {
        return false;
    }

    let changed = host.core().with_owned_player_mut_like_cpp(|player| {
        let state = player.resurrection_state_mut_like_cpp();
        state.death_timer_remaining_ms = 360_000;
        state.death_timer_deadline = Some(Instant::now() + Duration::from_millis(360_000));
        state.death_expire_time =
            updated_death_expire_time_like_cpp(pvp, state.death_expire_time, now, policy);
    });
    if changed.is_none() {
        return false;
    }
    host.sync_player_registry_state_like_cpp();

    // C++ `CalculateCorpseReclaimDelay()` is non-negative without `load`.
    send_corpse_reclaim_delay_like_cpp(host, calculate_corpse_reclaim_delay_ms_like_cpp(host));

    // Player.cpp:4437 forces visibility after the reclaim-delay phase. The
    // owner/residence fence is rechecked after the World visibility await.
    if current_player_is_in_world_like_cpp(host) {
        if !host
            .force_player_visibility_with_catalogs_like_cpp(creature_spawns)
            .await
        {
            return false;
        }
        return crate::recovery_fences::owner_is_current_in_residence(host, owner);
    }

    crate::recovery_fences::owner_is_current(host, owner)
}

pub(crate) fn stop_mirror_timers_like_cpp<H: CorpseHostLikeCpp>(host: &H) {
    // Player.cpp:609,711 sends every type unconditionally, even without an active bar.
    for timer in 0..3 {
        host.core()
            .send_packet_realm(&wow_packet::packets::misc::StopMirrorTimer { timer });
    }
}

pub(crate) fn current_player_is_in_world_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> bool {
    let Some(handle) = host.core().player_handle_like_cpp else {
        return false;
    };
    host.player_guid() == Some(handle.guid())
        && host
            .core()
            .with_owned_player_like_cpp(|player| {
                player.guid() == handle.guid() && player.unit().world().object().is_in_world()
            })
            .unwrap_or(false)
}

// ── SpawnCorpseBones ────────────────────────────────────────────────────────

/// C++ `Player::SpawnCorpseBones(triggerSave)`: forget the corpse location,
/// convert the corpse on the current map to bones, delete its rows and, when
/// a corpse was converted and the Player is not logging out with a save,
/// save the Player so it never loads as a ghost without a corpse.
pub async fn spawn_corpse_bones_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &ObjectGuidGenerator,
    trigger_save: bool,
) -> bool {
    match convert_corpse_to_bones_for_resurrection(host).await {
        CorpseBonesConversion::Unavailable => false,
        CorpseBonesConversion::NoCorpse => true,
        CorpseBonesConversion::Converted => {
            if trigger_save && !host.player_logout_with_save_like_cpp() {
                return host
                    .save_player_after_corpse_bones_like_cpp(item_guid_generator)
                    .await
                    == crate::ResurrectionSaveOutcome::Applied;
            }
            true
        }
    }
}

pub(crate) enum CorpseBonesConversion {
    Unavailable,
    NoCorpse,
    Converted,
}

pub(crate) async fn convert_corpse_to_bones_for_resurrection<H: CorpseHostLikeCpp>(
    host: &mut H,
) -> CorpseBonesConversion {
    set_player_corpse_location_like_cpp(host, None);
    let Some(binding) = crate::recovery_fences::owner(host) else {
        return CorpseBonesConversion::Unavailable;
    };
    if !crate::recovery_fences::owner_is_current_in_residence(host, binding) {
        return CorpseBonesConversion::Unavailable;
    }
    let owner = binding.guid;
    let Some(continuation) = host.core().with_owned_player_like_cpp(|player| {
        player.resurrection_state_like_cpp().delayed_after_teleport
    }) else {
        return CorpseBonesConversion::Unavailable;
    };
    let policy = host.hub_ref().config.death_policy_like_cpp;
    let bones_enabled =
        if current_map_entry_like_cpp(host).is_some_and(|entry| entry.is_battleground_or_arena()) {
            policy.bones_bg_or_arena
        } else {
            policy.bones_world
        };
    let now = unix_now();
    let Some(manager) = host.core().canonical_map_manager.as_ref().cloned() else {
        return CorpseBonesConversion::Unavailable;
    };
    let prepared = {
        let Ok(mut manager) = manager.lock() else {
            return CorpseBonesConversion::Unavailable;
        };
        let Some((key, revision)) =
            manager.player_active_residence_revision_like_cpp(binding.handle)
        else {
            return CorpseBonesConversion::Unavailable;
        };
        if binding.residence != Some((key, revision))
            || !manager
                .with_player_like_cpp(binding.handle, |player| {
                    let world = player.unit().world();
                    player.guid() == owner
                        && world.object().is_in_world()
                        && world.has_current_map()
                        && wow_map::MapKey::new(world.map_id(), world.instance_id()) == key
                        && player.resurrection_state_like_cpp().delayed_after_teleport
                            == continuation
                })
                .unwrap_or(false)
        {
            return CorpseBonesConversion::Unavailable;
        }
        let Some(map) = manager.find_map_mut(key.map_id, key.instance_id) else {
            return CorpseBonesConversion::Unavailable;
        };
        let map = map.map_mut();
        let Some(converted) = map.convert_corpse_to_bones_like_cpp(owner, bones_enabled, now)
        else {
            return CorpseBonesConversion::NoCorpse;
        };
        let destroyed = map.take_object_visibility_destroy_for_guid_like_cpp(converted.corpse_guid);
        crate::visibility_session::prepare_corpse_visibility_like_cpp(
            &manager,
            binding.handle,
            converted.bones_guid,
            destroyed,
            crate::visibility_session::CorpseBirthKind::Bones,
        )
    };
    // C++ enqueues the delete transaction (DBPool.cpp:302-325), then Map::AddToMap
    // immediately publishes bones. Native visibility is not the durable receipt.
    let published = crate::recovery_fences::owner_is_current_in_residence(host, binding)
        && prepared.as_ref().is_some_and(|prepared| {
            crate::visibility_session::publish_corpse_visibility_like_cpp(host, prepared)
        });

    // Preserve the existing Rust completion fence even when publication failed:
    // the committed native conversion still requires its delete attempt.
    let Some(port) = host.map_corpse_persistence_port_like_cpp() else {
        return CorpseBonesConversion::Unavailable;
    };
    if let wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Failed { reason } =
        port.delete_corpse_like_cpp(owner.counter() as u64).await
    {
        warn!(
            player = ?owner,
            "C++ Corpse::DeleteFromDB for bones conversion was not confirmed: {reason}"
        );
        // The port cannot distinguish a returned error before COMMIT from
        // an error after COMMIT. Neither outcome authorizes completion.
        return CorpseBonesConversion::Unavailable;
    }
    if !published
        || !crate::recovery_fences::owner_is_current_in_residence(host, binding)
        || host.core().with_owned_player_like_cpp(|player| {
            player.resurrection_state_like_cpp().delayed_after_teleport
        }) != Some(continuation)
    {
        return CorpseBonesConversion::Unavailable;
    }
    CorpseBonesConversion::Converted
}

// ── CreateCorpse ────────────────────────────────────────────────────────────

// ── BuildPlayerRepop ────────────────────────────────────────────────────────

/// C++ `Player::BuildPlayerRepop`.
pub async fn build_player_repop_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: CorpseLifecycleCatalogsLikeCpp<'_>,
) -> bool {
    crate::player_release::build_player_repop_like_cpp(host, catalogs).await
}

/// C++ `Player::Update` death-timer phase (`Player.cpp:1101`): only a running
/// timer on a known non-instanceable map without `SPELL_AURA_PREVENT_RESURRECTION`
/// advances. At expiry, build the corpse before the graveyard repop.
pub async fn tick_player_death_timer<H: CorpseHostLikeCpp>(
    host: &mut H,
    diff_ms: u32,
    catalogs: CorpseLifecycleCatalogsLikeCpp<'_>,
) {
    let Some((remaining_ms, death_timer_deadline)) =
        host.core().with_owned_player_like_cpp(|player| {
            let state = player.resurrection_state_like_cpp();
            (state.death_timer_remaining_ms, state.death_timer_deadline)
        })
    else {
        return;
    };
    let now = Instant::now();
    let (remaining_after_tick, expired) = if let Some(deadline) = death_timer_deadline {
        let expired = now >= deadline;
        let remaining_after_tick = if expired {
            0
        } else {
            u32::try_from(deadline.saturating_duration_since(now).as_millis())
                .unwrap_or(u32::MAX)
                .max(1)
        };
        (remaining_after_tick, expired)
    } else {
        // Loaded and fixture states without a deadline retain native diff accounting.
        let expired = diff_ms >= remaining_ms;
        let remaining_after_tick = if expired { 0 } else { remaining_ms - diff_ms };
        (remaining_after_tick, expired)
    };
    if std::env::var_os("RUSTYCORE_DEATH_TIMER_TRACE").is_some() && remaining_ms > 0 {
        let player_guid = host.player_guid();
        let delayed_after_teleport = host
            .core()
            .with_owned_player_like_cpp(|player| {
                player.resurrection_state_like_cpp().delayed_after_teleport
            })
            .flatten();
        let current_map_instanceable =
            current_map_entry_like_cpp(host).map(|entry| entry.is_instanceable_like_cpp());
        let prevent_resurrection_effect_count = host
            .hub_ref()
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_PREVENT_RESURRECTION,
            )
            .map(|effects| effects.len());
        let outcome = if remaining_ms == 0 {
            "gate_refused_remaining_zero"
        } else if current_map_instanceable != Some(false) {
            "gate_refused_map"
        } else if prevent_resurrection_effect_count.is_some_and(|count| count != 0) {
            "gate_refused_prevent_resurrection"
        } else if expired {
            "expiry_entering_existing_repop"
        } else {
            "timer_advanced"
        };
        tracing::info!(
            player_guid = ?player_guid,
            remaining_ms,
            remaining_after_tick,
            diff_ms,
            death_timer_deadline = ?death_timer_deadline,
            clock = if death_timer_deadline.is_some() { "monotonic" } else { "diff" },
            current_map_instanceable = ?current_map_instanceable,
            prevent_resurrection_effect_count = ?prevent_resurrection_effect_count,
            delayed_operation = ?delayed_after_teleport.map(|continuation| continuation.operation),
            delayed_phase = ?delayed_after_teleport.map(|continuation| continuation.phase),
            outcome,
            "RUST_DEATH_TIMER tick"
        );
    }
    if remaining_ms == 0
        || !current_map_entry_like_cpp(host).is_some_and(|entry| !entry.is_instanceable_like_cpp())
        || host
            .hub_ref()
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_PREVENT_RESURRECTION,
            )
            .is_some_and(|effects| !effects.is_empty())
    {
        return;
    }

    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        let state = player.resurrection_state_mut_like_cpp();
        state.death_timer_remaining_ms = remaining_after_tick;
        if expired {
            state.death_timer_deadline = None;
        }
    });

    if expired {
        if build_player_repop_like_cpp(host, catalogs).await {
            repop_at_graveyard_like_cpp(host, catalogs).await;
        }
    }
}

// ── RepopAtGraveyard / GetClosestGraveyard ──────────────────────────────────

/// C++ `TerrainMgr::GetZoneId` for an arbitrary location, with the same
/// grid-area lookup the login and movement paths use.
fn terrain_zone_id_like_cpp<H: CorpseHostLikeCpp>(
    host: &H,
    map_id: u32,
    position: Position,
) -> u32 {
    terrain_zone_and_area_like_cpp(host, map_id, position).0
}

/// C++ `GetZoneAndAreaId` for an arbitrary location (`(0, 0)` without terrain).
pub(crate) fn terrain_zone_and_area_like_cpp<H: CorpseHostLikeCpp>(
    host: &H,
    map_id: u32,
    position: Position,
) -> (u32, u32) {
    let hub = host.hub_ref();
    wow_world_core::terrain::zone_and_area_for_position_like_cpp(
        &hub.config.mmap_runtime_config_like_cpp().data_dir,
        map_id,
        position.x,
        position.y,
        hub.catalogs.area_table_store().map(|store| store.as_ref()),
        |map_id| {
            hub.catalogs
                .map_store()
                .as_deref()
                .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
                .unwrap_or(0)
        },
    )
    .unwrap_or((0, 0))
}

/// C++ `ObjectMgr::GetClosestGraveyard(location, team, player)` with the
/// Player as `conditionObject`. Returns the WorldSafeLocs id.
pub fn closest_graveyard_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    graveyards: &GraveyardStore,
    map_id: u32,
    position: Position,
) -> Option<u32> {
    let zone_id = terrain_zone_id_like_cpp(host, map_id, position);
    let team =
        wow_world_core::session::player_team_for_race_cpp(host.hub_ref().player_race_like_cpp())
            as u32;
    let (map_entry, corpse_map_id) = {
        let hub = host.hub_ref();
        let store = hub.catalogs.maps.store.as_ref();
        (
            store.and_then(|store| store.get(map_id).copied()),
            store
                .map(|store| store.corpse_map_id_like_cpp(map_id))
                .unwrap_or(-1),
        )
    };
    let context = GraveyardLookupContextLikeCpp {
        map_id,
        x: position.x,
        y: position.y,
        z: position.z,
        team,
        parent_map_id: map_entry.and_then(|entry| u32::try_from(entry.parent_map_id).ok()),
        corpse_map_id: u32::try_from(corpse_map_id).ok(),
        is_battleground_or_arena: map_entry.is_some_and(|entry| entry.is_battleground_or_arena()),
    };
    let Some(world_safe_locs) = host
        .hub_ref()
        .catalogs
        .world_safe_loc_store_like_cpp
        .clone()
    else {
        return None;
    };
    let area_store = host.hub_ref().catalogs.area_table_store().cloned();
    let visible_map_ids = host
        .core()
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .world()
                .phase_shift()
                .visible_map_ids_like_cpp()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    graveyards.closest_graveyard_like_cpp(
        zone_id,
        context,
        |area_id| {
            area_store
                .as_ref()
                .and_then(|store| store.get(area_id))
                .map(|area| u32::from(area.parent_area_id))
        },
        |store, zone_id| {
            store.closest_graveyard_in_zone_with_condition_object_like_cpp(
                zone_id,
                context,
                &world_safe_locs,
                |conditions| host.graveyard_conditions_meet_like_cpp(conditions),
                |map_id| visible_map_ids.contains(&map_id),
            )
        },
    )
}

/// C++ `m_deathTimer = 0` ("stop countdown until repop"); the World test
/// build also counts each `RepopAtGraveyard`.
pub(crate) fn stop_repop_countdown_like_cpp<H: CorpseHostLikeCpp>(host: &mut H) {
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        let state = player.resurrection_state_mut_like_cpp();
        state.death_timer_remaining_ms = 0;
        state.death_timer_deadline = None;
    });

    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures.combat.represented_repop_at_graveyard_count = fixtures
            .combat
            .represented_repop_at_graveyard_count
            .saturating_add(1);
    }
}

/// C++ `Player::RepopAtGraveyard`.
pub async fn repop_at_graveyard_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: CorpseLifecycleCatalogsLikeCpp<'_>,
) {
    let Some(position) = host.hub_ref().player_position_like_cpp() else {
        // No resolvable position: no graveyard can be searched, but C++ still
        // stops the repop countdown.
        stop_repop_countdown_like_cpp(host);
        return;
    };
    let map_id = u32::from(host.core().player_map_id_like_cpp());
    let alive = host.hub_ref().resolved_player_is_alive_like_cpp() == Some(true);
    let no_ghost_zone = host
        .hub_ref()
        .player_zone_area_like_cpp()
        .and_then(|(_, area_id)| {
            host.hub_ref()
                .catalogs
                .area_table_store()
                .and_then(|store| store.get(area_id).copied())
        })
        .is_some_and(|area| area.flags & AREA_FLAG_NO_GHOST_ON_RELEASE_LIKE_CPP != 0);
    let map_entry = current_map_entry_like_cpp(host);
    let below_min_height = position.z < host.hub_ref().player_min_height_like_cpp(position);

    // Such zones are considered unreachable as a ghost and the player must be
    // automatically revived.
    let should_resurrect = (!alive && no_ghost_zone)
        || map_entry.is_some_and(|entry| {
            entry.is_non_raid_dungeon_like_cpp() || entry.instance_type == wow_data::map::MAP_RAID
        })
        || host.hub_ref().player_on_transport_state_like_cpp() == Some(true)
        || below_min_height;
    if should_resurrect {
        if !crate::recovery_fences::begin(
            host,
            wow_entities::PlayerResurrectionOperation::ForcedRepop,
            wow_entities::PlayerResurrectionPhase::RecoveryBonesReady,
        ) {
            return;
        }
        match crate::recovery_fences::continue_operation(host, catalogs.resurrection()).await {
            crate::recovery_fences::Progress::Complete
            | crate::recovery_fences::Progress::Waiting => {}
            crate::recovery_fences::Progress::Continue | crate::recovery_fences::Progress::Held => {
                host.core_mut()
                    .kick("forced graveyard recovery did not complete; retain Player owner");
            }
        }
        return;
    }

    // Battleground and battlefield graveyard selectors are not represented;
    // C++ uses `ObjectMgr::GetClosestGraveyard` outside them.
    let closest = closest_graveyard_like_cpp(host, catalogs.graveyards, map_id, position);

    stop_repop_countdown_like_cpp(host);

    let grave = closest.and_then(|safe_loc_id| {
        host.hub_ref()
            .catalogs
            .world_safe_loc_store_like_cpp
            .as_ref()
            .and_then(|store| store.get(safe_loc_id))
            .map(|entry| (entry.map_id, entry.position))
    });
    if let Some((grave_map_id, grave_position)) = grave {
        let options = if should_resurrect {
            TELE_REVIVE_AT_TELEPORT_LIKE_CPP
        } else {
            TELE_TO_NONE_LIKE_CPP
        };
        host.teleport_to_with_options_like_cpp(
            catalogs.resurrection(),
            grave_map_id,
            grave_position,
            options,
        )
        .await;
        // not send if alive, because it used in TeleportTo()
        if host.hub_ref().resolved_player_is_alive_like_cpp() == Some(false) {
            // SMSG_DEATH_RELEASE_LOC is a realm-connection opcode (Opcodes.cpp:1333).
            host.core().send_packet_realm(&DeathReleaseLoc {
                map_id: grave_map_id as i32,
                x: grave_position.x,
                y: grave_position.y,
                z: grave_position.z,
            });
        }
    } else if below_min_height
        && let Some(homebind) = host
            .core()
            .with_owned_player_like_cpp(|player| player.gameplay_state().homebind)
            .flatten()
    {
        host.teleport_to_with_options_like_cpp(
            catalogs.resurrection(),
            homebind.map_id,
            homebind.position,
            TELE_TO_NONE_LIKE_CPP,
        )
        .await;
    }
    // C++ `Player::RepopAtGraveyard` clears this after teleport/homebind
    // handling (`Player.cpp:4790`). Re-resolve the canonical Player after the
    // awaited teleport rather than retaining an owner across it.
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player.remove_player_flags_like_cpp(PLAYER_FLAGS_IS_OUT_OF_BOUNDS);
    });
}

// ── HandleRepopRequest ──────────────────────────────────────────────────────

/// C++ `WorldSession::HandleRepopRequest` (`MiscHandler.cpp:59-83`) after
/// the packet is parsed.
pub async fn repop_request_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: CorpseLifecycleCatalogsLikeCpp<'_>,
) {
    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(false)
        || host.hub_ref().player_has_ghost_flag_like_cpp()
    {
        return;
    }

    // silently return, client should display the error by itself
    if host
        .hub_ref()
        .resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_PREVENT_RESURRECTION,
        )
        .is_some_and(|effects| !effects.is_empty())
    {
        return;
    }

    // the world update order is sessions, players, creatures; a creature can
    // kill the player after the session update but before the player update.
    let just_died = host
        .core()
        .with_owned_player_like_cpp(|player| {
            player.unit().death_state() == wow_constants::DeathState::JustDied
        })
        .unwrap_or(false);
    if just_died {
        if !kill_player_like_cpp(host, catalogs.creature_spawns).await {
            return;
        }
    }

    if let Err(reason) = host.remove_pet_for_repop_like_cpp(catalogs).await {
        warn!(
            account = host.core().account_id,
            reason, "release pet removal failed"
        );
        return;
    }
    if build_player_repop_like_cpp(host, catalogs).await {
        repop_at_graveyard_like_cpp(host, catalogs).await;
    }
}
