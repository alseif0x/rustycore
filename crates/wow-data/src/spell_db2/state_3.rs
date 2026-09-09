//! DB2 spell entry stores state definitions, part 3 of 3.
//!
//! Separated from the spell_db2.rs root under #646. Behaviour is preserved.

use super::*;

impl SpellVisualEffectNameStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "SpellVisualEffectName.db2",
            |id, idx, r| SpellVisualEffectNameEntry {
                id,
                model_file_data_id: r.get_field_i32(idx, 0),
                base_missile_speed: f32_field(r, idx, 1),
                scale: f32_field(r, idx, 2),
                min_allowed_scale: f32_field(r, idx, 3),
                max_allowed_scale: f32_field(r, idx, 4),
                alpha: f32_field(r, idx, 5),
                flags: r.get_field_u32(idx, 6),
                texture_file_data_id: r.get_field_i32(idx, 7),
                effect_radius: f32_field(r, idx, 8),
                effect_type: r.get_field_u32(idx, 9),
                generic_id: r.get_field_i32(idx, 10),
                ribbon_quality_id: r.get_field_u32(idx, 11),
                dissolve_effect_id: r.get_field_i32(idx, 12),
                model_position: r.get_field_i32(idx, 13),
            },
        )
    }
}

impl SpellVisualKitStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellVisualKit.db2", |id, idx, r| {
            SpellVisualKitEntry {
                id,
                fallback_spell_visual_kit_id: r.get_field_u32(idx, 0),
                delay_min: r.get_field_u16(idx, 1),
                delay_max: r.get_field_u16(idx, 2),
                fallback_priority: f32_field(r, idx, 3),
                flags: std::array::from_fn(|i| r.get_array_element(idx, 4, i, 32) as i32),
            }
        })
    }
}

impl SpellVisualMissileStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellVisualMissile.db2", |id, idx, r| {
            SpellVisualMissileEntry {
                id,
                cast_offset: f32_array::<3>(r, idx, 0),
                impact_offset: f32_array::<3>(r, idx, 1),
                spell_visual_effect_name_id: r.get_field_u16(idx, 2),
                sound_entries_id: r.get_field_u32(idx, 3),
                attachment: r.get_field_i8(idx, 4),
                destination_attachment: r.get_field_i8(idx, 5),
                cast_positioner_id: r.get_field_u16(idx, 6),
                impact_positioner_id: r.get_field_u16(idx, 7),
                follow_ground_height: r.get_field_i32(idx, 8),
                follow_ground_drop_speed: r.get_field_u32(idx, 9),
                follow_ground_approach: r.get_field_u16(idx, 10),
                flags: r.get_field_u32(idx, 11),
                spell_missile_motion_id: r.get_field_u16(idx, 12),
                anim_kit_id: r.get_field_u32(idx, 13),
                spell_visual_missile_set_id: r.get_field_u32(idx, 14),
            }
        })
    }
}

impl SpellXSpellVisualStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "SpellXSpellVisual.db2", |id, idx, r| {
            // Unlike the other relationship-backed Spell* tables loaded here,
            // SpellXSpellVisual keeps its record ID as physical field 0.  The
            // C++ DB2 metadata declares ID at logical column 0 and SpellID as
            // the relationship column, so payload fields begin at field 1.
            SpellXSpellVisualEntry {
                id,
                difficulty_id: r.get_field_u8(idx, 1),
                spell_visual_id: r.get_field_u32(idx, 2),
                probability: f32_field(r, idx, 3),
                flags: r.get_field_u8(idx, 4),
                priority: r.get_field_i32(idx, 5),
                spell_icon_file_id: r.get_field_i32(idx, 6),
                active_icon_file_id: r.get_field_i32(idx, 7),
                viewer_unit_condition_id: r.get_field_u16(idx, 8),
                viewer_player_condition_id: r.get_field_u32(idx, 9),
                caster_unit_condition_id: r.get_field_u16(idx, 10),
                caster_player_condition_id: r.get_field_u32(idx, 11),
                spell_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }

