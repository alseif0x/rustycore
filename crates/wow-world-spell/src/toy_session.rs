// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! CMSG_USE_TOY — bounded C++ guard path before spell execution.
//!
//! C++ `HandleUseToy` (`ToyHandler.cpp:54`) validates item template,
//! `CollectionMgr::HasToy`, item effect spell membership,
//! `SpellMgr::GetSpellInfo`, possession, and then creates/prepares a `Spell`
//! with toy-specific flags. Rust still uses the represented spell executor,
//! but preserves the C++ toy metadata that must reach `SpellCastData`.
//!
//! Moved verbatim from the World `handlers/collections/mod.rs` (#1363 slice
//! 4d); every World operation it calls is already on the spell execution host.

use tracing::{debug, warn};
use wow_constants::SpellCastResult;
use wow_entities::SpellCastMetadata;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::UseToy;
use wow_packet::packets::spell::{CastFailed, SpellCastVisual, SpellPreparePkt, SpellStartPkt};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

use crate::SpellExecutionHostLikeCpp;

pub const CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP: u32 = 0x08000;

pub async fn handle_use_toy_with_generator_like_cpp<H: SpellExecutionHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    mut pkt: wow_packet::WorldPacket,
) {
    let request = match UseToy::read(&mut pkt) {
        Ok(request) => request,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                "UseToy parse failed: {error}"
            );
            return;
        }
    };

    let item_id = match u32::try_from(request.cast.misc[0]) {
        Ok(item_id) if item_id != 0 => item_id,
        _ => return,
    };

    if host
        .hub_ref()
        .catalogs
        .item_storage_template(item_id)
        .is_none()
    {
        return;
    }

    if !{
        let (state, hub) = (host.lifecycle(), host.hub_ref());
        state.has_account_toy_like_cpp(hub, item_id)
    } {
        return;
    }

    if !host
        .hub_ref()
        .catalogs
        .toy_item_has_spell_effect_like_cpp(item_id, request.cast.spell_id)
    {
        return;
    }

    let Some(spell_store) = host.hub_ref().catalogs.spell_store() else {
        return;
    };
    let Some(spell_info) = spell_store.get(request.cast.spell_id).cloned() else {
        warn!(
            account = host.core().account_id,
            spell_id = request.cast.spell_id,
            item_id,
            "HandleUseToy: unknown spell id used by toy item"
        );
        return;
    };

    if host.core().player_is_possessing_like_cpp() {
        return;
    }

    let toy_cooldown_ms = host.hub_ref().catalogs.toy_item_spell_cooldown_ms_like_cpp(
        item_id,
        request.cast.spell_id,
        &spell_info,
    );
    let Some(remaining_ms) = ({
        let (s, h) = host.split_spell_state_ref();
        s.represented_spell_cooldown_remaining_ms_like_cpp(
            h,
            request.cast.spell_id,
            toy_cooldown_ms,
        )
    }) else {
        return;
    };
    if remaining_ms > 0 {
        debug!(
            account = host.core().account_id,
            item_id,
            spell_id = request.cast.spell_id,
            remaining_ms,
            "UseToy rejected by represented item-backed cooldown"
        );
        host.send_packet(&CastFailed {
            cast_id: request.cast.cast_id,
            spell_id: request.cast.spell_id,
            visual: request.cast.visual.clone(),
            reason: SpellCastResult::NotReady as i32,
            fail_arg1: 0,
            fail_arg2: 0,
        });
        return;
    }

    let Some(player_guid) = host.player_guid() else {
        return;
    };

    let Some(server_cast_id) =
        host.next_represented_spell_cast_guid_like_cpp(request.cast.spell_id)
    else {
        return;
    };
    host.send_packet(&SpellPreparePkt {
        client_cast_id: request.cast.cast_id,
        server_cast_id,
    });

    let metadata = SpellCastMetadata {
        from_client: true,
        misc: request.cast.misc,
        cast_item_entry: Some(item_id),
        cast_item_battle_pet_modifiers: None,
        cast_flags_ex: CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP,
        original_cast_id: request.cast.cast_id,
        unit_target_battle_pet_companion_guid: None,
        ..SpellCastMetadata::default()
    };

    let mut spell_target = request.cast.target.clone();
    let target_guid = if !spell_target.unit.is_empty() {
        spell_target.unit
    } else {
        spell_target.flags |= 0x2; // SpellCastTargetFlags::Unit
        spell_target.unit = player_guid;
        player_guid
    };

    let spell_visual = SpellCastVisual {
        spell_visual_id: request.cast.visual.spell_visual_id,
        script_visual_id: 0,
    };

    if spell_info.has_cast_time() {
        let start_pkt = SpellStartPkt {
            cast_data: Default::default(),
            caster: player_guid,
            cast_id: server_cast_id,
            original_cast_id: request.cast.cast_id,
            spell_id: request.cast.spell_id,
            visual: spell_visual.clone(),
            cast_flags: 0x0000_0002,
            cast_flags_ex: CAST_FLAG_EX_USE_TOY_SPELL_LIKE_CPP,
            cast_time_ms: spell_info.cast_time_ms,
            target: spell_target.clone(),
        };
        host.send_packet(&start_pkt);

        // Stamp the admitted residence so this timed toy cast is fenced
        // against reentry exactly like a prepared normal request.
        let mut metadata = metadata;
        metadata.prepared_residence_revision = {
            let (s, h) = host.split_spell_state_ref();
            s.current_player_residence_revision_like_cpp(h)
        };
        {
            let cast = Some(wow_entities::SpellCastState {
                spell_id: request.cast.spell_id,
                target_guid,
                target_data: crate::spell_cast_adapter::retain_targets(spell_target),
                cast_id: server_cast_id,
                cast_start_time: std::time::Instant::now(),
                cast_time_ms: spell_info.cast_time_ms,
                spell_visual: crate::spell_cast_adapter::retain_visual(spell_visual),
                metadata,
            });
            host.mutate_cast_execution_like_cpp(|state| state.active = cast)
                .is_some()
        };
    } else if let Err(error) =
        crate::execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
            host,
            item_guid_generator,
            creature_spawn_catalogs,
            request.cast.spell_id,
            target_guid,
            server_cast_id,
            spell_visual,
            spell_target,
            metadata,
        )
        .await
    {
        warn!(
            account = host.core().account_id,
            spell_id = request.cast.spell_id,
            item_id,
            "UseToy represented spell execution failed: {error}"
        );
    }

    debug!(
        account = host.core().account_id,
        item_id,
        spell_id = request.cast.spell_id,
        "UseToy executed through represented spell path"
    );
}
