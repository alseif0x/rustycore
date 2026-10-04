// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::context::AppTrainerCx;
use super::{TrainerAdmissionProofLikeCpp, TrainerOfferDecisionLikeCpp};
use super::{
    PreparedTrainerOfferLikeCpp, TrainerAcquisitionCompletionLikeCpp,
    TrainerAcquisitionPublicationLikeCpp, execute_trainer_acquisition_like_cpp,
};
use wow_core::ObjectGuid;
use wow_core::ObjectGuidGenerator;
use wow_constants::unit::NPCFlags1;
use wow_data::{
    BattlePetClassificationLikeCpp, ConditionEntriesByTypeStore, PlayerConditionStore,
    SkillLineAbilityCoverageLikeCpp, SkillLineAbilityRecord,
    SkillRaceClassInfoMatchCoverageLikeCpp, SkillStore,
    TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP, TRAINER_SPELL_STATE_KNOWN_LIKE_CPP,
    TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP, TrainerLikeCpp, TrainerSpellLikeCpp,
    TrainerStoreLikeCpp, SpellAcquisitionCatalogLikeCpp,
    SpellAcquisitionEffectsLookupLikeCpp, battle_pet_selection::BattlePetSelectionStoreLikeCpp,
};
use wow_world_core::session::{
    NpcInteractionAccessLikeCpp, OwnedSpellAcquisitionAccessLikeCpp,
    PacketPublicationAccessLikeCpp,
    TrainerInteractionRoleAccessLikeCpp,
};
use crate::PlayerConditionProjectionCxLikeCpp;
use wow_world_interaction::InteractionState;
use wow_packet::packets::trainer::{TrainerListPacket, TrainerListSpell};
use wow_spell_acquisition::PlayerSpellAcquisitionSnapshotLikeCpp;
use wow_world_lifecycle::ExclusivePlayerMoneyPersistenceLikeCpp;
use wow_world_spell::{
    RepresentedPlayerSpellLikeCpp, RepresentedPlayerSpellStateLikeCpp, SessionSpellState,
    represented_player_spell_record_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_spell::canonical_player_spell_runtime_like_cpp;

const TRAINER_LIST_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits();
pub const TRAINER_BUY_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits()
    | NPCFlags1::TRAINER_CLASS.bits()
    | NPCFlags1::TRAINER_PROFESSION.bits();
const TRAINER_GOSSIP_NPC_FLAGS_LIKE_CPP: u32 =
    NPCFlags1::GOSSIP.bits() | TRAINER_BUY_NPC_FLAGS_LIKE_CPP;

pub fn trainer_list_required_npc_flags_like_cpp(
    gossip_option: Option<(u32, u32)>,
) -> u32 {
    if gossip_option.is_some() {
        // Retain the target fork's combined gossip/trainer interaction route.
        TRAINER_GOSSIP_NPC_FLAGS_LIKE_CPP
    } else {
        TRAINER_LIST_NPC_FLAGS_LIKE_CPP
    }
}

pub enum TrainerBuyAdmissionLikeCpp {
    InteractionMismatch,
    TrainerStoreUnavailable,
    TrainerUnavailable,
    SpellUnavailable,
}

pub(super) fn find_trainer_buy_spell_like_cpp<'a>(
    interaction: &InteractionState,
    role: &TrainerInteractionRoleAccessLikeCpp<'_>,
    trainer_store: Option<&'a TrainerStoreLikeCpp>,
    source_guid: ObjectGuid,
    trainer_id: i32,
    spell_id: u32,
) -> Result<&'a TrainerSpellLikeCpp, TrainerBuyAdmissionLikeCpp> {
    if !interaction.trainer_interaction_role_matches_with_access_like_cpp(
        role,
        source_guid,
        trainer_id,
    ) {
        return Err(TrainerBuyAdmissionLikeCpp::InteractionMismatch);
    }
    let Some(trainer_store) = trainer_store else {
        return Err(TrainerBuyAdmissionLikeCpp::TrainerStoreUnavailable);
    };
    let Some(trainer) = trainer_store.get_trainer_like_cpp(trainer_id as u32) else {
        return Err(TrainerBuyAdmissionLikeCpp::TrainerUnavailable);
    };
    trainer
        .get_spell_like_cpp(spell_id)
        .ok_or(TrainerBuyAdmissionLikeCpp::SpellUnavailable)
}

