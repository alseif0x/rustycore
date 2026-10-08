//! Shared canonical loot-authority fixtures for packet and lifecycle scenarios,
//! and the F6-7 R2/R3 loot-authority regressions that drive them.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use wow_core::{ObjectGuid, Position};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
    LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP, LOOT_TYPE_CORPSE_LIKE_CPP, LootEntry, LootEntryFlags,
    LootResponse,
};

use super::{
    adopt_registered_creature_as_canonical_incarnation_like_cpp, attach_canonical_creature,
    install_limited_test_item_template, loot_type_for_client_like_cpp,
    make_canonical_creature_for_session, make_session_with_send_capacity,
    register_test_creature_like_cpp, represented_loot_object_guid_like_cpp,
    represented_loot_response_items_like_cpp, test_creature, test_creature_guid,
};
use crate::handlers::loot::rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp;
use crate::session::{SharedCanonicalMapManager, WorldSession};
use wow_loot::{
    OwnedLootAuthority, OwnedLootAuthorityLifecycle, OwnedLootAuthorityStamp, OwnedLootSnapshot,
};
use wow_map::MapManager;
use wow_world_core::session::OwnedLootAuthorityLookupOutcomeLikeCpp;

/// Test-only collapse of the explicit loot-authority lookup outcome.
///
/// F6-7 R2 deleted the production `Option` collapse, so the loot fixtures
/// answer it here, in one place and only in the loot test support module. It
/// panics on **both** `Absent` and `Unavailable` and names the actual outcome:
/// a fixture that reads either fact has a broken precondition, and
/// substituting a default authority would hide exactly the fact the slice
/// exists to keep explicit. There is no production re-export and no
/// `test-fixtures` availability.
#[cfg(test)]
pub(super) fn expect_found_like_cpp(
    o: OwnedLootAuthorityLookupOutcomeLikeCpp,
) -> OwnedLootAuthority {
    match o {
        OwnedLootAuthorityLookupOutcomeLikeCpp::Found(a) => a,
        other => panic!("expected Found, got {other:?}"),
    }
}

pub(super) fn insert_allowed_coin_loot_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    coins: u32,
) {
    session.loot.insert_cached_loot_for_owner_like_cpp(
        owner_guid,
        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
            coins,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: Vec::new(),
            looted_by_player: false,
        },
    );
    if matches!(
        session.represented_owned_loot_authority_outcome_like_cpp(owner_guid),
        OwnedLootAuthorityLookupOutcomeLikeCpp::Found(_)
    ) {
        install_cached_test_creature_loot_authority_like_cpp(session, owner_guid, player_guid);
    }
}

/// Legacy packet tests construct the result of `Unit::Kill` directly.
/// Install that fixture into the creature before exercising CMSG_LOOT_UNIT
/// so the request remains a pure read/reconciliation path, like C++.
pub(super) fn install_cached_test_creature_loot_authority_like_cpp(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    scope_player: ObjectGuid,
) {
    if let Some(loot) = session.loot.cached_loot_for_owner_mut_like_cpp(owner_guid) {
        // The fixtures describe the already-filtered post-FillLoot item
        // set. Rebuild only its derived counters; adding looters here
        // would erase negative eligibility cases the fixture represents.
        rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
    }
    session
        .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, scope_player)
        .expect("the kill-time loot fixture must install into its creature authority");
}

