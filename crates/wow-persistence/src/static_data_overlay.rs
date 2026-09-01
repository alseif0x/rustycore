//! SQLx-free sources for the remaining small DB2 overlays and enchant rules.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaTableHotfixRowLikeCpp {
    pub id: u32,
    pub continent_id: u16,
    pub parent_area_id: u16,
    pub area_bit: i16,
    pub exploration_level: i8,
    pub faction_group_mask: u8,
    pub mount_flags: i32,
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerTypeHotfixRowLikeCpp {
    pub id: u32,
    pub name_global_string_tag: String,
    pub cost_global_string_tag: String,
    pub power_type_enum: i8,
    pub min_power: i32,
    pub max_base_power: i32,
    pub center_power: i32,
    pub default_power: i32,
    pub display_modifier: i32,
    pub regen_interrupt_time_ms: i32,
    pub regen_peace: f32,
    pub regen_combat: f32,
    pub flags: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiMapXMapArtHotfixRowLikeCpp {
    pub id: u32,
    pub phase_id: i32,
    pub ui_map_art_id: i32,
    pub ui_map_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellEnchantProcPersistenceRowLikeCpp {
    pub enchant_id: u32,
    pub chance: f32,
    pub procs_per_minute: f32,
    pub hit_mask: u32,
    pub attributes_mask: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StaticDataRowsLoadOutcomeLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

/// One startup capability because C++ consumes this bounded family as immutable
/// DB2/rule overlays. It deliberately exposes no statement or generic table API.
pub trait StaticDataOverlayPersistencePortLikeCpp: Send + Sync {
    fn load_area_table_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StaticDataRowsLoadOutcomeLikeCpp<Vec<AreaTableHotfixRowLikeCpp>>,
    >;

    fn load_power_type_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StaticDataRowsLoadOutcomeLikeCpp<(
            Vec<PowerTypeHotfixRowLikeCpp>,
            Vec<PowerTypeHotfixRowLikeCpp>,
        )>,
    >;

    fn load_ui_map_x_map_art_hotfix_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StaticDataRowsLoadOutcomeLikeCpp<Vec<UiMapXMapArtHotfixRowLikeCpp>>,
    >;

    fn load_spell_enchant_proc_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        StaticDataRowsLoadOutcomeLikeCpp<Vec<SpellEnchantProcPersistenceRowLikeCpp>>,
    >;
}
