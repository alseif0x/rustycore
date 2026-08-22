// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit charm, possession and vehicle control.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CharmType {
    Charm = 0,
    Possess = 1,
    Vehicle = 2,
    Convert = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharmInfoState {
    pub pet_number: u32,
    pub command_state: u8,
    pub action_bar: [u32; MAX_UNIT_ACTION_BAR_INDEX],
    pub charm_spells: [u32; MAX_SPELL_CHARM],
    pub is_command_attack: bool,
    pub is_command_follow: bool,
    pub is_at_stay: bool,
    pub is_following: bool,
    pub is_returning: bool,
    pub stay_position: Option<(f32, f32, f32)>,
}

impl CharmInfoState {
    pub fn init_pet_action_bar_like_cpp(&mut self) {
        for index in ACTION_BAR_INDEX_START..ACTION_BAR_INDEX_PET_SPELL_START {
            self.action_bar[index] = make_unit_action_button_like_cpp(
                COMMAND_ATTACK_LIKE_CPP - index as u32,
                ACT_COMMAND_LIKE_CPP,
            );
        }
        for index in ACTION_BAR_INDEX_PET_SPELL_START..ACTION_BAR_INDEX_PET_SPELL_END {
            self.action_bar[index] = make_unit_action_button_like_cpp(0, ACT_PASSIVE_LIKE_CPP);
        }
        for index in ACTION_BAR_INDEX_PET_SPELL_END..ACTION_BAR_INDEX_END {
            self.action_bar[index] = make_unit_action_button_like_cpp(
                COMMAND_ATTACK_LIKE_CPP - (index - ACTION_BAR_INDEX_PET_SPELL_END) as u32,
                ACT_REACTION_LIKE_CPP,
            );
        }
    }

