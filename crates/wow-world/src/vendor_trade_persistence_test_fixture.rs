use std::sync::{Arc, Mutex};

use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerMoneyTransactionOutcomeLikeCpp,
    VendorRefundCleanupPersistenceLikeCpp, VendorTradePersistencePortLikeCpp,
    VendorTradePersistenceRequestLikeCpp,
};

use crate::session::WorldSession;
use wow_constants::NPCFlags1;
use wow_core::{ObjectGuid, Position};

pub(crate) fn register_vendor_for_trade_test_like_cpp(
    session: &mut WorldSession,
    guid: ObjectGuid,
    entry: u32,
) {
    session.register_world_creature(
        571,
        Position::new(5.0, 0.0, 0.0, 0.0),
        wow_packet::packets::update::CreatureCreateData {
            guid,
            entry,
            display_id: 100,
            native_display_id: 100,
            display_scale: 1.0,
            native_x_display_scale: 1.0,
            bounding_radius: 0.389,
            combat_reach: 1.0,
            health: 100,
            max_health: 100,
            level: 80,
            faction_template: 35,
            npc_flags: u64::from(NPCFlags1::VENDOR.bits()),
            unit_flags: 0,
            unit_flags2: 0,
            unit_flags3: 0,
            aura_state: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            display_power: 1,
            power: [0; 10],
            max_power: [0; 10],
            base_mana: 0,
            virtual_items: [(0, 0, 0); 3],
            base_attack_time: 2_000,
            ranged_attack_time: 0,
            movement_flags: 0,
            vehicle_id: 0,
            play_hover_anim: false,
            hover_height: 1.0,
            mount_display_id: 0,
            stand_state: 0,
            vis_flags: 0,
            anim_tier: 0,
            emote_state: 0,
            sheathe_state: wow_constants::unit::SheathState::Melee as u8,
            pvp_flags: 0,
            current_area_id: 0,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        },
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
}

pub(crate) struct VendorTradePersistencePortFixtureLikeCpp {
    outcome: PlayerMoneyTransactionOutcomeLikeCpp,
    pub(crate) requests: Arc<Mutex<Vec<VendorTradePersistenceRequestLikeCpp>>>,
}

impl VendorTradePersistencePortFixtureLikeCpp {
    pub(crate) fn definitely_rolled_back_like_cpp()
    -> (Self, Arc<Mutex<Vec<VendorTradePersistenceRequestLikeCpp>>>) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                outcome: PlayerMoneyTransactionOutcomeLikeCpp::DefinitelyRolledBack {
                    reason: "fixture rollback".to_string(),
                },
                requests: Arc::clone(&requests),
            },
            requests,
        )
    }
}

impl VendorTradePersistencePortLikeCpp for VendorTradePersistencePortFixtureLikeCpp {
    fn persist_vendor_trade_like_cpp(
        &self,
        request: VendorTradePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PlayerMoneyTransactionOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn clear_refund_metadata_like_cpp(
        &self,
        _request: VendorRefundCleanupPersistenceLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        Box::pin(async {
            PersistenceOutcomeLikeCpp::Failed {
                reason: "fixture rollback".to_string(),
            }
        })
    }
}
