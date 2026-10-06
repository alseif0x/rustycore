// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Registry synchronization sequence shared by World adapters.

#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::RegistrySyncInputs;
use wow_world_core::session::{
    PlayerRegistryControlBindingLikeCpp, PlayerRegistrySyncAccessLikeCpp,
};
use wow_world_loot::LootState;

/// Executes the established World-session registry publication order using typed owner access.
pub struct PlayerRegistrySyncContext<'a> {
    position: PlayerRegistrySyncAccessLikeCpp<'a>,
    control: PlayerRegistryControlBindingLikeCpp<'a>,
    loot: &'a LootState,
    #[cfg(any(test, feature = "test-fixtures"))]
    inputs: RegistrySyncInputs<'a>,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_hydration: Option<PlayerRegistryHydrationContext<'a>>,
}

impl<'a> PlayerRegistrySyncContext<'a> {
    /// Borrow the existing canonical-position, control-channel, and loot providers.
    ///
    /// The mutable-vitals participants arrive through the inert
    /// [`RegistrySyncInputs`] builder, lent by Stats/World at this final phase so
    /// Registry never overlaps a Stats capability's mutable borrow.
    pub fn new(
        position: PlayerRegistrySyncAccessLikeCpp<'a>,
        control: PlayerRegistryControlBindingLikeCpp<'a>,
        loot: &'a LootState,
        #[cfg(any(test, feature = "test-fixtures"))] inputs: RegistrySyncInputs<'a>,
    ) -> Self {
        Self {
            position,
            control,
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            inputs,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_hydration: None,
        }
    }

    /// Add the World unit-test hydration seam at its existing position in the sequence.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn with_fixture_hydration(mut self, hydration: PlayerRegistryHydrationContext<'a>) -> Self {
        self.fixture_hydration = Some(hydration);
        self
    }

    /// Update position, hydrate the explicit fixture seam, then publish loot and party state.
    pub fn sync(&self) {
        self.sync_selected_hydration_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.fixture_hydration.as_ref(),
        );
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn sync_with_fixture_hydration_like_cpp(
        &self,
        hydration: &PlayerRegistryHydrationContext<'_>,
    ) {
        self.sync_selected_hydration_like_cpp(Some(hydration));
    }

    fn sync_selected_hydration_like_cpp(
        &self,
        #[cfg(any(test, feature = "test-fixtures"))] hydration: Option<
            &PlayerRegistryHydrationContext<'_>,
        >,
    ) {
        self.position.update_registry_position(
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.inputs,
        );
        #[cfg(any(test, feature = "test-fixtures"))]
        if let Some(hydration) = hydration {
            hydration.hydrate();
        }
        let identities = self
            .loot
            .represented_loot_roll_command_identities_snapshot_like_cpp();
        let _ = self.control.replace_loot_rolls_like_cpp(identities);
        self.control.sync_party_member_party_type_like_cpp();
    }
}

