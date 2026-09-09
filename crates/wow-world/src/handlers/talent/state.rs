//! Talent handlers state definitions, part 1 of 1.
//!
//! Separated from the talent.rs root under #662. Behaviour is preserved.

use super::*;

pub(super) const CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits();

pub(super) const MIN_TALENT_RESET_LEVEL_LIKE_CPP: u8 = 15;

pub(super) const UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP: u32 = 14_867;

#[cfg(test)]
pub(super) const AT_LOGIN_RENAME_LIKE_CPP: u16 = 0x001;

#[cfg(test)]
pub(super) const AT_LOGIN_RESET_SPELLS_LIKE_CPP: u16 = 0x002;

pub(super) const AT_LOGIN_RESET_TALENTS_LIKE_CPP: u16 = 0x004;

impl WorldSession {
    /// Handle CMSG_LEARN_TALENT.
    ///
    /// C++ calls `Player::LearnTalent(TalentID, RequestedRank)` and sends
    /// `SendTalentsInfoData()` only on success. Rust currently has the
    /// represented talent snapshot plus DB2 spell-rank and talent-tab class
    /// validation, but not the complete C++ point/prerequisite/tier runtime.
    pub async fn handle_learn_talent(
        &mut self,
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

        if self.learn_represented_talent_like_cpp(
            talent_tabs,
            request.talent_id as u32,
            request.requested_rank,
        ) && let Some(talent_data) = self.resolved_update_talent_data_packet_like_cpp()
        {
            self.send_packet(&talent_data);
        }
    }

    /// Parse the unresolved-placeholder CMSG_LEARN_TALENTS payload.
    ///
    /// C++ expands each `uint16` talent id into a `LearnTalent` request with
    /// `RequestedRank = 0` and delegates to `HandleLearnTalentOpcode`.
    /// The inspected C++ opcode table still uses the shared `0xBADD`
    /// placeholder, so this handler is deliberately not registered for live
    /// dispatch until the real client opcode is resolved.
    pub async fn handle_learn_talents(
        &mut self,
        talent_tabs: &wow_data::TalentTabStore,
        mut packet: WorldPacket,
    ) {
        let request = match LearnTalents::read(&mut packet) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad LearnTalents: {error}");
                return;
            }
        };

        for talent_id in request.talent_ids {
            if self.learn_represented_talent_like_cpp(talent_tabs, u32::from(talent_id), 0) {
                if let Some(talent_data) = self.resolved_update_talent_data_packet_like_cpp() {
                    self.send_packet(&talent_data);
                }
            }
        }
    }

    /// Handle CMSG_CONFIRM_RESPEC_WIPE.
    ///
    /// C++ resolves `GetNPCIfCanInteractWith(..., UNIT_NPC_FLAG_TRAINER)`,
    /// accepts only `SPEC_RESET_TALENTS`, checks `Creature::CanResetTalents`,
    /// then runs `Player::ResetTalents`, sends talent data, and casts the
    /// visual spell. Rust keeps this as represented state until trainer-class
    /// matching, criteria/DB persistence, and visual cast runtime are canonical.
    pub async fn handle_confirm_respec_wipe_with_generator_like_cpp(
        &mut self,
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

        if !self.represented_can_confirm_respec_wipe_like_cpp(request.respec_master) {
            return;
        }

        self.remove_represented_feign_death_if_needed_like_cpp();

        if !self.represented_talents_loaded_like_cpp() {
            return;
        }

        // Preserve C++ ResetTalents ordering for these precondition-side
        // effects. The durable transaction covers money, reset metadata, and
        // talent topology. Pet/spell runtime removal, criteria, and packets are
        // published synchronously after COMMIT; exact `_SaveSpells` persistence
        // remains bounded until Rust retains the complete PlayerSpellMap state.
        self.record_represented_talent_reset_script_hook_like_cpp(false);
        self.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_TALENTS_LIKE_CPP, true);

        let Some(committed) = self
            .commit_represented_talent_reset_like_cpp(no_reset_talent_cost)
            .await
        else {
            return;
        };

        self.publish_committed_represented_talent_reset_like_cpp(
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

    #[cfg(test)]
    pub async fn handle_confirm_respec_wipe(&mut self, packet: WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_confirm_respec_wipe_with_generator_like_cpp(
            generators.item.as_ref(),
            false,
            packet,
        )
        .await;
    }

    pub(super) fn represented_can_confirm_respec_wipe_like_cpp(
        &mut self,
        respec_master: wow_core::ObjectGuid,
    ) -> bool {
        if self.player_level_like_cpp() < MIN_TALENT_RESET_LEVEL_LIKE_CPP {
            return false;
        }
        let player_class = self.player_class_like_cpp();

        if self.has_canonical_map_manager_like_cpp() {
            return self
                .represented_npc_can_interact_with_like_cpp(
                    respec_master,
                    CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP,
                    0,
                )
                .is_some_and(|creature| creature.trainer_class == player_class);
        }

        self.mutate_world_creature(respec_master, |creature| {
            (creature.npc_flags() & CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP) != 0
                && creature.trainer_class_like_cpp() == player_class
        })
        .unwrap_or(false)
    }
}
