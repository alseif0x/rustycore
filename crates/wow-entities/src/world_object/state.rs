//! World-object visibility and location model state definitions, part 1 of 1.
//!
//! Separated from the world_object.rs root under #648. Behaviour is preserved.

use super::*;

pub const MAPID_INVALID: u32 = u32::MAX;

/// TrinityCore `MAX_VISIBILITY_DISTANCE` (`SIZE_OF_GRIDS`).
pub const MAX_VISIBILITY_DISTANCE: f32 = 533.3333;

/// TrinityCore `SIGHT_RANGE_UNIT`.
pub const SIGHT_RANGE_UNIT: f32 = 50.0;

/// TrinityCore normal visibility distance.
pub const DEFAULT_VISIBILITY_DISTANCE: f32 = 100.0;

pub const VISIBILITY_DISTANCE_TINY: f32 = 25.0;

pub const VISIBILITY_DISTANCE_SMALL: f32 = 50.0;

pub const VISIBILITY_DISTANCE_LARGE: f32 = 200.0;

pub const VISIBILITY_DISTANCE_GIGANTIC: f32 = 400.0;

/// TrinityCore default instance/cinematic visibility distance.
pub const DEFAULT_VISIBILITY_INSTANCE: f32 = 170.0;

/// TrinityCore invalid terrain height sentinel.
pub const INVALID_HEIGHT: f32 = -100_000.0;

/// TrinityCore `MAX_HEIGHT` sentinel for unconstrained height search.
pub const MAX_HEIGHT: f32 = 100_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum VisibilityDistanceTypeLikeCpp {
    Normal = 0,
    Tiny = 1,
    Small = 2,
    Large = 3,
    Gigantic = 4,
    Infinite = 5,
}

impl VisibilityDistanceTypeLikeCpp {
    pub const MAX_LIKE_CPP: u8 = 6;

    pub const fn from_u8_like_cpp(value: u8) -> Self {
        match value {
            1 => Self::Tiny,
            2 => Self::Small,
            3 => Self::Large,
            4 => Self::Gigantic,
            5 => Self::Infinite,
            _ => Self::Normal,
        }
    }

    pub const fn distance_like_cpp(self) -> f32 {
        match self {
            Self::Normal => DEFAULT_VISIBILITY_DISTANCE,
            Self::Tiny => VISIBILITY_DISTANCE_TINY,
            Self::Small => VISIBILITY_DISTANCE_SMALL,
            Self::Large => VISIBILITY_DISTANCE_LARGE,
            Self::Gigantic => VISIBILITY_DISTANCE_GIGANTIC,
            Self::Infinite => MAX_VISIBILITY_DISTANCE,
        }
    }
}

/// TrinityCore `DEFAULT_HEIGHT_SEARCH`.
pub const DEFAULT_HEIGHT_SEARCH: f32 = 50.0;

/// TrinityCore `Z_OFFSET_FIND_HEIGHT`.
pub const Z_OFFSET_FIND_HEIGHT: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldObjectHeightQuery {
    pub vmap: bool,
    pub distance_to_search: f32,
}

impl Default for WorldObjectHeightQuery {
    fn default() -> Self {
        Self {
            vmap: true,
            distance_to_search: DEFAULT_HEIGHT_SEARCH,
        }
    }
}

/// Movement capabilities `UpdateAllowedPositionZ` reads off the owning `Unit`.
///
/// In C++ these come from `ToUnit()->CanFly()/CanSwim()/GetHoverOffset()` and
/// `GetTransport()`; `WorldObject` has no back-reference to its `Unit`, so the
/// caller injects them (the live entity knows its own movement flags).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllowedPositionZCaps {
    /// `GetTransport() != nullptr`: a passenger's Z is left to the transport.
    pub on_transport: bool,
    /// `Unit::CanFly()` — when set, terrain only *raises* Z (never pulls down).
    pub can_fly: bool,
    /// `Unit::CanSwim()` — kept for fidelity; without a parsed liquid map the swim
    /// branch folds to the ground branch.
    pub can_swim: bool,
    /// `Unit::GetHoverOffset()` (0 unless the unit is hovering).
    pub hover_offset: f32,
}

