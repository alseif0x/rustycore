use wow_core::ObjectGuid;

use crate::Creature;

pub const UNIT_MASK_SUMMON: u32 = 0x0000_0001;
pub const UNIT_MASK_MINION: u32 = 0x0000_0002;
pub const UNIT_MASK_GUARDIAN: u32 = 0x0000_0004;
pub const UNIT_MASK_TOTEM: u32 = 0x0000_0008;
pub const UNIT_MASK_PET: u32 = 0x0000_0010;
pub const UNIT_MASK_VEHICLE: u32 = 0x0000_0020;
pub const UNIT_MASK_HUNTER_PET: u32 = 0x0000_0080;
pub const UNIT_MASK_CONTROLABLE_GUARDIAN: u32 = 0x0000_0100;

pub const SUMMON_SLOT_ANY_TOTEM: i32 = -1;
pub const SUMMON_SLOT_TOTEM: i32 = 1;
pub const SUMMON_SLOT_TOTEM_2: i32 = 2;
pub const SUMMON_SLOT_TOTEM_3: i32 = 3;
pub const SUMMON_SLOT_TOTEM_4: i32 = 4;
pub const MAX_TOTEM_SLOT: i32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TotemType {
    Passive = 0,
    Active = 1,
    Statue = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TotemCreatedPacket {
    pub totem: ObjectGuid,
    pub slot: u8,
    pub duration_ms: i32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotemUpdateOutcome {
    Continue,
    Unsummon,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Totem {
    creature: Creature,
    unit_type_mask: u32,
    owner_guid: ObjectGuid,
    summoner_guid: ObjectGuid,
    properties_slot: i32,
    totem_type: TotemType,
    duration_ms: i32,
    follow_angle: f32,
    pending_unsummon_delay_ms: Option<u32>,
    unsummoned: bool,
    active_spell_has_cast_time: bool,
    cast_on_init: Vec<u32>,
    cleared_owner_totem_slot: Option<u8>,
}

impl Totem {
    pub fn new(owner_guid: ObjectGuid, properties_slot: i32) -> Self {
        Self {
            creature: Creature::new(false),
            unit_type_mask: UNIT_MASK_SUMMON | UNIT_MASK_MINION | UNIT_MASK_TOTEM,
            owner_guid,
            summoner_guid: owner_guid,
            properties_slot,
            totem_type: TotemType::Passive,
            duration_ms: 0,
            follow_angle: 0.0,
            pending_unsummon_delay_ms: None,
            unsummoned: false,
            active_spell_has_cast_time: false,
            cast_on_init: Vec::new(),
            cleared_owner_totem_slot: None,
        }
    }

    pub const fn creature(&self) -> &Creature {
        &self.creature
    }

    pub fn creature_mut(&mut self) -> &mut Creature {
        &mut self.creature
    }

    pub const fn unit_type_mask(&self) -> u32 {
        self.unit_type_mask
    }

    pub const fn is_totem(&self) -> bool {
        (self.unit_type_mask & UNIT_MASK_TOTEM) != 0
    }

    pub const fn owner_guid(&self) -> ObjectGuid {
        self.owner_guid
    }

    pub const fn summoner_guid(&self) -> ObjectGuid {
        self.summoner_guid
    }

    pub const fn properties_slot(&self) -> i32 {
        self.properties_slot
    }

    pub const fn totem_type(&self) -> TotemType {
        self.totem_type
    }

    pub const fn duration_ms(&self) -> i32 {
        self.duration_ms
    }

    pub fn set_totem_duration(&mut self, duration_ms: i32) {
        self.duration_ms = duration_ms;
    }

    pub const fn follow_angle(&self) -> f32 {
        self.follow_angle
    }

    pub fn set_follow_angle(&mut self, angle: f32) {
        self.follow_angle = angle;
    }

    pub const fn pending_unsummon_delay_ms(&self) -> Option<u32> {
        self.pending_unsummon_delay_ms
    }

    pub const fn is_unsummoned(&self) -> bool {
        self.unsummoned
    }

    pub const fn active_spell_has_cast_time(&self) -> bool {
        self.active_spell_has_cast_time
    }

    pub fn cast_on_init(&self) -> &[u32] {
        &self.cast_on_init
    }

    pub const fn cleared_owner_totem_slot(&self) -> Option<u8> {
        self.cleared_owner_totem_slot
    }

    pub fn get_spell(&self, slot: usize) -> u32 {
        self.creature
            .spells()
            .get(slot)
            .copied()
            .unwrap_or_default()
    }

    pub fn set_spell(&mut self, slot: usize, spell_id: u32) {
        self.creature.set_spell(slot, spell_id);
    }

    pub fn init_stats(
        &mut self,
        duration_ms: i32,
        owner_race_totem_display_id: Option<u32>,
        active_spell_has_cast_time: bool,
    ) {
        if let Some(display_id) = owner_race_totem_display_id {
            self.creature.set_display_id(
                display_id,
                true,
                Some(crate::CreatureModelDimensions {
                    bounding_radius: self.creature.unit().data().bounding_radius,
                    combat_reach: self.creature.unit().data().combat_reach,
                }),
            );
        }

        self.active_spell_has_cast_time = active_spell_has_cast_time;
        if active_spell_has_cast_time {
            self.totem_type = TotemType::Active;
        }
        self.duration_ms = duration_ms;
    }

    pub fn init_summon(&mut self) {
        self.cast_on_init.clear();
        if self.totem_type == TotemType::Passive && self.get_spell(0) != 0 {
            self.cast_on_init.push(self.get_spell(0));
        }
        if self.get_spell(1) != 0 {
            self.cast_on_init.push(self.get_spell(1));
        }
    }

    pub fn update(
        &mut self,
        diff_ms: u32,
        owner_alive: bool,
        self_alive: bool,
    ) -> TotemUpdateOutcome {
        if !owner_alive || !self_alive {
            self.unsummon(0, None);
            return TotemUpdateOutcome::Unsummon;
        }

        if self.duration_ms <= diff_ms as i32 {
            self.unsummon(0, None);
            return TotemUpdateOutcome::Unsummon;
        }

        self.duration_ms -= diff_ms as i32;
        TotemUpdateOutcome::Continue
    }

    pub fn unsummon(&mut self, ms_time: u32, owner_totem_slot: Option<u8>) {
        if ms_time != 0 {
            self.pending_unsummon_delay_ms = Some(ms_time);
            return;
        }

        self.pending_unsummon_delay_ms = None;
        self.unsummoned = true;
        self.cleared_owner_totem_slot = owner_totem_slot;
    }

    pub fn totem_created_packet(
        &self,
        totem_guid: ObjectGuid,
        selected_slot: i32,
        created_by_spell: u32,
    ) -> Option<TotemCreatedPacket> {
        if !(SUMMON_SLOT_TOTEM..MAX_TOTEM_SLOT).contains(&selected_slot) {
            return None;
        }

        Some(TotemCreatedPacket {
            totem: totem_guid,
            slot: (selected_slot - SUMMON_SLOT_TOTEM) as u8,
            duration_ms: self.duration_ms,
            spell_id: created_by_spell,
        })
    }

    pub fn positive_spell_effect_is_totem_immune(
        effect: SpellEffectKind,
        is_positive_spell: bool,
        target_is_unit_caster: bool,
        target_check_entry: bool,
    ) -> bool {
        effect != SpellEffectKind::Dummy
            && effect != SpellEffectKind::ScriptEffect
            && is_positive_spell
            && !target_is_unit_caster
            && !target_check_entry
    }

    pub const fn aura_effect_is_totem_immune(aura: SpellAuraKind) -> bool {
        matches!(
            aura,
            SpellAuraKind::PeriodicDamage
                | SpellAuraKind::PeriodicLeech
                | SpellAuraKind::ModFear
                | SpellAuraKind::Transform
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellEffectKind {
    Other,
    Dummy,
    ScriptEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellAuraKind {
    Other,
    PeriodicDamage,
    PeriodicLeech,
    ModFear,
    Transform,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_CREATURE_SPELLS;
    use wow_constants::{TypeId, TypeMask};
    use wow_core::guid::HighGuid;

    fn owner_guid() -> ObjectGuid {
        ObjectGuid::create_global(HighGuid::Player, 0, 1)
    }

    fn totem_guid() -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 530, 1, 100, 2)
    }

    #[test]
    fn totem_constructor_matches_cpp_minion_totem_base_state() {
        let totem = Totem::new(owner_guid(), SUMMON_SLOT_ANY_TOTEM);

        assert_eq!(
            totem.creature().unit().world().object().type_id(),
            TypeId::Unit
        );
        assert_eq!(
            totem.creature().unit().world().object().type_mask(),
            TypeMask::OBJECT | TypeMask::UNIT
        );
        assert_eq!(
            totem.unit_type_mask(),
            UNIT_MASK_SUMMON | UNIT_MASK_MINION | UNIT_MASK_TOTEM
        );
        assert!(totem.is_totem());
        assert_eq!(totem.owner_guid(), owner_guid());
        assert_eq!(totem.summoner_guid(), owner_guid());
        assert_eq!(totem.properties_slot(), SUMMON_SLOT_ANY_TOTEM);
        assert_eq!(totem.totem_type(), TotemType::Passive);
        assert_eq!(totem.duration_ms(), 0);
        assert!(!totem.is_unsummoned());
    }

    #[test]
    fn init_stats_and_summon_follow_cpp_spell_rules() {
        let mut passive = Totem::new(owner_guid(), SUMMON_SLOT_TOTEM);
        passive.set_spell(0, 100);
        passive.set_spell(1, 200);
        passive.init_stats(30_000, Some(1234), false);
        passive.init_summon();

        assert_eq!(passive.duration_ms(), 30_000);
        assert_eq!(passive.totem_type(), TotemType::Passive);
        assert_eq!(passive.get_spell(0), 100);
        assert_eq!(passive.get_spell(1), 200);
        assert_eq!(passive.cast_on_init(), &[100, 200]);
        assert_eq!(passive.creature().unit().data().display_id, 1234);

        let mut active = Totem::new(owner_guid(), SUMMON_SLOT_TOTEM);
        active.set_spell(0, 300);
        active.set_spell(1, 400);
        active.init_stats(30_000, None, true);
        active.init_summon();

        assert_eq!(active.totem_type(), TotemType::Active);
        assert_eq!(active.cast_on_init(), &[400]);
    }

    #[test]
    fn update_and_unsummon_follow_cpp_duration_shape() {
        let mut totem = Totem::new(owner_guid(), SUMMON_SLOT_TOTEM);
        totem.set_totem_duration(1_000);

        assert_eq!(totem.update(400, true, true), TotemUpdateOutcome::Continue);
        assert_eq!(totem.duration_ms(), 600);
        assert_eq!(totem.update(600, true, true), TotemUpdateOutcome::Unsummon);
        assert!(totem.is_unsummoned());

        let mut dead_owner = Totem::new(owner_guid(), SUMMON_SLOT_TOTEM);
        dead_owner.set_totem_duration(1_000);
        assert_eq!(
            dead_owner.update(1, false, true),
            TotemUpdateOutcome::Unsummon
        );

        let mut delayed = Totem::new(owner_guid(), SUMMON_SLOT_TOTEM);
        delayed.unsummon(500, None);
        assert_eq!(delayed.pending_unsummon_delay_ms(), Some(500));
        assert!(!delayed.is_unsummoned());
        delayed.unsummon(0, Some(2));
        assert!(delayed.is_unsummoned());
        assert_eq!(delayed.cleared_owner_totem_slot(), Some(2));
    }

    #[test]
    fn created_packet_uses_cpp_slot_offset() {
        let mut totem = Totem::new(owner_guid(), SUMMON_SLOT_TOTEM_3);
        totem.set_totem_duration(5_000);

        assert_eq!(
            totem.totem_created_packet(totem_guid(), SUMMON_SLOT_TOTEM_3, 123),
            Some(TotemCreatedPacket {
                totem: totem_guid(),
                slot: 2,
                duration_ms: 5_000,
                spell_id: 123,
            })
        );
        assert_eq!(
            totem.totem_created_packet(totem_guid(), SUMMON_SLOT_ANY_TOTEM, 123),
            None
        );
    }

    #[test]
    fn totem_immunity_predicates_match_cpp_special_cases() {
        assert!(Totem::positive_spell_effect_is_totem_immune(
            SpellEffectKind::Other,
            true,
            false,
            false
        ));
        assert!(!Totem::positive_spell_effect_is_totem_immune(
            SpellEffectKind::Dummy,
            true,
            false,
            false
        ));
        assert!(!Totem::positive_spell_effect_is_totem_immune(
            SpellEffectKind::Other,
            true,
            true,
            false
        ));
        assert!(Totem::aura_effect_is_totem_immune(
            SpellAuraKind::PeriodicDamage
        ));
        assert!(Totem::aura_effect_is_totem_immune(
            SpellAuraKind::PeriodicLeech
        ));
        assert!(Totem::aura_effect_is_totem_immune(SpellAuraKind::ModFear));
        assert!(Totem::aura_effect_is_totem_immune(SpellAuraKind::Transform));
        assert!(!Totem::aura_effect_is_totem_immune(SpellAuraKind::Other));
    }

    #[test]
    fn creature_spell_setter_ignores_out_of_range_slot_like_safe_bridge() {
        let mut totem = Totem::new(owner_guid(), SUMMON_SLOT_TOTEM);
        totem.set_spell(MAX_CREATURE_SPELLS, 1);
        assert_eq!(totem.creature().spells(), [0; MAX_CREATURE_SPELLS]);
    }
}
