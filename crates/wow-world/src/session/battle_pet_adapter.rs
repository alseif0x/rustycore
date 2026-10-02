// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battle pet adapter: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

#[cfg(any(test, feature = "test-fixtures"))]
use super::RepresentedBattlePetCageItemLikeCpp;
use super::{
    AuraApplication, Instant, ObjectGuid, RepresentedAuraEffectLikeCpp,
    RepresentedBattlePetCalculatedStatsLikeCpp, RepresentedBattlePetDataLikeCpp,
    RepresentedBattlePetLevelCriteriaLikeCpp, RepresentedBattlePetQueryCompanionLikeCpp,
    RepresentedBattlePetSaveInfoLikeCpp, RepresentedBattlePetSlotLikeCpp, WorldSession,
};
use super::{represented_aura_effect_amounts_like_cpp, warn};

#[cfg(test)]
use std::sync::atomic::{AtomicI64, Ordering};

#[cfg(test)]
pub(crate) static NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP: AtomicI64 = AtomicI64::new(1);

#[cfg(test)]
pub(crate) fn next_represented_battle_pet_guid_like_cpp() -> ObjectGuid {
    let counter = NEXT_REPRESENTED_BATTLE_PET_COUNTER_LIKE_CPP.fetch_add(1, Ordering::Relaxed);
    ObjectGuid::create_global(wow_core::guid::HighGuid::BattlePet, 0, counter)
}

pub(crate) fn apply_battle_pet_calculated_stats_like_cpp(
    pet: &mut RepresentedBattlePetDataLikeCpp,
    calculated_stats: Option<RepresentedBattlePetCalculatedStatsLikeCpp>,
) {
    if let Some(calculated_stats) = calculated_stats {
        pet.max_health = calculated_stats.max_health;
        pet.power = calculated_stats.power;
        pet.speed = calculated_stats.speed;
    }
    pet.health = pet.max_health;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetXpSourceLikeCpp {
    PetBattle,
    SpellEffect,
    Invalid,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetCageOutcomeLikeCpp {
    Caged(RepresentedBattlePetCageItemLikeCpp),
    NoJournalLock,
    UnknownPet,
    NotTradable,
    InBattleSlot,
    Damaged,
    InventoryUnavailable,
    StoreFailed,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetQualityOutcomeLikeCpp {
    Changed,
    NoJournalLock,
    UnknownPet,
    QualityAboveRare,
    CantBattle,
    NotUpgrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetGrantLevelOutcomeLikeCpp {
    Changed,
    NoJournalLock,
    UnknownPet,
    CantBattle,
    AlreadyMaxLevel,
    NoGrantedLevels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedBattlePetGrantExperienceOutcomeLikeCpp {
    Changed,
    NoJournalLock,
    UnknownPet,
    InvalidXpOrSource,
    CantBattle,
    AlreadyMaxLevel,
    MissingXpRow,
}

impl WorldSession {
    // ── Aura system ───────────────────────────────────────────────

    pub(in crate::session) fn apply_represented_battle_pet_xp_pct_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        effect: &wow_data::SpellEffectInfo,
    ) -> Result<(), &'static str> {
        let slot = self
            .next_player_visible_aura_slot_like_cpp()
            .ok_or("No free aura slots or missing Player aura owner")?;

        let multiplier = 1.0 + (effect.effect_base_points as f32 / 100.0);
        let aura = AuraApplication {
            spell_id,
            difficulty_id: self.core.current_map_difficulty_id_like_cpp(),
            caster_guid,
            slot,
            duration_total: 30_000,
            duration_remaining: 30_000,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1u32 << effect.effect_index,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::ModBattlePetXpPct),
            represented_amount: effect.effect_base_points,
            represented_effect_amounts: represented_aura_effect_amounts_like_cpp(effect),
            represented_misc_value: None,
            represented_multiplier: multiplier,
            applied_at: Instant::now(),
        };

        if !self.insert_player_visible_aura_like_cpp(aura) {
            return Err("Missing Player aura owner");
        }
        self.send_aura_update_applied(spell_id, slot, caster_guid, 30_000, 0x0000_0001, 0x1);

        Ok(())
    }

    /// Finish C++ `Player::DestroyItem` after the Login DB pet/receipt commit.
    ///
    /// The receipt makes this second database phase retryable: a reconnect may
    /// observe the already-created pet and finish deleting the cage without
    /// publishing another pet or visual.
    pub(in crate::session) async fn destroy_uncaged_battle_pet_item_durable_like_cpp(
        &mut self,
        source_item_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let player_db_guid = player_guid.counter() as u64;
        let item_db_guid = source_item_guid.counter() as u64;

        let (owner_guid, inventory_linked) = match self
            .uncage_item_state_like_cpp(player_db_guid, item_db_guid)
            .await
        {
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Loaded(state) => {
                (state.owner_guid, state.inventory_linked)
            }
            wow_persistence::PlayerUncageItemStateLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.core.account_id,
                    item_guid = item_db_guid,
                    %reason,
                    "Failed to inspect the uncaged battle-pet item"
                );
                return false;
            }
        };
        if owner_guid.is_some_and(|owner_guid| owner_guid != player_db_guid) {
            warn!(
                account = self.core.account_id,
                item_guid = item_db_guid,
                durable_owner = ?owner_guid,
                player_guid = player_db_guid,
                "Refusing to destroy an uncaged item owned by another character"
            );
            return false;
        }
        let Some((bag, slot, item)) = self.get_inventory_item_by_guid_like_cpp(source_item_guid)
        else {
            return owner_guid.is_none() && !inventory_linked;
        };
        let runtime_item = self.resolved_inventory_item_object_like_cpp(source_item_guid);
        self.destroy_inventory_full_stack_by_pos_with_expected_owner_like_cpp(
            bag,
            slot,
            item,
            runtime_item,
            owner_guid.map(|_| player_db_guid),
            "BattlePetUncage",
        )
        .await
    }

    #[cfg(test)]
    pub(crate) fn battle_pet_grant_battle_pet_experience_with_owner_auras_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        xp: u16,
        xp_source: RepresentedBattlePetXpSourceLikeCpp,
    ) -> RepresentedBattlePetGrantExperienceOutcomeLikeCpp {
        let multiplier = if xp_source == RepresentedBattlePetXpSourceLikeCpp::PetBattle {
            self.total_represented_aura_multiplier_like_cpp(
                RepresentedAuraEffectLikeCpp::ModBattlePetXpPct,
            )
        } else {
            1.0
        };

        self.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid, xp, xp_source, multiplier,
        )
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl crate::session::state::PetState {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn represented_dismissed_critter_guids_like_cpp(&self) -> &[ObjectGuid] {
        &self
            .battle_pet_test_fixture_like_cpp
            .represented_dismissed_critter_guids_like_cpp
    }
}

