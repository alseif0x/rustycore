//! Native Item `BonusData` values and ordered bonus-list application.

const MAX_ITEM_STATS: usize = 10;
const MAX_ITEM_SOCKETS: usize = 3;
const MAX_ITEM_EFFECTS: usize = 13;

const BONUS_ITEM_LEVEL: u32 = 1;
const BONUS_STAT: u32 = 2;
const BONUS_QUALITY: u32 = 3;
const BONUS_SUFFIX: u32 = 5;
const BONUS_SOCKET: u32 = 6;
const BONUS_APPEARANCE: u32 = 7;
const BONUS_REQUIRED_LEVEL: u32 = 8;
const BONUS_REPAIR_COST_MULTIPLIER: u32 = 10;
const BONUS_SCALING_STAT_DISTRIBUTION: u32 = 11;
const BONUS_DISENCHANT_LOOT_ID: u32 = 12;
const BONUS_SCALING_STAT_DISTRIBUTION_FIXED: u32 = 13;
const BONUS_BONDING: u32 = 16;
const BONUS_RELIC_TYPE: u32 = 17;
const BONUS_OVERRIDE_REQUIRED_LEVEL: u32 = 18;
const BONUS_AZERITE_TIER_UNLOCK_SET: u32 = 19;
const BONUS_OVERRIDE_CAN_DISENCHANT: u32 = 21;
const BONUS_OVERRIDE_CAN_SCRAP: u32 = 22;
const BONUS_ITEM_EFFECT_ID: u32 = 23;
const BONUS_REQUIRED_LEVEL_CURVE: u32 = 27;
const BONUS_ITEM_LIMIT_CATEGORY: u32 = 35;
const BONUS_PVP_ITEM_LEVEL_INCREMENT: u32 = 36;
const BONUS_OVERRIDE_CAN_SALVAGE: u32 = 39;
const BONUS_OVERRIDE_CAN_RECRAFT: u32 = 41;
const BONUS_ITEM_LEVEL_BASE: u32 = 42;
const BONUS_PVP_ITEM_LEVEL_BASE: u32 = 43;
const BONUS_OVERRIDE_CANNOT_TRADE_BOP: u32 = 46;
const BONUS_BONDING_WITH_PRIORITY: u32 = 47;
const BONUS_ITEM_OFFSET_CURVE: u32 = 48;
const BONUS_SCALING_CONFIG_AND_REQ_LEVEL: u32 = 49;
const BONUS_ITEM_BONUS_LIST: u32 = 50;
const BONUS_SCALING_CONFIG: u32 = 51;
const BONUS_CRAFTED_ITEM_LEVEL: u32 = 52;
const BONUS_SCALING_ITEM_LEVEL_BONUS: u32 = 53;

/// Immutable template facts needed to initialize native Item bonus state.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemBonusTemplateFacts {
    pub item_id: u32,
    pub quality: u32,
    pub item_level: u32,
    pub required_level: i32,
    pub item_stat_type: [i32; MAX_ITEM_STATS],
    pub stat_percent_editor: [i32; MAX_ITEM_STATS],
    pub item_stat_socket_cost_multiplier: [f32; MAX_ITEM_STATS],
    pub socket_color: [u32; MAX_ITEM_SOCKETS],
    pub bonding: u8,
    pub content_tuning_id: u32,
    pub player_level_to_item_level_curve_id: u32,
    pub item_level_offset_curve_id: u32,
    pub item_level_offset_item_level: u32,
    pub item_squish_era_id: u32,
    pub limit_category: u32,
    pub effect_ids: Vec<u32>,
    pub can_disenchant: bool,
    pub can_scrap: bool,
    pub can_salvage: bool,
    pub can_recraft: bool,
    pub cannot_trade_bind_on_pickup: bool,
}

/// One already-decoded row from a native ItemBonus list, in source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemBonusEntry {
    pub bonus_type: u32,
    pub values: [i32; 4],
}

