use super::*;
use crate::forever::{
    creation::{NumericItemTemplates, WorldSources},
    spells::{SpellConstructorFields, SpellDefinitionSeeds, SpellEffectValues},
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{
        BirthCatalog, BirthRecords, SkillAbilityRecord, SkillLineRecord, SkillRaceClassRecord,
        item_records::ItemRecords, item_specs::ItemSpecRecords,
    },
    forever_game_tables::InitialGameTables,
    forever_initialization::{ClassRecord, InitializationRecords, RaceRecord},
};
use wow_persistence::forever::{
    creation::{ClassLevelStats, CreationWorldRows, RaceStats, SkillTierRow, StartDefinition},
    spells::{SpellLearnRow, SpellRequiredRow},
};

pub(super) type SpellRow = (u32, SpellConstructorFields, u32, Vec<SpellEffectValues>);
pub(super) fn row(id: u32) -> SpellRow {
    (
        id,
        SpellConstructorFields {
            equipped_item_class: -1,
            ..Default::default()
        },
        0,
        vec![],
    )
}
pub(super) fn line(id: u32, category: i8) -> SkillLineRecord {
    SkillLineRecord {
        id,
        category,
        parent_skill: 0,
        parent_tier_index: 0,
        spell_icon_file: 0,
        can_link: 0,
        flags: 0,
        spell_book_spell: 0,
        expansion_name_shared_string: 0,
        horde_expansion_name_shared_string: 0,
    }
}
pub(super) fn ability(id: u32, before: u32, after: u32, skill: u16) -> SkillAbilityRecord {
    SkillAbilityRecord {
        id,
        skill_line: skill,
        spell: after as i32,
        min_skill_rank: 0,
        class_mask: 0,
        supercedes_spell: before as i32,
        acquire_method: 0,
        trivial_rank_high: 0,
        trivial_rank_low: 0,
        flags: 0,
        num_skill_ups: 0,
        unique_bit: 0,
        trade_skill_category: 0,
        skillup_skill_line: 0,
        field_5_5_4_67090_014: [0; 2],
        race_mask: 0,
    }
}
pub(super) fn rc(id: u32, skill: u16, tier: i16) -> SkillRaceClassRecord {
    SkillRaceClassRecord {
        id,
        skill,
        tier,
        race_mask: 0,
        class_mask: 0,
        flags: 0,
        availability: 1,
        min_level: 1,
    }
}
pub(super) fn spell_effect(kind: u32, aura: u32, skill: i32, value: f32) -> SpellEffectValues {
    SpellEffectValues {
        effect: kind,
        aura,
        misc_values: [skill, 0],
        base_points: value,
        ..Default::default()
    }
}
pub(super) struct Fixture {
    pub world: WorldSources,
    pub spells: SpellDefinitionSeeds,
    pub items: NumericItemTemplates,
}
impl Fixture {
    pub fn simple(rows: Vec<SpellRow>) -> Self {
        Self::new(rows, Default::default(), vec![], vec![], vec![])
    }
    pub fn new(
        rows: Vec<SpellRow>,
        birth: BirthRecords,
        tiers: Vec<SkillTierRow>,
        required: Vec<SpellRequiredRow>,
        learned: Vec<SpellLearnRow>,
    ) -> Self {
        let removal = Db2HotfixRemovalStoreLikeCpp::default();
        let birth: Arc<BirthCatalog> = Arc::new(
            birth
                .finish(Default::default(), Default::default(), &removal)
                .unwrap(),
        );
        let initialization = InitializationRecords {
            races: vec![RaceRecord {
                id: 1,
                flags: 0,
                faction: 0,
                cinematic: 0,
                resurrection_sickness_spell: 0,
                starting_level: 1,
                base_language: 0,
                creature_type: 0,
                alliance: 0,
                neutral_race: 0,
            }],
            classes: vec![ClassRecord {
                id: 1,
                flags: 0,
                starting_level: 1,
                cinematic: 0,
                default_spec: 0,
                strength_bonus: 0,
                primary_stat_priority: 0,
                display_power: 0,
                ranged_attack_per_agility: 0,
                attack_per_agility: 0,
                attack_per_strength: 0,
                spell_class_set: 0,
            }],
            ..Default::default()
        }
        .finish(Default::default(), Default::default(), &removal)
        .unwrap();
        let tables = InitialGameTables::parse_strs(
            "L\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\n",
            "L\tHealth\n",
            "L\tTotal\tKill\tJunk\tStats\tDivisor\n1\t400\t0\t0\t0\t0\n2\t500\t0\t0\t0\t0\n",
        )
        .unwrap();
        let order = birth.race_class_records().map(|rc| rc.id).collect(); // Synthetic, not native RC-order proof.
        let world = WorldSources::load(
            CreationWorldRows {
                definitions: vec![StartDefinition {
                    race: 1,
                    class: 1,
                    map: 0,
                    position: [0.; 4],
                    npe_map: None,
                    npe_position: [None; 4],
                    npe_transport: None,
                    intro_movie: None,
                    intro_scene: None,
                    npe_intro_scene: None,
                }],
                race_stats: vec![RaceStats {
                    race: 1,
                    modifiers: [0; 5],
                }],
                class_stats: vec![ClassLevelStats {
                    class: 1,
                    level: 1,
                    stats: [1; 5],
                }],
                skill_tiers: tiers,
                ..Default::default()
            },
            &initialization,
            &tables,
            2,
        )
        .unwrap()
        .with_birth_skills(Arc::clone(&birth))
        .unwrap()
        .with_birth_skill_lookup(order)
        .unwrap();
        let items = NumericItemTemplates::load(
            ItemRecords::default()
                .finish(Default::default(), Default::default(), &removal)
                .unwrap(),
            &ItemSpecRecords::default()
                .finish(Default::default(), Default::default(), &removal)
                .unwrap(),
            &initialization,
            vec![],
        )
        .unwrap();
        let spells =
            crate::forever::spells::spell_book_test_definitions(birth, rows, required, learned);
        Self {
            world,
            spells,
            items,
        }
    }
    pub fn sources(&self) -> SpellLearningSources<'_> {
        SpellLearningSources::new(&self.world, &self.spells, &self.items, 1, 1).unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Event {
    Cast(u32),
    Aura(u32),
    Fits(u32),
    Trait(i32),
    Points(u32),
    Skill(u32, u16, u16, u16),
    Criterion(SpellLearnCriterion, u32),
    MountQuery(u32),
    Mount(u32, bool, bool),
    Quest(u32),
    Message(SpellBookMessage, bool, bool),
    Order,
    OwnedAura(u32, Option<PlayerSpellState>),
    PetQuery(u32, u8),
    PetRemove(u32, u8),
    RemoveAura(u32),
    TitanOff,
    DualOff,
    Offhand,
}
#[derive(Default)]
pub(super) struct Effects<'a> {
    pub events: Vec<Event>,
    pub world: bool,
    pub loading: bool,
    pub form: u32,
    pub item_fits: bool,
    pub auras: HashSet<u32>,
    pub aura_states: HashSet<u32>,
    pub traits: HashMap<i32, u32>,
    pub free: u32,
    pub skills: HashMap<u32, (u16, u16)>,
    pub mounts: HashSet<u32>,
    pub fail: Option<Event>,
    pub bad_order: bool,
    pub cast_insert: Option<u32>,
    pub mount_reenter: bool,
    pub trait_unavailable: bool,
    pub skill_reenter: Option<(u32, u32)>,
    pub reentry_sources: Option<&'a SpellLearningSources<'a>>,
    pub pet_auras: HashSet<(u32, u8)>,
    pub pet_catalog_unavailable: bool,
    pub max_professions: u32,
    pub titan: bool,
    pub titan_penalty: u32,
    pub dual: bool,
    pub offhand: bool,
    pub aura_reenter: Option<(u32, RemovePlayerSpell)>,
}
impl Effects<'_> {
    pub fn quiet() -> Self {
        Self {
            loading: true,
            item_fits: true,
            max_professions: 2,
            ..Default::default()
        }
    }
    fn record(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail == Some(event) {
            Err("required effect failed")
        } else {
            Ok(())
        }
    }
}
impl SpellLearningEffects for Effects<'_> {
    type Error = &'static str;
    fn is_in_world(&self) -> bool {
        self.world
    }
    fn is_player_loading(&self) -> bool {
        self.loading
    }
    fn player_level(&self) -> u8 {
        10
    }
    fn shapeshift_form(&self) -> u32 {
        self.form
    }
    fn has_aura(&self, spell: u32) -> bool {
        self.auras.contains(&spell)
    }
    fn has_aura_state(&self, state: u32) -> bool {
        self.aura_states.contains(&state)
    }
    fn item_fits_spell(&mut self, view: &SpellDefinitionView<'_>) -> Result<bool, Self::Error> {
        self.record(Event::Fits(view.spell_id()))?;
        Ok(self.item_fits)
    }
    fn add_aura(&mut self, _: &mut PlayerSpellBook, spell: u32) -> Result<(), Self::Error> {
        self.record(Event::Aura(spell))
    }
    fn cast_triggered(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::Cast(spell))?;
        if let Some(id) = self.cast_insert.take() {
            book.add_temporary_spell(id);
        }
        Ok(())
    }
    fn trait_override(&mut self, id: i32) -> Result<Option<u32>, Self::Error> {
        self.record(Event::Trait(id))?;
        if self.trait_unavailable {
            return Err("trait catalog unavailable");
        }
        Ok(self.traits.get(&id).copied())
    }
    fn free_profession_points(&self) -> u32 {
        self.free
    }
    fn set_free_profession_points(&mut self, points: u32) -> Result<(), Self::Error> {
        self.record(Event::Points(points))?;
        self.free = points;
        Ok(())
    }
    fn pure_skill_value(&self, skill: u32) -> u16 {
        self.skills.get(&skill).map_or(0, |values| values.0)
    }
    fn pure_skill_maximum(&self, skill: u32) -> u16 {
        self.skills.get(&skill).map_or(0, |values| values.1)
    }
    fn has_skill(&self, skill: u32) -> bool {
        self.pure_skill_value(skill) != 0
    }
    fn set_skill(
        &mut self,
        book: &mut PlayerSpellBook,
        input: SkillUpdate,
    ) -> Result<(), Self::Error> {
        // Command observation only, NOT actual SetSkill/Unit or durability proof.
        self.record(Event::Skill(
            input.skill,
            input.step,
            input.value,
            input.maximum,
        ))?;
        self.skills
            .insert(input.skill, (input.value, input.maximum));
        if self
            .skill_reenter
            .is_some_and(|(skill, _)| input.skill == skill)
        {
            let (_, spell) = self.skill_reenter.take().unwrap();
            let sources = self.reentry_sources.expect("scoped test reentry sources");
            book.add_spell(AddPlayerSpell::learned(spell, true, true), sources, self)
                .map_err(|_| "reentrant learning failed")?;
        }
        Ok(())
    }
    fn update_criterion(
        &mut self,
        _: &mut PlayerSpellBook,
        kind: SpellLearnCriterion,
        id: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::Criterion(kind, id))
    }
    fn has_mount_definition(&mut self, spell: u32) -> Result<bool, Self::Error> {
        self.record(Event::MountQuery(spell))?;
        Ok(self.mounts.contains(&spell))
    }
    fn add_mount(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
        loading: bool,
    ) -> Result<(), Self::Error> {
        self.record(Event::Mount(spell, loading, book.has_spell(spell)))?;
        if self.mount_reenter {
            book.add_temporary_spell(spell);
        } // Same book: must not overwrite admitted node.
        Ok(())
    }
    fn quest_learn_spell(
        &mut self,
        _: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::Quest(spell))
    }
    fn book_order(
        &mut self,
        history: &[SpellBookMutation],
        _: usize,
    ) -> Result<Vec<u32>, Self::Error> {
        self.record(Event::Order)?;
        if self.bad_order {
            return Ok(vec![]);
        }
        let mut keys = vec![];
        for event in history {
            match *event {
                SpellBookMutation::Insert(id) => keys.push(id),
                SpellBookMutation::Erase(id) => keys.retain(|key| *key != id),
            }
        }
        keys.reverse();
        Ok(keys) // Synthetic order only, NOT pinned GNU proof.
    }
    fn publish(
        &mut self,
        book: &PlayerSpellBook,
        message: SpellBookMessage,
    ) -> Result<(), Self::Error> {
        let (old, new) = match message {
            SpellBookMessage::Superseded { old, new } => (old, new),
            SpellBookMessage::Learned { spell, .. } | SpellBookMessage::Unlearned { spell, .. } => {
                (spell, spell)
            }
        };
        self.record(Event::Message(
            message,
            book.has_active_spell(old),
            book.has_active_spell(new),
        ))
    }
    fn remove_owned_aura(
        &mut self,
        book: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::OwnedAura(
            spell,
            book.spell(spell).map(|entry| entry.state()),
        ))?;
        if self
            .aura_reenter
            .is_some_and(|(trigger, _)| trigger == spell)
        {
            let (_, request) = self.aura_reenter.take().unwrap();
            let sources = self.reentry_sources.expect("scoped test reentry sources");
            book.remove_spell(request, sources, self)
                .map_err(|_| "reentrant removal failed")?;
        }
        Ok(())
    }
    fn remove_pet_aura_if_defined(
        &mut self,
        _: &mut PlayerSpellBook,
        spell: u32,
        slot: u8,
    ) -> Result<(), Self::Error> {
        self.record(Event::PetQuery(spell, slot))?;
        if self.pet_catalog_unavailable {
            return Err("pet aura catalog unavailable");
        }
        if self.pet_auras.contains(&(spell, slot)) {
            self.record(Event::PetRemove(spell, slot))?;
        }
        Ok(())
    }
    fn max_primary_professions(&self) -> u32 {
        self.max_professions
    }
    fn can_titan_grip(&self) -> bool {
        self.titan
    }
    fn titan_grip_penalty_spell(&self) -> u32 {
        self.titan_penalty
    }
    fn remove_auras_due_to_spell(
        &mut self,
        _: &mut PlayerSpellBook,
        spell: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::RemoveAura(spell))
    }
    fn disable_titan_grip(&mut self) -> Result<(), Self::Error> {
        self.record(Event::TitanOff)?;
        self.titan = false;
        self.titan_penalty = 0;
        Ok(())
    }
    fn can_dual_wield(&self) -> bool {
        self.dual
    }
    fn disable_dual_wield(&mut self) -> Result<(), Self::Error> {
        self.record(Event::DualOff)?;
        self.dual = false;
        Ok(())
    }
    fn offhand_check_at_unlearn(&self) -> bool {
        self.offhand
    }
    fn auto_unequip_offhand(&mut self, _: &mut PlayerSpellBook) -> Result<(), Self::Error> {
        self.record(Event::Offhand)
    }
}
pub(super) fn insert(
    book: &mut PlayerSpellBook,
    id: u32,
    state: PlayerSpellState,
    active: bool,
    disabled: bool,
) {
    *book.try_emplace(id).0 = PlayerSpellEntry {
        state,
        active,
        disabled,
        ..Default::default()
    };
}
