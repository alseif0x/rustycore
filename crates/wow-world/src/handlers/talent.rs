// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use tracing::warn;

use wow_constants::ClientOpcodes;
use wow_constants::unit::NPCFlags1;
#[cfg(test)]
use wow_database::StatementDef;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::talent::{
    ConfirmRespecWipe, LearnTalent, LearnTalents, SPEC_RESET_TALENTS_LIKE_CPP,
};
use wow_packet::{ClientPacket, WorldPacket};

use crate::session::{
    RepresentedConfirmRespecWipeLikeCpp, RepresentedTalentRespecVisualSpellCastLikeCpp,
    WorldSession,
};

const CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits();
const MIN_TALENT_RESET_LEVEL_LIKE_CPP: u8 = 15;
const UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP: u32 = 14_867;
#[cfg(test)]
const AT_LOGIN_RENAME_LIKE_CPP: u16 = 0x001;
#[cfg(test)]
const AT_LOGIN_RESET_SPELLS_LIKE_CPP: u16 = 0x002;
const AT_LOGIN_RESET_TALENTS_LIKE_CPP: u16 = 0x004;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConfirmRespecWipe,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_confirm_respec_wipe",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LearnTalent,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_learn_talent",
    }
}

impl WorldSession {
    /// Handle CMSG_LEARN_TALENT.
    ///
    /// C++ calls `Player::LearnTalent(TalentID, RequestedRank)` and sends
    /// `SendTalentsInfoData()` only on success. Rust currently has the
    /// represented talent snapshot plus DB2 spell-rank and talent-tab class
    /// validation, but not the complete C++ point/prerequisite/tier runtime.
    pub async fn handle_learn_talent(&mut self, mut packet: WorldPacket) {
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

        if self.learn_represented_talent_like_cpp(request.talent_id as u32, request.requested_rank)
        {
            self.send_packet(&self.represented_update_talent_data_packet_like_cpp());
        }
    }

    /// Parse the unresolved-placeholder CMSG_LEARN_TALENTS payload.
    ///
    /// C++ expands each `uint16` talent id into a `LearnTalent` request with
    /// `RequestedRank = 0` and delegates to `HandleLearnTalentOpcode`.
    /// The inspected C++ opcode table still uses the shared `0xBADD`
    /// placeholder, so this handler is deliberately not registered for live
    /// dispatch until the real client opcode is resolved.
    pub async fn handle_learn_talents(&mut self, mut packet: WorldPacket) {
        let request = match LearnTalents::read(&mut packet) {
            Ok(request) => request,
            Err(error) => {
                warn!("Bad LearnTalents: {error}");
                return;
            }
        };

        for talent_id in request.talent_ids {
            if self.learn_represented_talent_like_cpp(u32::from(talent_id), 0) {
                self.send_packet(&self.represented_update_talent_data_packet_like_cpp());
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
    pub async fn handle_confirm_respec_wipe(&mut self, mut packet: WorldPacket) {
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

        self.record_represented_talent_reset_script_hook_like_cpp(false);
        self.remove_represented_at_login_flag_like_cpp(AT_LOGIN_RESET_TALENTS_LIKE_CPP, true);

        if !self.apply_represented_talent_reset_cost_like_cpp().await {
            return;
        }

        self.remove_represented_pet_not_in_slot_like_cpp();

        self.record_represented_confirm_respec_wipe_like_cpp(RepresentedConfirmRespecWipeLikeCpp {
            respec_master: request.respec_master,
            respec_type: request.respec_type,
        });
        if self.reset_represented_active_talents_like_cpp() {
            self.send_packet(&self.represented_update_talent_data_packet_like_cpp());
            if let Some(player_guid) = self.player_guid() {
                self.record_represented_talent_respec_visual_spell_cast_like_cpp(
                    RepresentedTalentRespecVisualSpellCastLikeCpp {
                        caster_guid: request.respec_master,
                        target_guid: player_guid,
                        spell_id: UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP,
                        triggered: true,
                        spell_runtime_unrepresented: true,
                    },
                );
            }
        }
    }

    fn represented_can_confirm_respec_wipe_like_cpp(
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, RwLock};

    use super::*;
    use crate::session::{
        RepresentedAtLoginFlagRemovalLikeCpp, RepresentedTalentResetScriptHookLikeCpp,
    };
    use wow_constants::BuyResult;
    use wow_constants::unit::UnitState;
    use wow_core::guid::HighGuid;
    use wow_core::{ObjectGuid, Position};
    use wow_entities::{PetStable, PetStableInfo, PetType};
    use wow_packet::ServerPacket;
    use wow_packet::packets::chat::PrintNotification;
    use wow_packet::packets::misc::BuyFailed;
    use wow_packet::packets::update::CreatureCreateData;

    use crate::session::{
        AuraApplication, RepresentedAuraEffectLikeCpp, RepresentedTalentRespecCriteriaEventLikeCpp,
        SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP, SessionPlayerController,
    };

    fn make_session_with_send_capacity(
        capacity: usize,
    ) -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
        let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
        (
            WorldSession::new(
                1,
                "TestAccount".into(),
                0,
                2,
                9,
                54261,
                vec![0u8; 40],
                "esES".into(),
                pkt_rx,
                send_tx,
            ),
            send_rx,
        )
    }

    fn confirm_respec_wipe_packet(respec_master: ObjectGuid, respec_type: u8) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_guid(&respec_master);
        packet.write_uint8(respec_type);
        packet
    }

    fn learn_talent_packet(talent_id: i32, requested_rank: u16) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_int32(talent_id);
        packet.write_uint16(requested_rank);
        packet
    }

