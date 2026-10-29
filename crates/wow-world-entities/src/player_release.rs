// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Fenced Player release and corpse creation across aura and SQL awaits.

use std::sync::Arc;

use tracing::warn;
use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_data::MapEntry;
use wow_entities::{
    Corpse, CorpseCustomizationChoice, CorpseType, Player, PlayerResurrectionContinuation,
    PlayerResurrectionOperation, PlayerResurrectionPhase,
};
use wow_map::{MapKey, PlayerHandle, PlayerResidenceLikeCpp};
use wow_packet::packets::misc::PreRessurect;
use wow_world_core::session::connection_identity::unix_now;

use crate::player_death::{
    CorpseBonesConversion, SPELL_AURA_INTERRUPT_FLAG_RELEASE_LIKE_CPP, SPELL_GHOST_LIKE_CPP,
    SPELL_WISP_SPIRIT_AURA_LIKE_CPP, SPELL_WISP_SPIRIT_RACIAL_LIKE_CPP, corpse_flags_like_cpp,
    corpse_item_cache_like_cpp, with_current_map_like_cpp,
};
use crate::{CorpseHostLikeCpp, ResurrectionSaveOutcome};

const PLAYER_EXTRA_PVP_DEATH: u32 = 0x0100;
const UNIT_FLAG_SKINNABLE: wow_constants::UnitFlags = wow_constants::UnitFlags::SKINNABLE;

#[derive(Clone, Copy)]
enum ReleaseOperation {
    Repop,
    CreateCorpse,
}

impl ReleaseOperation {
    fn state(self) -> PlayerResurrectionOperation {
        match self {
            Self::Repop => PlayerResurrectionOperation::Repop,
            Self::CreateCorpse => PlayerResurrectionOperation::CreateCorpse,
        }
    }

    fn matches(self, operation: PlayerResurrectionOperation) -> bool {
        matches!(
            (self, operation),
            (Self::Repop, PlayerResurrectionOperation::Repop)
                | (
                    Self::CreateCorpse,
                    PlayerResurrectionOperation::CreateCorpse
                )
        )
    }
}

#[derive(Clone, Copy)]
struct OwnerBinding {
    handle: PlayerHandle,
    revision: u64,
    residence: PlayerResidenceLikeCpp,
}

impl OwnerBinding {
    fn active_key(self) -> Option<MapKey> {
        match self.residence {
            PlayerResidenceLikeCpp::Active(key) => Some(key),
            PlayerResidenceLikeCpp::Detached => None,
        }
    }

    fn matches(
        self,
        core: &wow_world_core::session::SessionCore,
        manager: &wow_map::MapManager,
    ) -> bool {
        if core.player_guid() != Some(self.handle.guid())
            || core.player_handle_like_cpp != Some(self.handle)
        {
            return false;
        }
        manager.current_player_admission_like_cpp(self.handle.guid())
            == Some((self.handle, self.revision))
            && manager.player_residence_like_cpp(self.handle) == Some(self.residence)
    }
}

struct CreatedCorpse {
    corpse: Corpse,
    save: Option<wow_persistence::CorpseSaveRequestLikeCpp>,
}

/// C++ `Player::CreateCorpse`, guarded by the same Player-owned continuation
/// used by BuildPlayerRepop. This remains useful for lifecycle setup callers.
pub async fn create_corpse_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &ObjectGuidGenerator,
) -> Option<ObjectGuid> {
    let binding = begin(
        host,
        ReleaseOperation::CreateCorpse,
        PlayerResurrectionPhase::CorpseBonesInFlight,
    )?;
    create_and_publish(
        host,
        binding,
        ReleaseOperation::CreateCorpse,
        item_guid_generator,
        None,
    )
    .await
}