    /// Load C++'s effective spell-visual relation authority: DB2, official
    /// SQL, custom SQL, then final `hotfix_data` tombstones.
    pub fn load_effective_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        rows: impl IntoIterator<Item = SpellXSpellVisualEntry>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        for entry in rows {
            store.overlay_effective_row_like_cpp(entry);
        }

        let table_hash = store
            .table_hash_like_cpp()
            .context("SpellXSpellVisual.db2 store is missing its WDC4 table hash")?;
        store.apply_hotfix_removals_with_table_hash_like_cpp(table_hash, removals);
        Ok(store)
    }

    pub(super) fn overlay_effective_row_like_cpp(&mut self, entry: SpellXSpellVisualEntry) {
        self.entries.insert(entry.id, entry);
    }

    pub(super) fn apply_hotfix_removals_with_table_hash_like_cpp(
        &mut self,
        table_hash: u32,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) {
        self.entries
            .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    }
}

pub(super) fn load_store<T, S>(
    data_dir: &str,
    locale: &str,
    file_name: &str,
    mut read: impl FnMut(u32, usize, &Wdc4Reader) -> T,
) -> Result<S>
where
    S: FromEntries<T>,
{
    let path = Path::new(data_dir).join("dbc").join(locale).join(file_name);
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;

    let mut entries = Vec::with_capacity(reader.total_count());
    for (id, idx) in reader.iter_records() {
        entries.push(read(id, idx, &reader));
    }

    let mut store = S::from_entries(entries);
    store.set_table_hash_like_cpp(reader.table_hash());
    info!("Loaded {} rows from {}", store.len(), path.display());
    Ok(store)
}

pub(super) fn f32_field(reader: &Wdc4Reader, record_idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(record_idx, field))
}

pub(super) fn f32_array<const N: usize>(
    reader: &Wdc4Reader,
    record_idx: usize,
    field: usize,
) -> [f32; N] {
    std::array::from_fn(|i| f32::from_bits(reader.get_array_element(record_idx, field, i, 32)))
}

pub(super) trait FromEntries<T> {
    fn from_entries(entries: impl IntoIterator<Item = T>) -> Self;
    fn set_table_hash_like_cpp(&mut self, table_hash: u32);
    fn len(&self) -> usize;
}

/// C++ `SpellInfo::CalcDuration` boundary represented from DB2 duration rows.
///
/// Anchor: `SpellInfo.cpp:3894-3910`; player spell mods and passive `-1` fallback
/// are intentionally outside this helper because their runtime metadata is not
/// represented here. Missing entry/index returns `0`; `Duration == -1` remains
/// `-1`; otherwise Rust mirrors C++ `abs(Duration)`.
pub fn spell_duration_ms_like_cpp(
    duration_index: u32,
    duration_store: Option<&SpellDurationStore>,
) -> i32 {
    if duration_index == 0 {
        return 0;
    }
    let Some(entry) = duration_store.and_then(|store| store.get(duration_index)) else {
        return 0;
    };
    if entry.duration == -1 {
        -1
    } else {
        entry.duration.saturating_abs()
    }
}

/// C++ `SpellEffectInfo::CalcRadius` boundary represented from DB2 radius rows.
///
/// Anchor: `SpellInfo.cpp:653-692`; this models the no-caster overload used by
/// `Spell::EffectAddFarsight`: missing entry/index returns `0.0`; radius min is
/// used unless it is zero, in which case radius max is used. Random radius and
/// caster radius mods are intentionally outside this represented slice.
pub fn spell_effect_radius_like_cpp(
    radius_index: u32,
    radius_store: Option<&SpellRadiusStore>,
) -> f32 {
    if radius_index == 0 {
        return 0.0;
    }
    let Some(entry) = radius_store.and_then(|store| store.get(radius_index)) else {
        return 0.0;
    };
    if entry.radius_min == 0.0 {
        entry.radius_max
    } else {
        entry.radius_min
    }
}