/// Bounded fixture-only inputs for the existing one-way Player directory hydration.
#[cfg(any(test, feature = "test-fixtures"))]
pub struct PlayerRegistryHydrationContext<'a> {
    owner: wow_world_core::session::PlayerRegistryHydrationAccessLikeCpp<'a>,
    spells: &'a wow_world_spell::SessionSpellState,
    quests: &'a super::SessionQuestState,
    mount_vehicle_kit: &'a Option<wow_entities::Vehicle>,
    vehicle_seat_flags: &'a Option<i32>,
    vehicle_seat_id: &'a Option<u32>,
    pet_guid: &'a Option<wow_core::ObjectGuid>,
    consumer_test: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl<'a> PlayerRegistryHydrationContext<'a> {
    /// Store only the selected owner and fixture references; resolution remains at execution time.
    pub fn new(
        owner: wow_world_core::session::PlayerRegistryHydrationAccessLikeCpp<'a>,
        spells: &'a wow_world_spell::SessionSpellState,
        quests: &'a super::SessionQuestState,
        fixture_vehicle_and_pet: (
            &'a Option<wow_entities::Vehicle>,
            &'a Option<i32>,
            &'a Option<u32>,
            &'a Option<wow_core::ObjectGuid>,
        ),
        consumer_test: bool,
    ) -> Self {
        Self {
            owner,
            spells,
            quests,
            mount_vehicle_kit: fixture_vehicle_and_pet.0,
            vehicle_seat_flags: fixture_vehicle_and_pet.1,
            vehicle_seat_id: fixture_vehicle_and_pet.2,
            pet_guid: fixture_vehicle_and_pet.3,
            consumer_test,
        }
    }

    /// Apply the existing test-only spell, quest, mount, seat, and pet hydration.
    pub fn hydrate(&self) {
        if !self.consumer_test {
            return;
        }
        let known_spells = self.spells.known_spells_fixture_like_cpp();
        let quests = if self.owner.owner_handle_absent_like_cpp() {
            Some(self.quests.player_quest_gameplay_fixture_like_cpp())
        } else {
            self.owner.owned_player_quest_gameplay_snapshot_like_cpp()
        };
        self.owner.hydrate_player_directory_fixture_like_cpp(
            known_spells,
            quests,
            self.mount_vehicle_kit.clone(),
            *self.vehicle_seat_flags,
            *self.vehicle_seat_id,
            *self.pet_guid,
        );
    }
}

/// C++ registry publication of the logged-in Player's state: position, the
/// World-test fixture hydration seam, then loot and party state. World and
/// application handlers share this one construction; `world_test_consumer` is
/// the host's World-test flag (World passes `cfg!(test)`), which selects the
/// fixture hydration exactly where the World session used `#[cfg(test)]`.
pub fn sync_player_registry_state_like_cpp(
    hub: wow_world_core::session::HubRef<'_>,
    loot: &LootState,
    #[cfg(any(test, feature = "test-fixtures"))] spell_state: &wow_world_spell::SessionSpellState,
    #[cfg(any(test, feature = "test-fixtures"))] quest_state: &super::SessionQuestState,
    world_test_consumer: bool,
) {
    let core = hub.core;
    let (Some(guid), Some(registry)) = (core.player_guid(), &core.player_registry) else {
        return;
    };
    let position = core.player_registry_sync_access_like_cpp(
        #[cfg(any(test, feature = "test-fixtures"))]
        &hub.fixtures.movement.player_position,
        #[cfg(any(test, feature = "test-fixtures"))]
        &hub.fixtures.identity.player_level,
        #[cfg(any(test, feature = "test-fixtures"))]
        &hub.fixtures.vehicles.player_transport_login_state_like_cpp,
    );
    let control = core.player_registry_control_binding_like_cpp(guid, registry);
    let sync = PlayerRegistrySyncContext::new(
        position,
        control,
        loot,
        #[cfg(any(test, feature = "test-fixtures"))]
        RegistrySyncInputs::new_like_cpp(
            &hub.fixtures.combat.player_health_like_cpp,
            &hub.fixtures.combat.player_max_health_like_cpp,
            &hub.fixtures.combat.player_alive_like_cpp,
        ),
    );
    #[cfg(any(test, feature = "test-fixtures"))]
    let sync = if world_test_consumer {
        sync.with_fixture_hydration(PlayerRegistryHydrationContext::new(
            core.player_registry_hydration_access_like_cpp(),
            spell_state,
            quest_state,
            (
                &hub.fixtures.vehicles.player_mount_vehicle_kit_like_cpp,
                &hub.fixtures.vehicles.player_vehicle_seat_flags_like_cpp,
                &hub.fixtures.vehicles.player_vehicle_seat_id_like_cpp,
                &hub.fixtures.pets.represented_pet_guid_like_cpp,
            ),
            true,
        ))
    } else {
        sync
    };
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = world_test_consumer;
    sync.sync();
}