/// C++ `Player::BuildPlayerRepop`, returning true only after the corpse and the
/// complete canonical ghost projection are committed and published.
pub async fn build_player_repop_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: crate::CorpseLifecycleCatalogsLikeCpp<'_>,
) -> bool {
    let Some(binding) = begin(
        host,
        ReleaseOperation::Repop,
        PlayerResurrectionPhase::ReleaseWispAuraInFlight,
    ) else {
        return false;
    };
    let player_guid = binding.handle.guid();

    host.core().send_packet_realm(&PreRessurect { player_guid });

    if host.player_has_spell_like_cpp(SPELL_WISP_SPIRIT_RACIAL_LIKE_CPP) {
        if !host
            .cast_triggered_self_spell_like_cpp(
                catalogs.resurrection(),
                SPELL_WISP_SPIRIT_AURA_LIKE_CPP,
                None,
                None,
            )
            .await
        {
            return failed(host, player_guid, "Wisp ghost aura cast was not confirmed");
        }
        if !advance(
            host,
            binding,
            ReleaseOperation::Repop,
            PlayerResurrectionPhase::ReleaseWispAuraInFlight,
            PlayerResurrectionPhase::ReleaseGhostAuraInFlight,
        ) {
            return failed(
                host,
                player_guid,
                "Player owner changed after Wisp aura cast",
            );
        }
    } else if !advance(
        host,
        binding,
        ReleaseOperation::Repop,
        PlayerResurrectionPhase::ReleaseWispAuraInFlight,
        PlayerResurrectionPhase::ReleaseGhostAuraInFlight,
    ) {
        return failed(
            host,
            player_guid,
            "Player owner changed before ghost aura cast",
        );
    }

    if !host
        .cast_triggered_self_spell_like_cpp(
            catalogs.resurrection(),
            SPELL_GHOST_LIKE_CPP,
            None,
            None,
        )
        .await
    {
        return failed(
            host,
            player_guid,
            "required ghost aura cast was not confirmed",
        );
    }
    if !advance(
        host,
        binding,
        ReleaseOperation::Repop,
        PlayerResurrectionPhase::ReleaseGhostAuraInFlight,
        PlayerResurrectionPhase::CorpseBonesInFlight,
    ) {
        return failed(
            host,
            player_guid,
            "Player owner changed after ghost aura cast",
        );
    }
    host.remove_auras_with_interrupt_flags_like_cpp(SPELL_AURA_INTERRUPT_FLAG_RELEASE_LIKE_CPP, 0);

    let corpse_location = with_bound_player(host, binding, |player| {
        player.resurrection_state_like_cpp().corpse_location
    });
    let Some(corpse_location) = corpse_location else {
        return failed(
            host,
            player_guid,
            "canonical Player disappeared before corpse check",
        );
    };
    if corpse_location
        .is_some_and(|(map_id, _)| Some(map_id) == binding.active_key().map(|key| key.map_id))
    {
        return failed(
            host,
            player_guid,
            "Player already has a corpse on the current map",
        );
    }

    create_and_publish(
        host,
        binding,
        ReleaseOperation::Repop,
        catalogs.item_guid_generator,
        Some(catalogs.creature_spawns),
    )
    .await
    .is_some()
}