pub(super) fn two_sessions_with_authoritative_creature_loot_like_cpp(
    mut loot: CreatureLoot,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    WorldSession,
    flume::Receiver<Vec<u8>>,
    ObjectGuid,
    ObjectGuid,
    ObjectGuid,
) {
    let (mut first, first_rx) = make_session_with_send_capacity(32);
    let (mut second, second_rx) = make_session_with_send_capacity(32);
    let first_guid = ObjectGuid::create_player(1, 42);
    let second_guid = ObjectGuid::create_player(1, 43);
    let owner_guid = test_creature_guid(19_500);
    let shared_map = Arc::new(RwLock::new(crate::map_manager::MapManager::new()));

    first.set_player_guid(Some(first_guid));
    second.set_player_guid(Some(second_guid));
    install_limited_test_item_template(&mut first, 25, 0);
    install_limited_test_item_template(&mut second, 25, 0);
    first.set_player_position_like_cpp(Position::ZERO);
    second.set_player_position_like_cpp(Position::ZERO);
    first.set_map_manager(Arc::clone(&shared_map));
    second.set_map_manager(shared_map);
    register_test_creature_like_cpp(&mut first, test_creature(owner_guid, false));

    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![first_guid, second_guid];
    for entry in &mut loot.items {
        entry.allowed_looters = vec![first_guid, second_guid];
    }
    first
        .loot
        .insert_cached_loot_for_owner_like_cpp(owner_guid, loot);
    first
        .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, first_guid)
        .unwrap();

    first.set_active_loot_guid(owner_guid);
    let first_response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        first
            .loot
            .cached_loot_for_owner_like_cpp(owner_guid)
            .expect("loot cache entry should be seeded"),
        first_guid,
    );
    first.represented_on_loot_opened_like_cpp(owner_guid, first_guid, first_response);
    assert!(second.reconcile_represented_loot_cache_like_cpp(owner_guid, second_guid));
    second.set_active_loot_guid(owner_guid);
    let second_response = authoritative_test_loot_response_like_cpp(
        owner_guid,
        second
            .loot
            .cached_loot_for_owner_like_cpp(owner_guid)
            .expect("loot cache entry should be seeded"),
        second_guid,
    );
    second.represented_on_loot_opened_like_cpp(owner_guid, second_guid, second_response);

    (
        first,
        first_rx,
        second,
        second_rx,
        owner_guid,
        first_guid,
        second_guid,
    )
}

pub(super) fn authoritative_test_loot_like_cpp(coins: u32, with_item: bool) -> CreatureLoot {
    CreatureLoot {
        loot_guid: ObjectGuid::EMPTY,
        coins,
        unlooted_count: u8::from(with_item),
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: with_item
            .then(|| LootEntry {
                loot_list_id: 0,
                item_id: 25,
                quantity: 1,
                random_properties_id: 0,
                random_properties_seed: 0,
                item_context: 0,
                flags: LootEntryFlags::default(),
                allowed_looters: Vec::new(),
                roll_winner: ObjectGuid::EMPTY,
                ffa_looted_by: Vec::new(),
                taken: false,
            })
            .into_iter()
            .collect(),
        looted_by_player: false,
    }
}

