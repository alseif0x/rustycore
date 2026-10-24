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
//! Not represented here (no Rust owner yet; tracked in #43's follow-ups):
//! `RemovePet(nullptr, PET_SAVE_NOT_IN_SLOT, true)`, `SetRooted`,
//! `SetWaterWalking`, `StopMirrorTimers`, `PLAYER_LOCAL_FLAG_RELEASE_TIMER`,
//! the battleground/battlefield graveyard selectors and `OnPlayerRepop`.

use std::sync::Arc;

use tracing::warn;
use wow_core::{ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::{GraveyardLookupContextLikeCpp, GraveyardStore};
use wow_entities::{Corpse, CorpseCustomizationChoice, CorpseType};
use wow_packet::packets::misc::{CorpseReclaimDelay, DeathReleaseLoc, PreRessurect};
use wow_world_core::session::DeathPolicyLikeCpp;
use wow_world_core::session::connection_identity::unix_now;

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

fn current_map_instanceable_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> bool {
    current_map_entry_like_cpp(host).is_some_and(|entry| entry.is_instanceable_like_cpp())
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
fn calculate_corpse_reclaim_delay_ms_like_cpp<H: CorpseHostLikeCpp>(host: &H) -> u32 {
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
/// still in `JUST_DIED`: `setDeathState(CORPSE)`, clear dynamic flags, arm
/// the 6-minute death timer, `UpdateCorpseReclaimDelay`, and send the reclaim
/// delay. The fall, root, mirror timers and release-timer local flag have no
/// Rust owner yet.
pub fn kill_player_like_cpp<H: CorpseHostLikeCpp>(host: &mut H) {
    let policy = host.hub_ref().config.death_policy_like_cpp;
    let now = unix_now();
    let pvp = player_pvp_death_like_cpp(host);
    let changed = host.core().with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Corpse);
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .replace_all_dynamic_flags(0);
        let state = player.resurrection_state_mut_like_cpp();
        state.death_timer_active = true;
        state.death_expire_time =
            updated_death_expire_time_like_cpp(pvp, state.death_expire_time, now, policy);
    });
    if changed.is_none() {
        return;
    }
    host.sync_player_registry_state_like_cpp();

    // C++ `CalculateCorpseReclaimDelay()` is non-negative without `load`.
    send_corpse_reclaim_delay_like_cpp(host, calculate_corpse_reclaim_delay_ms_like_cpp(host));
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
    set_player_corpse_location_like_cpp(host, None);
    let Some(owner) = host.player_guid() else {
        return false;
    };
    let policy = host.hub_ref().config.death_policy_like_cpp;
    let bones_enabled =
        if current_map_entry_like_cpp(host).is_some_and(|entry| entry.is_battleground_or_arena()) {
            policy.bones_bg_or_arena
        } else {
            policy.bones_world
        };
    let now = unix_now();
    let converted = with_current_map_like_cpp(host, |map| {
        map.convert_corpse_to_bones_like_cpp(owner, bones_enabled, now)
    })
    .flatten();
    let Some(_converted) = converted else {
        return false;
    };

    // C++ `Map::ConvertCorpseToBones` commits `Corpse::DeleteFromDB` in its
    // own transaction.
    if let Some(port) = host.map_corpse_persistence_port_like_cpp() {
        if let wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Failed { reason } =
            port.delete_corpse_like_cpp(owner.counter() as u64).await
        {
            warn!(
                player = ?owner,
                "C++ Corpse::DeleteFromDB for bones conversion failed: {reason}"
            );
        }
    }

    if trigger_save && !host.player_logout_with_save_like_cpp() {
        host.save_player_after_corpse_bones_like_cpp(item_guid_generator)
            .await;
    }
    true
}

// ── CreateCorpse ────────────────────────────────────────────────────────────

/// The corpse built by C++ `Player::CreateCorpse` before `Map::AddCorpse`,
/// plus its `Corpse::SaveToDB` request when the map is not instanceable.
struct CreatedCorpseLikeCpp {
    corpse: Corpse,
    save: Option<wow_persistence::CorpseSaveRequestLikeCpp>,
}