/// Relevant immutable ItemScalingConfig fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemScalingConfigFacts {
    pub item_offset_curve_id: i32,
    pub item_level: i32,
    pub required_level: i32,
    pub item_squish_era_id: i32,
    pub flags: i32,
}

/// Relevant immutable ItemOffsetCurve fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemOffsetCurveFacts {
    pub curve_id: i32,
    pub offset: i32,
}

/// Relevant immutable ItemSquishEra fields at one native storage index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSquishEraFacts {
    pub patch: i32,
    pub flags: i32,
}

/// Borrowed source data for one synchronous BonusData operation.
///
/// Implementations must expose rows in native ItemBonus storage iteration order.
/// Missing lists yield an empty iterator; unavailable sources are errors.
pub trait ItemBonusSource {
    type Error;

    fn template_facts(&self, item_id: u32) -> Result<Option<ItemBonusTemplateFacts>, Self::Error>;

    fn azerite_tier_unlock_set_id(&self, item_id: u32) -> Result<Option<u32>, Self::Error>;

    fn bonus_list(&self, list_id: u32)
    -> Result<impl Iterator<Item = ItemBonusEntry>, Self::Error>;

    fn has_item_effect(&self, effect_id: u32) -> Result<bool, Self::Error>;

    fn scaling_config(&self, config_id: u32)
    -> Result<Option<ItemScalingConfigFacts>, Self::Error>;

    fn offset_curve(&self, curve_id: u32) -> Result<Option<ItemOffsetCurveFacts>, Self::Error>;

    /// Upper bound of the native ItemSquishEra ID-index table, not its row count.
    fn item_squish_era_index_bound(&self) -> Result<u32, Self::Error>;

    fn item_squish_era(&self, index: u32) -> Result<Option<ItemSquishEraFacts>, Self::Error>;

    /// The actual realm's already-resolved patch, or `None` when no realm exists.
    fn actual_realm_patch(&self) -> Option<i32>;
}

/// Failure while applying a BonusData prefix. Mutations completed before an error remain.
#[derive(Debug, PartialEq, Eq)]
pub enum ItemBonusDataError<E> {
    Source(E),
    BonusListCycle { list_id: u32 },
    EffectCapacityExceeded { effect_id: u32 },
    ArithmeticOverflow { field: &'static str },
    InvalidBondingValue { value: i32 },
}

#[derive(Debug, PartialEq, Eq)]
struct BonusDataPriorityState {
    suffix: i32,
    appearance: i32,
    disenchant_loot: i32,
    scaling_stat_distribution: i32,
    azerite_tier_unlock_set: i32,
    required_level_curve: i32,
    item_level: i32,
    pvp_item_level: i32,
    bonding: i32,
    scaling_config_item_level_bonus: i32,
    scaling_config_crafting_quality_item_level_bonus: i32,
    has_quality_bonus: bool,
    has_item_limit_category: bool,
}

impl BonusDataPriorityState {
    const fn zeroed() -> Self {
        Self {
            suffix: 0,
            appearance: 0,
            disenchant_loot: 0,
            scaling_stat_distribution: 0,
            azerite_tier_unlock_set: 0,
            required_level_curve: 0,
            item_level: 0,
            pvp_item_level: 0,
            bonding: 0,
            scaling_config_item_level_bonus: 0,
            scaling_config_crafting_quality_item_level_bonus: 0,
            has_quality_bonus: false,
            has_item_limit_category: false,
        }
    }

