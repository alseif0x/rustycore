// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{
    inventory::OwnedInventoryAccessLikeCpp,
    spell_acquisition::OwnedSpellAcquisitionAccessLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::PlayerSkillTestFixtureLikeCpp;
use crate::session::{
    PacketPublicationAccessLikeCpp, PlayerMoneyTransactionSessionAccessLikeCpp,
    PlayerRegistryControlBindingLikeCpp, PlayerRegistryHydrationAccessLikeCpp,
    PlayerRegistrySyncAccessLikeCpp, SessionCore,
};
use std::sync::Arc;
#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::{BTreeSet, HashMap};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::RepresentedPlayerSkillLikeCpp;
use wow_data::{SkillLineStore, SkillStore, SkillTiersStoreLikeCpp};
use wow_core::ObjectGuid;

/// Borrowed coordinator access to the existing canonical owners used by a
/// player spell acquisition. It exposes only short-lived, typed reborrows.
pub struct PlayerAcquisitionOwnerAccessLikeCpp<'a> {
    core: &'a mut SessionCore,
}

impl SessionCore {
    /// Build access to the canonical owners participating in acquisition.
    pub fn player_acquisition_owner_access_like_cpp(
        &mut self,
    ) -> PlayerAcquisitionOwnerAccessLikeCpp<'_> {
        PlayerAcquisitionOwnerAccessLikeCpp { core: self }
    }
}

impl PlayerAcquisitionOwnerAccessLikeCpp<'_> {
    /// Read the session's current player identity without exposing SessionCore.
    pub fn player_guid(&self) -> Option<ObjectGuid> {
        self.core.player_guid()
    }

    /// Update the canonical player's trainer interaction role when a trainer
    /// list is successfully published. `None` remains a missing owner rather
    /// than a handle-less fixture fallback.
    pub fn set_trainer_interaction_role_like_cpp(
        &self,
        source_guid: ObjectGuid,
        trainer_id: u32,
    ) -> bool {
        self.trainer_interaction_role_like_cpp()
            .set_trainer_interaction_like_cpp(source_guid, trainer_id)
    }

    /// Read only the canonical player's current trainer-role comparison. The
    /// caller owns the handle-less fixture policy and re-reads at each
    /// admission boundary.
    pub fn trainer_interaction_role_matches_like_cpp(
        &self,
        source_guid: ObjectGuid,
        trainer_id: i32,
    ) -> Option<bool> {
        self.trainer_interaction_role_like_cpp()
            .trainer_interaction_matches_like_cpp(source_guid, trainer_id)
    }

    /// Reborrow only the interaction-role view needed by the lower Interaction
    /// owner. This keeps the acquisition coordinator's exclusive SessionCore
    /// access while sharing one canonical role implementation with HubRef.
    pub fn trainer_interaction_role_like_cpp(
        &self,
    ) -> super::trainer_interaction::TrainerInteractionRoleAccessLikeCpp<'_> {
        self.core.trainer_interaction_role_access_like_cpp()
    }

    /// Reborrow the existing canonical inventory access for one operation.
    pub fn inventory(&self) -> OwnedInventoryAccessLikeCpp<'_> {
        self.core.owned_inventory_access_like_cpp()
    }

    /// Reborrow the existing canonical spell-acquisition access for one operation.
    pub fn spell_acquisition(&self) -> OwnedSpellAcquisitionAccessLikeCpp<'_> {
        self.core.owned_spell_acquisition_access_like_cpp()
    }

    /// Project the current Player spell runtime through the strict owned
    /// handle path without returning or cloning the canonical runtime itself.
    pub fn with_player_spell_runtime_like_cpp<R>(
        &self,
        project: impl FnOnce(&wow_entities::PlayerSpellRuntimeState) -> R,
    ) -> Option<R> {
        self.spell_acquisition()
            .with_player_spell_runtime_like_cpp(project)
    }

    /// Preserve the existing handle-less fixture branch without exposing the
    /// stored handle or the enclosing SessionCore.
    pub fn player_handle_absent_like_cpp(&self) -> bool {
        self.spell_acquisition().player_handle_absent_like_cpp()
    }

    /// Invalidate spell-hit aura authority before the fixture-owned runtime
    /// install path, matching the canonical install's pre-mutation fence.
    pub fn invalidate_spell_hit_aura_authority_like_cpp(&self) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }

    /// Reborrow the existing exclusive money-transaction access.
    pub fn money_transaction(&mut self) -> PlayerMoneyTransactionSessionAccessLikeCpp<'_> {
        self.core.player_money_transaction_access_like_cpp()
    }

    /// Reborrow the existing packet-publication access.
    pub fn packet_publication(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.core.packet_publication_access_like_cpp()
    }

    /// Reborrow the existing registry synchronization view with its exact
    /// caller-owned fixture inputs. Mutable vitals are lent separately through
    /// `RegistrySyncInputs` at the final publication phase.
    pub fn registry_sync<'a>(
        &'a self,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_position: &'a Option<wow_core::Position>,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_transport: &'a Option<Box<crate::session::PlayerTransportLoginStateLikeCpp>>,
    ) -> PlayerRegistrySyncAccessLikeCpp<'a> {
        self.core.player_registry_sync_access_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_position,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_transport,
        )
    }

    /// Resolve the current registry control binding at the publication point.
    pub fn registry_control_binding(&self) -> Option<PlayerRegistryControlBindingLikeCpp<'_>> {
        let guid = self.core.player_guid()?;
        let registry = self.core.player_registry()?;
        Some(
            self.core
                .player_registry_control_binding_like_cpp(guid, registry),
        )
    }

    /// Reborrow the narrow fixture-hydration capability for this owner.
    pub fn registry_hydration(&self) -> PlayerRegistryHydrationAccessLikeCpp<'_> {
        self.core.player_registry_hydration_access_like_cpp()
    }

    /// Publish the complete skill values image through Core's existing owner.
    pub fn publish_complete_skill_values_update_like_cpp(
        &self,
        skill_store: Option<&Arc<SkillStore>>,
        skill_lines: Option<&Arc<SkillLineStore>>,
        skill_tiers: Option<&Arc<SkillTiersStoreLikeCpp>>,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_inputs: (
            &HashMap<u16, RepresentedPlayerSkillLikeCpp>,
            &u8,
            &u8,
            &u8,
        ),
    ) {
        self.core.send_complete_player_skill_values_update_with_inputs_like_cpp(
            skill_store,
            skill_lines,
            skill_tiers,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_inputs,
        );
    }

    /// Replace the handle-less skill fixture through Core's existing exact
    /// fixture operation. Normal builds never expose this branch.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn replace_skill_runtime_exact_like_cpp(
        &self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
        occupied_slots: Option<u16>,
        tombstones: BTreeSet<u16>,
        fixture_inputs: (&mut PlayerSkillTestFixtureLikeCpp, &mut u16),
    ) -> bool {
        self.core.replace_player_skill_runtime_exact_like_cpp(
            skill_records,
            loaded,
            complete,
            occupied_slots,
            tombstones,
            fixture_inputs,
        )
    }
}