fn build_corpse_from_player_like_cpp<H: CorpseHostLikeCpp>(
    host: &H,
    corpse_low_guid: i64,
) -> Option<CreatedCorpseLikeCpp> {
    let owner = host.player_guid()?;
    let corpse_type = if player_pvp_death_like_cpp(host) {
        CorpseType::ResurrectablePvp
    } else {
        CorpseType::ResurrectablePve
    };
    let in_battleground = host.hub_ref().player_in_represented_battleground_like_cpp();
    let in_arena = in_arena_like_cpp(host);
    let instanceable = current_map_instanceable_like_cpp(host);
    let hub = host.hub_ref();
    let catalogs = &hub.catalogs;

    host.core()
        .with_owned_player_like_cpp(|player| {
            let world = player.unit().world();
            let position = world.position();
            if !position.is_valid_map_coord_like_cpp() {
                return None;
            }
            let map_id = world.map_id();
            let instance_id = world.instance_id();
            let race = player.race_like_cpp();
            let faction_template = catalogs.faction_template_for_race_like_cpp(race)?;

            let mut corpse = Corpse::new_at(corpse_type, unix_now());
            corpse
                .world_mut()
                .object_mut()
                .create(ObjectGuid::create_world_object(
                    wow_core::guid::HighGuid::Corpse,
                    0,
                    owner.realm_id(),
                    map_id as u16,
                    0,
                    0,
                    corpse_low_guid,
                ));
            corpse.world_mut().set_map(map_id, instance_id).ok()?;
            corpse.world_mut().relocate(position);
            corpse.world_mut().object_mut().set_scale(1.0);
            corpse.set_owner_guid(owner);
            // C++ `PhasingHandler::InheritPhaseShift(this, owner)`.
            *corpse.world_mut().phase_shift_mut() = world.phase_shift().clone();

            let flags =
                corpse_flags_like_cpp(player.unit().data().pvp_flags, in_battleground, in_arena);
            corpse.set_race(race);
            corpse.set_sex(player.data().native_sex);
            corpse.set_class(player.class_like_cpp());
            corpse.set_customizations(
                player
                    .gameplay_state()
                    .customizations
                    .iter()
                    .map(|choice| CorpseCustomizationChoice {
                        option_id: choice.option_id,
                        choice_id: choice.choice_id,
                    })
                    .collect(),
            );
            corpse.replace_all_flags(flags);
            corpse
                .set_display_id(u32::try_from(player.unit().data().native_display_id).unwrap_or(0));
            corpse.set_faction_template(faction_template);

            for (slot, visible) in player.data().visible_items.iter().enumerate() {
                let Ok(item_id) = u32::try_from(visible.item_id) else {
                    continue;
                };
                if item_id == 0 {
                    continue;
                }
                let display_id = catalogs
                    .item_display_id(item_id, u32::from(visible.item_appearance_mod_id))
                    .unwrap_or(0);
                let inventory_type = catalogs
                    .items
                    .store
                    .as_ref()
                    .and_then(|store| store.get(item_id))
                    .map(|item| item.inventory_type as u8 as u32)
                    .unwrap_or(0);
                corpse.set_item(slot, display_id | (inventory_type << 24));
            }

            // C++ "we do not need to save corpses for instances".
            let save = (!instanceable).then(|| {
                let data = corpse.data();
                wow_persistence::CorpseSaveRequestLikeCpp {
                    owner_guid: owner.counter() as u64,
                    pos_x: position.x,
                    pos_y: position.y,
                    pos_z: position.z,
                    orientation: position.orientation,
                    map_id: map_id as u16,
                    display_id: data.display_id,
                    item_cache: corpse_item_cache_like_cpp(&data.items),
                    race: data.race_id,
                    class: data.class,
                    sex: data.sex,
                    flags: data.flags as u8,
                    dynamic_flags: data.dynamic_flags as u8,
                    ghost_time: corpse.ghost_time() as u32,
                    corpse_type: corpse.corpse_type() as u8,
                    instance_id,
                    phases: corpse
                        .world()
                        .phase_shift()
                        .phases_like_cpp()
                        .map(|phase| phase.id())
                        .collect(),
                    customizations: data
                        .customizations
                        .iter()
                        .map(|choice| (choice.option_id, choice.choice_id))
                        .collect(),
                }
            });

            Some(CreatedCorpseLikeCpp { corpse, save })
        })
        .flatten()
}