async fn create_and_publish<H: CorpseHostLikeCpp>(
    host: &mut H,
    binding: OwnerBinding,
    operation: ReleaseOperation,
    item_guid_generator: &ObjectGuidGenerator,
    creature_spawns: Option<&crate::CreatureSpawnCatalogsLikeCpp>,
) -> Option<ObjectGuid> {
    let ghost_projection = creature_spawns.is_some();
    let player_guid = binding.handle.guid();
    let conversion = crate::player_death::convert_corpse_to_bones_for_resurrection(host).await;
    if !owner_is_current(host, binding) {
        failed(
            host,
            player_guid,
            "Player residence changed during old-corpse conversion",
        );
        return None;
    }
    match conversion {
        CorpseBonesConversion::Unavailable => {
            failed(host, player_guid, "old-corpse conversion was not confirmed");
            return None;
        }
        CorpseBonesConversion::NoCorpse => {}
        CorpseBonesConversion::Converted if ghost_projection => {
            // BuildPlayerRepop rejects a same-map corpse before CreateCorpse.
            // A converted corpse here is inconsistent state: saving it now
            // would snapshot an incomplete ghost, so retain the release fence.
            failed(
                host,
                player_guid,
                "BuildPlayerRepop found an old corpse after its map precondition",
            );
            return None;
        }
        CorpseBonesConversion::Converted => {
            // Standalone CreateCorpse retains C++ SpawnCorpseBones ordering:
            // save the current Player after the confirmed delete, before the
            // replacement corpse is built. The resurrection phase gate admits
            // this snapshot explicitly and is restored before continuing.
            if !host.player_logout_with_save_like_cpp()
                && !save_after_bones_before_create(host, binding, operation, item_guid_generator)
                    .await
            {
                return None;
            }
        }
    }

    if !advance(
        host,
        binding,
        operation,
        PlayerResurrectionPhase::CorpseBonesInFlight,
        PlayerResurrectionPhase::CorpseSaveInFlight,
    ) {
        failed(
            host,
            player_guid,
            "release phase changed before corpse construction",
        );
        return None;
    }
    let Some(created) = build_corpse_value(host, binding) else {
        failed(
            host,
            player_guid,
            "could not build a corpse from the canonical Player",
        );
        return None;
    };

    if let Some(save) = created.save.as_ref() {
        let Some(port) = host.map_corpse_persistence_port_like_cpp() else {
            failed(
                host,
                player_guid,
                "required corpse persistence port is unavailable",
            );
            return None;
        };
        match port.save_corpse_like_cpp(save.clone()).await {
            wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Committed => {}
            wow_persistence::CorpsePersistenceMutationOutcomeLikeCpp::Failed { reason } => {
                warn!(player = ?player_guid, %reason, "Corpse::SaveToDB was not confirmed");
                failed(
                    host,
                    player_guid,
                    "corpse persistence failed or was unconfirmed",
                );
                return None;
            }
        }
    }

    let Some(corpse_guid) = publish_corpse(host, binding, operation, created) else {
        failed(
            host,
            player_guid,
            "Player owner or residence changed before corpse publication",
        );
        return None;
    };

    if let Some(creature_spawns) = creature_spawns {
        // Map::AddToMap publishes the corpse before DEAD cleanup or movement.
        if !host
            .force_player_visibility_with_catalogs_like_cpp(creature_spawns)
            .await
            || !advance(
                host,
                binding,
                operation,
                PlayerResurrectionPhase::CorpsePublished,
                PlayerResurrectionPhase::CorpsePublished,
            )
        {
            failed(host, player_guid, "Player changed during corpse visibility");
            return None;
        }
        if with_bound_player_mut(host, binding, |player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Dead);
        })
        .is_none()
            || host.finish_player_non_alive_state_like_cpp().is_err()
            || !advance(
                host,
                binding,
                operation,
                PlayerResurrectionPhase::CorpsePublished,
                PlayerResurrectionPhase::CorpsePublished,
            )
        {
            failed(host, player_guid, "DEAD Player cleanup was not confirmed");
            return None;
        }
        if with_bound_player_mut(host, binding, |player| player.unit_mut().set_health(1)).is_none()
            || !publish_ghost_side_effects(host, player_guid)
        {
            failed(
                host,
                player_guid,
                "complete ghost projection could not be confirmed",
            );
            return None;
        }
    }

    if !complete(host, binding, operation) {
        failed(
            host,
            player_guid,
            "release continuation could not be completed",
        );
        return None;
    }
    Some(corpse_guid)
}