impl crate::session::HubRef<'_> {
    pub(crate) fn represented_critter_guid_like_cpp(&self) -> Option<ObjectGuid> {
        if let Some(guid) = self
            .core
            .with_owned_player_like_cpp(|player| player.unit().critter_guid_like_cpp())
        {
            return guid;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_critter_guid_like_cpp
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            None
        }
    }
}

impl crate::session::HubMut<'_> {
    pub(crate) fn set_represented_critter_guid_like_cpp(&mut self, guid: Option<ObjectGuid>) {
        if self
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player.unit_mut().set_critter_guid_like_cpp(guid);
            })
            .is_some()
        {
            return;
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_critter_guid_like_cpp = guid;
        }
    }

    /// C++ `WorldSession::HandleDismissCritter`, represented at the ownership gate.
    ///
    /// Full `ObjectAccessor::GetCreatureOrPetOrVehicle`, `Unit::IsSummon` and
    /// `TempSummon::UnSummon` remain part of the live companion runtime. This
    /// represented path preserves the C++ no-response semantics and only acts
    /// when the requested GUID is the player's active critter.
    pub(crate) fn represented_dismiss_critter_like_cpp(
        &mut self,
        critter_guid: ObjectGuid,
    ) -> bool {
        if self.shared().represented_critter_guid_like_cpp() != Some(critter_guid) {
            return false;
        }

        if let Some(companion) = self
            .shared()
            .represented_battle_pet_query_companion_like_cpp(critter_guid)
            && let Some(battle_pet_guid) = companion.battle_pet_companion_guid
            && self
                .shared()
                .represented_summoned_battle_pet_guid_like_cpp()
                == Some(battle_pet_guid)
        {
            let _ = self.set_represented_summoned_battle_pet_guid_like_cpp(None);
        }

        self.set_represented_critter_guid_like_cpp(None);
        #[cfg(any(test, feature = "test-fixtures"))]
        self.fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_dismissed_critter_guids_like_cpp
            .push(critter_guid);
        true
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/battle_pet_adapter/f3_shims.rs"]
mod f3_shims;