/// C++ `Player::CreateCorpse` followed by `BuildPlayerRepop`'s
/// `GetMap()->AddToMap(corpse)`. Returns the corpse GUID.
pub async fn create_corpse_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &ObjectGuidGenerator,
) -> Option<ObjectGuid> {
    // prevent the existence of 2 corpses for one player
    spawn_corpse_bones_like_cpp(host, item_guid_generator, true).await;

    let low_guid = with_current_map_like_cpp(host, |map| {
        map.generate_low_guid_like_cpp(wow_core::guid::HighGuid::Corpse)
            .ok()
    })
    .flatten()?;
    let created = build_corpse_from_player_like_cpp(host, low_guid)?;
    // C++ `SetPvPDeath(false)` after choosing the corpse type.
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player.set_pvp_death_like_cpp(false);
    });

    let corpse_guid = created.corpse.world().object().guid();
    let location = (
        created.corpse.world().map_id(),
        created.corpse.world().position(),
    );
    let added = with_current_map_like_cpp(host, |map| {
        map.add_corpse_to_map_like_cpp(created.corpse).is_ok()
    })
    .unwrap_or(false);
    if !added {
        return None;
    }
    // C++ `_corpseLocation.WorldRelocate(*this)`.
    set_player_corpse_location_like_cpp(host, Some(location));

    if let Some(save) = created.save
        && let Some(port) = host.map_corpse_persistence_port_like_cpp()
        && let wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Failed { reason } =
            port.save_corpse_like_cpp(save).await
    {
        warn!(corpse = ?corpse_guid, "C++ Corpse::SaveToDB failed: {reason}");
    }
    Some(corpse_guid)
}

// ── BuildPlayerRepop ────────────────────────────────────────────────────────

/// C++ `Player::BuildPlayerRepop`.
pub async fn build_player_repop_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: CorpseLifecycleCatalogsLikeCpp<'_>,
) {
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    // SMSG_PRE_RESSURECT is a realm-connection opcode (Opcodes.cpp:1915).
    host.core().send_packet_realm(&PreRessurect { player_guid });

    // If the player has the Wisp racial then cast the Wisp aura on them.
    if host.player_has_spell_like_cpp(SPELL_WISP_SPIRIT_RACIAL_LIKE_CPP) {
        host.cast_triggered_self_spell_like_cpp(catalogs, SPELL_WISP_SPIRIT_AURA_LIKE_CPP, None)
            .await;
    }
    host.cast_triggered_self_spell_like_cpp(catalogs, SPELL_GHOST_LIKE_CPP, None)
        .await;
    // C++ `AuraEffect::HandleAuraGhost` sets PLAYER_FLAGS_GHOST when 8326
    // applies; the represented aura has no effect handler, so the flag is
    // written here, at the same point of the release.
    crate::set_player_ghost_flag_like_cpp(host, true);

    host.remove_auras_with_interrupt_flags_like_cpp(SPELL_AURA_INTERRUPT_FLAG_RELEASE_LIKE_CPP, 0);

    // the player cannot have a corpse already on current map, only bones
    if player_corpse_location_like_cpp(host)
        .is_some_and(|(map_id, _)| map_id == u32::from(host.core().player_map_id_like_cpp()))
    {
        warn!(player = ?player_guid, "Player::BuildPlayerRepop: player already has a corpse");
        return;
    }

    if create_corpse_like_cpp(host, catalogs.item_guid_generator)
        .await
        .is_none()
    {
        warn!(player = ?player_guid, "Player::BuildPlayerRepop: error creating corpse");
        return;
    }

    // convert player body to ghost: setDeathState(DEAD), SetHealth(1)
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Dead);
        player.unit_mut().set_health(1);
        // BG - remove insignia related
        let flags = player.unit().unit_flags_like_cpp() - wow_constants::UnitFlags::SKINNABLE;
        player.unit_mut().set_unit_flags_like_cpp(flags);
    });
    host.sync_player_registry_state_like_cpp();
    host.core()
        .send_player_health_update_like_cpp(player_guid, 1);

    send_corpse_reclaim_delay_like_cpp(host, calculate_corpse_reclaim_delay_ms_like_cpp(host));

    // to prevent cheating: corpse->ResetGhostTime()
    let now = unix_now();
    let _ = with_current_map_like_cpp(host, |map| {
        if let Some(guid) = map.corpse_guid_by_player_like_cpp(player_guid)
            && let Some(corpse) = map.get_typed_corpse_mut(guid)
        {
            corpse.reset_ghost_time(now);
        }
    });
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
fn stop_repop_countdown_like_cpp<H: CorpseHostLikeCpp>(host: &mut H) {
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        player.resurrection_state_mut_like_cpp().death_timer_active = false;
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
        spawn_corpse_bones_like_cpp(host, catalogs.item_guid_generator, true).await;
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
        host.teleport_to_with_options_like_cpp(grave_map_id, grave_position, options)
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
            homebind.map_id,
            homebind.position,
            TELE_TO_NONE_LIKE_CPP,
        )
        .await;
    }
    // RemovePlayerFlag(PLAYER_FLAGS_IS_OUT_OF_BOUNDS): not represented.
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
        kill_player_like_cpp(host);
    }

    // RemovePet(nullptr, PET_SAVE_NOT_IN_SLOT, true): no Rust owner yet.
    build_player_repop_like_cpp(host, catalogs).await;
    repop_at_graveyard_like_cpp(host, catalogs).await;
}