async fn save_after_bones_before_create<H: CorpseHostLikeCpp>(
    host: &mut H,
    binding: OwnerBinding,
    operation: ReleaseOperation,
    item_guid_generator: &ObjectGuidGenerator,
) -> bool {
    let player_guid = binding.handle.guid();
    if !advance(
        host,
        binding,
        operation,
        PlayerResurrectionPhase::CorpseBonesInFlight,
        PlayerResurrectionPhase::ReadyToSavePlayer,
    ) {
        return failed(
            host,
            player_guid,
            "could not admit the old-bones Player save",
        );
    }
    match host
        .save_player_after_corpse_bones_like_cpp(item_guid_generator)
        .await
    {
        ResurrectionSaveOutcome::Applied => {
            if !owner_is_current(host, binding)
                || !advance(
                    host,
                    binding,
                    operation,
                    PlayerResurrectionPhase::SavingPlayer,
                    PlayerResurrectionPhase::CorpseBonesInFlight,
                )
            {
                return failed(
                    host,
                    player_guid,
                    "Player owner changed after old-bones save",
                );
            }
            true
        }
        ResurrectionSaveOutcome::Deferred => {
            failed(host, player_guid, "old-bones Player save was deferred");
            false
        }
        ResurrectionSaveOutcome::Unresolved => {
            failed(host, player_guid, "old-bones Player save was not confirmed");
            false
        }
    }
}

fn build_corpse_value<H: CorpseHostLikeCpp>(
    host: &H,
    binding: OwnerBinding,
) -> Option<CreatedCorpse> {
    let key = binding.active_key()?;
    let hub = host.hub_ref();
    let owner = binding.handle.guid();
    let map_entry = hub
        .catalogs
        .maps
        .store
        .as_ref()
        .and_then(|store| store.get(key.map_id).copied());
    let instanceable = map_entry.is_some_and(MapEntry::is_instanceable_like_cpp);
    let in_arena = map_entry.is_some_and(|entry| entry.instance_type == 4);
    let in_battleground = hub.player_in_represented_battleground_like_cpp();
    let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
    let mut manager = manager.lock().ok()?;
    if !binding.matches(hub.core, &manager) {
        return None;
    }
    let map = manager.find_map_mut(key.map_id, key.instance_id)?.map_mut();
    let low_guid = map
        .generate_low_guid_like_cpp(wow_core::guid::HighGuid::Corpse)
        .ok()?;
    let player = map.get_typed_player(owner)?;
    build_corpse_from_player(
        player,
        owner,
        key,
        low_guid,
        instanceable,
        in_battleground,
        in_arena,
        &hub.catalogs,
    )
}