/// Selected interaction and packet-channel participants for publishing a
/// successfully resolved trainer list.
pub struct AppTrainerListCx<'a> {
    interaction: &'a mut InteractionState,
    role: TrainerInteractionRoleAccessLikeCpp<'a>,
    packets: PacketPublicationAccessLikeCpp<'a>,
    npc: wow_world_core::session::AuraNpcAccessBuilderLikeCpp<'a>,
    aura: crate::PlayerAuraApplicationCxLikeCpp<'a>,
    pub(super) spell_access: OwnedSpellAcquisitionAccessLikeCpp<'a>,
    player_conditions: crate::PlayerConditionProjectionInputsLikeCpp<'a>,
    pub(super) catalogs: TrainerListCatalogsLikeCpp<'a>,
    account_id: u32,
    session_locale_name: &'a str,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) skill_fixture_complete: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) skill_fixture_occupied: &'a Option<u16>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) skill_fixture_tombstones: &'a std::collections::BTreeSet<u16>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) skill_fixture_loaded: &'a bool,
    pub(super) max_primary_trade_skills: &'a u8,
}

/// A readonly row view created only after the complete FeignDeath transition.
/// No snapshots are captured by construction; row queries stay at their call sites.
pub(super) struct TrainerOfferCxLikeCpp<'a> {
    pub(super) npc: NpcInteractionAccessLikeCpp<'a>,
    pub(super) spell_access: &'a OwnedSpellAcquisitionAccessLikeCpp<'a>,
    pub(super) spell_state: &'a SessionSpellState,
    pub(super) player_conditions: PlayerConditionProjectionCxLikeCpp<'a>,
    pub(super) catalogs: &'a TrainerListCatalogsLikeCpp<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) skill_fixture_complete: &'a bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) skill_fixture_occupied: &'a Option<u16>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) skill_fixture_tombstones: &'a std::collections::BTreeSet<u16>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(super) skill_fixture_loaded: &'a bool,
    pub(super) max_primary_trade_skills: &'a u8,
}

/// Exact selected catalog inputs read while resolving and evaluating one
/// trainer list. The application never receives the World catalog aggregate.
pub struct TrainerListCatalogsLikeCpp<'a> {
    trainer_store: Option<&'a std::sync::Arc<TrainerStoreLikeCpp>>,
    pub(super) spell_acquisition: Option<&'a SpellAcquisitionCatalogLikeCpp>,
    pub(super) skills: Option<&'a SkillStore>,
    pub(super) skill_lines: Option<&'a wow_data::SkillLineStore>,
    condition_store: Option<&'a ConditionEntriesByTypeStore>,
    player_condition_store: Option<&'a PlayerConditionStore>,
    pub(super) projection: super::projection::TrainerProjectionCatalogsLikeCpp<'a>,
}

impl<'a> TrainerListCatalogsLikeCpp<'a> {
    /// Borrow only the stores read by the trainer-list operation.
    pub fn new(
        trainer_store: Option<&'a std::sync::Arc<TrainerStoreLikeCpp>>,
        spell_acquisition: Option<&'a SpellAcquisitionCatalogLikeCpp>,
        skills: Option<&'a SkillStore>,
        skill_lines: Option<&'a wow_data::SkillLineStore>,
        condition_store: Option<&'a ConditionEntriesByTypeStore>,
        player_condition_store: Option<&'a PlayerConditionStore>,
        projection: super::projection::TrainerProjectionCatalogsLikeCpp<'a>,
    ) -> Self {
        Self {
            trainer_store,
            spell_acquisition,
            skills,
            skill_lines,
            condition_store,
            player_condition_store,
            projection,
        }
    }
}