    pub fn load_pet_action_bar_like_cpp(&mut self, data: &str) -> bool {
        self.init_pet_action_bar_like_cpp();

        let tokens: Vec<&str> = data.split(' ').filter(|token| !token.is_empty()).collect();
        if tokens.len() != (ACTION_BAR_INDEX_END - ACTION_BAR_INDEX_START) * 2 {
            return false;
        }

        for (offset, index) in (ACTION_BAR_INDEX_START..ACTION_BAR_INDEX_END).enumerate() {
            let type_token = tokens[offset * 2];
            let action_token = tokens[offset * 2 + 1];
            let (Some(active_type), Some(action)) = (
                type_token.parse::<u8>().ok(),
                action_token.parse::<u32>().ok(),
            ) else {
                continue;
            };

            self.action_bar[index] = make_unit_action_button_like_cpp(action, active_type);
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlSubsystem {
    pub owner_guid: Option<ObjectGuid>,
    pub minion_guid: Option<ObjectGuid>,
    pub summon_slots: [ObjectGuid; MAX_SUMMON_SLOT],
    pub gameobject_slots: [ObjectGuid; MAX_GAMEOBJECT_SLOT],
    pub owned_gameobjects: Vec<ObjectGuid>,
    pub last_charmer_guid: Option<ObjectGuid>,
    pub charmer_guid: Option<ObjectGuid>,
    pub charmed_guid: Option<ObjectGuid>,
    pub controlled_guids: HashSet<ObjectGuid>,
    pub controlled_by_player: bool,
    pub charm_type: Option<CharmType>,
    pub unit_moved_by_me: Option<ObjectGuid>,
    pub player_moving_me: Option<ObjectGuid>,
    pub shared_vision_guids: HashSet<ObjectGuid>,
    pub owner_attacked_notifications: Vec<ControlledOwnerAttackedNotification>,
    pub charm_info: Option<CharmInfoState>,
    pub old_faction_id: Option<u32>,
    pub walking_before_charm: bool,
}

impl ControlSubsystem {
    pub fn set_owner_guid(&mut self, owner: Option<ObjectGuid>) {
        self.owner_guid = owner;
    }

    pub fn set_minion_guid(&mut self, minion: Option<ObjectGuid>) {
        self.minion_guid = minion;
    }

    pub fn pet_guid(&self) -> ObjectGuid {
        self.summon_slots[SUMMON_SLOT_PET]
    }

    pub fn set_pet_guid(&mut self, pet: ObjectGuid) {
        self.summon_slots[SUMMON_SLOT_PET] = pet;
    }

    pub fn set_summon_slot(&mut self, slot: usize, guid: ObjectGuid) -> bool {
        let Some(target) = self.summon_slots.get_mut(slot) else {
            return false;
        };
        *target = guid;
        true
    }

    pub fn clear_summon_slot(&mut self, slot: usize) -> Option<ObjectGuid> {
        let target = self.summon_slots.get_mut(slot)?;
        let previous = *target;
        *target = ObjectGuid::EMPTY;
        Some(previous)
    }

    pub fn set_gameobject_slot(&mut self, slot: usize, guid: ObjectGuid) -> bool {
        let Some(target) = self.gameobject_slots.get_mut(slot) else {
            return false;
        };
        *target = guid;
        true
    }

    pub fn register_owned_gameobject_like_cpp(&mut self, guid: ObjectGuid) {
        self.owned_gameobjects.push(guid);
    }

    pub fn remove_owned_gameobject_like_cpp(&mut self, guid: ObjectGuid) -> bool {
        let before = self.owned_gameobjects.len();
        self.owned_gameobjects.retain(|known| *known != guid);
        before != self.owned_gameobjects.len()
    }

    pub fn clear_gameobject_slot_for_guid_like_cpp(&mut self, guid: ObjectGuid) -> bool {
        for slot in &mut self.gameobject_slots {
            if *slot == guid {
                *slot = ObjectGuid::EMPTY;
                return true;
            }
        }
        false
    }

    pub fn set_charmer(&mut self, charmer: ObjectGuid, controlled_by_player: bool) {
        self.last_charmer_guid = self.charmer_guid;
        self.charmer_guid = Some(charmer);
        self.controlled_by_player = controlled_by_player;
        self.init_charm_info();
    }

    pub fn remove_charmer(&mut self) {
        self.last_charmer_guid = self.charmer_guid;
        self.charmer_guid = None;
        self.controlled_by_player = false;
        self.charm_type = None;
        self.old_faction_id = None;
        self.delete_charm_info();
    }

    pub fn set_charmed(&mut self, charmed: ObjectGuid) {
        self.charmed_guid = Some(charmed);
        self.controlled_guids.insert(charmed);
    }

    pub fn remove_charmed(&mut self) {
        if let Some(charmed) = self.charmed_guid.take() {
            self.controlled_guids.remove(&charmed);
        }
    }

    pub fn apply_charm_as_controller(&mut self, charmed: ObjectGuid, controller_is_player: bool) {
        if controller_is_player {
            self.charmed_guid = Some(charmed);
        }
        self.controlled_guids.insert(charmed);
    }

    pub fn remove_charm_as_controller(
        &mut self,
        charmed: ObjectGuid,
        controlled_has_same_owner: bool,
        controlled_is_minion: bool,
        controlled_is_player: bool,
    ) {
        if self.charmed_guid == Some(charmed) {
            self.charmed_guid = None;
        }
        if controlled_is_player || !controlled_is_minion || !controlled_has_same_owner {
            self.controlled_guids.remove(&charmed);
        }
    }

    pub fn apply_charmed_by(
        &mut self,
        charmer: ObjectGuid,
        charm_type: CharmType,
        controlled_by_player: bool,
        old_faction_id: Option<u32>,
        was_walking: bool,
    ) -> bool {
        if self.charmer_guid.is_some() {
            return false;
        }
        self.charmer_guid = Some(charmer);
        self.controlled_by_player = controlled_by_player;
        self.charm_type = Some(charm_type);
        self.old_faction_id = old_faction_id;
        self.walking_before_charm = was_walking;
        if charm_type != CharmType::Vehicle {
            self.init_charm_info();
        }
        true
    }

    pub fn remove_charmed_by(
        &mut self,
        expected_charmer: Option<ObjectGuid>,
        is_guardian: bool,
    ) -> bool {
        let Some(charmer) = self.charmer_guid else {
            return false;
        };
        if expected_charmer.is_some_and(|expected| expected != charmer) {
            return false;
        }
        if self.charm_type != Some(CharmType::Vehicle) {
            self.last_charmer_guid = Some(charmer);
        }
        self.charmer_guid = None;
        self.controlled_by_player = false;
        self.charm_type = None;
        self.old_faction_id = None;
        if !is_guardian {
            self.delete_charm_info();
        }
        true
    }

    pub fn add_controlled(&mut self, guid: ObjectGuid) -> bool {
        self.controlled_guids.insert(guid)
    }

    pub fn remove_controlled(&mut self, guid: ObjectGuid) -> bool {
        if self.charmed_guid == Some(guid) {
            self.charmed_guid = None;
        }
        self.controlled_guids.remove(&guid)
    }

    pub fn clear_controlled(&mut self) {
        self.controlled_guids.clear();
        self.charmed_guid = None;
    }

    pub fn is_charmed(&self) -> bool {
        self.charmer_guid.is_some()
    }

    pub fn is_possessed(&self) -> bool {
        self.charm_type == Some(CharmType::Possess)
    }

    pub fn is_possessed_by_player(&self) -> bool {
        self.is_possessed() && self.controlled_by_player
    }

    pub fn is_possessing(&self) -> bool {
        self.charmed_guid.is_some()
    }

    pub fn is_possessing_guid(&self, guid: ObjectGuid) -> bool {
        self.charmed_guid == Some(guid)
    }

    pub fn charmer_or_owner_guid(&self) -> Option<ObjectGuid> {
        self.charmer_guid.or(self.owner_guid)
    }

    pub fn charmer_or_owner_or_self_guid(&self, own_guid: ObjectGuid) -> ObjectGuid {
        self.charmer_or_owner_guid().unwrap_or(own_guid)
    }

    pub fn init_charm_info(&mut self) -> &mut CharmInfoState {
        self.charm_info.get_or_insert_with(CharmInfoState::default)
    }

    pub fn delete_charm_info(&mut self) {
        self.charm_info = None;
    }

    pub fn has_charm_info(&self) -> bool {
        self.charm_info.is_some()
    }

    pub fn remove_all_controlled(&mut self) -> Vec<ObjectGuid> {
        let removed = self.controlled_guids.drain().collect();
        self.charmed_guid = None;
        removed
    }

    pub fn set_moved_unit(&mut self, target: Option<ObjectGuid>) {
        self.unit_moved_by_me = target;
    }

    pub fn set_player_moving_me(&mut self, player: Option<ObjectGuid>) {
        self.player_moving_me = player;
    }

    pub fn add_shared_vision(&mut self, guid: ObjectGuid) -> bool {
        self.shared_vision_guids.insert(guid)
    }

    pub fn remove_shared_vision(&mut self, guid: ObjectGuid) -> bool {
        self.shared_vision_guids.remove(&guid)
    }

    pub fn has_shared_vision(&self) -> bool {
        !self.shared_vision_guids.is_empty()
    }

    pub fn notify_controlled_owner_attacked_like_cpp(
        &mut self,
        controlled_creatures_with_ai: &[ObjectGuid],
        victim: ObjectGuid,
    ) {
        for controlled in controlled_creatures_with_ai {
            if self.controlled_guids.contains(controlled) {
                self.owner_attacked_notifications
                    .push(ControlledOwnerAttackedNotification {
                        controlled: *controlled,
                        victim,
                    });
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VehicleKitState {
    pub kit_id: u32,
    pub active: bool,
    pub installed: bool,
    pub vehicle: Option<Vehicle>,
}

impl VehicleKitState {
    pub const fn kit_id(&self) -> u32 {
        self.kit_id
    }

    pub const fn active(&self) -> bool {
        self.active
    }

    pub const fn installed(&self) -> bool {
        self.installed
    }

    pub const fn vehicle(&self) -> Option<&Vehicle> {
        self.vehicle.as_ref()
    }

    pub fn seat_count(&self) -> usize {
        self.vehicle
            .as_ref()
            .map_or(0, |vehicle| vehicle.seats().len())
    }

    pub fn usable_seat_num(&self) -> u32 {
        self.vehicle.as_ref().map_or(0, Vehicle::usable_seat_num)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleKitCreateOutcomeLikeCpp {
    pub kit_id: Option<u32>,
    pub created: bool,
    pub loading: bool,
    pub seat_count: usize,
    pub usable_seat_num: u32,
    pub unit_update_flag_vehicle_represented: bool,
    pub unit_type_mask_vehicle_represented: bool,
    pub send_set_vehicle_rec_id_represented: bool,
    pub set_spellclick_or_player_vehicle_npc_flag_represented: bool,
    pub remove_spellclick_or_player_vehicle_npc_flag_represented: bool,
    pub update_display_power_represented: bool,
    pub init_movement_info_for_base_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleKitInstallOutcomeLikeCpp {
    pub kit_id: Option<u32>,
    pub had_kit: bool,
    pub previous_installed: Option<bool>,
    pub installed: bool,
    pub script_on_install_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VehicleKitRemoveOutcomeLikeCpp {
    pub kit_id: Option<u32>,
    pub had_kit: bool,
    pub previous_installed: Option<bool>,
    pub on_remove_from_world: bool,
    pub send_set_vehicle_rec_id_zero_represented: bool,
    pub uninstall_represented: bool,
    pub remove_all_passengers_represented: bool,
    pub script_on_uninstall_represented: bool,
    pub kit_cleared: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleKitAddToWorldResetOutcomeLikeCpp {
    pub kit_id: u32,
    pub aim_create_represented: bool,
    pub ai_initialize_represented: bool,
    pub reset_evading: bool,
    pub reset_plan: VehicleResetPlan,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VehicleSubsystem {
    pub vehicle_guid: Option<ObjectGuid>,
    pub base_vehicle_guid: Option<ObjectGuid>,
    pub seat_id: Option<i8>,
    pub kit: Option<VehicleKitState>,
    pub last_create_outcome: Option<VehicleKitCreateOutcomeLikeCpp>,
}

impl VehicleSubsystem {
    pub fn enter_vehicle(&mut self, vehicle_guid: ObjectGuid, seat_id: Option<i8>) {
        self.vehicle_guid = Some(vehicle_guid);
        self.seat_id = seat_id;
    }

    pub fn exit_vehicle(&mut self) {
        self.vehicle_guid = None;
        self.seat_id = None;
    }

    pub fn set_vehicle_kit(&mut self, kit_id: u32, active: bool) {
        self.kit = Some(VehicleKitState {
            kit_id,
            active,
            installed: false,
            vehicle: None,
        });
    }

    pub fn create_vehicle_kit_like_cpp(
        &mut self,
        base_guid: ObjectGuid,
        base_position: Position,
        vehicle_id: Option<u32>,
        creature_entry: u32,
        loading: bool,
        seat_defs: Option<Vec<(i8, VehicleSeatInfo, VehicleSeatAddon)>>,
    ) -> VehicleKitCreateOutcomeLikeCpp {
        let Some(kit_id) = vehicle_id else {
            let outcome = VehicleKitCreateOutcomeLikeCpp {
                kit_id: None,
                created: false,
                loading,
                seat_count: 0,
                usable_seat_num: 0,
                unit_update_flag_vehicle_represented: false,
                unit_type_mask_vehicle_represented: false,
                send_set_vehicle_rec_id_represented: false,
                set_spellclick_or_player_vehicle_npc_flag_represented: false,
                remove_spellclick_or_player_vehicle_npc_flag_represented: false,
                update_display_power_represented: false,
                init_movement_info_for_base_represented: false,
            };
            self.last_create_outcome = Some(outcome.clone());
            return outcome;
        };
        let Some(seat_defs) = seat_defs else {
            let outcome = VehicleKitCreateOutcomeLikeCpp {
                kit_id: Some(kit_id),
                created: false,
                loading,
                seat_count: 0,
                usable_seat_num: 0,
                unit_update_flag_vehicle_represented: false,
                unit_type_mask_vehicle_represented: false,
                send_set_vehicle_rec_id_represented: false,
                set_spellclick_or_player_vehicle_npc_flag_represented: false,
                remove_spellclick_or_player_vehicle_npc_flag_represented: false,
                update_display_power_represented: false,
                init_movement_info_for_base_represented: false,
            };
            self.last_create_outcome = Some(outcome.clone());
            return outcome;
        };

        let vehicle = Vehicle::new(
            base_guid,
            TypeId::Unit,
            base_position,
            kit_id,
            creature_entry,
            seat_defs,
        );
        let seat_count = vehicle.seats().len();
        let usable_seat_num = vehicle.usable_seat_num();
        self.kit = Some(VehicleKitState {
            kit_id,
            active: true,
            installed: false,
            vehicle: Some(vehicle),
        });
        let outcome = VehicleKitCreateOutcomeLikeCpp {
            kit_id: Some(kit_id),
            created: true,
            loading,
            seat_count,
            usable_seat_num,
            unit_update_flag_vehicle_represented: true,
            unit_type_mask_vehicle_represented: true,
            send_set_vehicle_rec_id_represented: !loading,
            set_spellclick_or_player_vehicle_npc_flag_represented: usable_seat_num != 0,
            remove_spellclick_or_player_vehicle_npc_flag_represented: usable_seat_num == 0,
            update_display_power_represented: true,
            init_movement_info_for_base_represented: true,
        };
        self.last_create_outcome = Some(outcome.clone());
        outcome
    }

    pub fn install_vehicle_kit_like_cpp(&mut self) -> VehicleKitInstallOutcomeLikeCpp {
        let Some(kit) = self.kit.as_mut() else {
            return VehicleKitInstallOutcomeLikeCpp {
                kit_id: None,
                had_kit: false,
                previous_installed: None,
                installed: false,
                script_on_install_represented: false,
            };
        };

        let previous_installed = kit.installed;
        if !kit.installed {
            kit.installed = true;
            if let Some(vehicle) = kit.vehicle.as_mut() {
                vehicle.install();
            }
        }

        VehicleKitInstallOutcomeLikeCpp {
            kit_id: Some(kit.kit_id),
            had_kit: true,
            previous_installed: Some(previous_installed),
            installed: kit.installed,
            script_on_install_represented: true,
        }
    }

    pub fn reset_vehicle_kit_for_creature_add_to_world_like_cpp(
        &mut self,
        context: &CreatureAddToWorldVehicleResetContextLikeCpp,
        base_is_alive: bool,
    ) -> Option<VehicleKitAddToWorldResetOutcomeLikeCpp> {
        let kit = self.kit.as_mut()?;
        let vehicle = kit.vehicle.as_mut()?;
        let reset_plan = vehicle.reset_plan_like_cpp(
            false,
            base_is_alive,
            context.is_mechanical_creature,
            context.is_world_boss,
            &context.accessories,
        )?;

        Some(VehicleKitAddToWorldResetOutcomeLikeCpp {
            kit_id: kit.kit_id,
            aim_create_represented: true,
            ai_initialize_represented: true,
            reset_evading: false,
            reset_plan,
        })
    }

    pub fn remove_vehicle_kit_like_cpp(
        &mut self,
        on_remove_from_world: bool,
    ) -> VehicleKitRemoveOutcomeLikeCpp {
        let Some(kit) = self.kit.take() else {
            return VehicleKitRemoveOutcomeLikeCpp {
                kit_id: None,
                had_kit: false,
                previous_installed: None,
                on_remove_from_world,
                send_set_vehicle_rec_id_zero_represented: false,
                uninstall_represented: false,
                remove_all_passengers_represented: false,
                script_on_uninstall_represented: false,
                kit_cleared: false,
            };
        };

        if let Some(mut vehicle) = kit.vehicle {
            vehicle.uninstall();
        }

        VehicleKitRemoveOutcomeLikeCpp {
            kit_id: Some(kit.kit_id),
            had_kit: true,
            previous_installed: Some(kit.installed),
            on_remove_from_world,
            send_set_vehicle_rec_id_zero_represented: !on_remove_from_world,
            uninstall_represented: true,
            remove_all_passengers_represented: true,
            script_on_uninstall_represented: true,
            kit_cleared: true,
        }
    }

    pub fn clear_vehicle_kit(&mut self) {
        self.kit = None;
    }
}