    const fn initialized() -> Self {
        Self {
            suffix: i32::MAX,
            appearance: i32::MAX,
            disenchant_loot: i32::MAX,
            scaling_stat_distribution: i32::MAX,
            azerite_tier_unlock_set: i32::MAX,
            required_level_curve: i32::MAX,
            item_level: i32::MAX,
            pvp_item_level: i32::MAX,
            bonding: i32::MAX,
            scaling_config_item_level_bonus: i32::MAX,
            scaling_config_crafting_quality_item_level_bonus: i32::MAX,
            has_quality_bonus: false,
            has_item_limit_category: false,
        }
    }
}

/// One non-clone native BonusData value owner.
///
/// `new()` is the source's zeroed constructor state. Call `initialize` before
/// using it as template-derived item data.
#[derive(Debug, PartialEq)]
pub struct ItemBonusData {
    quality: u32,
    item_level: u32,
    item_level_bonus: i32,
    required_level: i32,
    item_stat_type: [i32; MAX_ITEM_STATS],
    stat_percent_editor: [i32; MAX_ITEM_STATS],
    item_stat_socket_cost_multiplier: [f32; MAX_ITEM_STATS],
    socket_color: [u32; MAX_ITEM_SOCKETS],
    bonding: u8,
    appearance_mod_id: u32,
    repair_cost_multiplier: f32,
    content_tuning_id: u32,
    player_level_to_item_level_curve_id: u32,
    disenchant_loot_id: u32,
    gem_item_level_bonus: [u32; MAX_ITEM_SOCKETS],
    gem_relic_type: [i32; MAX_ITEM_SOCKETS],
    gem_relic_rank_bonus: [u16; MAX_ITEM_SOCKETS],
    relic_type: i32,
    required_level_override: i32,
    azerite_tier_unlock_set_id: i32,
    suffix: u32,
    required_level_curve: i32,
    pvp_item_level: u16,
    pvp_item_level_bonus: i16,
    item_level_offset_curve_id: u32,
    item_level_offset_item_level: u32,
    item_level_offset: i32,
    item_squish_era_id: u32,
    scaling_config_crafting_quality_item_level_bonus: i32,
    scaling_config_item_level_bonus: i32,
    effects: [u32; MAX_ITEM_EFFECTS],
    effect_count: usize,
    limit_category: u32,
    can_disenchant: bool,
    can_scrap: bool,
    can_salvage: bool,
    can_recraft: bool,
    has_fixed_level: bool,
    cannot_trade_bind_on_pickup: bool,
    ignore_squish: bool,
    restrict_scaling_to_content_tuning: bool,
    scaling_config_uses_player_level: bool,
    state: BonusDataPriorityState,
}

impl ItemBonusData {
    /// Zeroed C++ Item constructor state; this is not initialized template data.
    pub const fn new() -> Self {
        Self {
            quality: 0,
            item_level: 0,
            item_level_bonus: 0,
            required_level: 0,
            item_stat_type: [0; MAX_ITEM_STATS],
            stat_percent_editor: [0; MAX_ITEM_STATS],
            item_stat_socket_cost_multiplier: [0.0; MAX_ITEM_STATS],
            socket_color: [0; MAX_ITEM_SOCKETS],
            bonding: 0,
            appearance_mod_id: 0,
            repair_cost_multiplier: 0.0,
            content_tuning_id: 0,
            player_level_to_item_level_curve_id: 0,
            disenchant_loot_id: 0,
            gem_item_level_bonus: [0; MAX_ITEM_SOCKETS],
            gem_relic_type: [0; MAX_ITEM_SOCKETS],
            gem_relic_rank_bonus: [0; MAX_ITEM_SOCKETS],
            relic_type: 0,
            required_level_override: 0,
            azerite_tier_unlock_set_id: 0,
            suffix: 0,
            required_level_curve: 0,
            pvp_item_level: 0,
            pvp_item_level_bonus: 0,
            item_level_offset_curve_id: 0,
            item_level_offset_item_level: 0,
            item_level_offset: 0,
            item_squish_era_id: 0,
            scaling_config_crafting_quality_item_level_bonus: 0,
            scaling_config_item_level_bonus: 0,
            effects: [0; MAX_ITEM_EFFECTS],
            effect_count: 0,
            limit_category: 0,
            can_disenchant: false,
            can_scrap: false,
            can_salvage: false,
            can_recraft: false,
            has_fixed_level: false,
            cannot_trade_bind_on_pickup: false,
            ignore_squish: false,
            restrict_scaling_to_content_tuning: false,
            scaling_config_uses_player_level: false,
            state: BonusDataPriorityState::zeroed(),
        }
    }

