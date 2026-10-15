//! `CMSG_CONFIRM_RESPEC_WIPE` with the `Player::ResetTalents` money/talent
//! commit and its post-COMMIT publication (#1363 slice 13b; moved from the
//! World `handlers/talent/state.rs`, `session/persistence/commit.rs` and the
//! committed-reset record of `session/money_persistence_contracts.rs`).

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::warn;
use wow_constants::BuyResult;
use wow_constants::unit::NPCFlags1;
use wow_packet::packets::talent::{ConfirmRespecWipe, SPEC_RESET_TALENTS_LIKE_CPP};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{
    HubMut, RepresentedConfirmRespecWipeLikeCpp, RepresentedTalentRespecVisualSpellCastLikeCpp,
};
use wow_world_inventory::BattlePetPurchaseHostLikeCpp;
use wow_world_lifecycle::{
    ExclusivePlayerMoneyPersistenceLikeCpp, PlayerMoneyCommitCancellationFenceLikeCpp,
    RepresentedTalentResetStatePlanLikeCpp,
};
use wow_world_spell::SpellbookHostLikeCpp;

use super::SessionLifecycleHostLikeCpp;

pub(super) const CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits();

pub(super) const MIN_TALENT_RESET_LEVEL_LIKE_CPP: u8 = 15;

pub const UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP: u32 = 14_867;

pub const AT_LOGIN_RESET_TALENTS_LIKE_CPP: u16 = 0x004;

/// A successfully committed reset whose covered runtime state still has to be
/// published synchronously before the money exclusion is released.
#[must_use]
pub struct CommittedRepresentedTalentResetLikeCpp {
    money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
    old_money: u64,
    new_money: u64,
    cost: u32,
    reset_time_secs: u64,
    state_plan: RepresentedTalentResetStatePlanLikeCpp,
}

/// Handle CMSG_CONFIRM_RESPEC_WIPE.
///
/// C++ resolves `GetNPCIfCanInteractWith(..., UNIT_NPC_FLAG_TRAINER)`,
/// accepts only `SPEC_RESET_TALENTS`, checks `Creature::CanResetTalents`,
/// then runs `Player::ResetTalents`, sends talent data, and casts the
/// visual spell. Rust keeps this as represented state until trainer-class
/// matching, criteria/DB persistence, and visual cast runtime are canonical.
pub async fn handle_confirm_respec_wipe_with_generator_like_cpp<H: SessionLifecycleHostLikeCpp>(
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

    if !{
        let mut hub = host.hub_mut();
        represented_can_confirm_respec_wipe_like_cpp(&mut hub, request.respec_master)
    } {
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

    let Some(committed) =
        commit_represented_talent_reset_like_cpp(host, no_reset_talent_cost).await
    else {
        return;
    };

    publish_committed_represented_talent_reset_like_cpp(
        host,
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

async fn commit_represented_talent_reset_like_cpp<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    no_reset_talent_cost: bool,
) -> Option<CommittedRepresentedTalentResetLikeCpp> {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    commit_represented_talent_reset_at_like_cpp(host, now_secs, no_reset_talent_cost).await
}

pub async fn commit_represented_talent_reset_at_like_cpp<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    now_secs: u64,
    no_reset_talent_cost: bool,
) -> Option<CommittedRepresentedTalentResetLikeCpp> {
    let cost = if no_reset_talent_cost {
        0
    } else {
        u64::from(
            host.hub_ref()
                .represented_next_reset_talents_cost_like_cpp(now_secs)?,
        )
    };

    let money_persistence = host
        .begin_exclusive_player_money_persistence_like_cpp()
        .await?;
    let old_money = BattlePetPurchaseHostLikeCpp::resolved_player_money_like_cpp(host)?;
    if old_money < cost {
        host.core()
            .send_buy_error(BuyResult::NotEnoughtMoney, None, 0);
        return None;
    }
    let new_money = old_money - cost;
    let player_guid = host.player_guid()?;
    let (state_plan, persistence_request) = {
        let (state, hub) = host.split_lifecycle_ref();
        state.represented_talent_reset_persistence_plan_like_cpp(
            hub,
            player_guid.counter() as u64,
            old_money,
            new_money,
            cost as u32,
            now_secs,
        )
    }?;
    // Unit fixtures without a lifecycle port explicitly model a successful
    // COMMIT. The failure seam proves that no covered runtime state is
    // published on a definite rollback.
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.test_fixtures_ref_like_cpp().is_some();
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test
        && host
            .lifecycle_mut()
            .loot_money_persistence_test_result_like_cpp()
            == Some(false)
    {
        return None;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    let bypass_database_like_cpp = world_test
        && (host
            .lifecycle_mut()
            .loot_money_persistence_test_result_like_cpp()
            == Some(true)
            || host
                .lifecycle_mut()
                .player_lifecycle_port_like_cpp()
                .is_none());
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let bypass_database_like_cpp = false;

    let money_persistence = if bypass_database_like_cpp {
        money_persistence
    } else {
        let port = host
            .lifecycle_mut()
            .player_lifecycle_port_like_cpp()
            .cloned()?;
        let mut cancellation_fence = PlayerMoneyCommitCancellationFenceLikeCpp::new(Arc::clone(
            host.lifecycle_mut()
                .durable_loot_money_persistence_tracker_like_cpp(),
        ));
        match port
            .persist_talent_reset_like_cpp(persistence_request)
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {
                cancellation_fence.disarm_like_cpp();
                money_persistence
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason } => {
                cancellation_fence.disarm_like_cpp();
                warn!(%reason, operation = "talent reset", "player-money transaction definitely rolled back");
                return None;
            }
            wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                host.lifecycle_mut()
                    .durable_loot_money_persistence_tracker_like_cpp()
                    .mark_indeterminate_like_cpp();
                cancellation_fence.disarm_like_cpp();
                host.core_mut().kick(
                    "player-money COMMIT outcome is unknown; relog required before another money mutation",
                );
                warn!(
                    %reason,
                    operation = "talent reset",
                    money_before = old_money,
                    money_after = new_money,
                    "player-money COMMIT outcome remains indeterminate; quarantined the session"
                );
                return None;
            }
        }
    };

    Some(CommittedRepresentedTalentResetLikeCpp {
        money_persistence,
        old_money,
        new_money,
        cost: cost as u32,
        reset_time_secs: now_secs,
        state_plan,
    })
}