/// One already-evaluated trainer row, retained by reference until the App
/// builds the outgoing packet so the World adapter does not clone store rows.
pub struct TrainerListOfferResultLikeCpp<'a> {
    pub trainer_spell: &'a TrainerSpellLikeCpp,
    pub decision: TrainerOfferDecisionLikeCpp,
    pub fallback_price: u32,
}

/// Resolve complete spell rows from canonical state first and the selected
/// absent-handle fixture only for the World test consumer.
pub fn complete_represented_player_spell_rows_like_cpp(
    access: &OwnedSpellAcquisitionAccessLikeCpp<'_>,
    spell_state: &SessionSpellState,
    consumer_test: bool,
) -> Option<std::collections::BTreeMap<i32, RepresentedPlayerSpellLikeCpp>> {
    fn complete_rows(
        runtime: &wow_entities::PlayerSpellRuntimeState,
    ) -> Option<std::collections::BTreeMap<i32, RepresentedPlayerSpellLikeCpp>> {
        (runtime.rows_loaded_like_cpp() && runtime.rows_complete_like_cpp()).then(|| {
            runtime
                .rows_like_cpp()
                .iter()
                .map(|(&id, row)| (id, represented_player_spell_record_like_cpp(row)))
                .collect()
        })
    }

    let canonical = access
        .with_player_spell_runtime_like_cpp(complete_rows)
        .flatten();
    #[cfg(any(test, feature = "test-fixtures"))]
    if canonical.is_none() && consumer_test && access.player_handle_absent_like_cpp() {
        let runtime = canonical_player_spell_runtime_like_cpp(
            spell_state.represented_spell_runtime_fixture_like_cpp(),
        );
        return complete_rows(&runtime);
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = (spell_state, consumer_test);
    canonical
}

impl<'a> AppTrainerListCx<'a> {
    pub fn new(
        interaction: &'a mut InteractionState,
        role: TrainerInteractionRoleAccessLikeCpp<'a>,
        packets: PacketPublicationAccessLikeCpp<'a>,
        npc: wow_world_core::session::AuraNpcAccessBuilderLikeCpp<'a>,
        spell_access: OwnedSpellAcquisitionAccessLikeCpp<'a>,
        aura: crate::PlayerAuraApplicationCxLikeCpp<'a>,
        player_conditions: crate::PlayerConditionProjectionInputsLikeCpp<'a>,
        catalogs: TrainerListCatalogsLikeCpp<'a>,
        account_id: u32,
        session_locale_name: &'a str,
        #[cfg(any(test, feature = "test-fixtures"))] skill_fixture_complete: &'a bool,
        #[cfg(any(test, feature = "test-fixtures"))] skill_fixture_occupied: &'a Option<u16>,
        #[cfg(any(test, feature = "test-fixtures"))] skill_fixture_tombstones: &'a std::collections::BTreeSet<u16>,
        #[cfg(any(test, feature = "test-fixtures"))] skill_fixture_loaded: &'a bool,
        max_primary_trade_skills: &'a u8,
    ) -> Self {
        Self {
            interaction,
            role,
            packets,
            npc,
            spell_access,
            aura,
            player_conditions,
            catalogs,
            account_id,
            session_locale_name,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_fixture_complete,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_fixture_occupied,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_fixture_tombstones,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_fixture_loaded,
            max_primary_trade_skills,
        }
    }

    /// Resolve and publish one trainer list in the original World phase
    /// order. The selected offer and full aura-removal providers are part of
    /// this App closure; this operation does not accept World callbacks.
    pub fn send_trainer_list_like_cpp(
        &mut self,
        hello: wow_packet::packets::gossip::Hello,
        gossip_option: Option<(u32, u32)>,
    ) {
        let trainer_guid = hello.unit;
        tracing::info!(
            account = self.account_id,
            trainer_guid = ?trainer_guid,
            "CMSG_TRAINER_LIST"
        );

        let required_npc_flags = trainer_list_required_npc_flags_like_cpp(gossip_option);
        let access = match self.aura.trainer_npc_view_like_cpp(&self.npc)
            .represented_npc_can_interact_with_like_cpp(trainer_guid, required_npc_flags, 0)
        {
            Some(access) => access,
            None => {
                tracing::warn!(
                    account = self.account_id,
                    trainer_guid = ?trainer_guid,
                    "Trainer GUID not found or not interactable"
                );
                return;
            }
        };
        let entry = access.entry;

        let trainer_store = match self.catalogs.trainer_store {
            Some(store) => std::sync::Arc::clone(store),
            None => return,
        };
        let Some(trainer) =
            resolve_creature_trainer_like_cpp(trainer_store.as_ref(), entry, gossip_option)
        else {
            tracing::warn!(
                account = self.account_id,
                entry = entry,
                gossip_option = ?gossip_option,
                "No creature trainer in the loaded C++ trainer store"
            );
            return;
        };
        let trainer_id = trainer.id_like_cpp();

        // The full effect sequence is owned by the selected aura operation,
        // and intentionally remains between trainer resolution and spell reads.
        self.aura.remove_represented_feign_death_if_needed_like_cpp();

        let mut offers = Vec::new();
        for trainer_spell in trainer.spells_like_cpp() {
            let row = self.offer_view_like_cpp();
            let decision = row.trainer_offer_decision_like_cpp(
                trainer_id,
                trainer_spell,
                access.faction_template_id,
            );
            let fallback_price = match &decision {
                TrainerOfferDecisionLikeCpp::Known(_)
                | TrainerOfferDecisionLikeCpp::Unavailable(_) => trainer_price_like_cpp(
                    trainer_spell.money_cost,
                    row.npc
                        .trainer_price_reputation_rank_like_cpp(access.faction_template_id),
                ),
                _ => 0,
            };
            offers.push(TrainerListOfferResultLikeCpp {
                trainer_spell,
                decision,
                fallback_price,
            });
        }

        let spell_count = offers
            .iter()
            .filter(|offer| {
                !matches!(
                    &offer.decision,
                    TrainerOfferDecisionLikeCpp::Hidden(_)
                )
            })
            .count();

        tracing::info!(
            account = self.account_id,
            trainer_id = trainer_id,
            spell_count,
            "Sending SMSG_TRAINER_LIST"
        );

        let greeting = trainer
            .greeting_for_locale_name_like_cpp(self.session_locale_name)
            .to_string();
        self.publish_trainer_list_like_cpp(
            trainer_guid,
            i32::from(trainer.trainer_type_like_cpp()),
            trainer_id,
            offers,
            greeting,
        );
    }
}

impl AppTrainerListCx<'_> {
    fn offer_view_like_cpp(&self) -> TrainerOfferCxLikeCpp<'_> {
        let (spell_state, npc, player_conditions) = self.aura
            .trainer_offer_views_like_cpp(&self.npc, &self.player_conditions);
        TrainerOfferCxLikeCpp {
            npc, spell_access: &self.spell_access, spell_state,
            player_conditions, catalogs: &self.catalogs,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_fixture_complete: self.skill_fixture_complete,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_fixture_occupied: self.skill_fixture_occupied,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_fixture_tombstones: self.skill_fixture_tombstones,
            #[cfg(any(test, feature = "test-fixtures"))]
            skill_fixture_loaded: self.skill_fixture_loaded,
            max_primary_trade_skills: self.max_primary_trade_skills,
        }
    }

    /// Standalone purchase revalidation keeps its readonly query semantics.
    pub fn trainer_offer_decision_like_cpp(
        &self,
        trainer_id: u32,
        trainer_spell: &TrainerSpellLikeCpp,
        faction_template_id: u32,
    ) -> TrainerOfferDecisionLikeCpp {
        self.offer_view_like_cpp().trainer_offer_decision_like_cpp(
            trainer_id, trainer_spell, faction_template_id,
        )
    }
}