fn build_corpse_from_player(
    player: &Player,
    owner: ObjectGuid,
    key: MapKey,
    low_guid: i64,
    instanceable: bool,
    in_battleground: bool,
    in_arena: bool,
    catalogs: &wow_world_core::session::SessionCatalogs,
) -> Option<CreatedCorpse> {
    let world = player.unit().world();
    let position = world.position();
    if !position.is_valid_map_coord_like_cpp()
        || world.map_id() != key.map_id
        || world.instance_id() != key.instance_id
    {
        return None;
    }
    let race = player.race_like_cpp();
    let faction_template = catalogs.faction_template_for_race_like_cpp(race)?;
    let corpse_type = if player.extra_flags() & PLAYER_EXTRA_PVP_DEATH != 0 {
        CorpseType::ResurrectablePvp
    } else {
        CorpseType::ResurrectablePve
    };

    let mut corpse = Corpse::new_at(corpse_type, unix_now());
    corpse
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Corpse,
            0,
            owner.realm_id(),
            key.map_id as u16,
            0,
            0,
            low_guid,
        ));
    corpse
        .world_mut()
        .set_map(key.map_id, key.instance_id)
        .ok()?;
    corpse.world_mut().relocate(position);
    corpse.world_mut().object_mut().set_scale(1.0);
    corpse.set_owner_guid(owner);
    *corpse.world_mut().phase_shift_mut() = world.phase_shift().clone();

    let flags = corpse_flags_like_cpp(player.unit().data().pvp_flags, in_battleground, in_arena);
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
    corpse.set_display_id(u32::try_from(player.unit().data().native_display_id).unwrap_or(0));
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

    // C++ `Player::CreateCorpse` skips `SaveToDB` for instanceable maps.
    let save = (!instanceable).then(|| {
        let data = corpse.data();
        wow_persistence::CorpseSaveRequestLikeCpp {
            owner_guid: owner.counter() as u64,
            pos_x: position.x,
            pos_y: position.y,
            pos_z: position.z,
            orientation: position.orientation,
            map_id: key.map_id as u16,
            display_id: data.display_id,
            item_cache: corpse_item_cache_like_cpp(&data.items),
            race: data.race_id,
            class: data.class,
            sex: data.sex,
            flags: data.flags as u8,
            dynamic_flags: data.dynamic_flags as u8,
            ghost_time: corpse.ghost_time() as u32,
            corpse_type: corpse.corpse_type() as u8,
            instance_id: key.instance_id,
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
    Some(CreatedCorpse { corpse, save })
}

fn publish_corpse<H: CorpseHostLikeCpp>(
    host: &H,
    binding: OwnerBinding,
    operation: ReleaseOperation,
    created: CreatedCorpse,
) -> Option<ObjectGuid> {
    let key = binding.active_key()?;
    let hub = host.hub_ref();
    let manager_arc = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
    let mut manager = manager_arc.lock().ok()?;
    if !binding.matches(hub.core, &manager)
        || !manager.with_player_like_cpp(binding.handle, |player| {
            player
                .resurrection_state_like_cpp()
                .delayed_after_teleport
                .is_some_and(|continuation| {
                    operation.matches(continuation.operation)
                        && continuation.phase == PlayerResurrectionPhase::CorpseSaveInFlight
                })
        })?
    {
        return None;
    }

    let corpse_guid = created.corpse.world().object().guid();
    let location = (
        created.corpse.world().map_id(),
        created.corpse.world().position(),
    );
    if manager
        .find_map_mut(key.map_id, key.instance_id)?
        .map_mut()
        .add_corpse_to_map_like_cpp(created.corpse)
        .is_err()
    {
        return None;
    }
    manager.with_player_mut_like_cpp(binding.handle, |player| {
        player.set_pvp_death_like_cpp(false);
        let state = player.resurrection_state_mut_like_cpp();
        state.corpse_location = Some(location);
        let continuation = state.delayed_after_teleport.as_mut()?;
        continuation.phase = PlayerResurrectionPhase::CorpsePublished;
        Some(())
    })??;
    Some(corpse_guid)
}

fn publish_ghost_side_effects<H: CorpseHostLikeCpp>(host: &mut H, player_guid: ObjectGuid) -> bool {
    host.hub_mut().set_player_water_walking(true);
    let stunned = host
        .core()
        .with_owned_player_like_cpp(|player| {
            player
                .unit()
                .has_unit_state(wow_constants::UnitState::STUNNED.bits())
        })
        .unwrap_or(true);
    if !host.is_logging_out_for_repop() && !stunned {
        host.hub_mut().set_player_rooted_like_cpp(false);
    }
    // C++ Player.cpp:4304–4309 removes SKINNABLE after water walking/unrooting.
    if host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            let flags = player.unit().unit_flags_like_cpp() - UNIT_FLAG_SKINNABLE;
            player.unit_mut().set_unit_flags_like_cpp(flags);
        })
        .is_none()
    {
        return false;
    }
    host.sync_player_registry_state_like_cpp();
    host.send_packet(&wow_packet::packets::misc::CorpseReclaimDelay {
        remaining: crate::player_death::calculate_corpse_reclaim_delay_ms_like_cpp(host),
    });
    let reset = with_current_map_like_cpp(host, |map| {
        let guid = map.corpse_guid_by_player_like_cpp(player_guid)?;
        let corpse = map.get_typed_corpse_mut(guid)?;
        corpse.reset_ghost_time(unix_now());
        Some(())
    })
    .flatten();
    if reset.is_some() {
        crate::player_death::stop_mirror_timers_like_cpp(host);
    }
    reset.is_some()
}