/// Publish every runtime effect covered by the committed reset before the
/// shared money guard is released. There is deliberately no `.await`
/// between entry and `drop(money_persistence)`: cancellation cannot expose
/// a durable fee/talent reset with the old session state still live.
async fn publish_committed_represented_talent_reset_like_cpp<H: SessionLifecycleHostLikeCpp>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    committed: CommittedRepresentedTalentResetLikeCpp,
    request: RepresentedConfirmRespecWipeLikeCpp,
    visual_spell_id: u32,
) {
    let CommittedRepresentedTalentResetLikeCpp {
        money_persistence,
        old_money,
        new_money,
        cost,
        reset_time_secs,
        state_plan,
    } = committed;

    host.hub_mut().remove_represented_pet_not_in_slot_like_cpp();
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(fixtures) = host.test_fixtures_like_cpp() {
        fixtures
            .progression
            .represented_confirm_respec_wipe_requests_like_cpp
            .push(request);
    }

    debug_assert_eq!(
        host.hub_ref().represented_active_talent_group_like_cpp(),
        Some(state_plan.active_group)
    );
    for (talent_id, rank) in &state_plan.active_talents {
        wow_world_spell::remove_represented_active_talent_side_effects_like_cpp(
            host, *talent_id, *rank,
        );
    }
    if !host
        .hub_mut()
        .install_reset_talent_groups_like_cpp(state_plan.post_talents.clone())
    {
        host.core_mut()
            .kick("canonical Player talent owner became unavailable after talent-reset COMMIT");
        return;
    }
    host.refresh_represented_talent_points_like_cpp();

    if !host.stage_player_money_change_like_cpp(old_money, new_money) {
        host.core_mut()
            .kick("canonical Player money owner became unavailable after talent-reset COMMIT");
        return;
    }
    if !host
        .hub_mut()
        .set_represented_talent_reset_state_like_cpp(cost, reset_time_secs)
    {
        host.core_mut().kick(
            "canonical Player specialization owner became unavailable after talent-reset COMMIT",
        );
        return;
    }
    host.hub_mut()
        .record_represented_talent_respec_criteria_like_cpp(cost);

    if let Some(talent_data) = host.hub_ref().resolved_update_talent_data_packet_like_cpp() {
        host.send_packet(&talent_data);
    }
    if let Some(player_guid) = host.player_guid() {
        {
            let (s, mut h) = SpellbookHostLikeCpp::split_spell_state_mut(host);
            s.record_represented_talent_respec_visual_spell_cast_like_cpp(
                &mut h,
                RepresentedTalentRespecVisualSpellCastLikeCpp {
                    caster_guid: request.respec_master,
                    target_guid: player_guid,
                    spell_id: visual_spell_id,
                    triggered: true,
                    spell_runtime_unrepresented: true,
                },
            )
        };
    }

    drop(money_persistence);
    BattlePetPurchaseHostLikeCpp::drain_represented_quest_objective_progress_with_generator_like_cpp(
        host,
        item_guid_generator,
    )
    .await;
}