    /// Reset to a template's facts, then apply its optional Azerite unlock.
    pub fn initialize<S: ItemBonusSource>(
        &mut self,
        template: &ItemBonusTemplateFacts,
        source: &S,
    ) -> Result<(), ItemBonusDataError<S::Error>> {
        self.quality = template.quality;
        self.item_level = template.item_level;
        self.item_level_bonus = 0;
        self.required_level = template.required_level;
        self.item_stat_type = template.item_stat_type;
        self.stat_percent_editor = template.stat_percent_editor;
        self.item_stat_socket_cost_multiplier = template.item_stat_socket_cost_multiplier;
        self.socket_color = template.socket_color;
        self.gem_item_level_bonus = [0; MAX_ITEM_SOCKETS];
        self.gem_relic_type = [-1; MAX_ITEM_SOCKETS];
        self.gem_relic_rank_bonus = [0; MAX_ITEM_SOCKETS];
        self.bonding = checked_bonding(i32::from(template.bonding))?;

        self.appearance_mod_id = 0;
        self.repair_cost_multiplier = 1.0;
        self.content_tuning_id = template.content_tuning_id;
        self.player_level_to_item_level_curve_id = template.player_level_to_item_level_curve_id;
        self.disenchant_loot_id = 0;
        self.relic_type = -1;
        self.has_fixed_level = false;
        self.required_level_override = 0;
        self.azerite_tier_unlock_set_id = 0;
        if let Some(unlock_set_id) = source
            .azerite_tier_unlock_set_id(template.item_id)
            .map_err(ItemBonusDataError::Source)?
        {
            self.azerite_tier_unlock_set_id = unlock_set_id as i32;
        }

        self.suffix = 0;
        self.required_level_curve = 0;
        self.pvp_item_level = 0;
        self.pvp_item_level_bonus = 0;
        self.item_level_offset_curve_id = template.item_level_offset_curve_id;
        self.item_level_offset_item_level = template.item_level_offset_item_level;
        self.item_level_offset = 0;
        self.item_squish_era_id = template.item_squish_era_id;
        self.scaling_config_item_level_bonus = 0;
        self.scaling_config_crafting_quality_item_level_bonus = 0;

        self.effect_count = 0;
        for &effect_id in &template.effect_ids {
            self.push_effect(effect_id)?;
        }
        for effect in &mut self.effects[self.effect_count..] {
            *effect = 0;
        }

        self.limit_category = template.limit_category;
        self.can_disenchant = template.can_disenchant;
        self.can_scrap = template.can_scrap;
        self.can_salvage = template.can_salvage;
        self.can_recraft = template.can_recraft;
        self.cannot_trade_bind_on_pickup = template.cannot_trade_bind_on_pickup;
        self.ignore_squish = false;
        self.restrict_scaling_to_content_tuning = false;
        self.scaling_config_uses_player_level = false;
        self.state = BonusDataPriorityState::initialized();
        Ok(())
    }

    /// Missing templates leave every value and priority unchanged.
    pub fn initialize_from_instance<S: ItemBonusSource>(
        &mut self,
        item_id: u32,
        bonus_list_ids: &[u32],
        source: &S,
    ) -> Result<(), ItemBonusDataError<S::Error>> {
        let Some(template) = source
            .template_facts(item_id)
            .map_err(ItemBonusDataError::Source)?
        else {
            return Ok(());
        };

        self.initialize(&template, source)?;
        for &list_id in bonus_list_ids {
            self.add_bonus_list(list_id, source)?;
        }
        Ok(())
    }