pub(super) fn authoritative_test_loot_response_like_cpp(
    owner_guid: ObjectGuid,
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> LootResponse {
    LootResponse {
        owner: owner_guid,
        loot_obj: loot.loot_guid,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
        loot_method: loot.loot_method,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: loot.coins,
        items: represented_loot_response_items_like_cpp(loot, player_guid),
        currencies: Vec::new(),
        acquired: true,
        ae_looting: false,
    }
}

pub(super) fn represented_disenchant_test_outputs_like_cpp(
    winner_guid: ObjectGuid,
    item_id: u32,
) -> Vec<LootEntry> {
    (0..2)
        .map(|loot_list_id| LootEntry {
            loot_list_id,
            item_id,
            quantity: 1,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags: LootEntryFlags {
                follow_loot_rules: true,
                ..Default::default()
            },
            allowed_looters: vec![winner_guid],
            roll_winner: winner_guid,
            ffa_looted_by: Vec::new(),
            taken: false,
        })
        .collect()
}

/// The map key the F6-7 R5 one-incarnation fixture is registered and attached
/// under: the player's resolved residence key, which is what the designated
/// lookup addresses.
const SINGLE_INCARNATION_TEST_MAP_ID: u16 = 571;

/// F6-7 R5. The one-incarnation fixture the foreign/stale-alias refusal
/// regression drives.
///
/// Production registration publishes the legacy representation; the canonical
/// incarnation is then derived from that *same* registered representation, so
/// both stores hold ONE incarnation — one loot allocation and one health-state
/// revision timeline. That is what makes an **allocation** or **revision**
/// refusal distinguishable from a foreign-incarnation refusal: the
/// representations the regression offers differ from the incarnation only in the
/// allocation they carry or in the health revision they replay, never in the
/// timeline identity.
pub(super) struct SingleIncarnationLootFixtureLikeCpp {
    pub(super) session: WorldSession,
    pub(super) send_rx: flume::Receiver<Vec<u8>>,
    pub(super) legacy: crate::map_manager::SharedMapManager,
    pub(super) canonical: SharedCanonicalMapManager,
    pub(super) owner_guid: ObjectGuid,
    pub(super) player_guid: ObjectGuid,
    pub(super) owner_allocation: OwnedLootAuthority,
}

impl SingleIncarnationLootFixtureLikeCpp {
    pub(super) fn map_key_like_cpp(&self) -> wow_map::MapKey {
        wow_map::MapKey::new(u32::from(SINGLE_INCARNATION_TEST_MAP_ID), 0)
    }

    /// The registered legacy representation, cloned exactly as the production
    /// mirror sites transport it.
    pub(super) fn transported_legacy_representation_like_cpp(&self) -> wow_entities::Creature {
        self.legacy
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature(SINGLE_INCARNATION_TEST_MAP_ID, 0, self.owner_guid)
            .expect("the fixture registered the legacy representation")
            .creature
            .clone()
    }

    pub(super) fn canonical_creature_like_cpp(&self) -> wow_entities::Creature {
        self.canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_map(u32::from(SINGLE_INCARNATION_TEST_MAP_ID), 0)
            .expect("the fixture attached the canonical map instance")
            .map()
            .with_creature_like_cpp(self.owner_guid, Clone::clone)
            .expect("the fixture admitted the canonical incarnation")
    }

    /// Mutate the canonical incarnation under its own map lock, the same path
    /// the production guards take.
    pub(super) fn with_canonical_incarnation_mut_like_cpp<R>(
        &self,
        mutate: impl FnOnce(&mut wow_entities::Creature) -> R,
    ) -> R {
        self.canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_map_mut(u32::from(SINGLE_INCARNATION_TEST_MAP_ID), 0)
            .expect("the fixture attached the canonical map instance")
            .map_mut()
            .with_creature_mut_like_cpp(self.owner_guid, mutate)
            .expect("the fixture admitted the canonical incarnation")
    }

    pub(super) fn canonical_allocation_like_cpp(&self) -> OwnedLootAuthority {
        self.session
            .read_canonical_creature_loot_authority_on_map_like_cpp(
                self.owner_guid,
                self.map_key_like_cpp(),
            )
            .expect("the fixture publishes the canonical incarnation's allocation")
    }

    pub(super) fn legacy_allocation_like_cpp(&self) -> OwnedLootAuthority {
        self.session
            .read_legacy_creature_loot_authority_on_map_like_cpp(
                self.owner_guid,
                self.map_key_like_cpp(),
            )
            .expect("the fixture publishes the legacy representation's allocation")
    }
}

/// One admitted incarnation in both stores, with the fixture's own used pool
/// installed on the incarnation's allocation.
pub(super) fn single_incarnation_loot_fixture_like_cpp(
    counter: i64,
    owner_coins: u32,
) -> SingleIncarnationLootFixtureLikeCpp {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let owner_guid = test_creature_guid(counter);
    let player_guid = ObjectGuid::create_player(1, counter);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(SINGLE_INCARNATION_TEST_MAP_ID, Position::ZERO);
    // Production registration with no canonical manager configured: the
    // legitimate legacy-only path publishes the representation.
    register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
    let legacy = session
        .core
        .map_manager
        .clone()
        .expect("registration configures the legacy map manager");
    // The canonical incarnation is derived from that registered representation,
    // so the fixture owns one incarnation instead of two.
    let canonical: SharedCanonicalMapManager = Arc::new(Mutex::new(MapManager::default()));
    canonical
        .lock()
        .expect("the fixture canonical manager is uncontended")
        .create_world_map(u32::from(SINGLE_INCARNATION_TEST_MAP_ID), 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    adopt_registered_creature_as_canonical_incarnation_like_cpp(
        &legacy,
        &canonical,
        owner_guid,
        u32::from(SINGLE_INCARNATION_TEST_MAP_ID),
        0,
    );
    let owner_allocation = session
        .read_canonical_creature_loot_authority_on_map_like_cpp(
            owner_guid,
            wow_map::MapKey::new(u32::from(SINGLE_INCARNATION_TEST_MAP_ID), 0),
        )
        .expect("the fixture publishes the canonical incarnation's allocation");
    assert!(
        session
            .read_legacy_creature_loot_authority_on_map_like_cpp(
                owner_guid,
                wow_map::MapKey::new(u32::from(SINGLE_INCARNATION_TEST_MAP_ID), 0),
            )
            .is_some_and(|legacy| legacy.shares_storage_like_cpp(&owner_allocation)),
        "the fixture must expose ONE incarnation: both stores hold the same allocation"
    );
    assert!(
        owner_allocation
            .initialize_shared_like_cpp(allowed_creature_loot_like_cpp(
                owner_guid,
                owner_coins,
                player_guid,
            ))
            .installed(),
        "the incarnation's own pool must be live"
    );
    SingleIncarnationLootFixtureLikeCpp {
        session,
        send_rx,
        legacy,
        canonical,
        owner_guid,
        player_guid,
        owner_allocation,
    }
}

/// A claimable creature pool: coins, one item, and the player allowed to take
/// either. Used for both the incarnation's own allocation and the foreign one.
pub(super) fn allowed_creature_loot_like_cpp(
    owner_guid: ObjectGuid,
    coins: u32,
    player_guid: ObjectGuid,
) -> CreatureLoot {
    let mut loot = authoritative_test_loot_like_cpp(coins, true);
    loot.loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
    loot.allowed_looters = vec![player_guid];
    for entry in &mut loot.items {
        entry.allowed_looters = vec![player_guid];
    }
    loot
}

/// The complete state of one loot allocation. Identity is backing-`Arc` identity
/// and is asserted separately by the caller; this is what a refused operation
/// must leave untouched, including the whole pool.
#[derive(Debug, PartialEq)]
pub(super) struct LootAllocationObservablesLikeCpp {
    lifecycle: OwnedLootAuthorityLifecycle,
    stamp: OwnedLootAuthorityStamp,
    generation: u64,
    shared: Option<OwnedLootSnapshot>,
    personal: HashMap<ObjectGuid, OwnedLootSnapshot>,
}

pub(super) fn loot_allocation_observables_like_cpp(
    authority: &OwnedLootAuthority,
) -> LootAllocationObservablesLikeCpp {
    LootAllocationObservablesLikeCpp {
        lifecycle: authority.lifecycle_like_cpp(),
        stamp: authority.stamp_like_cpp(),
        generation: authority.generation_like_cpp(),
        shared: authority.shared_snapshot_like_cpp(),
        personal: authority.personal_snapshots_like_cpp(),
    }
}

/// The publication surface of one creature representation: what a refused
/// application must not change and must not announce.
#[derive(Debug, PartialEq)]
pub(super) struct CreaturePublicationObservablesLikeCpp {
    health: u64,
    max_health: u64,
    death_state: wow_constants::unit::DeathState,
    health_state_revision: u64,
    changed_fields: u8,
    npc_flags: u32,
    loot_stamp: OwnedLootAuthorityStamp,
}

pub(super) fn creature_publication_observables_like_cpp(
    creature: &wow_entities::Creature,
) -> CreaturePublicationObservablesLikeCpp {
    CreaturePublicationObservablesLikeCpp {
        health: creature.unit().data().health,
        max_health: creature.unit().data().max_health,
        death_state: creature.unit().death_state(),
        health_state_revision: creature.unit().health_state_revision_like_cpp(),
        changed_fields: creature.unit().world().object().changed_fields().bits(),
        npc_flags: creature.ai_ownership().npc_flags,
        loot_stamp: creature.loot_authority_like_cpp().stamp_like_cpp(),
    }
}

#[path = "r2_designated_owner.rs"]
mod r2_designated_owner;
#[path = "r3_reconciliation_retirement.rs"]
mod r3_reconciliation_retirement;