/// Pure decision half of `WorldObject::UpdateAllowedPositionZ` (`Object.cpp:1411`),
/// taking the already-resolved `GetMapHeight` result so it can be reused without a
/// live `WorldObject`/environment (e.g. the global respawn driver).
///
/// `ground` is `GetMapHeight(x, y, z)` (the `Z_OFFSET_FIND_HEIGHT` probe offset is
/// the caller's responsibility, matching `GetMapHeight`). `on_transport` is handled
/// by the caller before this is reached.
///
/// The grounded clamp `[ground_z+hover, max_z+hover]` reduces to a single point
/// (`ground+hover`) because, with no parsed liquid map, `GetMapWaterOrGroundLevel`
/// has no water ceiling and `max_z == ground_z` — exactly C++ behaviour when liquid
/// data is unavailable, so the swim case collapses to "sit on ground".
#[must_use]
pub fn allowed_position_z_from_ground_like_cpp(
    is_unit: bool,
    ground: f32,
    z: f32,
    caps: AllowedPositionZCaps,
) -> f32 {
    if caps.on_transport {
        return z;
    }
    if ground <= INVALID_HEIGHT {
        // No terrain under the probe: C++ leaves Z untouched in every branch.
        return z;
    }
    if is_unit {
        let target = ground + caps.hover_offset;
        if caps.can_fly {
            // Flying: only lift out of the ground, never pull down.
            z.max(target)
        } else {
            // Grounded (and swim w/o liquid): clamp onto the surface.
            target
        }
    } else {
        // Non-unit (GameObject/etc.): snapped flat to the ground.
        ground
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineOfSightOptions {
    pub check_dynamic: bool,
}

/// Endpoint passed to the LOS bridge.
///
/// TrinityCore adjusts object LOS endpoints with `GetCollisionHeight()` and
/// `GetHitSpherePointFor(...)` before calling `Map::isInLineOfSight`. RustyCore models those
/// endpoint transforms here when higher layers have hydrated the resolved runtime scalar state
/// (`WorldObject::collision_height_like_cpp` and combat reach). Metadata is true only when the
/// endpoint position was actually changed by that mechanism.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct LineOfSightEndpoint {
    pub position: Position,
    pub collision_height_adjusted: bool,
    pub hit_sphere_adjusted: bool,
}

impl LineOfSightEndpoint {
    pub const fn raw_position(position: Position) -> Self {
        Self {
            position,
            collision_height_adjusted: false,
            hit_sphere_adjusted: false,
        }
    }
}

/// Query passed from `WorldObject` LOS helpers to the map/terrain bridge.
///
/// `from` and `to` are already shaped like TrinityCore's LOS endpoints: collision-height and
/// hit-sphere transforms are applied in `wow-entities` from resolved runtime scalars, while the
/// environment remains a consumer that performs terrain/dynamic-tree LOS only.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct LineOfSightQuery<'a> {
    pub source: &'a WorldObject,
    pub target: Option<&'a WorldObject>,
    pub from: LineOfSightEndpoint,
    pub to: LineOfSightEndpoint,
    pub options: LineOfSightOptions,
}

impl<'a> LineOfSightQuery<'a> {
    pub fn to_position_like_cpp(
        source: &'a WorldObject,
        position: Position,
        options: LineOfSightOptions,
    ) -> Self {
        let to = source.position_with_collision_height_like_cpp(position);
        let from = if source.is_player() {
            source.own_position_with_collision_height_like_cpp()
        } else {
            source.get_hit_sphere_point_for_like_cpp(to.position)
        };

        Self {
            source,
            target: None,
            from,
            to,
            options,
        }
    }

    pub fn to_object_like_cpp(
        source: &'a WorldObject,
        target: &'a WorldObject,
        options: LineOfSightOptions,
    ) -> Self {
        let target_dest = source.position_with_collision_height_like_cpp(source.position());
        let to = if target.is_player() {
            target.position_with_explicit_collision_height_like_cpp(
                target.position(),
                source.collision_height_like_cpp(),
            )
        } else {
            target.get_hit_sphere_point_for_like_cpp(target_dest.position)
        };

        let source_dest = target.position_with_collision_height_like_cpp(target.position());
        let from = if source.is_player() {
            source.own_position_with_collision_height_like_cpp()
        } else {
            source.get_hit_sphere_point_for_like_cpp(source_dest.position)
        };

        Self {
            source,
            target: Some(target),
            from,
            to,
            options,
        }
    }
}

