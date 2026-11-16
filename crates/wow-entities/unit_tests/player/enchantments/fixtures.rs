use crate::player::enchantments::{
    SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP, SkillEnchantmentEffectOperation,
    SkillEnchantmentEffects, SkillEnchantmentItemFacts, SkillEnchantmentItemTemplate,
    SkillEnchantmentRecord, SkillEnchantmentSource,
};
use crate::{
    PlayerSkills, ProfessionItemMove, SkillAbilityMetadata, SkillCatalog, SkillCriteria,
    SkillLineMetadata, SkillRaceClassMetadata, SkillRewardSpellMetadata, SkillSetEffects,
    SkillSetOutcome, SkillSetSource, SkillUpdate, SkillUpdateState,
};
use std::collections::BTreeMap;
use wow_core::ObjectGuid;

#[derive(Default)]
pub(super) struct Sources {
    pub(super) enchantments: BTreeMap<u32, SkillEnchantmentRecord>,
    pub(super) item_templates: BTreeMap<u32, SkillEnchantmentItemTemplate>,
    pub(super) scaling_columns: BTreeMap<(u8, i32), f32>,
    pub(super) scaling_row_count: usize,
    pub(super) missing_enchantment: Option<u32>,
    pub(super) failed_enchantment: Option<u32>,
    pub(super) failed_template: Option<u32>,
    pub(super) fail_scaling: bool,
    pub(super) skill_lines: BTreeMap<u32, SkillLineMetadata>,
}

impl Sources {
    pub fn add_enchantment(&mut self, row: SkillEnchantmentRecord) {
        self.enchantments.insert(row.id(), row);
    }

    pub fn add_template(&mut self, id: u32, inventory_type: i32) {
        self.item_templates
            .insert(id, SkillEnchantmentItemTemplate::new(inventory_type, 0, 0));
    }

    pub fn add_skill_line(&mut self, id: u32) {
        self.skill_lines.insert(
            id,
            SkillLineMetadata {
                id,
                category: 1,
                parent_skill: 0,
                parent_tier_index: 0,
            },
        );
    }
}

impl SkillEnchantmentSource for Sources {
    type Error = &'static str;

    fn enchantment(&self, id: u32) -> Result<Option<SkillEnchantmentRecord>, Self::Error> {
        if self.failed_enchantment == Some(id) {
            return Err("enchantment source failure");
        }
        if self.missing_enchantment == Some(id) {
            return Ok(None);
        }
        Ok(self.enchantments.get(&id).copied())
    }

    fn item_template(&self, id: u32) -> Result<Option<SkillEnchantmentItemTemplate>, Self::Error> {
        if self.failed_template == Some(id) {
            return Err("item-template source failure");
        }
        Ok(self.item_templates.get(&id).copied())
    }

    fn spell_scaling_row_count(&self) -> Result<usize, Self::Error> {
        if self.fail_scaling {
            return Err("scaling source failure");
        }
        Ok(self.scaling_row_count)
    }

    fn spell_scaling_column(&self, level: u8, class: i32) -> Result<Option<f32>, Self::Error> {
        if self.fail_scaling {
            return Err("scaling source failure");
        }
        Ok(self.scaling_columns.get(&(level, class)).copied())
    }
}

pub(super) struct Effects {
    pub(super) items: Vec<SkillEnchantmentItemFacts>,
    pub(super) level: u8,
    pub(super) limits: (u32, u32),
    pub(super) condition_fits: bool,
    pub(super) operations: Vec<SkillEnchantmentEffectOperation>,
    pub(super) fail_operation_at: Option<usize>,
    pub(super) callback_mutations: BTreeMap<usize, CallbackMutation>,
    pub(super) level_from_skill: Option<u32>,
    pub(super) skill_lines: BTreeMap<u32, SkillLineMetadata>,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            level: 70,
            limits: (0, 0),
            condition_fits: true,
            operations: Vec::new(),
            fail_operation_at: None,
            callback_mutations: BTreeMap::new(),
            level_from_skill: None,
            skill_lines: BTreeMap::new(),
        }
    }
}

pub(super) enum CallbackMutation {
    AddTemporarySkillBonus { skill: u32, amount: i32 },
    RemoveItem(ObjectGuid),
    ReplaceItem(SkillEnchantmentItemFacts),
}

