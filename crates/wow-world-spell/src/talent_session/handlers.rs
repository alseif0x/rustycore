// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The talent packet handlers, moved from the World
//! `handlers/talent/state.rs` (#1363 slice 9b). The registrations stay in
//! `wow_world_application::register_talent_handlers_like_cpp`; the World
//! adapter `handlers/talent/host.rs` calls these with the session as host.

use tracing::warn;
use wow_constants::unit::NPCFlags1;
use wow_packet::packets::talent::{ConfirmRespecWipe, LearnTalent, SPEC_RESET_TALENTS_LIKE_CPP};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{HubMut, RepresentedConfirmRespecWipeLikeCpp};

use super::{TalentHostLikeCpp, learn_represented_talent_like_cpp};

pub(super) const CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits();

pub(super) const MIN_TALENT_RESET_LEVEL_LIKE_CPP: u8 = 15;

pub const UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP: u32 = 14_867;

pub const AT_LOGIN_RESET_TALENTS_LIKE_CPP: u16 = 0x004;

/// Handle CMSG_LEARN_TALENT.
///
/// C++ calls `Player::LearnTalent(TalentID, RequestedRank)` and sends
/// `SendTalentsInfoData()` only on success. Rust currently has the
/// represented talent snapshot plus DB2 spell-rank and talent-tab class
/// validation, but not the complete C++ point/prerequisite/tier runtime.
pub async fn handle_learn_talent<H: TalentHostLikeCpp>(
    host: &mut H,
    talent_tabs: &wow_data::TalentTabStore,
    mut packet: WorldPacket,
) {
    let request = match LearnTalent::read(&mut packet) {
        Ok(request) => request,
        Err(error) => {
            warn!("Bad LearnTalent: {error}");
            return;
        }
    };

    if request.talent_id < 0 {
        return;
    }

    if learn_represented_talent_like_cpp(
        host,
        talent_tabs,
        request.talent_id as u32,
        request.requested_rank,
    ) && let Some(talent_data) = host.hub_ref().resolved_update_talent_data_packet_like_cpp()
    {
        host.send_packet(&talent_data);
    }
}

/// Handle CMSG_CONFIRM_RESPEC_WIPE.
///
/// C++ resolves `GetNPCIfCanInteractWith(..., UNIT_NPC_FLAG_TRAINER)`,
/// accepts only `SPEC_RESET_TALENTS`, checks `Creature::CanResetTalents`,
/// then runs `Player::ResetTalents`, sends talent data, and casts the
/// visual spell. Rust keeps this as represented state until trainer-class
/// matching, criteria/DB persistence, and visual cast runtime are canonical.
pub async fn handle_confirm_respec_wipe_with_generator_like_cpp<H: TalentHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    no_reset_talent_cost: bool,
    mut packet: WorldPacket,
) {
    let request = match ConfirmRespecWipe::read(&mut packet) {
        Ok(request) => request,
        Err(error) => {
            warn!("Bad ConfirmRespecWipe: {error}");
            return;
        }
    };

    if request.respec_type != SPEC_RESET_TALENTS_LIKE_CPP {
        return;
    }

    if !represented_can_confirm_respec_wipe_like_cpp(&mut host.hub_mut(), request.respec_master) {
        return;
    }

    host.remove_represented_feign_death_if_needed_like_cpp();

    if !host.represented_talents_loaded_like_cpp() {
        return;
    }

    // Preserve C++ ResetTalents ordering for these precondition-side
    // effects. The durable transaction covers money, reset metadata, and
    // talent topology. Pet/spell runtime removal, criteria, and packets are
    // published synchronously after COMMIT; exact `_SaveSpells` persistence
    // remains bounded until Rust retains the complete PlayerSpellMap state.
    host.hub_mut()
        .record_represented_talent_reset_script_hook_like_cpp(false);
    host.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_TALENTS_LIKE_CPP, true);

    let Some(committed) = host
        .commit_represented_talent_reset_like_cpp(no_reset_talent_cost)
        .await
    else {
        return;
    };

    host.publish_committed_represented_talent_reset_like_cpp(
        item_guid_generator,
        committed,
        RepresentedConfirmRespecWipeLikeCpp {
            respec_master: request.respec_master,
            respec_type: request.respec_type,
        },
        UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP,
    )
    .await;
}

fn represented_can_confirm_respec_wipe_like_cpp(
    hub: &mut HubMut<'_>,
    respec_master: wow_core::ObjectGuid,
) -> bool {
    if hub.shared().player_level_like_cpp() < MIN_TALENT_RESET_LEVEL_LIKE_CPP {
        return false;
    }
    let player_class = hub.shared().player_class_like_cpp();

    if hub.core.canonical_map_manager.is_some() {
        return hub
            .shared()
            .represented_npc_can_interact_with_like_cpp(
                respec_master,
                CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP,
                0,
            )
            .is_some_and(|creature| creature.trainer_class == player_class);
    }

    hub.core
        .mutate_world_creature(respec_master, |creature| {
            (creature.npc_flags() & CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP) != 0
                && creature.trainer_class_like_cpp() == player_class
        })
        .unwrap_or(false)
}