impl TrainerOfferCxLikeCpp<'_> {
    fn complete_spell_rows_like_cpp(
        &self,
    ) -> Option<std::collections::BTreeMap<i32, RepresentedPlayerSpellLikeCpp>> {
        fn complete_rows(
            runtime: &wow_entities::PlayerSpellRuntimeState,
        ) -> Option<std::collections::BTreeMap<i32, RepresentedPlayerSpellLikeCpp>> {
            (runtime.rows_loaded_like_cpp() && runtime.rows_complete_like_cpp()).then(|| {
                runtime
                    .rows_like_cpp()
                    .iter()
                    .map(|(&id, row)| (id, represented_player_spell_record_like_cpp(row)))
                    .collect()
            })
        }

        let canonical = self
            .spell_access
            .with_player_spell_runtime_like_cpp(complete_rows)
            .flatten();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical.is_none()
            && self.player_conditions.consumer_test
            && self.spell_access.player_handle_absent_like_cpp()
        {
            let runtime = canonical_player_spell_runtime_like_cpp(
                self.spell_state.represented_spell_runtime_fixture_like_cpp(),
            );
            return complete_rows(&runtime);
        }
        canonical
    }

    fn class_race_proof_like_cpp(&self, spell_id: u32) -> TrainerAdmissionProofLikeCpp {
        let Some(skills) = self.catalogs.skills else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        match skills.skill_line_ability_coverage_by_spell_like_cpp(spell_id) {
            SkillLineAbilityCoverageLikeCpp::CoveredZero => {
                TrainerAdmissionProofLikeCpp::Proven(true)
            }
            SkillLineAbilityCoverageLikeCpp::Indeterminate(_) => {
                TrainerAdmissionProofLikeCpp::Indeterminate
            }
            SkillLineAbilityCoverageLikeCpp::Rows(rows) => trainer_spell_class_race_fit_like_cpp(
                skills,
                rows,
                self.player_conditions
                    .player_access_like_cpp()
                    .player_race_like_cpp(),
                self.player_conditions
                    .player_access_like_cpp()
                    .player_class_like_cpp(),
            ),
        }
    }

    /// Evaluate the complete per-spell offer preflight from selected owners.
    /// The projection outcome remains a distinct branch for the selected App
    /// acquisition provider to finish at this same point in the list loop.
    pub fn trainer_offer_decision_like_cpp(
        &self,
        trainer_id: u32,
        trainer_spell: &TrainerSpellLikeCpp,
        faction_template_id: u32,
    ) -> TrainerOfferDecisionLikeCpp {
        if i32::try_from(trainer_spell.spell_id).is_err() {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                super::TrainerUnavailableReasonLikeCpp::InvalidEffectiveMetadata,
            );
        }
        let Some(spell_rows) = self.complete_spell_rows_like_cpp() else {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                super::TrainerUnavailableReasonLikeCpp::InvalidEffectiveMetadata,
            );
        };
        let Some(skill_rows) = self
            .player_conditions
            .complete_player_skill_records_like_cpp()
        else {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                super::TrainerUnavailableReasonLikeCpp::InvalidEffectiveMetadata,
            );
        };
        let required_skill = if trainer_spell.req_skill_line == 0 {
            None
        } else {
            let (Ok(skill_id), Ok(rank)) = (
                u16::try_from(trainer_spell.req_skill_line),
                u16::try_from(trainer_spell.req_skill_rank),
            ) else {
                return TrainerOfferDecisionLikeCpp::Unavailable(
                    super::TrainerUnavailableReasonLikeCpp::InvalidEffectiveMetadata,
                );
            };
            Some((u32::from(skill_id), rank))
        };
        let knows_spell = |candidate: u32| {
            i32::try_from(candidate).ok().is_some_and(|candidate| {
                spell_rows.get(&candidate).is_some_and(|row| {
                    row.state != RepresentedPlayerSpellStateLikeCpp::Removed && !row.disabled
                })
            })
        };
        let acquisition = self.catalogs.spell_acquisition;
        let battle_pet = match acquisition
            .map(|catalog| catalog.battle_pet_classification_like_cpp(trainer_spell.spell_id))
        {
            Some(BattlePetClassificationLikeCpp::NotBattlePet) => {
                super::TrainerBattlePetProofLikeCpp::NotBattlePet
            }
            Some(BattlePetClassificationLikeCpp::Species(species_id)) => {
                super::TrainerBattlePetProofLikeCpp::Species(species_id)
            }
            Some(BattlePetClassificationLikeCpp::Indeterminate(_)) | None => {
                super::TrainerBattlePetProofLikeCpp::Indeterminate
            }
        };
        let effective_price = trainer_price_like_cpp(
            trainer_spell.money_cost,
            self.npc
                .trainer_price_reputation_rank_like_cpp(faction_template_id),
        );
        let preflight = prepare_trainer_offer_like_cpp(TrainerOfferInputLikeCpp {
            source_spell_id: trainer_spell.spell_id,
            is_exact_member: true,
            class_race: self.class_race_proof_like_cpp(trainer_spell.spell_id),
            condition: self.player_conditions.trainer_spell_condition_proof_like_cpp(
                self.catalogs.condition_store,
                self.catalogs.player_condition_store,
                trainer_id,
                trainer_spell.spell_id,
            ),
            directly_known: knows_spell(trainer_spell.spell_id),
            required_skill,
            skill_rows,
            required_abilities: trainer_spell.req_ability,
            spell_rows,
            required_level: trainer_spell.req_level,
            player_level: self
                .player_conditions
                .player_access_like_cpp()
                .player_level_like_cpp(),
            product: acquisition
                .map(|catalog| trainer_spell_product_like_cpp(catalog, trainer_spell.spell_id))
                .unwrap_or(TrainerProductLikeCpp::InvalidOrUnsupportedWrapper),
            battle_pet,
            effective_price,
        });
        match preflight {
            TrainerOfferPreflightLikeCpp::Decision(decision) => decision,
            TrainerOfferPreflightLikeCpp::NeedsProjection(projection) => {
                self.finish_trainer_offer_projection_like_cpp(projection)
            }
        }
    }

}

