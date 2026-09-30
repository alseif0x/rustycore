use super::*;
use wow_entities::creature_create::CreatureCreateData;
use wow_movement::MoveSpline;

/// CREATE inputs and the immutable spline captured with them, never a motor.
#[derive(Debug)]
pub struct CreatureCreateFacts {
    data: CreatureCreateData,
    guid: ObjectGuid,
    entry: u32,
    map_id: u32,
    instance_id: u32,
    position: Position,
    world_combat_reach: f32,
    current_hp: u32,
    max_hp: u32,
    level: u8,
    npc_flags: u64,
    spline: Option<MoveSpline>,
}

impl CreatureCreateFacts {
    pub(super) fn compatible(creature: &Creature) -> Self {
        let mut data = WorldCreature::create_data_from_canonical_like_cpp(creature);
        // Match the presentation overrides applied by the old from_canonical
        // read bridge, without its aura-authority/motion/runtime mutations.
        let ai = creature.ai_ownership();
        data.npc_flags = (u64::from(ai.npc_flags2) << 32) | u64::from(ai.npc_flags);
        data.unit_flags = ai.unit_flags;
        data.unit_flags2 = ai.unit_flags2;
        data.unit_flags3 = ai.unit_flags3;
        data.damage_school = creature.melee_damage_school_like_cpp();
        data.ai_anim_kit_id = creature.unit().ai_anim_kit_id_like_cpp();
        data.movement_anim_kit_id = creature.unit().movement_anim_kit_id_like_cpp();
        data.melee_anim_kit_id = creature.unit().melee_anim_kit_id_like_cpp();
        Self {
            data,
            guid: creature.guid(),
            entry: creature.entry(),
            map_id: creature.unit().world().map_id(),
            instance_id: creature.unit().world().instance_id(),
            position: creature.position(),
            world_combat_reach: creature.unit().world().combat_reach(),
            current_hp: creature.ai_current_health().min(u64::from(u32::MAX)) as u32,
            max_hp: creature.ai_max_health().min(u64::from(u32::MAX)) as u32,
            level: creature.ai_level(),
            npc_flags: (u64::from(ai.npc_flags2) << 32) | u64::from(ai.npc_flags),
            spline: None,
        }
    }

    pub(super) fn legacy(creature: &WorldCreature) -> Self {
        Self {
            data: creature.create_data.clone(),
            guid: creature.guid(),
            entry: creature.entry(),
            map_id: creature.map_id(),
            instance_id: creature.instance_id(),
            position: creature.position(),
            world_combat_reach: creature.creature.unit().world().combat_reach(),
            current_hp: creature.current_hp(),
            max_hp: creature.max_hp(),
            level: creature.level(),
            npc_flags: creature.npc_flags_mask_like_cpp(),
            spline: creature.active_move_spline_like_cpp().cloned(),
        }
    }

    pub fn create_data(&self) -> &CreatureCreateData {
        &self.data
    }

    pub fn guid(&self) -> ObjectGuid {
        self.guid
    }

    pub fn entry(&self) -> u32 {
        self.entry
    }

    pub fn map_id(&self) -> u32 {
        self.map_id
    }

    pub fn instance_id(&self) -> u32 {
        self.instance_id
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn world_combat_reach(&self) -> f32 {
        self.world_combat_reach
    }

    pub fn current_hp(&self) -> u32 {
        self.current_hp
    }

    pub fn max_hp(&self) -> u32 {
        self.max_hp
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn npc_flags_mask(&self) -> u64 {
        self.npc_flags
    }

    pub fn active_move_spline(&self) -> Option<&MoveSpline> {
        self.spline.as_ref()
    }
}