fn begin<H: CorpseHostLikeCpp>(
    host: &H,
    operation: ReleaseOperation,
    phase: PlayerResurrectionPhase,
) -> Option<OwnerBinding> {
    let binding = current_owner(host)?;
    let installed = with_bound_player_mut(host, binding, |player| {
        let state = player.resurrection_state_mut_like_cpp();
        if state.delayed_after_teleport.is_some() {
            return false;
        }
        state.delayed_after_teleport = Some(PlayerResurrectionContinuation {
            operation: operation.state(),
            phase,
        });
        true
    })?;
    installed.then_some(binding)
}

fn current_owner<H: CorpseHostLikeCpp>(host: &H) -> Option<OwnerBinding> {
    let core = host.core();
    let guid = core.player_guid()?;
    let handle = core.player_handle_like_cpp?;
    if handle.guid() != guid {
        return None;
    }
    let manager = Arc::clone(core.canonical_map_manager.as_ref()?);
    let manager = manager.lock().ok()?;
    let (current, revision) = manager.current_player_admission_like_cpp(guid)?;
    if current != handle {
        return None;
    }
    let residence = manager.player_residence_like_cpp(handle)?;
    if !matches!(residence, PlayerResidenceLikeCpp::Active(_))
        || manager.with_player_like_cpp(handle, |_| ()).is_none()
    {
        return None;
    }
    Some(OwnerBinding {
        handle,
        revision,
        residence,
    })
}

fn owner_is_current<H: CorpseHostLikeCpp>(host: &H, binding: OwnerBinding) -> bool {
    let core = host.core();
    let Some(manager) = core.canonical_map_manager.as_ref().map(Arc::clone) else {
        return false;
    };
    let Ok(manager) = manager.lock() else {
        return false;
    };
    binding.matches(core, &manager)
}

fn with_bound_player<H: CorpseHostLikeCpp, R>(
    host: &H,
    binding: OwnerBinding,
    read: impl FnOnce(&Player) -> R,
) -> Option<R> {
    let core = host.core();
    let manager = Arc::clone(core.canonical_map_manager.as_ref()?);
    let manager = manager.lock().ok()?;
    binding
        .matches(core, &manager)
        .then(|| manager.with_player_like_cpp(binding.handle, read))?
}

fn with_bound_player_mut<H: CorpseHostLikeCpp, R>(
    host: &H,
    binding: OwnerBinding,
    write: impl FnOnce(&mut Player) -> R,
) -> Option<R> {
    let core = host.core();
    let manager = Arc::clone(core.canonical_map_manager.as_ref()?);
    let mut manager = manager.lock().ok()?;
    if !binding.matches(core, &manager) {
        return None;
    }
    manager.with_player_mut_like_cpp(binding.handle, write)
}

fn advance<H: CorpseHostLikeCpp>(
    host: &H,
    binding: OwnerBinding,
    operation: ReleaseOperation,
    expected: PlayerResurrectionPhase,
    next: PlayerResurrectionPhase,
) -> bool {
    with_bound_player_mut(host, binding, |player| {
        let Some(continuation) = player
            .resurrection_state_mut_like_cpp()
            .delayed_after_teleport
            .as_mut()
        else {
            return false;
        };
        if !operation.matches(continuation.operation) || continuation.phase != expected {
            return false;
        }
        continuation.phase = next;
        true
    })
    .unwrap_or(false)
}

fn complete<H: CorpseHostLikeCpp>(
    host: &H,
    binding: OwnerBinding,
    operation: ReleaseOperation,
) -> bool {
    if !advance(
        host,
        binding,
        operation,
        PlayerResurrectionPhase::CorpsePublished,
        PlayerResurrectionPhase::Complete,
    ) {
        return false;
    }
    with_bound_player_mut(host, binding, |player| {
        player.clear_completed_resurrection_continuation()
    })
    .unwrap_or(false)
}

fn failed<H: CorpseHostLikeCpp>(host: &mut H, player: ObjectGuid, reason: &str) -> bool {
    warn!(player = ?player, %reason, "Player release did not complete");
    host.core_mut().kick(reason);
    false
}
