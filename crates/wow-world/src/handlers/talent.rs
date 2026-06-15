// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use tracing::warn;

use wow_constants::ClientOpcodes;
use wow_constants::unit::NPCFlags1;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::talent::{ConfirmRespecWipe, SPEC_RESET_TALENTS_LIKE_CPP};
use wow_packet::{ClientPacket, WorldPacket};

use crate::session::{RepresentedConfirmRespecWipeLikeCpp, WorldSession};

const CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits();
const MIN_TALENT_RESET_LEVEL_LIKE_CPP: u8 = 15;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::ConfirmRespecWipe,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_confirm_respec_wipe",
    }
}

impl WorldSession {
    /// Handle CMSG_CONFIRM_RESPEC_WIPE.
    ///
    /// C++ resolves `GetNPCIfCanInteractWith(..., UNIT_NPC_FLAG_TRAINER)`,
    /// accepts only `SPEC_RESET_TALENTS`, checks `Creature::CanResetTalents`,
    /// then runs `Player::ResetTalents`, sends talent data, and casts the
    /// visual spell. Rust keeps this as represented state until talent reset,
    /// trainer-class matching, costs, and visual cast runtime are canonical.
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

        self.record_represented_confirm_respec_wipe_like_cpp(RepresentedConfirmRespecWipeLikeCpp {
            respec_master: request.respec_master,
            respec_type: request.respec_type,
        });
    }

    fn represented_can_confirm_respec_wipe_like_cpp(
        &mut self,
        respec_master: wow_core::ObjectGuid,
    ) -> bool {
        if self.player_level_like_cpp() < MIN_TALENT_RESET_LEVEL_LIKE_CPP {
            return false;
        }

        if self.has_canonical_map_manager_like_cpp() {
            return self
                .represented_npc_can_interact_with_like_cpp(
                    respec_master,
                    CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP,
                    0,
                )
                .is_some();
        }

        self.mutate_world_creature(respec_master, |creature| {
            (creature.npc_flags() & CONFIRM_RESPEC_WIPE_NPC_FLAGS_LIKE_CPP) != 0
        })
        .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use super::*;
    use wow_core::guid::HighGuid;
    use wow_core::{ObjectGuid, Position};
    use wow_packet::packets::update::CreatureCreateData;

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

    fn test_creature_guid(counter: u32) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, counter, 1)
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
    }

    #[tokio::test]
    async fn confirm_respec_wipe_records_talent_reset_with_trainer_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(1);
        let trainer = test_creature_guid(77);
        register_test_trainer(&mut session, trainer, NPCFlags1::TRAINER.bits());

        session
            .handle_confirm_respec_wipe(confirm_respec_wipe_packet(
                trainer,
                SPEC_RESET_TALENTS_LIKE_CPP,
            ))
            .await;

        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            session.represented_confirm_respec_wipe_requests_like_cpp(),
            &[RepresentedConfirmRespecWipeLikeCpp {
                respec_master: trainer,
                respec_type: SPEC_RESET_TALENTS_LIKE_CPP,
            }]
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
}