impl AppTrainerListCx<'_> {
    /// Replace the trainer-window role immediately before publishing the
    /// completed packet, preserving the original list success ordering.
    pub fn publish_trainer_list_like_cpp(
        &mut self,
        trainer_guid: ObjectGuid,
        trainer_type: i32,
        trainer_id: u32,
        offers: Vec<TrainerListOfferResultLikeCpp<'_>>,
        greeting: String,
    ) {
        let spells = offers
            .into_iter()
            .filter_map(|offer| {
                trainer_list_spell_like_cpp(
                    offer.trainer_spell,
                    offer.decision,
                    offer.fallback_price,
                )
            })
            .collect();
        self.interaction
            .set_trainer_interaction_role_with_access_like_cpp(
                &self.role,
                trainer_guid,
                trainer_id,
            );
        let packet = TrainerListPacket {
            trainer_guid,
            trainer_type,
            trainer_id: trainer_id as i32,
            spells,
            greeting,
        };
        self.packets.send_packet(&packet);
    }
}

/// Selected participants for the complete TrainerBuy controller. The admitted
/// acquisition runtime remains a sub-operation; interaction state, stores and
/// the generator are borrowed from their existing owners.
pub struct AppTrainerBuyCx<'a> {
    pub(super) acquisition: AppTrainerCx<'a>,
    pub(super) interaction: &'a mut InteractionState,
    pub(super) trainer_store: Option<&'a TrainerStoreLikeCpp>,
    pub(super) battle_pet_selection: &'a BattlePetSelectionStoreLikeCpp,
    pub(super) item_guid_generator: &'a ObjectGuidGenerator,
}