    fn learn_talents_packet(talent_ids: &[u16]) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_bits(talent_ids.len() as u32, 6);
        packet.flush_bits();
        for talent_id in talent_ids {
            packet.write_uint16(*talent_id);
        }
        packet
    }

    fn test_talent_entry_like_cpp(id: u32, rank: u8, spell_id: i32) -> wow_data::TalentEntry {
        let mut spell_rank = [0; 9];
        spell_rank[usize::from(rank)] = spell_id;
        wow_data::TalentEntry {
            id,
            description: String::new(),
            tier_id: 0,
            flags: 0,
            column_index: 0,
            tab_id: 0,
            class_id: 0,
            spec_id: 0,
            spell_id,
            overrides_spell_id: 0,
            required_spell_id: 0,
            category_mask: [0; 2],
            spell_rank,
            prereq_talent: [0; 3],
            prereq_rank: [0; 3],
        }
    }

    fn test_spell_info_like_cpp(spell_id: i32) -> wow_data::SpellInfo {
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            effects: Vec::new(),
        }
    }

    fn test_learn_spell_info_like_cpp(spell_id: i32, trigger_spell: i32) -> wow_data::SpellInfo {
        let mut spell = test_spell_info_like_cpp(spell_id);
        spell.effects = vec![wow_data::SpellEffectInfo {
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL,
            effect_trigger_spell: trigger_spell,
            ..wow_data::SpellEffectInfo::default()
        }];
        spell
    }

    fn install_test_talent_store(session: &mut WorldSession, talents: &[(u32, u8, i32)]) {
        install_test_talent_store_with_tab_class_mask(session, talents, 1);
    }

    fn install_test_talent_store_with_tab_class_mask(
        session: &mut WorldSession,
        talents: &[(u32, u8, i32)],
        class_mask: i32,
    ) {
        install_test_talent_entries_with_tab_class_mask(
            session,
            talents
                .iter()
                .map(|(talent_id, rank, spell_id)| {
                    test_talent_entry_like_cpp(*talent_id, *rank, *spell_id)
                })
                .collect::<Vec<_>>(),
            class_mask,
        );
    }

    fn install_test_talent_entries_with_tab_class_mask(
        session: &mut WorldSession,
        talents: Vec<wow_data::TalentEntry>,
        class_mask: i32,
    ) {
        let spell_ids = talents
            .iter()
            .flat_map(|talent| talent.spell_rank)
            .filter(|spell_id| *spell_id > 0)
            .collect::<Vec<_>>();
        session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries(talents)));
        session.set_talent_tab_store(Arc::new(wow_data::TalentTabStore::from_entries([
            wow_data::TalentTabEntry {
                id: 0,
                name: String::new(),
                background_file: String::new(),
                order_index: 0,
                race_mask: 0,
                class_mask,
                pet_talent_mask: 0,
                spell_icon_id: 0,
            },
        ])));
        session.set_player_class_like_cpp(1);
        session.set_player_level_like_cpp(80);
        session.set_num_talents_at_level_store(Arc::new(
            wow_data::progression_rewards::NumTalentsAtLevelStore::from_entries([
                wow_data::progression_rewards::NumTalentsAtLevelEntry {
                    id: 80,
                    num_talents: 71,
                    num_talents_death_knight: 71,
                    num_talents_demon_hunter: 71,
                },
            ]),
        ));

        let mut spell_store = wow_data::SpellStore::new();
        for spell_id in spell_ids {
            spell_store.insert(spell_id, test_spell_info_like_cpp(spell_id));
        }
        session.set_spell_store(Arc::new(spell_store));
    }

    fn install_test_talent_entries_with_spell_store_like_cpp(
        session: &mut WorldSession,
        talents: Vec<wow_data::TalentEntry>,
        spell_store: wow_data::SpellStore,
    ) {
        session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries(talents)));
        session.set_talent_tab_store(Arc::new(wow_data::TalentTabStore::from_entries([
            wow_data::TalentTabEntry {
                id: 0,
                name: String::new(),
                background_file: String::new(),
                order_index: 0,
                race_mask: 0,
                class_mask: 1,
                pet_talent_mask: 0,
                spell_icon_id: 0,
            },
        ])));
        session.set_player_class_like_cpp(1);
        session.set_player_level_like_cpp(80);
        session.set_num_talents_at_level_store(Arc::new(
            wow_data::progression_rewards::NumTalentsAtLevelStore::from_entries([
                wow_data::progression_rewards::NumTalentsAtLevelEntry {
                    id: 80,
                    num_talents: 71,
                    num_talents_death_knight: 71,
                    num_talents_demon_hunter: 71,
                },
            ]),
        ));
        session.set_spell_store(Arc::new(spell_store));
    }

    fn install_test_talent_store_without_tab(
        session: &mut WorldSession,
        talents: &[(u32, u8, i32)],
    ) {
        session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries(
            talents.iter().map(|(talent_id, rank, spell_id)| {
                test_talent_entry_like_cpp(*talent_id, *rank, *spell_id)
            }),
        )));
        session.set_player_class_like_cpp(1);
        session.set_player_level_like_cpp(80);
        session.set_num_talents_at_level_store(Arc::new(
            wow_data::progression_rewards::NumTalentsAtLevelStore::from_entries([
                wow_data::progression_rewards::NumTalentsAtLevelEntry {
                    id: 80,
                    num_talents: 71,
                    num_talents_death_knight: 71,
                    num_talents_demon_hunter: 71,
                },
            ]),
        ));

        let mut spell_store = wow_data::SpellStore::new();
        for (_, _, spell_id) in talents {
            spell_store.insert(*spell_id, test_spell_info_like_cpp(*spell_id));
        }
        session.set_spell_store(Arc::new(spell_store));
    }

    fn test_creature_guid(counter: u32) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, counter, 1)
    }

    fn visible_aura(slot: u8, flags2: u32) -> AuraApplication {
        AuraApplication {
            spell_id: 90_000 + i32::from(slot),
            caster_guid: ObjectGuid::EMPTY,
            slot,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: flags2,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        }
    }

    fn test_creature_create_data(guid: ObjectGuid, npc_flags: u32) -> CreatureCreateData {
        CreatureCreateData {
            guid,
            entry: 123,
            display_id: 100,
            native_display_id: 100,
            health: 100,
            max_health: 100,
            level: 60,
            faction_template: 35,
            npc_flags: u64::from(npc_flags),
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 0,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        }
    }

    fn register_test_trainer(session: &mut WorldSession, guid: ObjectGuid, npc_flags: u32) {
        register_test_trainer_with_class(session, guid, npc_flags, 1);
    }

    fn register_test_trainer_with_class(
        session: &mut WorldSession,
        guid: ObjectGuid,
        npc_flags: u32,
        trainer_class: u8,
    ) {
        session.set_map_manager(Arc::new(RwLock::new(crate::map_manager::MapManager::new())));
        session.set_loaded_player_identity_like_cpp(0, 1, 1, 80, 0);
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        session.set_player_position_like_cpp(Position::new(0.0, 0.0, 0.0, 0.0));
        session.register_world_creature(
            0,
            Position::new(1.0, 0.0, 0.0, 0.0),
            test_creature_create_data(guid, npc_flags),
            1,
            2,
            5.0,
            0,
            0,
            0,
            0,
            None,
            0,
            0,
            0,
            0,
            0,
        );
        session
            .mutate_world_creature(guid, |creature| {
                creature
                    .creature
                    .set_trainer_class_runtime_like_cpp(trainer_class);
            })
            .expect("test trainer exists");
    }

    fn add_canonical_test_trainer_like_cpp(
        canonical: &Arc<Mutex<wow_map::MapManager>>,
        guid: ObjectGuid,
        position: Position,
        npc_flags: u32,
        trainer_class: u8,
    ) {
        let mut creature = wow_entities::Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature.unit_mut().world_mut().object_mut().set_entry(123);
        creature.unit_mut().world_mut().set_map(0, 0).unwrap();
        creature.unit_mut().world_mut().relocate(position);
        creature.unit_mut().world_mut().set_combat_reach(1.0);
        creature.unit_mut().set_level(80);
        creature.unit_mut().set_max_health(100);
        creature.unit_mut().set_health(100);
        creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
        creature.set_trainer_class_runtime_like_cpp(trainer_class);
        creature.unit_mut().world_mut().object_mut().add_to_world();

        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .expect("canonical map")
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
            )
            .unwrap();
    }

    fn add_canonical_test_pet_like_cpp(
        canonical: &Arc<Mutex<wow_map::MapManager>>,
        guid: ObjectGuid,
        owner_guid: ObjectGuid,
        position: Position,
    ) {
        let mut pet = wow_entities::Pet::new(owner_guid, PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(guid);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(500);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .set_map(0, 0)
            .unwrap();
        pet.creature_mut().unit_mut().world_mut().relocate(position);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();

        canonical
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .expect("canonical map")
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
            .unwrap();
    }

    fn represented_current_pet_stable_like_cpp(pet_number: u32) -> PetStable {
        PetStable {
            current_pet_index: Some(0),
            active_pets: vec![Some(PetStableInfo {
                pet_number,
                creature_id: 500,
                pet_type: PetType::Hunter,
                health: 100,
                level: 80,
                ..PetStableInfo::default()
            })],
            stabled_pets: Vec::new(),
            unslotted_pets: Vec::new(),
        }
    }

    #[tokio::test]
    async fn confirm_respec_wipe_records_talent_reset_with_trainer_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let trainer = test_creature_guid(77);
        register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
        session.mark_represented_talents_loaded_like_cpp();
        session.set_player_gold_like_cpp(20_000);

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        let reset_update = send_rx
            .try_recv()
            .expect("C++ sends SendTalentsInfoData after successful ResetTalents");
        assert_eq!(
            reset_update,
            session
                .represented_update_talent_data_packet_like_cpp()
                .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert_eq!(session.player_gold_like_cpp(), 10_000);
        assert_eq!(session.represented_talent_reset_cost_like_cpp(), 10_000);
        assert_ne!(session.represented_talent_reset_time_secs_like_cpp(), 0);
        assert_eq!(
            session.represented_talent_respec_criteria_events_like_cpp(),
            &[
                RepresentedTalentRespecCriteriaEventLikeCpp::MoneySpentOnRespecs { amount: 10_000 },
                RepresentedTalentRespecCriteriaEventLikeCpp::TotalRespecs { quantity: 1 },
            ],
            "C++ Player::ResetTalents updates money-spent and total-respec criteria after ModifyMoney"
        );
        assert_eq!(
            session.represented_talent_reset_script_hooks_like_cpp(),
            &[RepresentedTalentResetScriptHookLikeCpp { no_cost: false }],
            "C++ Player::ResetTalents starts with sScriptMgr->OnPlayerTalentsReset(this, noCost)"
        );
        assert!(
            session
                .represented_at_login_flag_removals_like_cpp()
                .is_empty(),
            "C++ only calls RemoveAtLoginFlag when AT_LOGIN_RESET_TALENTS is present"
        );
        assert_eq!(
            session.represented_confirm_respec_wipe_requests_like_cpp(),
            &[RepresentedConfirmRespecWipeLikeCpp {
                respec_master: trainer,
                respec_type: SPEC_RESET_TALENTS_LIKE_CPP,
            }]
        );
        assert_eq!(
            session.represented_talent_respec_visual_spell_casts_like_cpp(),
            &[RepresentedTalentRespecVisualSpellCastLikeCpp {
                caster_guid: trainer,
                target_guid: ObjectGuid::create_player(1, 42),
                spell_id: UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP,
                triggered: true,
                spell_runtime_unrepresented: true,
            }],
            "C++ casts unit->CastSpell(_player, 14867, true) after SendTalentsInfoData"
        );
    }

    #[tokio::test]
    async fn confirm_respec_wipe_respects_no_reset_talent_cost_config_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let trainer = test_creature_guid(78);
        register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
        session.mark_represented_talents_loaded_like_cpp();
        session.set_no_reset_talent_cost_like_cpp(true);
        session.set_player_gold_like_cpp(0);
        session.set_represented_talent_reset_state_like_cpp(500_000, 123);

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        let reset_update = send_rx
            .try_recv()
            .expect("C++ ResetTalents skips money checks when CONFIG_NO_RESET_TALENT_COST is true");
        assert_eq!(
            reset_update,
            session
                .represented_update_talent_data_packet_like_cpp()
                .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            session.player_gold_like_cpp(),
            0,
            "C++ no-cost reset does not modify player money"
        );
        assert_eq!(
            session.represented_talent_reset_cost_like_cpp(),
            0,
            "C++ SetTalentResetCost receives the zero cost when NoResetTalentsCost bypasses the money gate"
        );
        assert_ne!(
            session.represented_talent_reset_time_secs_like_cpp(),
            123,
            "C++ still updates TalentResetTime inside the final !noCost block"
        );
        assert_eq!(
            session.represented_talent_respec_criteria_events_like_cpp(),
            &[
                RepresentedTalentRespecCriteriaEventLikeCpp::MoneySpentOnRespecs { amount: 0 },
                RepresentedTalentRespecCriteriaEventLikeCpp::TotalRespecs { quantity: 1 },
            ],
            "C++ still updates respec criteria inside the final !noCost block with cost zero"
        );
        assert_eq!(
            session.represented_confirm_respec_wipe_requests_like_cpp(),
            &[RepresentedConfirmRespecWipeLikeCpp {
                respec_master: trainer,
                respec_type: SPEC_RESET_TALENTS_LIKE_CPP,
            }]
        );
    }

    #[tokio::test]
    async fn login_at_login_reset_talents_resets_without_cost_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        install_test_talent_store(&mut session, &[(101, 0, 50_101)]);
        session.mark_represented_talents_loaded_like_cpp();
        session.set_player_gold_like_cpp(9_999);

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;
        let _learn_update = send_rx
            .try_recv()
            .expect("C++ sends SendTalentsInfoData after LearnTalent");
        assert!(session.known_spells_like_cpp().contains(&50_101));

        session.set_represented_at_login_flags_like_cpp(
            AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_RESET_TALENTS_LIKE_CPP,
        );

        assert!(
            session.apply_represented_login_talent_reset_if_needed_like_cpp(),
            "C++ CharacterHandler applies AT_LOGIN_RESET_TALENTS through ResetTalents(true)"
        );

        let reset_update = send_rx
            .try_recv()
            .expect("C++ resends SendTalentsInfoData after ResetTalents(true)");
        assert_eq!(
            reset_update,
            session
                .represented_update_talent_data_packet_like_cpp()
                .to_bytes()
        );
        let notification = send_rx
            .try_recv()
            .expect("C++ SendNotification(LANG_RESET_TALENTS)");
        assert_eq!(
            notification,
            PrintNotification {
                notify_text: "Your talents have been reset.".to_string(),
            }
            .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            session.represented_talent_reset_script_hooks_like_cpp(),
            &[RepresentedTalentResetScriptHookLikeCpp { no_cost: true }]
        );
        assert_eq!(
            session.represented_at_login_flags_like_cpp(),
            AT_LOGIN_RENAME_LIKE_CPP,
            "C++ removes only AT_LOGIN_RESET_TALENTS"
        );
        assert_eq!(
            session.represented_at_login_flag_removals_like_cpp(),
            &[RepresentedAtLoginFlagRemovalLikeCpp {
                flags: AT_LOGIN_RESET_TALENTS_LIKE_CPP,
                persist: true,
                db_statement_unrepresented: true,
            }]
        );
        assert_eq!(
            session.player_gold_like_cpp(),
            9_999,
            "C++ ResetTalents(true) skips ModifyMoney and criteria/accounting"
        );
        assert_eq!(session.represented_talent_reset_cost_like_cpp(), 0);
        assert_eq!(session.represented_talent_reset_time_secs_like_cpp(), 0);
        assert!(
            session
                .represented_talent_respec_criteria_events_like_cpp()
                .is_empty(),
            "C++ skips criteria updates when noCost=true"
        );
        assert!(
            !session.known_spells_like_cpp().contains(&50_101),
            "C++ ResetTalents(true) still removes active talents"
        );
    }

    #[tokio::test]
    async fn login_at_login_reset_spells_removes_known_spells_and_notifies_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        session.set_known_spells_like_cpp(vec![118, 133, 19740]);
        session.learn_dependent_known_spell_like_cpp(19740);
        session.set_represented_at_login_flags_like_cpp(
            AT_LOGIN_RENAME_LIKE_CPP
                | AT_LOGIN_RESET_SPELLS_LIKE_CPP
                | AT_LOGIN_RESET_TALENTS_LIKE_CPP,
        );

        assert!(
            session.apply_represented_login_spell_reset_if_needed_like_cpp(),
            "C++ CharacterHandler applies AT_LOGIN_RESET_SPELLS through Player::ResetSpells"
        );

        let notification = send_rx
            .try_recv()
            .expect("C++ SendNotification(LANG_RESET_SPELLS)");
        assert_eq!(
            notification,
            PrintNotification {
                notify_text: "Your spells have been reset.".to_string(),
            }
            .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert!(
            session.known_spells_like_cpp().is_empty(),
            "C++ ResetSpells(false) removes every spell in the copied PlayerSpellMap before relearning defaults"
        );
        let spell_save_statements = WorldSession::character_spell_save_statements_like_cpp(
            42,
            session.represented_player_spell_rows_like_cpp(),
        );
        assert_eq!(spell_save_statements.len(), 2);
        assert!(
            spell_save_statements
                .iter()
                .all(|stmt| stmt.sql()
                    == wow_database::CharStatements::DEL_CHAR_SPELL_BY_SPELL.sql()),
            "C++ RemoveSpell marks loaded non-dependent spells as PLAYERSPELL_REMOVED so _SaveSpells deletes character_spell rows"
        );
        assert!(
            spell_save_statements
                .iter()
                .all(|stmt| stmt.params()[0] != wow_database::SqlParam::I32(19740)),
            "C++ dependent spells learned during represented login are not normal character_spell rows to delete"
        );
        assert_eq!(
            session.represented_at_login_flags_like_cpp(),
            AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_RESET_TALENTS_LIKE_CPP,
            "C++ removes only AT_LOGIN_RESET_SPELLS"
        );
        assert_eq!(
            session.represented_at_login_flag_removals_like_cpp(),
            &[RepresentedAtLoginFlagRemovalLikeCpp {
                flags: AT_LOGIN_RESET_SPELLS_LIKE_CPP,
                persist: true,
                db_statement_unrepresented: true,
            }]
        );
    }

    #[tokio::test]
    async fn learn_talent_updates_represented_active_group_and_sends_talents_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        install_test_talent_store(&mut session, &[(101, 2, 50_101)]);
        session.mark_represented_talents_loaded_like_cpp();
        session.set_represented_active_talent_group_like_cpp(1);
        session.set_represented_bonus_talent_groups_like_cpp(1);

        session
            .handle_learn_talent(learn_talent_packet(101, 2))
            .await;

        let sent = send_rx
            .try_recv()
            .expect("C++ sends SendTalentsInfoData after successful LearnTalent");
        assert_eq!(
            sent,
            session
                .represented_update_talent_data_packet_like_cpp()
                .to_bytes()
        );
        let packet = session.represented_update_talent_data_packet_like_cpp();
        assert_eq!(
            packet.groups[1].talents,
            vec![wow_packet::packets::misc::TalentInfoLikeCpp {
                talent_id: 101,
                rank: 2,
            }]
        );
        assert_eq!(
            packet.unspent_talent_points, 68,
            "C++ LearnTalent recomputes CharacterPoints from CalculateTalentsPoints - spent talents"
        );
    }

    #[tokio::test]
    async fn learn_talents_delegates_each_id_as_rank_zero_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        install_test_talent_store(&mut session, &[(101, 0, 50_101), (202, 0, 50_202)]);
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talents(learn_talents_packet(&[101, 202]))
            .await;

        let first = send_rx
            .try_recv()
            .expect("first C++ delegated LearnTalent should send talent data");
        let second = send_rx
            .try_recv()
            .expect("second C++ delegated LearnTalent should send talent data");
        assert!(!first.is_empty());
        assert!(!second.is_empty());
        assert!(send_rx.try_recv().is_err());

        let packet = session.represented_update_talent_data_packet_like_cpp();
        assert_eq!(
            packet.groups[0].talents,
            vec![
                wow_packet::packets::misc::TalentInfoLikeCpp {
                    talent_id: 101,
                    rank: 0,
                },
                wow_packet::packets::misc::TalentInfoLikeCpp {
                    talent_id: 202,
                    rank: 0,
                },
            ]
        );
        assert_eq!(packet.unspent_talent_points, 69);
    }

    #[tokio::test]
    async fn learn_talent_rejects_zero_character_points_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        install_test_talent_store(&mut session, &[(101, 0, 50_101)]);
        session.mark_represented_talents_loaded_like_cpp();
        session.set_player_character_points_like_cpp(0);

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .groups[0]
                .talents
                .is_empty()
        );
        assert_eq!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .unspent_talent_points,
            0
        );
    }

    #[tokio::test]
    async fn learn_talent_rejects_unloaded_snapshot_without_sending_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        install_test_talent_store(&mut session, &[(101, 0, 50_101)]);

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .groups[0]
                .talents
                .is_empty()
        );
        assert!(!session.represented_talents_loaded_like_cpp());
    }

    #[tokio::test]
    async fn learn_talent_rejects_missing_talent_tab_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        install_test_talent_store_without_tab(&mut session, &[(101, 0, 50_101)]);
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .groups[0]
                .talents
                .is_empty()
        );
    }

    #[tokio::test]
    async fn learn_talent_rejects_wrong_class_talent_tab_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        install_test_talent_store_with_tab_class_mask(&mut session, &[(101, 0, 50_101)], 1 << 1);
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .groups[0]
                .talents
                .is_empty()
        );
    }

    #[tokio::test]
    async fn learn_talent_rejects_rank_outside_cpp_max_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        install_test_talent_store(&mut session, &[(101, 0, 50_101)]);
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(101, 9))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .groups[0]
                .talents
                .is_empty()
        );
    }

    #[tokio::test]
    async fn learn_talent_rejects_known_or_higher_rank_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        install_test_talent_store(&mut session, &[(101, 1, 50_101)]);
        session.mark_represented_talents_loaded_like_cpp();
        assert!(session.load_represented_talent_row_like_cpp(101, 1, 0));

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .groups[0]
                .talents,
            vec![wow_packet::packets::misc::TalentInfoLikeCpp {
                talent_id: 101,
                rank: 1,
            }]
        );
    }

    #[tokio::test]
    async fn learn_talent_enforces_prereq_rank_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let prereq = test_talent_entry_like_cpp(101, 1, 50_101);
        let mut dependent = test_talent_entry_like_cpp(202, 0, 50_202);
        dependent.prereq_talent[0] = 101;
        dependent.prereq_rank[0] = 1;
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![prereq, dependent], 1);
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(202, 0))
            .await;
        assert!(send_rx.try_recv().is_err());

        assert!(session.load_represented_talent_row_like_cpp(101, 1, 0));
        session
            .handle_learn_talent(learn_talent_packet(202, 0))
            .await;

        let sent = send_rx
            .try_recv()
            .expect("C++ accepts dependent talent once prereq rank is known");
        assert!(!sent.is_empty());
        assert!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .groups[0]
                .talents
                .contains(&wow_packet::packets::misc::TalentInfoLikeCpp {
                    talent_id: 202,
                    rank: 0,
                })
        );
    }

    #[tokio::test]
    async fn learn_talent_enforces_tier_spent_points_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let filler = test_talent_entry_like_cpp(101, 4, 50_101);
        let mut tier_one = test_talent_entry_like_cpp(202, 0, 50_202);
        tier_one.tier_id = 1;
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![filler, tier_one], 1);
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(202, 0))
            .await;
        assert!(send_rx.try_recv().is_err());

        assert!(session.load_represented_talent_row_like_cpp(101, 4, 0));
        session
            .handle_learn_talent(learn_talent_packet(202, 0))
            .await;

        let sent = send_rx
            .try_recv()
            .expect("C++ accepts tier-one talent after five points in the tree");
        assert!(!sent.is_empty());
        assert!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .groups[0]
                .talents
                .contains(&wow_packet::packets::misc::TalentInfoLikeCpp {
                    talent_id: 202,
                    rank: 0,
                })
        );
    }

    #[tokio::test]
    async fn learn_talent_rejects_invalid_learn_spell_trigger_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let talent = test_talent_entry_like_cpp(101, 0, 50_101);
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(50_101, test_learn_spell_info_like_cpp(50_101, 60_101));
        install_test_talent_entries_with_spell_store_like_cpp(
            &mut session,
            vec![talent],
            spell_store,
        );
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert!(
            session
                .represented_update_talent_data_packet_like_cpp()
                .groups[0]
                .talents
                .is_empty(),
            "C++ SpellMgr::IsSpellValid rejects SPELL_EFFECT_LEARN_SPELL when TriggerSpell is missing"
        );
        assert!(!session.known_spells_like_cpp().contains(&50_101));
    }

    #[tokio::test]
    async fn learn_talent_learns_active_talent_spell_and_trigger_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let talent = test_talent_entry_like_cpp(101, 0, 50_101);
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(50_101, test_learn_spell_info_like_cpp(50_101, 60_101));
        spell_store.insert(60_101, test_spell_info_like_cpp(60_101));
        install_test_talent_entries_with_spell_store_like_cpp(
            &mut session,
            vec![talent],
            spell_store,
        );
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;

        assert!(
            !send_rx
                .try_recv()
                .expect("C++ sends talent data after AddTalent succeeds")
                .is_empty()
        );
        assert!(session.known_spells_like_cpp().contains(&50_101));
        assert!(
            session.known_spells_like_cpp().contains(&60_101),
            "C++ Player::AddTalent learns the talent spell, whose LearnSpell effect teaches TriggerSpell"
        );
    }

    #[tokio::test]
    async fn learn_talent_upgrade_removes_previous_rank_spell_and_trigger_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let mut talent = test_talent_entry_like_cpp(101, 0, 50_101);
        talent.spell_rank[1] = 50_102;
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(50_101, test_learn_spell_info_like_cpp(50_101, 60_101));
        spell_store.insert(60_101, test_spell_info_like_cpp(60_101));
        spell_store.insert(50_102, test_learn_spell_info_like_cpp(50_102, 60_102));
        spell_store.insert(60_102, test_spell_info_like_cpp(60_102));
        install_test_talent_entries_with_spell_store_like_cpp(
            &mut session,
            vec![talent],
            spell_store,
        );
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;
        assert!(!send_rx.try_recv().expect("rank 0 learn sends").is_empty());
        assert!(session.known_spells_like_cpp().contains(&50_101));
        assert!(session.known_spells_like_cpp().contains(&60_101));

        session
            .handle_learn_talent(learn_talent_packet(101, 1))
            .await;

        assert!(!send_rx.try_recv().expect("rank 1 learn sends").is_empty());
        assert!(
            !session.known_spells_like_cpp().contains(&50_101),
            "C++ Player::AddTalent removes the previous rank spell before learning the new rank"
        );
        assert!(
            !session.known_spells_like_cpp().contains(&60_101),
            "C++ Player::AddTalent removes direct LearnSpell triggers from the previous rank"
        );
        assert!(session.known_spells_like_cpp().contains(&50_102));
        assert!(session.known_spells_like_cpp().contains(&60_102));
    }

    #[tokio::test]
    async fn learn_talent_removes_change_talent_interrupt_auras_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        install_test_talent_store(&mut session, &[(101, 0, 50_101)]);
        session.mark_represented_talents_loaded_like_cpp();
        session.visible_auras.insert(
            1,
            visible_aura(1, SPELL_AURA_INTERRUPT_FLAG2_CHANGE_TALENT_LIKE_CPP),
        );
        session.visible_auras.insert(2, visible_aura(2, 0));

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;

        assert!(
            send_rx.try_recv().is_ok(),
            "C++ sends updates after successful AddTalent and aura removal"
        );
        assert!(
            !session.visible_auras.contains_key(&1),
            "C++ Player::AddTalent(learning=true) removes ChangeTalent interrupt auras"
        );
        assert!(
            session.visible_auras.contains_key(&2),
            "unrelated auras survive ChangeTalent interrupt removal"
        );
    }

    #[tokio::test]
    async fn learn_talent_adds_override_spell_pair_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let mut talent = test_talent_entry_like_cpp(101, 0, 50_101);
        talent.spell_id = 70_101;
        talent.overrides_spell_id = 60_101;
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![talent], 1);
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;

        assert!(send_rx.try_recv().is_ok());
        let overrides = session
            .represented_override_spells_like_cpp()
            .get(&60_101)
            .expect("C++ Player::AddTalent calls AddOverrideSpell when OverridesSpellID is set");
        assert!(
            overrides.contains(&70_101),
            "C++ AddOverrideSpell stores overridden spell id -> replacement talent SpellID"
        );
    }

    #[tokio::test]
    async fn confirm_respec_wipe_resets_active_talents_and_removes_overrides_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(3);
        let trainer = test_creature_guid(88);
        let mut talent = test_talent_entry_like_cpp(101, 0, 50_101);
        talent.spell_id = 70_101;
        talent.overrides_spell_id = 60_101;
        register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![talent], 1);
        session.mark_represented_talents_loaded_like_cpp();
        session.set_player_gold_like_cpp(20_000);

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;
        let _learn_update = send_rx
            .try_recv()
            .expect("C++ sends SendTalentsInfoData after LearnTalent");
        assert!(session.known_spells_like_cpp().contains(&50_101));
        assert!(
            session
                .represented_override_spells_like_cpp()
                .get(&60_101)
                .is_some_and(|overrides| overrides.contains(&70_101))
        );

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        let reset_update = send_rx
            .try_recv()
            .expect("C++ sends SendTalentsInfoData after successful ResetTalents");
        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            session.represented_confirm_respec_wipe_requests_like_cpp(),
            &[RepresentedConfirmRespecWipeLikeCpp {
                respec_master: trainer,
                respec_type: SPEC_RESET_TALENTS_LIKE_CPP,
            }]
        );
        assert!(
            !session.known_spells_like_cpp().contains(&50_101),
            "C++ RemoveTalent removes the active talent spell"
        );
        assert!(
            session.represented_override_spells_like_cpp().is_empty(),
            "C++ RemoveTalent calls RemoveOverrideSpell for OverridesSpellID"
        );
        assert_eq!(
            reset_update,
            session
                .represented_update_talent_data_packet_like_cpp()
                .to_bytes()
        );
        let packet = session.represented_update_talent_data_packet_like_cpp();
        assert!(
            packet.groups[0].talents.is_empty(),
            "C++ ResetTalents removes talents from the active group"
        );
        assert_eq!(packet.unspent_talent_points, 71);
    }

    #[tokio::test]
    async fn confirm_respec_wipe_removes_active_pet_not_in_slot_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(3);
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        let trainer = test_creature_guid(91);
        let player_guid = ObjectGuid::create_player(1, 42);
        let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 42);
        let position = Position::new(0.0, 0.0, 0.0, 0.0);
        register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 0,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            player_guid,
            "TalentPetRemove".to_string(),
            position,
            0,
            1,
            1,
            80,
            0,
        ));
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .expect("canonical player map");
        add_canonical_test_trainer_like_cpp(
            &canonical,
            trainer,
            Position::new(1.0, 0.0, 0.0, 0.0),
            NPCFlags1::TRAINER.bits(),
            1,
        );
        add_canonical_test_pet_like_cpp(&canonical, pet_guid, player_guid, position);
        session.set_represented_pet_mode_state_like_cpp(
            Some(pet_guid),
            wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
            wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
        );
        session.set_represented_pet_stable_like_cpp(represented_current_pet_stable_like_cpp(42));
        session.mark_represented_talents_loaded_like_cpp();
        session.set_player_gold_like_cpp(20_000);

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        let _reset_update = send_rx
            .try_recv()
            .expect("C++ sends SendTalentsInfoData after successful ResetTalents");
        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            session.represented_pet_guid_like_cpp(),
            None,
            "C++ ResetTalents calls RemovePet(nullptr, PET_SAVE_NOT_IN_SLOT, true)"
        );
        assert_eq!(
            session.represented_pet_stable_current_index_like_cpp(),
            None,
            "C++ Player::RemovePet resets PetStable::CurrentPetIndex for PET_SAVE_NOT_IN_SLOT"
        );
        assert_eq!(
            session.represented_temporary_unsummoned_pet_number_like_cpp(),
            0,
            "C++ talent reset does not use the temporary-unsummoned pet slot"
        );
        let manager = canonical.lock().unwrap();
        assert!(
            manager
                .find_map(0, 0)
                .unwrap()
                .map()
                .get_typed_pet(pet_guid)
                .is_none(),
            "C++ Player::RemovePet adds the live pet object to removal"
        );
    }

    #[tokio::test]
    async fn confirm_respec_wipe_rejects_without_money_before_removing_talents_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(3);
        let trainer = test_creature_guid(89);
        let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 0, 0, 500, 43);
        let mut talent = test_talent_entry_like_cpp(101, 0, 50_101);
        talent.spell_id = 70_101;
        talent.overrides_spell_id = 60_101;
        register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
        install_test_talent_entries_with_tab_class_mask(&mut session, vec![talent], 1);
        session.mark_represented_talents_loaded_like_cpp();

        session
            .handle_learn_talent(learn_talent_packet(101, 0))
            .await;
        let _learn_update = send_rx
            .try_recv()
            .expect("C++ sends SendTalentsInfoData after LearnTalent");
        session.set_represented_pet_mode_state_like_cpp(
            Some(pet_guid),
            wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
            wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP,
        );
        session.set_represented_pet_stable_like_cpp(represented_current_pet_stable_like_cpp(43));
        session.set_player_gold_like_cpp(9_999);
        session.set_represented_at_login_flags_like_cpp(
            AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_RESET_TALENTS_LIKE_CPP,
        );

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        let buy_failed = send_rx
            .try_recv()
            .expect("C++ SendBuyError(BUY_ERR_NOT_ENOUGHT_MONEY) rejects ResetTalents");
        assert_eq!(
            buy_failed,
            BuyFailed {
                vendor_guid: ObjectGuid::EMPTY,
                muid: 0,
                reason: BuyResult::NotEnoughtMoney,
            }
            .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert_eq!(session.player_gold_like_cpp(), 9_999);
        assert_eq!(session.represented_talent_reset_cost_like_cpp(), 0);
        assert_eq!(session.represented_talent_reset_time_secs_like_cpp(), 0);
        assert_eq!(
            session.represented_talent_reset_script_hooks_like_cpp(),
            &[RepresentedTalentResetScriptHookLikeCpp { no_cost: false }],
            "C++ calls OnPlayerTalentsReset before the money gate"
        );
        assert_eq!(
            session.represented_at_login_flags_like_cpp(),
            AT_LOGIN_RENAME_LIKE_CPP,
            "C++ removes only AT_LOGIN_RESET_TALENTS before the money gate"
        );
        assert_eq!(
            session.represented_at_login_flag_removals_like_cpp(),
            &[RepresentedAtLoginFlagRemovalLikeCpp {
                flags: AT_LOGIN_RESET_TALENTS_LIKE_CPP,
                persist: true,
                db_statement_unrepresented: true,
            }],
            "C++ RemoveAtLoginFlag(AT_LOGIN_RESET_TALENTS, true) executes CHAR_UPD_REM_AT_LOGIN_FLAG before ResetTalents can fail"
        );
        assert!(
            session
                .represented_talent_respec_criteria_events_like_cpp()
                .is_empty(),
            "C++ returns before criteria updates when the money gate fails"
        );
        assert!(
            session
                .represented_confirm_respec_wipe_requests_like_cpp()
                .is_empty()
        );
        assert!(
            session
                .represented_talent_respec_visual_spell_casts_like_cpp()
                .is_empty(),
            "C++ returns before the 14867 visual cast when ResetTalents fails"
        );
        assert!(
            session.known_spells_like_cpp().contains(&50_101),
            "C++ checks money before RemoveTalent"
        );
        assert_eq!(
            session.represented_pet_guid_like_cpp(),
            Some(pet_guid),
            "C++ returns before RemovePet when ResetTalents fails the money gate"
        );
        assert_eq!(
            session.represented_pet_stable_current_index_like_cpp(),
            Some(0),
            "C++ leaves the current pet slot untouched when ResetTalents returns false"
        );
        assert!(
            session
                .represented_override_spells_like_cpp()
                .get(&60_101)
                .is_some_and(|overrides| overrides.contains(&70_101)),
            "C++ keeps override spells when ResetTalents returns false"
        );
    }

    #[tokio::test]
    async fn confirm_respec_wipe_removes_feign_death_before_reset_talents_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let trainer = test_creature_guid(90);
        let feign_slot = 7;
        register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 0,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            ObjectGuid::create_player(1, 42),
            "TalentTester".to_string(),
            Position::new(0.0, 0.0, 0.0, 0.0),
            0,
            1,
            1,
            80,
            0,
        ));
        session.mark_represented_talents_loaded_like_cpp();
        session.set_player_gold_like_cpp(9_999);
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .expect("canonical player map");
        add_canonical_test_trainer_like_cpp(
            &canonical,
            trainer,
            Position::new(1.0, 0.0, 0.0, 0.0),
            NPCFlags1::TRAINER.bits(),
            1,
        );
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().add_unit_state(UnitState::DIED.bits());
            })
            .expect("canonical player");
        let mut feign_death = visible_aura(feign_slot, 0);
        feign_death.spell_id = 5384;
        feign_death.represented_effect = Some(RepresentedAuraEffectLikeCpp::FeignDeath);
        session.visible_auras.insert(feign_slot, feign_death);

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        let _aura_removed = send_rx
            .try_recv()
            .expect("C++ removes fake death before ResetTalents can fail");
        let buy_failed = send_rx
            .try_recv()
            .expect("C++ then sends the insufficient-money ResetTalents failure");
        assert_eq!(
            buy_failed,
            BuyFailed {
                vendor_guid: ObjectGuid::EMPTY,
                muid: 0,
                reason: BuyResult::NotEnoughtMoney,
            }
            .to_bytes()
        );
        assert!(send_rx.try_recv().is_err());
        assert!(!session.visible_auras.contains_key(&feign_slot));
        assert_eq!(
            session
                .mutate_canonical_player_like_cpp(|player| {
                    player.unit().has_unit_state(UnitState::DIED.bits())
                })
                .expect("canonical player"),
            false,
            "C++ RemoveAurasByType(SPELL_AURA_FEIGN_DEATH) clears UNIT_STATE_DIED before ResetTalents"
        );
        assert_eq!(session.player_gold_like_cpp(), 9_999);
        assert!(
            session
                .represented_confirm_respec_wipe_requests_like_cpp()
                .is_empty(),
            "C++ returns from failed ResetTalents before recording the accepted represented reset"
        );
    }

    #[test]
    fn represented_next_reset_talents_cost_matches_cpp_branches() {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let month = 30 * 24 * 60 * 60;
        let now = 10 * month;

        session.set_represented_talent_reset_state_like_cpp(0, now);
        assert_eq!(
            session.represented_next_reset_talents_cost_like_cpp(now),
            10_000
        );

        session.set_represented_talent_reset_state_like_cpp(10_000, now);
        assert_eq!(
            session.represented_next_reset_talents_cost_like_cpp(now),
            50_000
        );

        session.set_represented_talent_reset_state_like_cpp(50_000, now);
        assert_eq!(
            session.represented_next_reset_talents_cost_like_cpp(now),
            100_000
        );

        session.set_represented_talent_reset_state_like_cpp(100_000, now);
        assert_eq!(
            session.represented_next_reset_talents_cost_like_cpp(now),
            150_000
        );

        session.set_represented_talent_reset_state_like_cpp(500_000, now);
        assert_eq!(
            session.represented_next_reset_talents_cost_like_cpp(now),
            500_000
        );

        session.set_represented_talent_reset_state_like_cpp(500_000, now - month);
        assert_eq!(
            session.represented_next_reset_talents_cost_like_cpp(now),
            450_000
        );

        session.set_represented_talent_reset_state_like_cpp(100_000, 0);
        assert_eq!(
            session.represented_next_reset_talents_cost_like_cpp(now),
            100_000
        );
    }

    #[tokio::test]
    async fn confirm_respec_wipe_rejects_non_talent_reset_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let trainer = test_creature_guid(78);
        register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                wow_packet::packets::talent::SPEC_RESET_PET_TALENTS_LIKE_CPP,
            ))
            .await;

        assert!(
            session
                .represented_confirm_respec_wipe_requests_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn confirm_respec_wipe_rejects_non_trainer_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let vendor = test_creature_guid(79);
        register_test_trainer(&mut session, vendor, NPCFlags1::VENDOR.bits());

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                vendor,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        assert!(
            session
                .represented_confirm_respec_wipe_requests_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn confirm_respec_wipe_rejects_low_level_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let trainer = test_creature_guid(80);
        register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());
        session.set_player_level_like_cpp(14);

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        assert!(
            session
                .represented_confirm_respec_wipe_requests_like_cpp()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn confirm_respec_wipe_rejects_mismatched_trainer_class_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send_capacity(1);
        let trainer = test_creature_guid(81);
        register_test_trainer_with_class(&mut session, trainer, NPCFlags1::TRAINER.bits(), 2);
        session.set_player_class_like_cpp(1);
        session.mark_represented_talents_loaded_like_cpp();
        session.set_player_gold_like_cpp(20_000);

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        assert!(
            session
                .represented_confirm_respec_wipe_requests_like_cpp()
                .is_empty(),
            "C++ Creature::CanResetTalents requires player class to match creature_template.trainer_class"
        );
    }
}
