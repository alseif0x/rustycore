//! Feature/test-only bindings and observations for the existing Kill APP family.
use super::*;
#[cfg(feature = "test-fixtures")]
pub use super::creature_kill_contracts::RepresentedCreatureKillEventLikeCpp as CreatureKillEvent;

mod registration;
pub use registration::{broadcast_info, broadcast_info_with_command};

pub struct CreatureKillObservations<'a> {
    pub events: &'a [RepresentedCreatureKillEventLikeCpp],
    pub pending_loot: &'a Vec<ObjectGuid>,
    pub pet_guid: &'a Option<ObjectGuid>,
    pub pet_created_by_spell: &'a u32,
    pub pet_react_state: &'a u8,
    pub pet_command_state: &'a u8,
}

impl WorldSession {
    pub fn fixture_kill_bind_player(
        &mut self,
        guid: ObjectGuid,
        placement: Option<(Position, Option<ObjectGuid>)>,
    ) {
        self.player_guid = Some(guid);
        if let Some((position, group_guid)) = placement {
            self.player_position = Some(position);
            self.group_guid = group_guid;
        }
    }

    pub fn fixture_kill_prepare_melee(
        &mut self, guid: ObjectGuid, target: ObjectGuid,
        level: u8, xp: u32, next_level_xp: u32,
    ) {
        self.player_guid = Some(guid);
        self.set_player_level_like_cpp(level);
        self.set_player_xp_like_cpp(xp);
        self.set_player_next_level_xp_like_cpp(next_level_xp);
        self.combat_target = Some(target);
        self.in_combat = true;
    }

    pub fn fixture_kill_set_pet_mode(
        &mut self, pet_guid: Option<ObjectGuid>, react_state: u8, command_state: u8,
    ) {
        self.set_represented_pet_mode_state_like_cpp(pet_guid, react_state, command_state);
    }

    pub async fn fixture_kill_apply_damage(
        &mut self, spell_id: Option<i32>, target_guid: ObjectGuid, damage_amount: u32,
    ) -> Result<(), &'static str> {
        let generators = self.id_generators_for_test_like_cpp();
        self.apply_damage_with_generator_like_cpp(
            generators.item.as_ref(), spell_id, target_guid, damage_amount,
        ).await
    }

    pub fn fixture_kill_tick_combat(&mut self) {
        self.tick_combat_sync();
    }

    pub fn fixture_kill_observations(&self) -> CreatureKillObservations<'_> {
        CreatureKillObservations {
            events: &self.represented_creature_kill_events_like_cpp,
            pending_loot: &self.pending_creature_kill_loot_like_cpp,
            pet_guid: &self.represented_pet_guid_like_cpp,
            pet_created_by_spell: &self.represented_pet_created_by_spell_like_cpp,
            pet_react_state: &self.represented_pet_react_state_like_cpp,
            pet_command_state: &self.represented_pet_command_state_like_cpp,
        }
    }

    pub fn fixture_kill_loot(&self, guid: ObjectGuid) -> Option<&wow_entities::CreatureLoot> {
        self.loot_table.get(&guid)
    }
}
