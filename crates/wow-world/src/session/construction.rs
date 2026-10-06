// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Construction: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use std::sync::Arc;

use crate::session::state::InstanceState;
use crate::session::state::InteractionState;
use crate::session::state::InventoryState;
use crate::session::state::LootState;
use crate::session::state::SessionCatalogs;
use crate::session::state::SessionCore;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::state::SessionFixtures;
use crate::session::state::SessionLifecycleState;
use crate::session::state::SessionPhaseRail;
use crate::session::state::SessionQuestState;
use crate::session::state::SessionSocialLimits;
use crate::session::state::SessionSpellState;
use crate::session::state::SessionWorldConfig;
use crate::session::state::SessionWorldView;
use crate::session::state::VisibilityState;
use crate::session::state::WorldEntitiesState;

#[cfg(test)]
use super::SeedableRng;
#[cfg(test)]
use super::StdRng;
use super::{HashMap, Instant};
use super::{WorldPacket, WorldSession, connection};

impl WorldSession {
    pub(in crate::session) const MIN_ITEM_LEVEL_LIKE_CPP: u32 = 1;
    pub(in crate::session) const MAX_ITEM_LEVEL_LIKE_CPP: u32 = 1300;

    /// Create a new session with the given account info and channels.
    pub fn new(
        account_id: u32,
        account_name: String,
        security: u8,
        expansion: u8,
        account_expansion: u8,
        build: u32,
        session_key: Vec<u8>,
        locale: String,
        packet_rx: flume::Receiver<WorldPacket>,
        send_tx: flume::Sender<Vec<u8>>,
        packet_handler_registry: Arc<crate::session::registry::WorldPacketHandlerRegistry>,
    ) -> Self {
        let (session_command_tx, session_command_rx) = flume::bounded(256);
        // One outstanding request per phase at most: the producer waits for the
        // completion boundary of each pass before issuing the next one, so a
        // full rail means a producer that did not wait.
        let (session_phase_tx, session_phase_rx) = flume::bounded(2);

        // The instance endpoint keeps the pre-#297 default; the kernel does not
        // hardcode a world-server address of its own.
        let mut connection = wow_session::SessionConnection::new(send_tx, packet_rx);
        connection.set_instance_endpoint([127, 0, 0, 1], 8086);

        Self {
            core: SessionCore::new_for_world_session_like_cpp(
                account_id,
                account_name,
                security,
                expansion,
                account_expansion,
                build,
                locale,
                session_command_tx,
                session_command_rx,
                connection,
                session_key,
            ),
            lifecycle: SessionLifecycleState::new_like_cpp(),
            phase: SessionPhaseRail {
                tx: session_phase_tx,
                rx: session_phase_rx,
            },
            loot: LootState::new_like_cpp(),
            catalogs: SessionCatalogs::default(),
            config: SessionWorldConfig::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            fixtures: SessionFixtures::default(),
            inventory: InventoryState::new_like_cpp(),
            spell_state: SessionSpellState::new_like_cpp(),
            social: SessionSocialLimits::with_recruit_a_friend_limits_like_cpp(85, 4),
            instances: InstanceState::new_like_cpp(),
            world_entities: WorldEntitiesState::new_like_cpp(),
            visibility: VisibilityState::new_like_cpp(),
            interaction: InteractionState::new_like_cpp(),
            quest_state: SessionQuestState::new(),
            view: SessionWorldView {
                is_pvp_realm_like_cpp: false,
                is_ffa_pvp_realm_like_cpp: false,
                combat_tick_last_at_like_cpp: Instant::now(),
                last_presented_creature_melee_health_state_revision_like_cpp: 0,
                taxi_node_map_ids_like_cpp: HashMap::new(),
                active_area_trigger: None,
                #[cfg(test)]
                area_trigger_script_dispatcher_like_cpp: None,
            },
            dispatch_table: packet_handler_registry,
        }
    }

    #[cfg(test)]
    pub(crate) fn seed_represented_runtime_rng_like_cpp(&mut self, seed: u64) {
        self.core.driver.represented_runtime_rng_like_cpp = StdRng::seed_from_u64(seed);
    }
}