/// Resolve the loaded C++-shaped trainer mapping, including the target-fork
/// gossip-option fallback to the creature's default trainer.
pub fn resolve_creature_trainer_like_cpp<'a>(
    store: &'a TrainerStoreLikeCpp,
    entry: u32,
    gossip_option: Option<(u32, u32)>,
) -> Option<&'a TrainerLikeCpp> {
    let selected_trainer_id = gossip_option
        .map(|(menu_id, option_id)| {
            store.get_creature_trainer_for_gossip_option_like_cpp(entry, menu_id, option_id)
        })
        .unwrap_or(0);
    let trainer_id = if selected_trainer_id != 0 {
        selected_trainer_id
    } else {
        // Target-fork C++ fallback in `Player::OnGossipSelect`.
        store.get_creature_default_trainer_like_cpp(entry)
    };
    if trainer_id == 0 {
        return None;
    }
    store.get_trainer_like_cpp(trainer_id)
}

/// Convert the resolved offer outcome into one wire-list row. The caller only
/// supplies a fallback price for Known/Unavailable, where C++ reads it.
fn trainer_list_spell_like_cpp(
    trainer_spell: &TrainerSpellLikeCpp,
    decision: TrainerOfferDecisionLikeCpp,
    fallback_price: u32,
) -> Option<TrainerListSpell> {
    let (usable, money_cost) = match decision {
        TrainerOfferDecisionLikeCpp::Hidden(_) => return None,
        TrainerOfferDecisionLikeCpp::Known(_) => {
            (TRAINER_SPELL_STATE_KNOWN_LIKE_CPP, fallback_price)
        }
        TrainerOfferDecisionLikeCpp::Unavailable(_) => {
            (TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP, fallback_price)
        }
        TrainerOfferDecisionLikeCpp::Available(offer) => {
            (TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP, offer.effective_price)
        }
        TrainerOfferDecisionLikeCpp::AvailableBattlePet(offer) => {
            (TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP, offer.effective_price)
        }
    };
    Some(TrainerListSpell {
        spell_id: trainer_spell.spell_id as i32,
        money_cost,
        req_skill_line: trainer_spell.req_skill_line as i32,
        req_skill_rank: trainer_spell.req_skill_rank as i32,
        req_ability: trainer_spell.req_ability.map(|ability| ability as i32),
        usable,
        req_level: trainer_spell.req_level,
    })
}

