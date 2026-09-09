//! C++-shaped spell stores state definitions, part 4 of 4. operations, part 1 of 2.
//!
//! The inherent impl is divided by responsibility under #646; every
//! method keeps its original body.

use super::*;

impl SpellStore {
    /// Create a new empty spell store.
    pub fn new() -> Self {
        Self {
            spells: HashMap::new(),
            spell_info_keys_like_cpp: crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::default(),
            spell_effects_by_difficulty: HashMap::new(),
            spell_misc_attributes: HashMap::new(),
            spell_misc_attributes_by_difficulty: HashMap::new(),
            spell_interrupt_flags: HashMap::new(),
            spell_interrupt_rows_by_id: BTreeMap::new(),
            spell_hit_categories_by_difficulty: HashMap::new(),
            spell_hit_misc_by_difficulty: HashMap::new(),
            spell_hit_effect_mechanics_by_difficulty: HashMap::new(),
            spell_shapeshift_masks: HashMap::new(),
            implicit_target_conditions: HashMap::new(),
        }
    }
    pub(super) fn make_pair64_like_cpp(low: i32, high: i32) -> u64 {
        u64::from(low as u32) | (u64::from(high as u32) << 32)
    }
    pub fn effects_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<&[SpellEffectInfo]> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = HashSet::new();
        loop {
            if let Some(effects) = self
                .spell_effects_by_difficulty
                .get(&(spell_id, difficulty_id))
            {
                return Some(effects);
            }
            if difficulty_id == 0 || !visited.insert(difficulty_id) {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }
        self.spells.get(&spell_id).map(|spell| spell.effects())
    }
    /// Resolve the fields consumed by C++ spell-hit logic through the
    /// requested difficulty and its `FallbackDifficultyID` chain.
    ///
    /// `SpellCategories`, `SpellMisc`, and every `SpellEffect` slot fall back
    /// independently. This matters when a difficulty overrides only one of
    /// those contributors.
    pub fn hit_metadata_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<SpellHitMetadataLikeCpp> {
        let mut metadata = SpellHitMetadataLikeCpp::default();
        let mut has_metadata = false;
        let mut categories_resolved = false;
        let mut misc_resolved = false;
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = [false; 256];

        loop {
            let visited_slot = &mut visited[usize::from(difficulty_id)];
            if *visited_slot {
                break;
            }
            *visited_slot = true;

            if !categories_resolved
                && let Some(categories) = self
                    .spell_hit_categories_by_difficulty
                    .get(&(spell_id, difficulty_id))
            {
                metadata.category_id = categories.category_id;
                metadata.charge_category_id = categories.charge_category_id;
                metadata.defense_type = categories.defense_type;
                metadata.spell_mechanic = categories.spell_mechanic;
                categories_resolved = true;
                has_metadata = true;
            }
            if !misc_resolved
                && let Some(misc) = self
                    .spell_hit_misc_by_difficulty
                    .get(&(spell_id, difficulty_id))
            {
                metadata.school_mask = misc.school_mask;
                misc_resolved = true;
                has_metadata = true;
            }
            if let Some(effect_mechanics) = self
                .spell_hit_effect_mechanics_by_difficulty
                .get(&(spell_id, difficulty_id))
            {
                has_metadata = true;
                for (&effect_index, effect) in effect_mechanics {
                    metadata
                        .effect_mechanics
                        .entry(effect_index)
                        .or_insert(effect.mechanic);
                }
            }

            if difficulty_id == 0 {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }

        has_metadata.then_some(metadata)
    }
    pub(in crate::spell) fn empty_spell_info_like_cpp(spell_id: i32) -> SpellInfo {
        SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: Vec::new(),
        }
    }
    pub(super) fn spell_effect_from_db2_like_cpp(
        effect: &crate::spell_db2::SpellEffectDb2Entry,
    ) -> SpellEffectInfo {
        SpellEffectInfo {
            effect_index: u32::try_from(effect.effect_index).unwrap_or(0),
            effect: effect.effect,
            effect_aura: i32::from(effect.effect_aura),
            effect_base_points: effect.effect_base_points,
            effect_die_sides: effect.effect_die_sides,
            effect_spell_class_mask: effect.effect_spell_class_mask,
            effect_misc_value_1: effect.effect_misc_value[0],
            effect_misc_value_2: effect.effect_misc_value[1],
            effect_trigger_spell: effect.effect_trigger_spell,
            effect_radius_index_1: effect.effect_radius_index[0],
            position_facing: effect.effect_pos_facing,
            chain_targets: effect.effect_chain_targets,
            implicit_target_1: u32::try_from(effect.implicit_target[0]).unwrap_or(0),
            implicit_target_2: u32::try_from(effect.implicit_target[1]).unwrap_or(0),
        }
    }
    pub(super) fn hydrate_primary_effect_like_cpp(info: &mut SpellInfo) {
        info.effects.sort_by_key(|effect| effect.effect_index);
        if let Some(primary) = info.effects.iter().find(|effect| effect.effect != 0) {
            info.effect_type = primary.effect;
            info.effect_base_points = primary.effect_base_points;
            info.effect_bonus_coefficient = 0.0;
            info.aura_type = Some(primary.effect_aura);
        }
    }
    /// Load the exact regular `SpellInfo` key authority that still has
    /// non-core Hotfix DB2 contributors outside #509.
    pub fn load_spell_info_key_seed_from_hotfix_rows_like_cpp(
        data_dir: &str,
        locale: &str,
        spell_name_store: &crate::spell_db2::SpellNameStore,
        hotfix_removals: &crate::Db2HotfixRemovalStoreLikeCpp,
        hotfix_overlays: crate::SpellInfoKeyHotfixOverlaysLikeCpp,
    ) -> Result<Self> {
        let spell_info_keys_like_cpp =
            crate::spell_info_keys::SpellInfoKeyStoreLikeCpp::load_from_hotfix_rows_like_cpp(
                data_dir,
                locale,
                spell_name_store,
                hotfix_removals,
                hotfix_overlays,
            )?;
        let mut store = Self::new();
        store.spell_info_keys_like_cpp = spell_info_keys_like_cpp;
        Ok(store)
    }
    /// Hydrate the represented `SpellInfo` payload from already-effective DB2
    /// authorities while preserving C++ `SpellMgr::LoadSpellInfoStore` order.
    pub fn hydrate_effective_core_db2_like_cpp(
        self,
        stores: EffectiveCoreSpellDb2StoresLikeCpp,
    ) -> Self {
        let mut store = Self::from_spell_db2_stores_like_cpp(
            &stores.spell_categories,
            &stores.spell_misc,
            &stores.spell_effect,
            &stores.spell_shapeshift,
        );
        store.spell_info_keys_like_cpp = self.spell_info_keys_like_cpp;
        store.apply_db2_interrupts_like_cpp(&stores.spell_interrupts);
        store.apply_db2_cast_times_like_cpp(&stores.spell_misc, &stores.spell_cast_times);
        store.apply_db2_cooldowns_like_cpp(&stores.spell_cooldowns);
        store.apply_db2_casting_requirements_like_cpp(&stores.spell_casting_requirements);
        store.apply_db2_power_costs_like_cpp(&stores.spell_power, &stores.spell_power_difficulty);
        store.apply_interrupt_flag_corrections_like_cpp();

        info!(
            "Loaded {} spells from SpellMisc/SpellEffect DB2 with hotfix overlay",
            store.spells.len()
        );
        store
    }
    /// Whether C++ `SpellMgr::GetSpellInfo` has an exact regular-spell key.
    ///
    /// This is deliberately separate from [`Self::get`]. `get` exposes the
    /// subset of `SpellInfo` payload fields Rust currently hydrates, whereas
    /// C++ creates existence keys from twenty DB2 contributors.
    pub fn contains_spell_info_exact_like_cpp(&self, spell_id: u32, difficulty_id: u8) -> bool {
        self.spell_info_keys_like_cpp
            .contains_exact_like_cpp(spell_id, difficulty_id)
    }
    /// Exact regular `SpellInfo` keys in deterministic `(SpellID, Difficulty)` order.
    pub fn spell_info_keys_in_order_like_cpp(&self) -> Vec<(u32, u8)> {
        self.spell_info_keys_like_cpp.exact_keys_in_order_like_cpp()
    }
    /// Whether C++ `GetSpellInfo(id, DIFFICULTY_NONE)` would find a regular
    /// or server-side spell.
    ///
    /// The shipped DB2 has no difficulty-zero row, but C++ permits SQL
    /// overlays to add one and then follows its `FallbackDifficultyID` chain.
    /// Keep that behavior for loader foreign-key checks while stopping an
    /// invalid custom cycle instead of hanging startup forever.
    pub fn contains_spell_info_difficulty_none_like_cpp(
        &self,
        serverside_spells: &ServersideSpellStoreLikeCpp,
        difficulty_store: &crate::difficulty::DifficultyStore,
        spell_id: u32,
    ) -> bool {
        let mut difficulty_id = 0u8;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(difficulty_id) {
                return false;
            }
            if self.contains_spell_info_exact_like_cpp(spell_id, difficulty_id)
                || serverside_spells
                    .get_serverside_spell_like_cpp(spell_id, u32::from(difficulty_id))
                    .is_some()
            {
                return true;
            }
            let Some(difficulty) = difficulty_store.get(u32::from(difficulty_id)) else {
                return false;
            };
            difficulty_id = difficulty.fallback_difficulty_id;
        }
    }
    /// Whether C++ `_GetSpellInfo(id)` would find any regular difficulty.
    pub fn contains_spell_info_any_difficulty_like_cpp(&self, spell_id: u32) -> bool {
        self.spell_info_keys_like_cpp
            .contains_any_difficulty_like_cpp(spell_id)
    }
    pub fn spell_info_key_count_like_cpp(&self) -> usize {
        self.spell_info_keys_like_cpp.len()
    }
    pub(super) fn apply_db2_hit_metadata_like_cpp(
        &mut self,
        spell_categories_store: &crate::spell_db2::SpellCategoriesStore,
        spell_misc_store: &crate::spell_db2::SpellMiscStore,
        spell_effect_store: &crate::spell_db2::SpellEffectDb2Store,
    ) {
        for categories in spell_categories_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(categories.spell_id) else {
                continue;
            };
            let row = SpellHitCategoriesRowLikeCpp {
                record_id: categories.id,
                // C++ assigns the signed DB2 fields directly into the
                // corresponding uint32 SpellInfo members.
                category_id: categories.category as u32,
                charge_category_id: categories.charge_category as u32,
                defense_type: categories.defense_type,
                spell_mechanic: categories.mechanic,
            };
            self.spell_hit_categories_by_difficulty
                .entry((spell_id, categories.difficulty_id))
                .and_modify(|current| {
                    if row.record_id > current.record_id {
                        *current = row;
                    }
                })
                .or_insert(row);
        }

        for misc in spell_misc_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(misc.spell_id) else {
                continue;
            };
            let row = SpellHitMiscRowLikeCpp {
                record_id: misc.id,
                school_mask: misc.school_mask,
            };
            self.spell_hit_misc_by_difficulty
                .entry((spell_id, misc.difficulty_id))
                .and_modify(|current| {
                    if row.record_id > current.record_id {
                        *current = row;
                    }
                })
                .or_insert(row);
        }

        for effect in spell_effect_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(effect.spell_id) else {
                continue;
            };
            let Ok(difficulty_id) = u8::try_from(effect.difficulty_id) else {
                continue;
            };
            let Ok(effect_index) = u32::try_from(effect.effect_index) else {
                continue;
            };
            if effect_index >= MAX_SPELL_EFFECTS_LIKE_CPP as u32 {
                continue;
            }
            let row = SpellHitEffectMechanicRowLikeCpp {
                record_id: effect.id,
                mechanic: effect.effect_mechanic,
            };
            self.spell_hit_effect_mechanics_by_difficulty
                .entry((spell_id, difficulty_id))
                .or_default()
                .entry(effect_index)
                .and_modify(|current| {
                    if row.record_id > current.record_id {
                        *current = row;
                    }
                })
                .or_insert(row);
        }
    }
    pub(in crate::spell) fn from_spell_db2_stores_like_cpp(
        spell_categories_store: &crate::spell_db2::SpellCategoriesStore,
        spell_misc_store: &crate::spell_db2::SpellMiscStore,
        spell_effect_store: &crate::spell_db2::SpellEffectDb2Store,
        spell_shapeshift_store: &crate::spell_db2::SpellShapeshiftStore,
    ) -> Self {
        let mut store = Self::new();

        store.apply_db2_hit_metadata_like_cpp(
            spell_categories_store,
            spell_misc_store,
            spell_effect_store,
        );

        for misc in spell_misc_store.entries_like_cpp() {
            let Ok(spell_id) = i32::try_from(misc.spell_id) else {
                continue;
            };
            let difficulty_id = misc.difficulty_id;
            let attributes = misc.attributes.map(|attribute| attribute as u32);
            store
                .spell_misc_attributes_by_difficulty
                .insert((spell_id, difficulty_id), attributes);
            if difficulty_id != 0 {
                continue;
            }
            store
                .spells
                .entry(spell_id)
                .or_insert_with(|| Self::empty_spell_info_like_cpp(spell_id));
            store.spell_misc_attributes.insert(spell_id, attributes);
        }

        for effect in spell_effect_store.entries_like_cpp() {
            if effect.effect == 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(effect.spell_id) else {
                continue;
            };
            let Ok(difficulty_id) = u8::try_from(effect.difficulty_id) else {
                continue;
            };
            let converted = Self::spell_effect_from_db2_like_cpp(effect);
            store
                .spell_effects_by_difficulty
                .entry((spell_id, difficulty_id))
                .or_default()
                .push(converted.clone());
            if difficulty_id != 0 {
                continue;
            }
            let spell = store
                .spells
                .entry(spell_id)
                .or_insert_with(|| Self::empty_spell_info_like_cpp(spell_id));
            spell.effects.push(converted);
        }

        for shapeshift in spell_shapeshift_store.entries_like_cpp() {
            if shapeshift.spell_id <= 0 {
                continue;
            }
            store.spell_shapeshift_masks.insert(
                shapeshift.spell_id,
                (
                    Self::make_pair64_like_cpp(
                        shapeshift.shapeshift_mask[0],
                        shapeshift.shapeshift_mask[1],
                    ),
                    Self::make_pair64_like_cpp(
                        shapeshift.shapeshift_exclude[0],
                        shapeshift.shapeshift_exclude[1],
                    ),
                ),
            );
        }

        for spell in store.spells.values_mut() {
            Self::hydrate_primary_effect_like_cpp(spell);
        }

        store
    }
    /// C++ `SpellMgr::LoadSpellInfoStore` copies the difficulty-specific
    /// `SpellInterrupts` row into `SpellInfo`. The current Rust `SpellInfo`
    /// keeps related DB2 joins in `SpellStore`, so retain both interrupt masks
    /// here without widening every dynamically constructed test SpellInfo.
    pub(in crate::spell) fn apply_db2_interrupts_like_cpp(
        &mut self,
        spell_interrupts_store: &crate::spell_db2::SpellInterruptsStore,
    ) {
        for interrupts in spell_interrupts_store.entries_like_cpp() {
            self.store_signed_interrupt_row_by_id_like_cpp(
                interrupts.id,
                interrupts.spell_id,
                interrupts.difficulty_id,
                interrupts.aura_interrupt_flags,
                interrupts.channel_interrupt_flags,
            );
        }
        self.rebuild_interrupt_flags_from_rows_like_cpp();
    }
    /// Apply one file/hotfix `SpellInterrupts` row. DB2 stores the bit fields
    /// as signed integers, while C++ preserves their complete `uint32` bit
    /// pattern in `SpellInfo`.
    pub(in crate::spell) fn store_signed_interrupt_row_by_id_like_cpp(
        &mut self,
        row_id: u32,
        spell_id: u32,
        difficulty_id: u8,
        aura_interrupt_flags: [i32; 2],
        channel_interrupt_flags: [i32; 2],
    ) -> bool {
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        self.spell_interrupt_rows_by_id.insert(
            row_id,
            SpellInterruptRowLikeCpp {
                key: (spell_id, difficulty_id),
                flags: (
                    aura_interrupt_flags.map(|flag| flag as u32),
                    channel_interrupt_flags.map(|flag| flag as u32),
                ),
            },
        );
        true
    }
    /// Rebuild the relational lookup once per load phase. C++ DB2 storage is
    /// indexed and iterated by ascending record ID, so later IDs win if two
    /// records resolve to the same spell/difficulty key.
    pub(in crate::spell) fn rebuild_interrupt_flags_from_rows_like_cpp(&mut self) {
        self.spell_interrupt_flags.clear();
        for row in self.spell_interrupt_rows_by_id.values() {
            self.spell_interrupt_flags.insert(row.key, row.flags);
        }
    }
    /// Import world-DB `serverside_spell` interrupt masks into the same
    /// effective lookup used by live aura/channel decisions. C++ inserts these
    /// SpellInfo rows before applying corrections; effective file plus SQL
    /// `SpellName` IDs were already rejected while the server-side store was
    /// built.
    pub fn apply_serverside_spell_interrupts_like_cpp(
        &mut self,
        serverside_spells: &ServersideSpellStoreLikeCpp,
    ) {
        for info in serverside_spells
            .spell_infos_by_spell_and_difficulty
            .values()
        {
            let Ok(spell_id) = i32::try_from(info.row.spell_id) else {
                continue;
            };
            self.insert_spell_interrupt_flags_for_difficulty_like_cpp(
                spell_id,
                info.row.difficulty_id as u8,
                info.row.aura_interrupt_flags,
                info.row.channel_interrupt_flags,
            );
        }
        self.apply_interrupt_flag_corrections_like_cpp();
    }
    /// Interrupt-mask subset of C++ `SpellMgr::LoadSpellInfoCorrections`.
    /// `ApplySpellFix` mutates every difficulty variant, so update every stored
    /// key for each affected spell after DB2/hotfix/server-side composition.
    pub(in crate::spell) fn apply_interrupt_flag_corrections_like_cpp(&mut self) {
        const HOSTILE_ACTION_RECEIVED: u32 = 0x0000_0001;
        const DAMAGE: u32 = 0x0000_0002;
        const ACTION: u32 = 0x0000_0004;
        const MOVING: u32 = 0x0000_0008;
        const ANIM: u32 = 0x0000_0020;
        const LEAVE_WORLD: u32 = 0x0008_0000;

        for spell_id in [61_719, 29_726, 63_414, 24_314, 99_252] {
            if self.spells.contains_key(&spell_id)
                && !self
                    .spell_interrupt_flags
                    .keys()
                    .any(|(known_spell_id, _)| *known_spell_id == spell_id)
            {
                self.spell_interrupt_flags
                    .insert((spell_id, 0), ([0; 2], [0; 2]));
            }
        }

        for ((spell_id, _), (aura, channel)) in &mut self.spell_interrupt_flags {
            match *spell_id {
                // Easter Lay Noblegarden Egg Aura.
                61_719 => aura[0] = HOSTILE_ACTION_RECEIVED | DAMAGE,
                // Test Ribbon Pole Channel.
                29_726 => channel[0] &= !ACTION,
                // Spinning Up (Mimiron).
                63_414 => *channel = [0; 2],
                // Threatening Gaze.
                24_314 => aura[0] |= ACTION | MOVING | ANIM,
                // Blaze of Glory.
                99_252 => aura[0] |= LEAVE_WORLD,
                _ => {}
            }
        }
    }
    /// [M0.1/#14] Apply DB2 cast times onto already-built SpellInfo rows.
    ///
    /// Mirrors the C++ SpellInfo ctor `CastTimeEntry =
    /// sSpellCastTimesStore.LookupEntry(_misc->CastingTimeIndex)` (SpellInfo.cpp:1185)
    /// + `CalcCastTime`: cast time = `max(Base, Minimum)`, clamped to ≥ 0
    /// (SpellInfo.cpp:3922). Must run AFTER the hotfix merge, which overwrites
    /// `cast_time_ms` (and would clobber this back to 0).
    pub(in crate::spell) fn apply_db2_cast_times_like_cpp(
        &mut self,
        spell_misc_store: &crate::spell_db2::SpellMiscStore,
        spell_cast_times_store: &crate::spell_db2::SpellCastTimesStore,
    ) {
        for misc in spell_misc_store.entries_like_cpp() {
            if misc.difficulty_id != 0 || misc.casting_time_index == 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(misc.spell_id) else {
                continue;
            };
            let Some(entry) = spell_cast_times_store.get(u32::from(misc.casting_time_index)) else {
                continue;
            };
            if let Some(spell) = self.spells.get_mut(&spell_id) {
                spell.cast_time_ms = entry.base.max(entry.minimum).max(0) as u32;
            }
        }
    }
    /// [M0.1/#14] Apply DB2 per-spell cooldowns onto already-built SpellInfo rows.
    ///
    /// Mirrors C++ SpellInfo `RecoveryTime/CategoryRecoveryTime` from
    /// `sSpellCooldownsStore` (SpellInfo.cpp:1263) and `GetRecoveryTime() =
    /// max(RecoveryTime, CategoryRecoveryTime)` (SpellInfo.cpp:3981) — the per-spell
    /// cooldown the cast gate checks (`recovery_time_ms`). `StartRecoveryTime` (the
    /// GCD) is a separate mechanic and is intentionally left to the GCD path.
    /// Must run AFTER the hotfix merge (which overwrites `recovery_time_ms`).
    pub(in crate::spell) fn apply_db2_cooldowns_like_cpp(
        &mut self,
        spell_cooldowns_store: &crate::spell_db2::SpellCooldownsStore,
    ) {
        for entry in spell_cooldowns_store.entries_like_cpp() {
            if entry.difficulty_id != 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(entry.spell_id) else {
                continue;
            };
            if let Some(spell) = self.spells.get_mut(&spell_id) {
                spell.recovery_time_ms =
                    entry.recovery_time.max(entry.category_recovery_time).max(0) as u32;
            }
        }
    }
    /// [M0.1/#72] Apply DB2 power costs onto already-built SpellInfo rows.
    ///
    /// Mirrors C++ `SpellMgr::LoadSpellInfoStore`, which stores
    /// `SpellPowerEntry` rows in `SpellInfo::PowerCosts` keyed by
    /// `SpellID`/difficulty/order (`SpellMgr.cpp:2550`, `DB2Stores.cpp:301`).
    /// C++ `SpellMgr::LoadSpellInfoStore` copies
    /// `SpellCastingRequirements::RequiresSpellFocus` into an already
    /// constructed `SpellInfo`. It never creates a spell from this table, so a
    /// requirements row for an unknown spell stays inert.
    pub(in crate::spell) fn apply_db2_casting_requirements_like_cpp(
        &mut self,
        spell_casting_requirements_store: &crate::spell_db2::SpellCastingRequirementsStore,
    ) {
        // C++'s DB2 iteration assigns this DIFFICULTY_NONE slot in record-ID
        // order, so a duplicated SpellID resolves to the highest record ID.
        let mut effective_by_spell: HashMap<i32, (u32, u16)> = HashMap::new();
        for entry in spell_casting_requirements_store.entries_like_cpp() {
            effective_by_spell
                .entry(entry.spell_id)
                .and_modify(|current| {
                    if entry.id > current.0 {
                        *current = (entry.id, entry.requires_spell_focus);
                    }
                })
                .or_insert((entry.id, entry.requires_spell_focus));
        }

        for spell in self.spells.values_mut() {
            spell.requires_spell_focus = effective_by_spell
                .get(&spell.spell_id)
                .map_or(0, |(_, requires_spell_focus)| {
                    u32::from(*requires_spell_focus)
                });
        }
    }
    pub(in crate::spell) fn apply_db2_power_costs_like_cpp(
        &mut self,
        spell_power_store: &crate::spell_db2::SpellPowerStore,
        spell_power_difficulty_store: &crate::spell_db2::SpellPowerDifficultyStore,
    ) {
        for spell in self.spells.values_mut() {
            spell.power_costs.clear();
        }

        // C++ walks `sSpellPowerStore` through its record-ID ordered index, so
        // two rows that collide on the same spell and order index must resolve
        // to the highest record ID rather than to a `HashMap` iteration order.
        for power in spell_power_store.entries_by_record_id_like_cpp() {
            if power.spell_id == 0 {
                continue;
            }
            let Ok(spell_id) = i32::try_from(power.spell_id) else {
                continue;
            };

            let (difficulty_id, order_index) = spell_power_difficulty_store
                .get(power.id)
                .map(|difficulty| (difficulty.difficulty_id, difficulty.order_index))
                .unwrap_or((0, power.order_index));
            if difficulty_id != 0 {
                continue;
            }

            let Some(spell) = self.spells.get_mut(&spell_id) else {
                continue;
            };
            let power_cost = SpellPowerCostInfoLikeCpp {
                order_index,
                power_type: power.power_type,
                mana_cost: power.mana_cost,
                mana_cost_per_level: power.mana_cost_per_level,
                mana_per_second: power.mana_per_second,
                power_cost_pct: power.power_cost_pct,
                power_cost_max_pct: power.power_cost_max_pct,
                power_pct_per_second: power.power_pct_per_second,
                required_aura_spell_id: power.required_aura_spell_id,
                optional_cost: power.optional_cost,
            };

            if let Some(existing) = spell
                .power_costs
                .iter_mut()
                .find(|existing| existing.order_index == order_index)
            {
                *existing = power_cost;
            } else {
                spell.power_costs.push(power_cost);
            }
            spell.power_costs.sort_by_key(|entry| entry.order_index);
        }
    }
    /// Look up a spell by ID.
    pub fn get(&self, spell_id: i32) -> Option<&SpellInfo> {
        self.spells.get(&spell_id)
    }
    /// Resolve the `SpellMisc` attributes owned by the same difficulty-specific
    /// C++ `SpellInfo` selected by `SpellMgr::GetSpellInfo`.
    pub fn misc_attributes_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<[u32; 15]> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = HashSet::new();
        loop {
            if let Some(attributes) = self
                .spell_misc_attributes_by_difficulty
                .get(&(spell_id, difficulty_id))
                .copied()
            {
                return Some(attributes);
            }
            if difficulty_id == 0 || !visited.insert(difficulty_id) {
                break;
            }
            difficulty_id = difficulty_store
                .and_then(|store| store.get(u32::from(difficulty_id)))
                .map_or(0, |difficulty| difficulty.fallback_difficulty_id);
        }
        self.spell_misc_attributes.get(&spell_id).copied()
    }
    pub fn has_attribute_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
        attribute_word: usize,
        attribute: u32,
    ) -> bool {
        self.misc_attributes_for_difficulty_like_cpp(
            spell_id,
            requested_difficulty_id,
            difficulty_store,
        )
        .and_then(|attributes| attributes.get(attribute_word).copied())
        .is_some_and(|attributes| attributes & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute0_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[0] & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute(SpellAttr1)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute1_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[1] & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute(SpellAttr2)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute2_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[2] & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute(SpellAttr4)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute4_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[4] & attribute != 0)
    }
    /// C++ `SpellInfo::HasAttribute(SpellAttr8)` for attributes hydrated from `SpellMisc.db2`.
    pub fn has_attribute8_like_cpp(&self, spell_id: i32, attribute: u32) -> bool {
        self.spell_misc_attributes
            .get(&spell_id)
            .is_some_and(|attributes| attributes[8] & attribute != 0)
    }
    /// C++ `SpellInfo::Stances` / `StancesNot` for login passive-cast gates.
    pub fn shapeshift_masks_like_cpp(&self, spell_id: i32) -> (u64, u64) {
        self.spell_shapeshift_masks
            .get(&spell_id)
            .copied()
            .unwrap_or((0, 0))
    }
    /// C++ `SpellInfo::IsPassive`, for the represented paths that currently
    /// only need the `SPELL_ATTR0_PASSIVE` gate.
    pub fn is_passive_like_cpp(&self, spell_id: i32) -> bool {
        self.has_attribute0_like_cpp(spell_id, attributes::SPELL_ATTR0_PASSIVE)
    }
    /// C++ `SpellInfo::IsChanneled`.
    pub fn is_channeled_like_cpp(&self, spell_id: i32) -> bool {
        self.has_attribute1_like_cpp(
            spell_id,
            attributes::SPELL_ATTR1_IS_CHANNELLED | attributes::SPELL_ATTR1_IS_SELF_CHANNELLED,
        )
    }
    /// Resolve the C++ `SpellInterrupts` row for one spell/difficulty.
    ///
    /// `SpellMgr::GetSpellInfo` tries the exact map difficulty before walking
    /// `DifficultyEntry::FallbackDifficultyID`. Keep both aura and channel
    /// words coupled to the same selected row rather than merging metadata
    /// across difficulties.
    pub fn interrupt_flags_for_difficulty_like_cpp(
        &self,
        spell_id: i32,
        requested_difficulty_id: u8,
        difficulty_store: Option<&crate::difficulty::DifficultyStore>,
    ) -> Option<([u32; 2], [u32; 2])> {
        let mut difficulty_id = requested_difficulty_id;
        let mut visited = [false; 256];
        loop {
            if let Some(flags) = self
                .spell_interrupt_flags
                .get(&(spell_id, difficulty_id))
                .copied()
            {
                return Some(flags);
            }

            let visited_entry = &mut visited[usize::from(difficulty_id)];
            if *visited_entry {
                return None;
            }
            *visited_entry = true;

            difficulty_id = difficulty_store?.fallback_difficulty_id_like_cpp(difficulty_id)?;
        }
    }
    /// C++ `SpellInfo::HasAuraInterruptFlag` for the two
    /// `SpellAuraInterruptFlags` words loaded from difficulty zero.
    ///
    /// Transitional callers without map context retain the original base-row
    /// behavior; live paths should call the difficulty-aware variant.
    pub fn aura_interrupt_flags_like_cpp(&self, spell_id: i32) -> Option<[u32; 2]> {
        self.aura_interrupt_flags_for_difficulty_like_cpp(spell_id, 0, None)
    }
}