impl Effects {
    pub fn add_item(&mut self, item: SkillEnchantmentItemFacts) {
        self.items.push(item);
    }

    pub fn add_skill_line(&mut self, id: u32) {
        self.skill_lines.insert(
            id,
            SkillLineMetadata {
                id,
                category: 1,
                parent_skill: 0,
                parent_tier_index: 0,
            },
        );
    }

    pub fn item(&self, guid: ObjectGuid) -> Option<SkillEnchantmentItemFacts> {
        self.items
            .iter()
            .copied()
            .find(|item| item.item_guid() == guid)
    }

    fn apply_callback_mutation(
        &mut self,
        skills: &mut PlayerSkills,
        callback_index: usize,
    ) -> Result<(), &'static str> {
        let Some(mutation) = self.callback_mutations.remove(&callback_index) else {
            return Ok(());
        };
        match mutation {
            CallbackMutation::AddTemporarySkillBonus { skill, amount } => {
                skills
                    .modify_bonus(self, skill, amount, false, || {})
                    .map_err(|_| "reentrant skill bonus failed")?;
            }
            CallbackMutation::RemoveItem(guid) => {
                self.items.retain(|item| item.item_guid() != guid);
            }
            CallbackMutation::ReplaceItem(replacement) => {
                self.items
                    .retain(|item| item.item_guid() != replacement.item_guid());
                self.items.push(replacement);
            }
        }
        Ok(())
    }
}

impl SkillEnchantmentEffects for Effects {
    type Error = &'static str;

    fn item_in_slot(&self, slot: u8) -> Result<Option<SkillEnchantmentItemFacts>, Self::Error> {
        Ok(self
            .items
            .iter()
            .copied()
            .find(|item| item.inventory_slot() == slot))
    }

    fn item_by_guid(
        &self,
        guid: ObjectGuid,
    ) -> Result<Option<SkillEnchantmentItemFacts>, Self::Error> {
        Ok(self.item(guid))
    }

    fn player_level(&self, skills: &PlayerSkills) -> Result<u8, Self::Error> {
        let Some(skill) = self.level_from_skill else {
            return Ok(self.level);
        };
        Ok(self.level.saturating_add(skills.value(skill) as u8))
    }

    fn item_level_limits(&self, _: &PlayerSkills) -> Result<(u32, u32), Self::Error> {
        Ok(self.limits)
    }

    fn enchantment_condition_fits(&self, _: &PlayerSkills, _: u32) -> Result<bool, Self::Error> {
        Ok(self.condition_fits)
    }

    fn apply_operation(
        &mut self,
        skills: &mut PlayerSkills,
        operation: SkillEnchantmentEffectOperation,
    ) -> Result<(), Self::Error> {
        let callback_index = self.operations.len();
        if self.fail_operation_at == Some(callback_index) {
            return Err("effect callback failure");
        }
        self.operations.push(operation);
        self.apply_callback_mutation(skills, callback_index)
    }
}

impl SkillCatalog for Effects {
    fn skill_line(&self, id: u32) -> Option<SkillLineMetadata> {
        self.skill_lines.get(&id).copied()
    }

    fn child_lines(&self, _: u32) -> impl Iterator<Item = SkillLineMetadata> + '_ {
        std::iter::empty()
    }

    fn abilities_for_skill(&self, _: u32) -> impl Iterator<Item = SkillAbilityMetadata> + '_ {
        std::iter::empty()
    }
}

impl SkillCatalog for Sources {
    fn skill_line(&self, id: u32) -> Option<SkillLineMetadata> {
        self.skill_lines.get(&id).copied()
    }

    fn child_lines(&self, _: u32) -> impl Iterator<Item = SkillLineMetadata> + '_ {
        std::iter::empty()
    }

    fn abilities_for_skill(&self, _: u32) -> impl Iterator<Item = SkillAbilityMetadata> + '_ {
        std::iter::empty()
    }
}

impl SkillSetSource for Sources {
    type Error = &'static str;

    fn race(&self) -> u8 {
        1
    }
    fn class(&self) -> u8 {
        1
    }