/// Classify the wrapper/direct product from the already-selected immutable
/// acquisition catalog, preserving the trainer path's checked target rules.
pub fn trainer_spell_product_like_cpp(
    catalog: &SpellAcquisitionCatalogLikeCpp,
    spell_id: u32,
) -> TrainerProductLikeCpp {
    const SPELL_EFFECT_LEARN_SPELL_LIKE_CPP: u32 = 36;
    let effects = match catalog.acquisition_effects_like_cpp(spell_id) {
        SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) => effects,
        SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage
        | SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_) => {
            return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
        }
    };
    let mut saw_learn = false;
    let mut targets = Vec::new();
    for effect in effects {
        let Ok(effect_type) = effect.effect_type_checked() else {
            return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
        };
        if effect_type != SPELL_EFFECT_LEARN_SPELL_LIKE_CPP {
            continue;
        }
        saw_learn = true;
        // C++ casts the wrapper on the player. Pet and other explicit target
        // families are not valid player-learning evidence.
        if effect.targets_unit_pet_like_cpp() || !effect.targets_player_like_cpp() {
            return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
        }
        let Ok(target) = effect.trigger_spell_id_checked() else {
            return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
        };
        targets.push(target);
    }
    targets.sort_unstable();
    targets.dedup();
    if saw_learn {
        TrainerProductLikeCpp::Wrapper {
            valid_learn_targets: targets,
        }
    } else {
        TrainerProductLikeCpp::Direct
    }
}