/// Bridge for `WorldObject` helpers whose C++ implementation delegates to `Map`/terrain.
///
/// This keeps `wow-entities` independent from `wow-map` while preserving the represented C++
/// call shape. LOS endpoints are built here with C++ collision-height/hit-sphere semantics when
/// the resolved runtime scalar state is present; terrain/vmap/dynamic-tree collision remains
/// owned by the environment implementation.
pub trait WorldObjectEnvironment {
    fn map_id(&self) -> u32;
    fn instance_id(&self) -> u32;
    fn visibility_range(&self) -> f32;

    fn visibility_override(&self, _object: &WorldObject) -> Option<f32> {
        None
    }

    fn creature_sight_distance(&self, _object: &WorldObject) -> Option<f32> {
        None
    }

    fn player_on_cinematic(&self, _object: &WorldObject) -> bool {
        false
    }

    fn line_of_sight(&self, _query: LineOfSightQuery<'_>) -> bool;

    fn map_height(
        &self,
        _object: &WorldObject,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32;

    fn floor_z(&self, _object: &WorldObject, position: Position, max_search_dist: f32) -> f32;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldLocation {
    pub(super) map_id: u32,
    pub(super) position: Position,
}

impl Default for WorldLocation {
    fn default() -> Self {
        Self::new(MAPID_INVALID, 0.0, 0.0, 0.0, 0.0)
    }
}

impl WorldLocation {
    pub fn new(map_id: u32, x: f32, y: f32, z: f32, orientation: f32) -> Self {
        Self {
            map_id,
            position: Position::new(x, y, z, normalize_orientation(orientation)),
        }
    }

    pub const fn map_id(&self) -> u32 {
        self.map_id
    }

    pub const fn position(&self) -> Position {
        self.position
    }

    pub fn world_relocate(&mut self, map_id: u32, position: Position) {
        self.map_id = map_id;
        self.position = normalized_position(position);
    }

    pub fn relocate(&mut self, position: Position) {
        self.position = normalized_position(position);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VisibleMapIdRef {
    pub(super) references: i32,
}

impl VisibleMapIdRef {
    pub const fn references(&self) -> i32 {
        self.references
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiMapPhaseIdRef {
    pub(super) references: i32,
}

impl UiMapPhaseIdRef {
    pub const fn references(&self) -> i32 {
        self.references
    }
}

pub(super) const DEFAULT_PHASE: u32 = 169;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmoothPhasingInfoLikeCpp {
    pub replace_object: Option<ObjectGuid>,
    pub replace_active: bool,
    pub stop_anim_kits: bool,
    pub disabled: bool,
}

impl Default for SmoothPhasingInfoLikeCpp {
    fn default() -> Self {
        Self {
            replace_object: None,
            replace_active: true,
            stop_anim_kits: true,
            disabled: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SmoothPhasingStorageLikeCpp {
    Single(SmoothPhasingInfoLikeCpp),
    ViewerDependent(BTreeMap<ObjectGuid, SmoothPhasingInfoLikeCpp>),
}

impl Default for SmoothPhasingStorageLikeCpp {
    fn default() -> Self {
        Self::Single(SmoothPhasingInfoLikeCpp::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SmoothPhasingLikeCpp {
    pub(super) storage: SmoothPhasingStorageLikeCpp,
}

impl SmoothPhasingLikeCpp {
    pub fn set_viewer_dependent_info_like_cpp(
        &mut self,
        seer: ObjectGuid,
        info: SmoothPhasingInfoLikeCpp,
    ) {
        if !matches!(
            self.storage,
            SmoothPhasingStorageLikeCpp::ViewerDependent(_)
        ) {
            self.storage = SmoothPhasingStorageLikeCpp::ViewerDependent(BTreeMap::new());
        }

        if let SmoothPhasingStorageLikeCpp::ViewerDependent(viewer_info) = &mut self.storage {
            viewer_info.insert(seer, info);
        }
    }

    pub fn clear_viewer_dependent_info_like_cpp(&mut self, seer: ObjectGuid) {
        if let SmoothPhasingStorageLikeCpp::ViewerDependent(viewer_info) = &mut self.storage {
            viewer_info.remove(&seer);
        }
    }

    pub fn set_single_info_like_cpp(&mut self, info: SmoothPhasingInfoLikeCpp) {
        self.storage = SmoothPhasingStorageLikeCpp::Single(info);
    }

    pub fn is_replacing_like_cpp(&self, guid: ObjectGuid) -> bool {
        matches!(
            self.storage,
            SmoothPhasingStorageLikeCpp::Single(SmoothPhasingInfoLikeCpp {
                replace_object: Some(replace_object),
                ..
            }) if replace_object == guid
        )
    }

    pub fn is_being_replaced_for_seer_like_cpp(&self, seer: ObjectGuid) -> bool {
        match &self.storage {
            SmoothPhasingStorageLikeCpp::ViewerDependent(viewer_info) => viewer_info
                .get(&seer)
                .is_some_and(|smooth_phasing_info| !smooth_phasing_info.disabled),
            SmoothPhasingStorageLikeCpp::Single(_) => false,
        }
    }

    pub fn info_for_seer_like_cpp(&self, seer: ObjectGuid) -> Option<&SmoothPhasingInfoLikeCpp> {
        match &self.storage {
            SmoothPhasingStorageLikeCpp::ViewerDependent(viewer_info) => viewer_info.get(&seer),
            SmoothPhasingStorageLikeCpp::Single(info) => Some(info),
        }
    }

    pub fn disable_replacement_for_seer_like_cpp(&mut self, seer: ObjectGuid) {
        if let SmoothPhasingStorageLikeCpp::ViewerDependent(viewer_info) = &mut self.storage {
            if let Some(smooth_phasing_info) = viewer_info.get_mut(&seer) {
                smooth_phasing_info.disabled = true;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseRef {
    pub(super) id: u32,
    pub(super) flags: PhaseFlags,
    pub(super) references: i32,
}

impl PhaseRef {
    pub const fn id(&self) -> u32 {
        self.id
    }

    pub const fn flags(&self) -> PhaseFlags {
        self.flags
    }

    pub const fn references(&self) -> i32 {
        self.references
    }

    pub const fn is_personal(&self) -> bool {
        self.flags.contains(PhaseFlags::PERSONAL)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseShift {
    pub(super) flags: PhaseShiftFlags,
    pub(super) personal_guid: ObjectGuid,
    pub(super) phases: BTreeMap<u32, PhaseRef>,
    pub(super) visible_map_ids: BTreeMap<u32, VisibleMapIdRef>,
    pub(super) ui_map_phase_ids: BTreeMap<u32, UiMapPhaseIdRef>,
    pub(super) non_cosmetic_references: i32,
    pub(super) cosmetic_references: i32,
    pub(super) personal_references: i32,
    pub(super) default_references: i32,
    pub(super) is_db_phase_shift: bool,
}

impl Default for PhaseShift {
    fn default() -> Self {
        Self {
            flags: PhaseShiftFlags::UNPHASED,
            personal_guid: ObjectGuid::EMPTY,
            phases: BTreeMap::new(),
            visible_map_ids: BTreeMap::new(),
            ui_map_phase_ids: BTreeMap::new(),
            non_cosmetic_references: 0,
            cosmetic_references: 0,
            personal_references: 0,
            default_references: 0,
            is_db_phase_shift: false,
        }
    }
}

impl PhaseShift {
    pub fn from_phases(phases: impl IntoIterator<Item = u32>) -> Self {
        let mut phase_shift = Self::default();
        for phase_id in phases {
            phase_shift.add_phase_like_cpp(phase_id, PhaseFlags::NONE, 1);
        }
        phase_shift
    }

    pub fn insert(&mut self, phase_id: u32) {
        self.add_phase_like_cpp(phase_id, PhaseFlags::NONE, 1);
    }

    pub fn add_phase_like_cpp(
        &mut self,
        phase_id: u32,
        flags: PhaseFlags,
        references: i32,
    ) -> bool {
        let inserted = !self.phases.contains_key(&phase_id);
        let entry = self.phases.entry(phase_id).or_insert(PhaseRef {
            id: phase_id,
            flags,
            references: 0,
        });
        entry.references += references;
        let phase_ref = *entry;
        self.modify_phase_reference_counters(phase_ref, references);
        inserted
    }

    pub fn remove_phase_like_cpp(&mut self, phase_id: u32) -> bool {
        let Some(mut phase_ref) = self.phases.get(&phase_id).copied() else {
            return false;
        };

        phase_ref.references -= 1;
        self.modify_phase_reference_counters(phase_ref, -1);
        if phase_ref.references == 0 {
            self.phases.remove(&phase_id);
            return true;
        }

        if let Some(entry) = self.phases.get_mut(&phase_id) {
            entry.references = phase_ref.references;
        }
        false
    }

    pub fn remove_phase_all_references_like_cpp(&mut self, phase_id: u32) -> Option<PhaseRef> {
        let phase_ref = self.phases.remove(&phase_id)?;
        self.modify_phase_reference_counters(phase_ref, -phase_ref.references);
        Some(phase_ref)
    }

    pub fn has_phase_like_cpp(&self, phase_id: u32) -> bool {
        self.phases.contains_key(&phase_id)
    }

    pub fn phase_ref_like_cpp(&self, phase_id: u32) -> Option<&PhaseRef> {
        self.phases.get(&phase_id)
    }

    pub fn phases_like_cpp(&self) -> impl Iterator<Item = &PhaseRef> + '_ {
        self.phases.values()
    }

    pub fn phase_snapshot_like_cpp(&self) -> Vec<PhaseRef> {
        self.phases.values().copied().collect()
    }

    pub fn phase_count_like_cpp(&self) -> usize {
        self.phases.len()
    }

    pub const fn flags_like_cpp(&self) -> PhaseShiftFlags {
        self.flags
    }

    pub fn set_flags_like_cpp(&mut self, flags: PhaseShiftFlags) {
        self.flags = flags;
    }

    pub const fn personal_guid_like_cpp(&self) -> ObjectGuid {
        self.personal_guid
    }

    pub fn set_personal_guid_like_cpp(&mut self, guid: ObjectGuid) {
        self.personal_guid = guid;
    }

    pub fn set_always_visible_like_cpp(&mut self, apply: bool) {
        self.flags.set(PhaseShiftFlags::ALWAYS_VISIBLE, apply);
    }

    pub fn set_inversed_like_cpp(&mut self, apply: bool) {
        self.flags.set(PhaseShiftFlags::INVERSE, apply);
        self.update_unphased_flag_like_cpp();
    }

    pub fn set_db_phase_shift_like_cpp(&mut self, is_db_phase_shift: bool) {
        self.is_db_phase_shift = is_db_phase_shift;
    }

    pub const fn is_db_phase_shift_like_cpp(&self) -> bool {
        self.is_db_phase_shift
    }

    pub fn clear(&mut self) {
        self.clear_phases_like_cpp();
        self.visible_map_ids.clear();
        self.ui_map_phase_ids.clear();
    }

    pub fn clear_phases_like_cpp(&mut self) {
        self.flags &= PhaseShiftFlags::ALWAYS_VISIBLE | PhaseShiftFlags::INVERSE;
        self.personal_guid = ObjectGuid::EMPTY;
        self.phases.clear();
        self.non_cosmetic_references = 0;
        self.cosmetic_references = 0;
        self.personal_references = 0;
        self.default_references = 0;
        self.update_unphased_flag_like_cpp();
    }

    pub fn add_visible_map_id_like_cpp(&mut self, visible_map_id: u32, references: i32) -> bool {
        let inserted = !self.visible_map_ids.contains_key(&visible_map_id);
        let entry = self
            .visible_map_ids
            .entry(visible_map_id)
            .or_insert(VisibleMapIdRef { references: 0 });
        entry.references += references;
        inserted
    }

    pub fn clear_visible_map_ids_like_cpp(&mut self) {
        self.visible_map_ids.clear();
    }

    pub fn remove_visible_map_id_like_cpp(&mut self, visible_map_id: u32) -> bool {
        let Some(entry) = self.visible_map_ids.get_mut(&visible_map_id) else {
            return false;
        };
        entry.references -= 1;
        if entry.references == 0 {
            self.visible_map_ids.remove(&visible_map_id);
            return true;
        }
        false
    }

    pub fn remove_visible_map_id_all_references_like_cpp(
        &mut self,
        visible_map_id: u32,
    ) -> Option<VisibleMapIdRef> {
        self.visible_map_ids.remove(&visible_map_id)
    }

    pub fn has_visible_map_id_like_cpp(&self, visible_map_id: u32) -> bool {
        self.visible_map_ids.contains_key(&visible_map_id)
    }

    pub fn visible_map_id_count_like_cpp(&self) -> usize {
        self.visible_map_ids.len()
    }

    pub fn visible_map_ids_like_cpp(&self) -> impl Iterator<Item = u32> + '_ {
        self.visible_map_ids.keys().copied()
    }

    pub fn visible_map_id_snapshot_like_cpp(&self) -> Vec<(u32, VisibleMapIdRef)> {
        self.visible_map_ids
            .iter()
            .map(|(visible_map_id, visible_map_ref)| (*visible_map_id, visible_map_ref.clone()))
            .collect()
    }

    pub fn visible_map_id_ref_like_cpp(&self, visible_map_id: u32) -> Option<&VisibleMapIdRef> {
        self.visible_map_ids.get(&visible_map_id)
    }

    pub fn add_ui_map_phase_id_like_cpp(&mut self, ui_map_phase_id: u32, references: i32) -> bool {
        let inserted = !self.ui_map_phase_ids.contains_key(&ui_map_phase_id);
        let entry = self
            .ui_map_phase_ids
            .entry(ui_map_phase_id)
            .or_insert(UiMapPhaseIdRef { references: 0 });
        entry.references += references;
        inserted
    }

    pub fn clear_ui_map_phase_ids_like_cpp(&mut self) {
        self.ui_map_phase_ids.clear();
    }

    pub fn remove_ui_map_phase_id_like_cpp(&mut self, ui_map_phase_id: u32) -> bool {
        let Some(entry) = self.ui_map_phase_ids.get_mut(&ui_map_phase_id) else {
            return false;
        };
        entry.references -= 1;
        if entry.references == 0 {
            self.ui_map_phase_ids.remove(&ui_map_phase_id);
            return true;
        }
        false
    }

    pub fn has_ui_map_phase_id_like_cpp(&self, ui_map_phase_id: u32) -> bool {
        self.ui_map_phase_ids.contains_key(&ui_map_phase_id)
    }

    pub fn ui_map_phase_ids_like_cpp(&self) -> impl Iterator<Item = u32> + '_ {
        self.ui_map_phase_ids.keys().copied()
    }

    pub fn ui_map_phase_id_ref_like_cpp(&self, ui_map_phase_id: u32) -> Option<&UiMapPhaseIdRef> {
        self.ui_map_phase_ids.get(&ui_map_phase_id)
    }

    pub fn can_see(&self, other: &Self) -> bool {
        if self.flags.contains(PhaseShiftFlags::UNPHASED)
            && other.flags.contains(PhaseShiftFlags::UNPHASED)
        {
            return true;
        }

        if self.flags.contains(PhaseShiftFlags::ALWAYS_VISIBLE)
            || other.flags.contains(PhaseShiftFlags::ALWAYS_VISIBLE)
        {
            return true;
        }

        if self.flags.contains(PhaseShiftFlags::INVERSE)
            && other.flags.contains(PhaseShiftFlags::INVERSE)
        {
            return true;
        }

        let exclude_phases_with_flag = if self.flags.contains(PhaseShiftFlags::NO_COSMETIC)
            && other.flags.contains(PhaseShiftFlags::NO_COSMETIC)
        {
            PhaseFlags::COSMETIC
        } else {
            PhaseFlags::NONE
        };

        if !self.flags.contains(PhaseShiftFlags::INVERSE)
            && !other.flags.contains(PhaseShiftFlags::INVERSE)
        {
            return self.phases.iter().any(|(phase_id, phase_ref)| {
                other.phases.contains_key(phase_id)
                    && !phase_ref.flags.intersects(exclude_phases_with_flag)
                    && (!phase_ref.flags.contains(PhaseFlags::PERSONAL)
                        || self.personal_guid == other.personal_guid)
            });
        }

        if other.flags.contains(PhaseShiftFlags::INVERSE) {
            return check_inverse_phase_shift_like_cpp(self, other, exclude_phases_with_flag);
        }

        check_inverse_phase_shift_like_cpp(other, self, exclude_phases_with_flag)
    }

    pub fn has_personal_phase_like_cpp(&self) -> bool {
        self.phases.values().any(PhaseRef::is_personal)
    }

    pub(super) fn modify_phase_reference_counters(&mut self, phase_ref: PhaseRef, references: i32) {
        if self.is_db_phase_shift {
            return;
        }

        if phase_ref.flags.contains(PhaseFlags::COSMETIC) {
            self.cosmetic_references += references;
        } else if phase_ref.id != DEFAULT_PHASE {
            self.non_cosmetic_references += references;
        } else {
            self.default_references += references;
        }

        if phase_ref.flags.contains(PhaseFlags::PERSONAL) {
            self.personal_references += references;
        }

        self.flags
            .set(PhaseShiftFlags::NO_COSMETIC, self.cosmetic_references != 0);
        self.update_unphased_flag_like_cpp();
        self.update_personal_guid_like_cpp();
    }

    pub(super) fn update_unphased_flag_like_cpp(&mut self) {
        let unphased_flag = if !self.flags.contains(PhaseShiftFlags::INVERSE) {
            PhaseShiftFlags::UNPHASED
        } else {
            PhaseShiftFlags::INVERSE_UNPHASED
        };
        let opposite_flag = if !self.flags.contains(PhaseShiftFlags::INVERSE) {
            PhaseShiftFlags::INVERSE_UNPHASED
        } else {
            PhaseShiftFlags::UNPHASED
        };

        self.flags.remove(opposite_flag);
        self.flags.set(
            unphased_flag,
            !(self.non_cosmetic_references != 0 && self.default_references == 0),
        );
    }

    pub(super) fn update_personal_guid_like_cpp(&mut self) {
        if self.personal_references == 0 {
            self.personal_guid = ObjectGuid::EMPTY;
        }
    }
}

pub(super) fn check_inverse_phase_shift_like_cpp(
    phase_shift: &PhaseShift,
    excluded_phase_shift: &PhaseShift,
    exclude_phases_with_flag: PhaseFlags,
) -> bool {
    if phase_shift.flags.contains(PhaseShiftFlags::UNPHASED)
        && excluded_phase_shift
            .flags
            .contains(PhaseShiftFlags::INVERSE_UNPHASED)
    {
        return false;
    }

    for phase in phase_shift.phases.values() {
        if phase.flags.intersects(exclude_phases_with_flag) {
            continue;
        }

        if excluded_phase_shift
            .phases
            .get(&phase.id)
            .is_some_and(|excluded| !excluded.flags.intersects(exclude_phases_with_flag))
        {
            return false;
        }
    }

    true
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldObject {
    pub(super) object: EntityObject,
    pub(super) location: WorldLocation,
    pub(super) instance_id: u32,
    pub(super) has_current_map: bool,
    pub(super) phase_shift: PhaseShift,
    pub(super) suppressed_phase_shift: PhaseShift,
    pub(super) db_phase: i32,
    pub(super) name: String,
    pub(super) is_active: bool,
    pub(super) is_far_visible: bool,
    pub(super) visibility_distance_override: Option<f32>,
    pub(super) is_world_object: bool,
    pub(super) static_floor_z: f32,
    pub(super) zone_id: u32,
    pub(super) area_id: u32,
    pub(super) combat_reach: f32,
    pub(super) collision_height_like_cpp: f32,
    pub(super) current_cell: Option<(u32, u32)>,
    pub(super) smooth_phasing: Option<SmoothPhasingLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapBindingError {
    ObjectInWorld,
    AlreadyBound {
        old_map_id: u32,
        old_instance_id: u32,
        new_map_id: u32,
        new_instance_id: u32,
    },
    NoCurrentMap,
}

pub(super) fn normalized_position(mut position: Position) -> Position {
    position.orientation = normalize_orientation(position.orientation);
    position
}

pub(super) fn normalize_orientation(mut orientation: f32) -> f32 {
    orientation %= TAU;
    if orientation < 0.0 {
        orientation += TAU;
    }
    orientation
}