    /// Apply a list in source row order, expanding nested lists depth first.
    pub fn add_bonus_list<S: ItemBonusSource>(
        &mut self,
        list_id: u32,
        source: &S,
    ) -> Result<(), ItemBonusDataError<S::Error>> {
        let mut stack = vec![(
            list_id,
            source
                .bonus_list(list_id)
                .map_err(ItemBonusDataError::Source)?,
        )];

        loop {
            let next_row = stack.last_mut().and_then(|(_, rows)| rows.next());
            let Some(row) = next_row else {
                if stack.pop().is_none() {
                    break;
                }
                continue;
            };
            if row.bonus_type == BONUS_ITEM_BONUS_LIST {
                let nested_id = row.values[0] as u32;
                if stack.iter().any(|(active_id, _)| *active_id == nested_id) {
                    return Err(ItemBonusDataError::BonusListCycle { list_id: nested_id });
                }
                stack.push((
                    nested_id,
                    source
                        .bonus_list(nested_id)
                        .map_err(ItemBonusDataError::Source)?,
                ));
            } else {
                self.apply_bonus_row(row.bonus_type, row.values, source)?;
            }
        }
        Ok(())
    }

    /// Apply one native bonus row. Nested-list rows use the same guarded traversal.
    pub fn add_bonus<S: ItemBonusSource>(
        &mut self,
        bonus_type: u32,
        values: [i32; 4],
        source: &S,
    ) -> Result<(), ItemBonusDataError<S::Error>> {
        if bonus_type == BONUS_ITEM_BONUS_LIST {
            self.add_bonus_list(values[0] as u32, source)
        } else {
            self.apply_bonus_row(bonus_type, values, source)
        }
    }

