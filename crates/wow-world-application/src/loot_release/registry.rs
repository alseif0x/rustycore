// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;

#[cfg(any(test, feature = "test-fixtures"))]
pub struct LootReleaseRegistryFixtureRefsLikeCpp<'a> {
    position: &'a Option<wow_core::Position>,
    health: &'a u32,
    max_health: &'a u32,
    alive: &'a bool,
    level: &'a u8,
    transport: &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
    spells: &'a wow_world_spell::SessionSpellState,
    quests: &'a crate::SessionQuestState,
    mount_vehicle_kit: &'a Option<wow_entities::Vehicle>,
    vehicle_seat_flags: &'a Option<i32>,
    vehicle_seat_id: &'a Option<u32>,
    pet_guid: &'a Option<ObjectGuid>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> LootReleaseRegistryFixtureRefsLikeCpp<'a> {
    pub fn new_like_cpp(
        position: &'a Option<wow_core::Position>, health: &'a u32,
        max_health: &'a u32, alive: &'a bool, level: &'a u8,
        transport: &'a Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
        spells: &'a wow_world_spell::SessionSpellState, quests: &'a crate::SessionQuestState,
        mount_vehicle_kit: &'a Option<wow_entities::Vehicle>, vehicle_seat_flags: &'a Option<i32>,
        vehicle_seat_id: &'a Option<u32>, pet_guid: &'a Option<ObjectGuid>,
    ) -> Self {
        Self { position, health, max_health, alive, level, transport, spells, quests,
            mount_vehicle_kit, vehicle_seat_flags, vehicle_seat_id, pet_guid }
    }
}

impl LootReleaseCxLikeCpp<'_> {
    pub(super) fn sync_player_registry_state_like_cpp(&self) {
        let Some(control) = self.owner.registry_control_binding_like_cpp() else {
            return;
        };
        let position = self.owner.registry_sync_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))] self.registry_fixtures.position,
            #[cfg(any(test, feature = "test-fixtures"))] self.registry_fixtures.health,
            #[cfg(any(test, feature = "test-fixtures"))] self.registry_fixtures.max_health,
            #[cfg(any(test, feature = "test-fixtures"))] self.registry_fixtures.alive,
            #[cfg(any(test, feature = "test-fixtures"))] self.registry_fixtures.level,
            #[cfg(any(test, feature = "test-fixtures"))] self.registry_fixtures.transport,
        );
        let sync = crate::PlayerRegistrySyncContext::new(position, control, self.loot);
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.consumer_test {
            sync.with_fixture_hydration(crate::PlayerRegistryHydrationContext::new(
                self.owner.registry_hydration_like_cpp(), self.registry_fixtures.spells,
                self.registry_fixtures.quests,
                (self.registry_fixtures.mount_vehicle_kit, self.registry_fixtures.vehicle_seat_flags,
                    self.registry_fixtures.vehicle_seat_id, self.registry_fixtures.pet_guid),
                true,
            )).sync();
            return;
        }
        sync.sync();
    }
}