    fn default_skill_records(
        &self,
    ) -> Result<impl Iterator<Item = SkillRaceClassMetadata> + '_, Self::Error> {
        Ok(std::iter::empty())
    }

    fn skill_race_class_info(
        &self,
        skill: u32,
    ) -> Result<Option<SkillRaceClassMetadata>, Self::Error> {
        Ok(self
            .skill_lines
            .get(&skill)
            .map(|line| SkillRaceClassMetadata {
                id: line.id,
                skill: line.id as u16,
                flags: 0,
                min_level: 1,
                tier: 0,
            }))
    }

    fn skill_tier_value(&self, _: u32, _: u32) -> Option<u32> {
        None
    }

    fn default_skill_request(&self, _: SkillRaceClassMetadata, _: u8) -> Option<SkillUpdate> {
        None
    }

    fn reward_spell(&self, _: u32) -> Result<Option<SkillRewardSpellMetadata>, Self::Error> {
        Ok(None)
    }

    fn first_spell_in_chain(&self, spell: u32) -> u32 {
        spell
    }

    fn race_in_mask(&self, _: u64, _: u32) -> bool {
        false
    }
}

impl SkillSetEffects for Effects {
    type Error = &'static str;

    fn request_client_update(&mut self) {}
    fn player_level(&self) -> u8 {
        self.level
    }
    fn is_in_world(&self) -> bool {
        true
    }

    fn update_enchantments(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
        _: u16,
        _: u16,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn meets_player_condition(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn meets_skill_ability_conditions(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn add_reward_spell(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
        _: u32,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn learn_reward_spell(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
        _: u32,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn update_mount_capability(&mut self, _: &mut PlayerSkills) -> Result<(), Self::Error> {
        Ok(())
    }

    fn update_criteria(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
        _: SkillCriteria,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn refresh_bonus_auras(
        &mut self,
        _: &mut PlayerSkills,
        _: u32,
        _: u32,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn store_profession_item(
        &mut self,
        _: &mut PlayerSkills,
        _: u8,
    ) -> Result<ProfessionItemMove, Self::Error> {
        Ok(ProfessionItemMove::StoredOrAbsent)
    }

    fn display_inventory_full(&mut self, _: &mut PlayerSkills) -> Result<(), Self::Error> {
        Ok(())
    }

    fn remove_spell(&mut self, _: &mut PlayerSkills, _: u32) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub fn skills_with_value(skill: u32, value: u16) -> (PlayerSkills, Sources, Effects) {
    let mut sources = Sources::default();
    sources.add_skill_line(skill);
    let mut effects = Effects::default();
    effects.add_skill_line(skill);
    let mut skills = PlayerSkills::initialize([skill], || {});
    let outcome = skills
        .set_skill(
            SkillUpdate {
                skill,
                step: 0,
                value,
                maximum: value.max(1),
            },
            &sources,
            &mut effects,
        )
        .expect("fixture skill admission succeeds");
    assert_eq!(outcome, SkillSetOutcome::Finished);
    assert_eq!(skills.status(skill), Some((0, SkillUpdateState::New)));
    assert_eq!(skills.pure_value(skill), value);
    assert_eq!(skills.has_skill(skill), value != 0);
    effects.operations.clear();
    (skills, sources, effects)
}

pub fn enchantment(
    id: u32,
    effects: [u32; 3],
    amounts: [i32; 3],
    arguments: [u32; 3],
) -> SkillEnchantmentRecord {
    SkillEnchantmentRecord::new(
        id, effects, amounts, arguments, 0, [0.0; 3], 0, 0, 0, 0, 0, 0, 0,
    )
}

pub fn item_facts(
    guid_number: u32,
    item_id: u32,
    inventory_slot: u8,
    equipped: bool,
    broken: bool,
    enchantments: [u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
    durations: [u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
    socket_colors: [u32; 3],
    gems: [Option<u32>; 3],
) -> SkillEnchantmentItemFacts {
    SkillEnchantmentItemFacts::new(
        ObjectGuid::create_item(1, i64::from(guid_number)),
        item_id,
        inventory_slot,
        equipped,
        broken,
        enchantments,
        durations,
        socket_colors,
        gems,
    )
}