    fn apply_bonus_row<S: ItemBonusSource>(
        &mut self,
        bonus_type: u32,
        values: [i32; 4],
        source: &S,
    ) -> Result<(), ItemBonusDataError<S::Error>> {
        match bonus_type {
            BONUS_ITEM_LEVEL => add_i32(&mut self.item_level_bonus, values[0], "item_level_bonus")?,
            BONUS_STAT => {
                let index = self
                    .item_stat_type
                    .iter()
                    .position(|kind| *kind == values[0] || *kind == -1);
                if let Some(index) = index {
                    self.item_stat_type[index] = values[0];
                    add_i32(
                        &mut self.stat_percent_editor[index],
                        values[1],
                        "stat_percent_editor",
                    )?;
                }
            }
            BONUS_QUALITY => {
                let quality = values[0] as u32;
                if !self.state.has_quality_bonus {
                    self.quality = quality;
                    self.state.has_quality_bonus = true;
                } else if self.quality < quality {
                    self.quality = quality;
                }
            }
            BONUS_SUFFIX => {
                if values[1] < self.state.suffix {
                    self.suffix = values[0] as u32;
                    self.state.suffix = values[1];
                }
            }
            BONUS_SOCKET => {
                let mut socket_count = values[0] as u32;
                for socket in &mut self.socket_color {
                    if socket_count == 0 {
                        break;
                    }
                    if *socket == 0 {
                        *socket = values[1] as u32;
                        socket_count -= 1;
                    }
                }
            }
            BONUS_APPEARANCE => {
                if values[1] < self.state.appearance {
                    self.appearance_mod_id = values[0] as u32;
                    self.state.appearance = values[1];
                }
            }
            BONUS_REQUIRED_LEVEL => {
                add_i32(&mut self.required_level, values[0], "required_level")?;
            }
            BONUS_REPAIR_COST_MULTIPLIER => {
                self.repair_cost_multiplier *= (values[0] as f32) * 0.01_f32;
            }
            BONUS_SCALING_STAT_DISTRIBUTION | BONUS_SCALING_STAT_DISTRIBUTION_FIXED => {
                if values[1] < self.state.scaling_stat_distribution {
                    self.content_tuning_id = values[2] as u32;
                    self.player_level_to_item_level_curve_id = values[3] as u32;
                    self.state.scaling_stat_distribution = values[1];
                    self.has_fixed_level = bonus_type == BONUS_SCALING_STAT_DISTRIBUTION_FIXED;
                }
            }
            BONUS_DISENCHANT_LOOT_ID => {
                if values[1] < self.state.disenchant_loot {
                    self.disenchant_loot_id = values[0] as u32;
                    self.state.disenchant_loot = values[1];
                }
            }
            BONUS_BONDING => self.bonding = checked_bonding(values[0])?,
            BONUS_RELIC_TYPE => self.relic_type = values[0],
            BONUS_OVERRIDE_REQUIRED_LEVEL => self.required_level_override = values[0],
            BONUS_AZERITE_TIER_UNLOCK_SET => {
                if values[1] < self.state.azerite_tier_unlock_set {
                    self.azerite_tier_unlock_set_id = values[0];
                    self.state.azerite_tier_unlock_set = values[1];
                }
            }
            BONUS_OVERRIDE_CAN_DISENCHANT => self.can_disenchant = values[0] != 0,
            BONUS_OVERRIDE_CAN_SCRAP => self.can_scrap = values[0] != 0,
            BONUS_ITEM_EFFECT_ID => {
                let effect_id = values[0] as u32;
                if source
                    .has_item_effect(effect_id)
                    .map_err(ItemBonusDataError::Source)?
                {
                    self.push_effect(effect_id)?;
                }
            }
            BONUS_REQUIRED_LEVEL_CURVE => {
                if values[2] < self.state.required_level_curve {
                    self.required_level_curve = values[0];
                    self.state.required_level_curve = values[2];
                    if values[1] != 0 {
                        self.content_tuning_id = values[1] as u32;
                    }
                }
            }
            BONUS_ITEM_LIMIT_CATEGORY => {
                if !self.state.has_item_limit_category {
                    self.limit_category = values[0] as u32;
                    self.state.has_item_limit_category = true;
                }
            }
            BONUS_PVP_ITEM_LEVEL_INCREMENT => {
                let sum = i32::from(self.pvp_item_level_bonus)
                    .checked_add(values[0])
                    .ok_or(ItemBonusDataError::ArithmeticOverflow {
                        field: "pvp_item_level_bonus",
                    })?;
                self.pvp_item_level_bonus = sum as i16;
            }
            BONUS_OVERRIDE_CAN_SALVAGE => self.can_salvage = values[0] != 0,
            BONUS_OVERRIDE_CAN_RECRAFT => self.can_recraft = values[0] != 0,
            BONUS_ITEM_LEVEL_BASE => {
                if values[1] < self.state.item_level {
                    self.item_level = values[0] as u32;
                    self.state.item_level = values[1];
                }
            }
            BONUS_PVP_ITEM_LEVEL_BASE => {
                if values[1] < self.state.pvp_item_level {
                    self.pvp_item_level = values[0] as u16;
                    self.state.pvp_item_level = values[1];
                }
            }
            BONUS_OVERRIDE_CANNOT_TRADE_BOP => {
                self.cannot_trade_bind_on_pickup = values[0] != 0;
            }
            BONUS_BONDING_WITH_PRIORITY => {
                if values[1] < self.state.bonding {
                    self.bonding = checked_bonding(values[0])?;
                    self.state.bonding = values[1];
                }
            }
            BONUS_ITEM_OFFSET_CURVE => {
                if values[3] < self.state.scaling_stat_distribution {
                    self.item_level_offset_curve_id = values[0] as u32;
                    self.item_level_offset_item_level = values[1] as u32;
                    self.state.scaling_stat_distribution = values[3];
                }
            }
            BONUS_SCALING_CONFIG_AND_REQ_LEVEL => {
                if values[1] < self.state.scaling_stat_distribution {
                    self.state.scaling_stat_distribution = values[1];
                    if let Some(config) = source
                        .scaling_config(values[0] as u32)
                        .map_err(ItemBonusDataError::Source)?
                    {
                        self.apply_scaling_config_offset(config, source)?;
                        self.scaling_config_uses_player_level = false;
                        self.item_squish_era_id = config.item_squish_era_id as u32;
                        self.apply_scaling_flags(config.flags);
                        if values[1] < self.state.required_level_curve {
                            self.item_level_offset_item_level = config.item_level as u32;
                            self.required_level_override = config.required_level;
                            self.required_level_curve = 0;
                        }
                    }
                }
            }
            BONUS_ITEM_BONUS_LIST => {}
            BONUS_SCALING_CONFIG => {
                if values[1] < self.state.scaling_stat_distribution {
                    self.state.scaling_stat_distribution = values[1];
                    if let Some(config) = source
                        .scaling_config(values[0] as u32)
                        .map_err(ItemBonusDataError::Source)?
                    {
                        self.apply_scaling_config_offset(config, source)?;
                        self.scaling_config_uses_player_level = true;
                        self.item_level_offset_item_level = 0;
                        self.item_squish_era_id = config.item_squish_era_id as u32;
                        self.apply_scaling_flags(config.flags);
                    }
                }
            }
            BONUS_CRAFTED_ITEM_LEVEL => {
                if values[3] < self.state.scaling_config_crafting_quality_item_level_bonus {
                    let mut is_squished = false;
                    if let Some(realm_patch) = source.actual_realm_patch() {
                        let mut squish_index = values[1].checked_add(1).ok_or(
                            ItemBonusDataError::ArithmeticOverflow {
                                field: "crafted_item_level_squish_start",
                            },
                        )? as u32;
                        let index_bound = source
                            .item_squish_era_index_bound()
                            .map_err(ItemBonusDataError::Source)?;
                        while squish_index < index_bound {
                            if let Some(era) = source
                                .item_squish_era(squish_index)
                                .map_err(ItemBonusDataError::Source)?
                                && era.flags & 1 == 0
                            {
                                is_squished = era.patch <= realm_patch;
                                break;
                            }
                            squish_index += 1;
                        }
                    }
                    self.scaling_config_crafting_quality_item_level_bonus =
                        values[if is_squished { 2 } else { 0 }];
                    self.state.scaling_config_crafting_quality_item_level_bonus = values[3];
                }
            }
            BONUS_SCALING_ITEM_LEVEL_BONUS => {
                if values[1] < self.state.scaling_config_item_level_bonus {
                    self.scaling_config_item_level_bonus = values[0];
                    self.state.scaling_config_item_level_bonus = values[1];
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn apply_scaling_config_offset<S: ItemBonusSource>(
        &mut self,
        config: ItemScalingConfigFacts,
        source: &S,
    ) -> Result<(), ItemBonusDataError<S::Error>> {
        if let Some(offset) = source
            .offset_curve(config.item_offset_curve_id as u32)
            .map_err(ItemBonusDataError::Source)?
        {
            self.item_level_offset_curve_id = offset.curve_id as u32;
            self.item_level_offset = offset.offset;
        }
        Ok(())
    }

    fn apply_scaling_flags(&mut self, flags: i32) {
        if flags & 1 != 0 {
            self.ignore_squish = true;
        }
        if flags & 2 != 0 {
            self.restrict_scaling_to_content_tuning = true;
        }
    }

    fn push_effect<E>(&mut self, effect_id: u32) -> Result<(), ItemBonusDataError<E>> {
        if self.effect_count == MAX_ITEM_EFFECTS {
            return Err(ItemBonusDataError::EffectCapacityExceeded { effect_id });
        }
        self.effects[self.effect_count] = effect_id;
        self.effect_count += 1;
        Ok(())
    }

    pub const fn quality(&self) -> u32 {
        self.quality
    }
    pub const fn item_level(&self) -> u32 {
        self.item_level
    }
    pub const fn item_level_bonus(&self) -> i32 {
        self.item_level_bonus
    }
    pub const fn required_level(&self) -> i32 {
        self.required_level
    }
    pub const fn item_stat_type(&self) -> &[i32; MAX_ITEM_STATS] {
        &self.item_stat_type
    }
    pub const fn stat_percent_editor(&self) -> &[i32; MAX_ITEM_STATS] {
        &self.stat_percent_editor
    }
    pub const fn item_stat_socket_cost_multiplier(&self) -> &[f32; MAX_ITEM_STATS] {
        &self.item_stat_socket_cost_multiplier
    }
    pub const fn socket_color(&self) -> &[u32; MAX_ITEM_SOCKETS] {
        &self.socket_color
    }
    pub const fn bonding(&self) -> u8 {
        self.bonding
    }
    pub const fn appearance_mod_id(&self) -> u32 {
        self.appearance_mod_id
    }
    pub const fn repair_cost_multiplier(&self) -> f32 {
        self.repair_cost_multiplier
    }
    pub const fn content_tuning_id(&self) -> u32 {
        self.content_tuning_id
    }
    pub const fn player_level_to_item_level_curve_id(&self) -> u32 {
        self.player_level_to_item_level_curve_id
    }
    pub const fn disenchant_loot_id(&self) -> u32 {
        self.disenchant_loot_id
    }
    pub const fn gem_item_level_bonus(&self) -> &[u32; MAX_ITEM_SOCKETS] {
        &self.gem_item_level_bonus
    }
    pub const fn gem_relic_type(&self) -> &[i32; MAX_ITEM_SOCKETS] {
        &self.gem_relic_type
    }
    pub const fn gem_relic_rank_bonus(&self) -> &[u16; MAX_ITEM_SOCKETS] {
        &self.gem_relic_rank_bonus
    }
    pub const fn relic_type(&self) -> i32 {
        self.relic_type
    }
    pub const fn required_level_override(&self) -> i32 {
        self.required_level_override
    }
    pub const fn azerite_tier_unlock_set_id(&self) -> i32 {
        self.azerite_tier_unlock_set_id
    }
    pub const fn suffix(&self) -> u32 {
        self.suffix
    }
    pub const fn required_level_curve(&self) -> i32 {
        self.required_level_curve
    }
    pub const fn pvp_item_level(&self) -> u16 {
        self.pvp_item_level
    }
    pub const fn pvp_item_level_bonus(&self) -> i16 {
        self.pvp_item_level_bonus
    }
    pub const fn item_level_offset_curve_id(&self) -> u32 {
        self.item_level_offset_curve_id
    }
    pub const fn item_level_offset_item_level(&self) -> u32 {
        self.item_level_offset_item_level
    }
    pub const fn item_level_offset(&self) -> i32 {
        self.item_level_offset
    }
    pub const fn item_squish_era_id(&self) -> u32 {
        self.item_squish_era_id
    }
    pub const fn scaling_config_crafting_quality_item_level_bonus(&self) -> i32 {
        self.scaling_config_crafting_quality_item_level_bonus
    }
    pub const fn scaling_config_item_level_bonus(&self) -> i32 {
        self.scaling_config_item_level_bonus
    }
    pub fn effects(&self) -> &[u32] {
        &self.effects[..self.effect_count]
    }
    pub const fn limit_category(&self) -> u32 {
        self.limit_category
    }
    pub const fn can_disenchant(&self) -> bool {
        self.can_disenchant
    }
    pub const fn can_scrap(&self) -> bool {
        self.can_scrap
    }
    pub const fn can_salvage(&self) -> bool {
        self.can_salvage
    }
    pub const fn can_recraft(&self) -> bool {
        self.can_recraft
    }
    pub const fn has_fixed_level(&self) -> bool {
        self.has_fixed_level
    }
    pub const fn cannot_trade_bind_on_pickup(&self) -> bool {
        self.cannot_trade_bind_on_pickup
    }
    pub const fn ignore_squish(&self) -> bool {
        self.ignore_squish
    }
    pub const fn restrict_scaling_to_content_tuning(&self) -> bool {
        self.restrict_scaling_to_content_tuning
    }
    pub const fn scaling_config_uses_player_level(&self) -> bool {
        self.scaling_config_uses_player_level
    }
}

fn checked_bonding<E>(value: i32) -> Result<u8, ItemBonusDataError<E>> {
    if (0..=15).contains(&value) {
        Ok(value as u8)
    } else {
        Err(ItemBonusDataError::InvalidBondingValue { value })
    }
}

fn add_i32<E>(
    target: &mut i32,
    value: i32,
    field: &'static str,
) -> Result<(), ItemBonusDataError<E>> {
    *target = target
        .checked_add(value)
        .ok_or(ItemBonusDataError::ArithmeticOverflow { field })?;
    Ok(())
}

#[cfg(test)]
#[path = "../../unit_tests/item/bonuses/mod.rs"]
mod tests;
