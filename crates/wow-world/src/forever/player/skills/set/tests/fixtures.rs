use super::*;
use crate::forever::player::skills::SkillUpdateState;
use std::cell::Cell;
use std::sync::Arc;
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{BirthRecords, SkillAbilityRecord, SkillLineRecord, SkillRaceClassRecord},
    forever_game_tables::InitialGameTables,
};
use wow_persistence::forever::creation::{
    ClassLevelStats, CreationWorldRows, RaceStats, SkillTierRow, StartDefinition,
};

pub(super) fn line(
    id: u32,
    category: i8,
    parent_skill: u32,
    parent_tier_index: i32,
) -> SkillLineRecord {
    SkillLineRecord {
        id,
        category,
        parent_skill,
        parent_tier_index,
        spell_icon_file: 0,
        can_link: 0,
        flags: 0,
        spell_book_spell: 0,
        expansion_name_shared_string: 0,
        horde_expansion_name_shared_string: 0,
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
pub(super) fn ability(id: u32, skill: u16, before: u32, after: u32) -> SkillAbilityRecord {
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
pub(super) fn rewards(skills: &[u16]) -> Vec<SkillAbilityRecord> {
    skills
        .iter()
        .enumerate()
        .map(|(id, &skill)| {
            let mut row = ability(id as u32 + 1, skill, 0, u32::from(skill) + 1000);
            row.acquire_method = 2;
            row
        })
        .collect()
}
pub(super) fn world(
    birth: Arc<BirthCatalog>,
    tiers: Vec<SkillTierRow>,
    order: Vec<u32>,
) -> WorldSources {
    let tables = InitialGameTables::parse_strs(
        "L\t1\t2\t3\t4\t5\t6\t7\t8\t9\t10\t11\t12\t13\t14\t15\n",
        "L\tHealth\n",
        "L\tTotal\tKill\tJunk\tStats\tDivisor\n1\t400\t0\t0\t0\t0\n2\t500\t0\t0\t0\t0\n",
    )
    .unwrap();
    let initialization = wow_data::forever_initialization::InitializationRecords {
        races: vec![wow_data::forever_initialization::RaceRecord {
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
        classes: vec![wow_data::forever_initialization::ClassRecord {
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
    .finish(
        Default::default(),
        Default::default(),
        &Db2HotfixRemovalStoreLikeCpp::default(),
    )
    .unwrap();
    WorldSources::load(
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
    .with_birth_skills(birth)
    .unwrap()
    .with_birth_skill_lookup(order)
    .unwrap()
}
pub(super) struct Fixture {
    pub birth: Arc<BirthCatalog>,
    pub world: WorldSources,
    pub spells: SpellDefinitionSeeds,
}
impl Fixture {
    pub fn new(lines: Vec<SkillLineRecord>, relations: Vec<SkillAbilityRecord>) -> Self {
        Self::with_rc(lines, relations, vec![], vec![], vec![])
    }
    pub fn with_rc(
        lines: Vec<SkillLineRecord>,
        relations: Vec<SkillAbilityRecord>,
        race_class: Vec<SkillRaceClassRecord>,
        tiers: Vec<SkillTierRow>,
        order: Vec<u32>,
    ) -> Self {
        let birth = Arc::new(
            BirthRecords {
                skill_lines: lines,
                abilities: relations,
                race_class,
                ..Default::default()
            }
            .finish(
                Default::default(),
                Default::default(),
                &Db2HotfixRemovalStoreLikeCpp::default(),
            )
            .unwrap(),
        );
        let world = world(Arc::clone(&birth), tiers, order);
        let spells = crate::forever::spells::skill_set_test_definitions(Arc::clone(&birth))
            .with_spell_ranks()
            .unwrap();
        Self {
            birth,
            world,
            spells,
        }
    }
    pub fn sources(&self) -> SkillSetSources<'_> {
        SkillSetSources::new(&self.world, &self.spells, 1, 1).unwrap()
    }
}
pub(super) fn update(skill: u32, step: u16, value: u16, maximum: u16) -> SkillUpdate {
    SkillUpdate {
        skill,
        step,
        value,
        maximum,
    }
}
pub(super) fn allocated(ids: impl IntoIterator<Item = u32>) -> PlayerSkills {
    PlayerSkills::from_ids(ids)
}
pub(super) fn activate(
    skills: &mut PlayerSkills,
    skill: u32,
    value: u16,
    maximum: u16,
    state: SkillUpdateState,
) {
    let status = skills.status.get_mut(&skill).unwrap();
    status.state = state;
    let field = &mut skills.fields[usize::from(status.slot)];
    field.step = 1;
    field.rank = value;
    field.maximum = maximum;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Event {
    Enchant(u32, u16, u16, u16),
    Learn(u32, u16, Option<SkillUpdateState>),
    Mount,
    Criteria(u32, SkillCriteria, u16),
    Aura(u32, u32, Option<SkillUpdateState>),
    Store(u8),
    Full,
    Remove(u32),
    PlayerCondition(u32),
    AbilityConditions(u32),
}
#[derive(Default)]
pub(super) struct Effects<'a> {
    pub events: Vec<Event>,
    pub full_at: Option<u8>,
    pub fail_at: Option<usize>,
    pub bonus: Option<(&'a BirthCatalog, i32)>,
    pub reenter: Option<(&'a SkillSetSources<'a>, SkillUpdate)>,
    pub level: Option<u8>,
    pub in_world: bool,
    pub deny_player: Option<u32>,
    pub deny_ability: Option<u32>,
    pub add_false: bool,
    pub flip_world_after_grant: bool,
    pub grants: Vec<(u32, u32, bool)>,
    pub level_queries: Cell<usize>,
    pub world_queries: Cell<usize>,
    pub level_after_grant: Option<u8>,
}
impl Effects<'_> {
    fn record(&mut self, event: Event) -> Result<(), &'static str> {
        self.events.push(event);
        if self.fail_at == Some(self.events.len() - 1) {
            Err("unavailable required effect")
        } else {
            Ok(())
        }
    }
    fn grant(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        skill: u32,
        live: bool,
    ) -> Result<(), &'static str> {
        self.record(Event::Learn(
            skill,
            skills.pure_value(skill),
            skills.status(skill).map(|(_, state)| state),
        ))?;
        self.grants.push((spell, skill, live));
        if let Some((sources, input)) = self.reenter.take() {
            skills
                .set_skill(input, sources, self)
                .map_err(|_| "nested required effect failed")?;
        }
        if self.flip_world_after_grant {
            self.in_world = !self.in_world;
        }
        if let Some(level) = self.level_after_grant {
            self.level = Some(level);
        }
        Ok(())
    }
}
impl SkillSetEffects for Effects<'_> {
    type Error = &'static str;
    fn player_level(&self) -> u8 {
        self.level_queries.set(self.level_queries.get() + 1);
        self.level.unwrap_or(1)
    }
    fn is_in_world(&self) -> bool {
        self.world_queries.set(self.world_queries.get() + 1);
        self.in_world
    }
    fn update_enchantments(
        &mut self,
        skills: &mut PlayerSkills,
        skill: u32,
        old: u16,
        new: u16,
    ) -> Result<(), Self::Error> {
        self.record(Event::Enchant(skill, old, new, skills.pure_value(skill)))
    }
    fn meets_player_condition(
        &mut self,
        _skills: &mut PlayerSkills,
        condition: u32,
    ) -> Result<bool, Self::Error> {
        self.record(Event::PlayerCondition(condition))?;
        Ok(self.deny_player != Some(condition))
    }
    fn meets_skill_ability_conditions(
        &mut self,
        _skills: &mut PlayerSkills,
        ability: u32,
    ) -> Result<bool, Self::Error> {
        self.record(Event::AbilityConditions(ability))?;
        Ok(self.deny_ability != Some(ability))
    }
    fn add_reward_spell(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        skill: u32,
    ) -> Result<bool, Self::Error> {
        self.grant(skills, spell, skill, false)?;
        Ok(!self.add_false)
    }
    fn learn_reward_spell(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        skill: u32,
    ) -> Result<(), Self::Error> {
        self.grant(skills, spell, skill, true)
    }
    fn update_mount_capability(&mut self, _skills: &mut PlayerSkills) -> Result<(), Self::Error> {
        self.record(Event::Mount)
    }
    fn update_criteria(
        &mut self,
        skills: &mut PlayerSkills,
        skill: u32,
        criterion: SkillCriteria,
    ) -> Result<(), Self::Error> {
        self.record(Event::Criteria(skill, criterion, skills.pure_value(skill)))
    }
    fn refresh_bonus_auras(
        &mut self,
        skills: &mut PlayerSkills,
        skill: u32,
        aura_type: u32,
    ) -> Result<(), Self::Error> {
        self.record(Event::Aura(
            skill,
            aura_type,
            skills.status(skill).map(|(_, state)| state),
        ))?;
        if let Some((birth, bonus)) = self.bonus
            && aura_type == 30
        {
            skills
                .modify_bonus(birth, skill, bonus, false)
                .map_err(|_| "bonus mutation failed")?;
        }
        Ok(())
    }
    fn store_profession_item(
        &mut self,
        _skills: &mut PlayerSkills,
        slot: u8,
    ) -> Result<ProfessionItemMove, Self::Error> {
        self.record(Event::Store(slot))?;
        Ok(if self.full_at == Some(slot) {
            ProfessionItemMove::InventoryFull
        } else {
            ProfessionItemMove::StoredOrAbsent
        })
    }
    fn display_inventory_full(&mut self, _skills: &mut PlayerSkills) -> Result<(), Self::Error> {
        self.record(Event::Full)
    }
    fn remove_spell(&mut self, _skills: &mut PlayerSkills, spell: u32) -> Result<(), Self::Error> {
        self.record(Event::Remove(spell))
    }
}