/// Evaluate the covered spell-ability rows against the identity already read
/// by the World adapter, preserving the original mask and row order.
pub fn trainer_spell_class_race_fit_like_cpp(
    skills: &SkillStore,
    rows: &[SkillLineAbilityRecord],
    race: u8,
    class: u8,
) -> TrainerAdmissionProofLikeCpp {
    let race_mask = wow_data::skill::race_mask_for_race_like_cpp(race);
    let Some(class_mask) = class
        .checked_sub(1)
        .and_then(|bit| 1_i32.checked_shl(bit.into()))
    else {
        return TrainerAdmissionProofLikeCpp::Indeterminate;
    };
    if race_mask == 0 {
        return TrainerAdmissionProofLikeCpp::Indeterminate;
    }
    let mut indeterminate = false;
    for row in rows {
        if row.race_mask != 0 && row.race_mask & race_mask == 0 {
            continue;
        }
        if row.class_mask != 0 && row.class_mask & class_mask == 0 {
            continue;
        }
        match skills.skill_race_class_info_coverage_for_player_like_cpp(row.skill_line, race, class)
        {
            SkillRaceClassInfoMatchCoverageLikeCpp::Row(_) => {
                return TrainerAdmissionProofLikeCpp::Proven(true);
            }
            SkillRaceClassInfoMatchCoverageLikeCpp::CoveredZero => {}
            SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(_) => indeterminate = true,
        }
    }
    if indeterminate {
        TrainerAdmissionProofLikeCpp::Indeterminate
    } else {
        TrainerAdmissionProofLikeCpp::Proven(false)
    }
}

impl<'a> AppTrainerBuyCx<'a> {
    pub fn new(
        acquisition: AppTrainerCx<'a>,
        interaction: &'a mut InteractionState,
        trainer_store: Option<&'a TrainerStoreLikeCpp>,
        battle_pet_selection: &'a BattlePetSelectionStoreLikeCpp,
        item_guid_generator: &'a ObjectGuidGenerator,
    ) -> Self {
        Self {
            acquisition,
            interaction,
            trainer_store,
            battle_pet_selection,
            item_guid_generator,
        }
    }

    /// Revalidate the selected trainer window and loaded trainer spell at one
    /// admission point. Callers retain the existing NPC/feign-death ordering
    /// before invoking this controller.
    pub fn admit_buy_like_cpp(
        &self,
        source_guid: ObjectGuid,
        trainer_id: i32,
        spell_id: u32,
    ) -> Result<(), TrainerBuyAdmissionLikeCpp> {
        let role: TrainerInteractionRoleAccessLikeCpp<'_> =
            self.acquisition.owner.trainer_interaction_role_like_cpp();
        find_trainer_buy_spell_like_cpp(
            self.interaction,
            &role,
            self.trainer_store,
            source_guid,
            trainer_id,
            spell_id,
        )
        .map(|_| ())
    }

    /// Re-read the trainer spell after the money-fence await and copy the row
    /// only at the point where the offer is recomputed.
    pub fn resolve_buy_spell_after_boundary_like_cpp(
        &self,
        source_guid: ObjectGuid,
        trainer_id: i32,
        spell_id: u32,
    ) -> Result<TrainerSpellLikeCpp, TrainerBuyAdmissionLikeCpp> {
        let role: TrainerInteractionRoleAccessLikeCpp<'_> =
            self.acquisition.owner.trainer_interaction_role_like_cpp();
        find_trainer_buy_spell_like_cpp(
            self.interaction,
            &role,
            self.trainer_store,
            source_guid,
            trainer_id,
            spell_id,
        )
        .cloned()
    }

    /// Run the admitted purchase through the existing money/commit/runtime
    /// fences while retaining the broader controller's selected participants.
    pub async fn execute_admitted_acquisition_like_cpp(
        &mut self,
        exclusion: ExclusivePlayerMoneyPersistenceLikeCpp,
        offer: &PreparedTrainerOfferLikeCpp,
        snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
        money_before: u64,
        money_after: u64,
        publication: &TrainerAcquisitionPublicationLikeCpp,
    ) -> TrainerAcquisitionCompletionLikeCpp<ExclusivePlayerMoneyPersistenceLikeCpp> {
        execute_trainer_acquisition_like_cpp(
            &mut self.acquisition,
            exclusion,
            offer,
            snapshot,
            money_before,
            money_after,
            publication,
        )
        .await
    }
}
