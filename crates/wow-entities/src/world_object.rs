use std::collections::BTreeMap;
use std::f32::consts::{PI, TAU};

use wow_constants::{PhaseFlags, PhaseShiftFlags, TypeId, TypeMask};
use wow_core::{ObjectGuid, Position};

use crate::{
    EntityObject,
    vehicle::{calculate_passenger_offset, calculate_passenger_position},
};

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
    map_id: u32,
    position: Position,
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
    references: i32,
}

impl VisibleMapIdRef {
    pub const fn references(&self) -> i32 {
        self.references
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiMapPhaseIdRef {
    references: i32,
}

impl UiMapPhaseIdRef {
    pub const fn references(&self) -> i32 {
        self.references
    }
}

const DEFAULT_PHASE: u32 = 169;

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
enum SmoothPhasingStorageLikeCpp {
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
    storage: SmoothPhasingStorageLikeCpp,
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
    id: u32,
    flags: PhaseFlags,
    references: i32,
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
    flags: PhaseShiftFlags,
    personal_guid: ObjectGuid,
    phases: BTreeMap<u32, PhaseRef>,
    visible_map_ids: BTreeMap<u32, VisibleMapIdRef>,
    ui_map_phase_ids: BTreeMap<u32, UiMapPhaseIdRef>,
    non_cosmetic_references: i32,
    cosmetic_references: i32,
    personal_references: i32,
    default_references: i32,
    is_db_phase_shift: bool,
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

    fn modify_phase_reference_counters(&mut self, phase_ref: PhaseRef, references: i32) {
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

    fn update_unphased_flag_like_cpp(&mut self) {
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

    fn update_personal_guid_like_cpp(&mut self) {
        if self.personal_references == 0 {
            self.personal_guid = ObjectGuid::EMPTY;
        }
    }
}

fn check_inverse_phase_shift_like_cpp(
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
    object: EntityObject,
    location: WorldLocation,
    instance_id: u32,
    has_current_map: bool,
    phase_shift: PhaseShift,
    suppressed_phase_shift: PhaseShift,
    db_phase: i32,
    name: String,
    is_active: bool,
    is_far_visible: bool,
    visibility_distance_override: Option<f32>,
    is_world_object: bool,
    static_floor_z: f32,
    zone_id: u32,
    area_id: u32,
    combat_reach: f32,
    collision_height_like_cpp: f32,
    current_cell: Option<(u32, u32)>,
    smooth_phasing: Option<SmoothPhasingLikeCpp>,
}

impl WorldObject {
    pub fn new(is_world_object: bool, type_id: TypeId, type_mask: TypeMask) -> Self {
        Self {
            object: EntityObject::new(type_id, type_mask),
            location: WorldLocation::default(),
            instance_id: 0,
            has_current_map: false,
            phase_shift: PhaseShift::default(),
            suppressed_phase_shift: PhaseShift::default(),
            db_phase: 0,
            name: String::new(),
            is_active: false,
            is_far_visible: false,
            visibility_distance_override: None,
            is_world_object,
            static_floor_z: INVALID_HEIGHT,
            zone_id: 0,
            area_id: 0,
            combat_reach: 0.0,
            collision_height_like_cpp: 0.0,
            current_cell: None,
            smooth_phasing: None,
        }
    }

    pub const fn object(&self) -> &EntityObject {
        &self.object
    }

    pub fn object_mut(&mut self) -> &mut EntityObject {
        &mut self.object
    }

    pub const fn guid(&self) -> ObjectGuid {
        self.object.guid()
    }

    pub const fn map_id(&self) -> u32 {
        self.location.map_id()
    }

    pub const fn instance_id(&self) -> u32 {
        self.instance_id
    }

    pub const fn position(&self) -> Position {
        self.location.position()
    }

    pub fn relocate(&mut self, position: Position) {
        self.location.relocate(position);
    }

    pub fn world_relocate(&mut self, map_id: u32, position: Position) {
        self.location.world_relocate(map_id, position);
        self.object.bind_map(map_id, self.instance_id);
    }

    pub fn set_map(&mut self, map_id: u32, instance_id: u32) -> Result<(), MapBindingError> {
        if self.object.is_in_world() {
            return Err(MapBindingError::ObjectInWorld);
        }

        if self.has_current_map {
            if self.map_id() == map_id && self.instance_id == instance_id {
                return Ok(());
            }
            return Err(MapBindingError::AlreadyBound {
                old_map_id: self.map_id(),
                old_instance_id: self.instance_id,
                new_map_id: map_id,
                new_instance_id: instance_id,
            });
        }

        self.has_current_map = true;
        self.location.map_id = map_id;
        self.instance_id = instance_id;
        self.object.bind_map(map_id, instance_id);
        Ok(())
    }

    pub fn reset_map(&mut self) -> Result<(), MapBindingError> {
        if !self.has_current_map {
            return Err(MapBindingError::NoCurrentMap);
        }
        if self.object.is_in_world() {
            return Err(MapBindingError::ObjectInWorld);
        }

        self.has_current_map = false;
        self.current_cell = None;
        self.object.set_grid_presence(false);
        Ok(())
    }

    pub const fn has_current_map(&self) -> bool {
        self.has_current_map
    }

    pub const fn phase_shift(&self) -> &PhaseShift {
        &self.phase_shift
    }

    pub fn phase_shift_mut(&mut self) -> &mut PhaseShift {
        &mut self.phase_shift
    }

    pub const fn suppressed_phase_shift(&self) -> &PhaseShift {
        &self.suppressed_phase_shift
    }

    pub fn suppressed_phase_shift_mut(&mut self) -> &mut PhaseShift {
        &mut self.suppressed_phase_shift
    }

    pub const fn db_phase(&self) -> i32 {
        self.db_phase
    }

    pub fn set_db_phase(&mut self, db_phase: i32) {
        self.db_phase = db_phase;
    }

    pub fn in_same_phase(&self, other: &Self) -> bool {
        self.phase_shift.can_see(&other.phase_shift)
    }

    pub fn set_current_cell(&mut self, cell_x: u32, cell_y: u32) {
        self.current_cell = Some((cell_x, cell_y));
        self.object.set_grid_presence(true);
    }

    pub fn clear_current_cell(&mut self) {
        self.current_cell = None;
        self.object.set_grid_presence(false);
    }

    pub const fn current_cell(&self) -> Option<(u32, u32)> {
        self.current_cell
    }

    pub const fn smooth_phasing_like_cpp(&self) -> Option<&SmoothPhasingLikeCpp> {
        self.smooth_phasing.as_ref()
    }

    pub fn smooth_phasing_mut_like_cpp(&mut self) -> Option<&mut SmoothPhasingLikeCpp> {
        self.smooth_phasing.as_mut()
    }

    pub fn get_or_create_smooth_phasing_like_cpp(&mut self) -> &mut SmoothPhasingLikeCpp {
        self.smooth_phasing
            .get_or_insert_with(SmoothPhasingLikeCpp::default)
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
    }

    pub const fn is_far_visible(&self) -> bool {
        self.is_far_visible
    }

    pub fn set_far_visible(&mut self, far_visible: bool) {
        self.is_far_visible = far_visible;
    }

    pub const fn visibility_distance_override_like_cpp(&self) -> Option<f32> {
        self.visibility_distance_override
    }

    pub fn set_visibility_distance_override_like_cpp(
        &mut self,
        visibility_type: VisibilityDistanceTypeLikeCpp,
    ) {
        if self.is_player() {
            return;
        }
        self.visibility_distance_override = Some(visibility_type.distance_like_cpp());
    }

    pub const fn is_world_object(&self) -> bool {
        self.is_world_object
    }

    pub const fn static_floor_z(&self) -> f32 {
        self.static_floor_z
    }

    pub fn set_static_floor_z(&mut self, static_floor_z: f32) {
        self.static_floor_z = static_floor_z;
    }

    pub const fn zone_id(&self) -> u32 {
        self.zone_id
    }

    pub const fn area_id(&self) -> u32 {
        self.area_id
    }

    pub fn set_zone_and_area(&mut self, zone_id: u32, area_id: u32) {
        self.zone_id = zone_id;
        self.area_id = area_id;
    }

    pub const fn combat_reach(&self) -> f32 {
        self.combat_reach
    }

    pub fn set_combat_reach(&mut self, combat_reach: f32) {
        self.combat_reach = combat_reach.max(0.0);
    }

    pub const fn collision_height_like_cpp(&self) -> f32 {
        self.collision_height_like_cpp
    }

    pub fn set_collision_height_like_cpp(&mut self, height: f32) {
        self.collision_height_like_cpp = height.max(0.0);
    }

    pub fn get_hit_sphere_point_for_like_cpp(&self, dest: Position) -> LineOfSightEndpoint {
        let height = self.collision_height_like_cpp();
        let mut contact = self.position();
        contact.z += height;

        let dx = dest.x - contact.x;
        let dy = dest.y - contact.y;
        let dz = dest.z - contact.z;
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        let move_dist = dest.distance(&self.position()).min(self.combat_reach());
        let hit_sphere_adjusted = len > 0.0 && move_dist > 0.0;
        if hit_sphere_adjusted {
            contact.x += dx / len * move_dist;
            contact.y += dy / len * move_dist;
            contact.z += dz / len * move_dist;
        }
        contact.orientation = self.absolute_angle_to_position(contact);

        LineOfSightEndpoint {
            position: contact,
            collision_height_adjusted: height > 0.0,
            hit_sphere_adjusted,
        }
    }

    pub fn exact_distance(&self, other: &Self) -> f32 {
        self.position().distance(&other.position())
    }

    pub fn exact_distance_2d(&self, other: &Self) -> f32 {
        self.position().distance_2d(&other.position())
    }

    pub fn distance(&self, other: &Self) -> f32 {
        (self.exact_distance(other) - self.combat_reach - other.combat_reach).max(0.0)
    }

    pub fn distance_to_position(&self, position: Position) -> f32 {
        (self.position().distance(&position) - self.combat_reach).max(0.0)
    }

    pub fn distance_2d(&self, other: &Self) -> f32 {
        (self.exact_distance_2d(other) - self.combat_reach - other.combat_reach).max(0.0)
    }

    pub fn distance_z(&self, other: &Self) -> f32 {
        ((self.position().z - other.position().z).abs() - self.combat_reach - other.combat_reach)
            .max(0.0)
    }

    pub fn is_in_map(&self, other: &Self) -> bool {
        self.object.is_in_world()
            && other.object.is_in_world()
            && self.has_current_map
            && other.has_current_map
            && self.map_id() == other.map_id()
            && self.instance_id == other.instance_id
    }

    pub fn is_within_dist(
        &self,
        other: &Self,
        dist: f32,
        is_3d: bool,
        include_own_radius: bool,
        include_target_radius: bool,
    ) -> bool {
        let mut max_dist = dist;
        if include_own_radius {
            max_dist += self.combat_reach;
        }
        if include_target_radius {
            max_dist += other.combat_reach;
        }

        if is_3d {
            self.position().distance_sq(&other.position()) < max_dist * max_dist
        } else {
            self.position().distance_2d_sq(&other.position()) < max_dist * max_dist
        }
    }

    pub fn is_within_dist_in_map(&self, other: &Self, dist: f32, is_3d: bool) -> bool {
        self.is_in_map(other)
            && self.in_same_phase(other)
            && self.is_within_dist(other, dist, is_3d, true, true)
    }

    pub fn absolute_angle_to_position(&self, position: Position) -> f32 {
        normalize_orientation(
            (position.y - self.position().y).atan2(position.x - self.position().x),
        )
    }

    pub fn absolute_angle_to(&self, other: &Self) -> f32 {
        self.absolute_angle_to_position(other.position())
    }

    pub fn to_absolute_angle(&self, relative_angle: f32) -> f32 {
        normalize_orientation(relative_angle + self.position().orientation)
    }

    pub fn to_relative_angle(&self, absolute_angle: f32) -> f32 {
        normalize_orientation(absolute_angle - self.position().orientation)
    }

    pub fn relative_angle_to_position(&self, position: Position) -> f32 {
        self.to_relative_angle(self.absolute_angle_to_position(position))
    }

    pub fn relative_angle_to(&self, other: &Self) -> f32 {
        self.relative_angle_to_position(other.position())
    }

    pub fn has_in_arc(&self, arc: f32, target: &Self, border: f32) -> bool {
        if std::ptr::eq(self, target) {
            return true;
        }

        let arc = normalize_orientation(arc);
        let mut angle = self.relative_angle_to(target);
        if angle > PI {
            angle -= TAU;
        }

        let left_border = -(arc / border);
        let right_border = arc / border;
        left_border <= angle && angle <= right_border
    }

    pub fn is_in_front(&self, target: &Self, arc: f32) -> bool {
        self.has_in_arc(arc, target, 2.0)
    }

    pub fn is_in_back(&self, target: &Self, arc: f32) -> bool {
        !self.has_in_arc(TAU - arc, target, 2.0)
    }

    pub fn has_position_in_line(&self, position: Position, obj_size: f32, width: f32) -> bool {
        if !self.has_position_in_arc(PI, position, 2.0) {
            return false;
        }

        let width = width + obj_size;
        let angle = self.relative_angle_to_position(position);
        angle.sin().abs() * self.position().distance_2d(&position) < width
    }

    pub fn has_in_line(&self, target: &Self, width: f32) -> bool {
        self.has_position_in_line(target.position(), target.combat_reach(), width)
    }

    pub fn has_position_in_arc(&self, arc: f32, position: Position, border: f32) -> bool {
        let arc = normalize_orientation(arc);
        let mut angle = self.relative_angle_to_position(position);
        if angle > PI {
            angle -= TAU;
        }

        let left_border = -(arc / border);
        let right_border = arc / border;
        left_border <= angle && angle <= right_border
    }

    pub fn is_within_box(
        &self,
        center: Position,
        x_radius: f32,
        y_radius: f32,
        z_radius: f32,
    ) -> bool {
        let rotation = TAU - center.orientation;
        let sin = rotation.sin();
        let cos = rotation.cos();
        let position = self.position();

        let box_dist_x = position.x - center.x;
        let box_dist_y = position.y - center.y;
        let rot_x = center.x + box_dist_x * cos - box_dist_y * sin;
        let rot_y = center.y + box_dist_y * cos + box_dist_x * sin;

        let dx = rot_x - center.x;
        let dy = rot_y - center.y;
        let dz = position.z - center.z;
        dx.abs() <= x_radius && dy.abs() <= y_radius && dz.abs() <= z_radius
    }

    pub fn is_within_double_vertical_cylinder(
        &self,
        center: Position,
        radius: f32,
        height: f32,
    ) -> bool {
        self.position().distance_2d_sq(&center) < radius * radius
            && (self.position().z - center.z).abs() <= height
    }

    pub fn get_visibility_range(&self, environment: &impl WorldObjectEnvironment) -> f32 {
        if let Some(override_range) = self.visibility_distance_override_like_cpp() {
            if !self.is_player() {
                return override_range;
            }
        }
        if environment.visibility_override(self).is_some() && !self.is_player() {
            environment
                .visibility_override(self)
                .unwrap_or(environment.visibility_range())
        } else if self.is_far_visible() && !self.is_player() {
            MAX_VISIBILITY_DISTANCE
        } else {
            environment.visibility_range()
        }
    }

    pub fn get_sight_range(
        &self,
        target: Option<&WorldObject>,
        environment: &impl WorldObjectEnvironment,
    ) -> f32 {
        if self.is_unit() {
            if self.is_player() {
                if let Some(target) = target {
                    if let Some(override_range) = target.visibility_distance_override_like_cpp() {
                        if !target.is_player() {
                            return override_range;
                        }
                    }
                    if let Some(override_range) = environment.visibility_override(target) {
                        if !target.is_player() {
                            return override_range;
                        }
                    }
                    if target.is_far_visible() && !target.is_player() {
                        return MAX_VISIBILITY_DISTANCE;
                    }
                }

                if environment.player_on_cinematic(self) {
                    DEFAULT_VISIBILITY_INSTANCE
                } else {
                    environment.visibility_range()
                }
            } else {
                environment
                    .creature_sight_distance(self)
                    .unwrap_or(SIGHT_RANGE_UNIT)
            }
        } else if self.object.type_id() == TypeId::DynamicObject && self.is_active() {
            environment.visibility_range()
        } else {
            0.0
        }
    }

    pub fn is_within_los(
        &self,
        position: Position,
        environment: &impl WorldObjectEnvironment,
        options: LineOfSightOptions,
    ) -> bool {
        if !self.object.is_in_world() {
            return true;
        }

        if !self.is_in_environment(environment) {
            return false;
        }

        environment.line_of_sight(LineOfSightQuery::to_position_like_cpp(
            self, position, options,
        ))
    }

    pub fn is_within_los_in_map(
        &self,
        other: &Self,
        environment: &impl WorldObjectEnvironment,
        options: LineOfSightOptions,
    ) -> bool {
        self.is_in_map(other)
            && self.is_in_environment(environment)
            && environment.line_of_sight(LineOfSightQuery::to_object_like_cpp(self, other, options))
    }

    pub fn get_map_height(
        &self,
        environment: &impl WorldObjectEnvironment,
        x: f32,
        y: f32,
        z: f32,
        query: WorldObjectHeightQuery,
    ) -> f32 {
        if !self.is_in_environment(environment) {
            return INVALID_HEIGHT;
        }

        let query_z = if z == MAX_HEIGHT {
            z
        } else {
            z + Z_OFFSET_FIND_HEIGHT
        };
        environment.map_height(self, x, y, query_z, query)
    }

    pub fn update_ground_position_z(
        &self,
        environment: &impl WorldObjectEnvironment,
        x: f32,
        y: f32,
        z: f32,
        hover_offset: f32,
    ) -> f32 {
        let new_z = self.get_map_height(environment, x, y, z, WorldObjectHeightQuery::default());
        if new_z > INVALID_HEIGHT {
            new_z + if self.is_unit() { hover_offset } else { 0.0 }
        } else {
            z
        }
    }

    pub fn get_floor_z(&self, environment: &impl WorldObjectEnvironment) -> f32 {
        if !self.object.is_in_world() || !self.is_in_environment(environment) {
            return self.static_floor_z;
        }

        let position = Position::new(
            self.position().x,
            self.position().y,
            self.position().z + Z_OFFSET_FIND_HEIGHT,
            self.position().orientation,
        );
        self.static_floor_z
            .max(environment.floor_z(self, position, DEFAULT_HEIGHT_SEARCH))
    }

    pub fn transport_global_position_from_offset(
        &self,
        transport_position: Position,
        passenger_offset: Position,
    ) -> Position {
        calculate_passenger_position(passenger_offset, transport_position)
    }

    pub fn transport_offset_from_position(&self, transport_position: Position) -> Position {
        calculate_passenger_offset(self.position(), transport_position)
    }

    pub fn relocate_on_transport(
        &mut self,
        transport_position: Position,
        passenger_offset: Position,
    ) -> Position {
        let global =
            self.transport_global_position_from_offset(transport_position, passenger_offset);
        self.relocate(global);
        global
    }

    fn own_position_with_collision_height_like_cpp(&self) -> LineOfSightEndpoint {
        self.position_with_collision_height_like_cpp(self.position())
    }

    fn position_with_collision_height_like_cpp(&self, position: Position) -> LineOfSightEndpoint {
        self.position_with_explicit_collision_height_like_cpp(
            position,
            self.collision_height_like_cpp(),
        )
    }

    fn position_with_explicit_collision_height_like_cpp(
        &self,
        mut position: Position,
        height: f32,
    ) -> LineOfSightEndpoint {
        let adjusted_height = height.max(0.0);
        if adjusted_height > 0.0 {
            position.z += adjusted_height;
        }

        LineOfSightEndpoint {
            position,
            collision_height_adjusted: adjusted_height > 0.0,
            hit_sphere_adjusted: false,
        }
    }

    fn is_in_environment(&self, environment: &impl WorldObjectEnvironment) -> bool {
        self.has_current_map
            && self.map_id() == environment.map_id()
            && self.instance_id == environment.instance_id()
    }

    fn is_player(&self) -> bool {
        matches!(self.object.type_id(), TypeId::Player | TypeId::ActivePlayer)
            || self
                .object
                .type_mask()
                .intersects(TypeMask::PLAYER | TypeMask::ACTIVE_PLAYER)
    }

    fn is_unit(&self) -> bool {
        self.object
            .type_mask()
            .intersects(TypeMask::UNIT | TypeMask::PLAYER | TypeMask::ACTIVE_PLAYER)
            || matches!(
                self.object.type_id(),
                TypeId::Unit | TypeId::Player | TypeId::ActivePlayer
            )
    }
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

fn normalized_position(mut position: Position) -> Position {
    position.orientation = normalize_orientation(position.orientation);
    position
}

fn normalize_orientation(mut orientation: f32) -> f32 {
    orientation %= TAU;
    if orientation < 0.0 {
        orientation += TAU;
    }
    orientation
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct LosSnapshot {
        from: LineOfSightEndpoint,
        to: LineOfSightEndpoint,
        has_target: bool,
    }

    #[derive(Debug, Clone)]
    struct TestEnvironment {
        map_id: u32,
        instance_id: u32,
        visibility_range: f32,
        visibility_override: Option<f32>,
        creature_sight_distance: Option<f32>,
        cinematic: bool,
        los: bool,
        los_calls: Cell<usize>,
        last_los: RefCell<Option<LosSnapshot>>,
        height: f32,
        floor: f32,
    }

    impl Default for TestEnvironment {
        fn default() -> Self {
            Self {
                map_id: 571,
                instance_id: 1,
                visibility_range: DEFAULT_VISIBILITY_DISTANCE,
                visibility_override: None,
                creature_sight_distance: None,
                cinematic: false,
                los: true,
                los_calls: Cell::new(0),
                last_los: RefCell::new(None),
                height: INVALID_HEIGHT,
                floor: INVALID_HEIGHT,
            }
        }
    }

    impl WorldObjectEnvironment for TestEnvironment {
        fn map_id(&self) -> u32 {
            self.map_id
        }

        fn instance_id(&self) -> u32 {
            self.instance_id
        }

        fn visibility_range(&self) -> f32 {
            self.visibility_range
        }

        fn visibility_override(&self, _object: &WorldObject) -> Option<f32> {
            self.visibility_override
        }

        fn creature_sight_distance(&self, _object: &WorldObject) -> Option<f32> {
            self.creature_sight_distance
        }

        fn player_on_cinematic(&self, _object: &WorldObject) -> bool {
            self.cinematic
        }

        fn line_of_sight(&self, query: LineOfSightQuery<'_>) -> bool {
            self.los_calls.set(self.los_calls.get() + 1);
            *self.last_los.borrow_mut() = Some(LosSnapshot {
                from: query.from,
                to: query.to,
                has_target: query.target.is_some(),
            });
            self.los
        }

        fn map_height(
            &self,
            _object: &WorldObject,
            _x: f32,
            _y: f32,
            z: f32,
            _query: WorldObjectHeightQuery,
        ) -> f32 {
            if (z - (10.0 + Z_OFFSET_FIND_HEIGHT)).abs() < 0.001 {
                self.height
            } else {
                INVALID_HEIGHT
            }
        }

        fn floor_z(&self, _object: &WorldObject, position: Position, _max_search_dist: f32) -> f32 {
            if (position.z - (10.0 + Z_OFFSET_FIND_HEIGHT)).abs() < 0.001 {
                self.floor
            } else {
                INVALID_HEIGHT
            }
        }
    }

    fn assert_position_close(actual: Position, expected: Position) {
        assert!(
            (actual.x - expected.x).abs() < 0.0001,
            "x: {actual:?} != {expected:?}"
        );
        assert!(
            (actual.y - expected.y).abs() < 0.0001,
            "y: {actual:?} != {expected:?}"
        );
        assert!(
            (actual.z - expected.z).abs() < 0.0001,
            "z: {actual:?} != {expected:?}"
        );
        assert!(
            (actual.orientation - expected.orientation).abs() < 0.0001,
            "orientation: {actual:?} != {expected:?}"
        );
    }

    fn prepare_in_world(object: &mut WorldObject, position: Position) {
        object.set_map(571, 1).unwrap();
        object.object_mut().add_to_world();
        object.relocate(position);
    }

    fn last_los(environment: &TestEnvironment) -> LosSnapshot {
        environment
            .last_los
            .borrow()
            .expect("LOS query should be captured")
    }

    #[test]
    fn world_location_defaults_match_cpp_world_location() {
        let location = WorldLocation::default();

        assert_eq!(location.map_id(), MAPID_INVALID);
        assert_eq!(location.position(), Position::ZERO);
    }

    #[test]
    fn world_object_constructor_matches_cpp_base_state() {
        let object = WorldObject::new(true, TypeId::Unit, TypeMask::UNIT);

        assert_eq!(object.object().type_id(), TypeId::Unit);
        assert_eq!(object.object().type_mask(), TypeMask::UNIT);
        assert_eq!(object.map_id(), MAPID_INVALID);
        assert_eq!(object.instance_id(), 0);
        assert!(!object.has_current_map());
        assert!(object.is_world_object());
        assert!(!object.is_active());
        assert!(!object.is_far_visible());
        assert_eq!(object.zone_id(), 0);
        assert_eq!(object.area_id(), 0);
        assert_eq!(object.db_phase(), 0);
        assert_eq!(object.combat_reach(), 0.0);
        assert_eq!(object.collision_height_like_cpp(), 0.0);
        assert_eq!(object.static_floor_z(), INVALID_HEIGHT);
        assert_eq!(object.current_cell(), None);
        assert!(object.smooth_phasing_like_cpp().is_none());
    }

    #[test]
    fn smooth_phasing_storage_matches_cpp_single_and_viewer_dependent_shape() {
        let seer = ObjectGuid::create_player(1, 1);
        let other_seer = ObjectGuid::create_player(1, 2);
        let replacement = ObjectGuid::new(1, 3);
        let mut smooth_phasing = SmoothPhasingLikeCpp::default();

        assert!(smooth_phasing.info_for_seer_like_cpp(seer).is_some());
        assert!(!smooth_phasing.is_being_replaced_for_seer_like_cpp(seer));

        smooth_phasing.set_single_info_like_cpp(SmoothPhasingInfoLikeCpp {
            replace_object: Some(replacement),
            ..SmoothPhasingInfoLikeCpp::default()
        });
        assert!(smooth_phasing.is_replacing_like_cpp(replacement));
        assert!(smooth_phasing.info_for_seer_like_cpp(other_seer).is_some());

        smooth_phasing.set_viewer_dependent_info_like_cpp(
            seer,
            SmoothPhasingInfoLikeCpp {
                replace_object: Some(replacement),
                ..SmoothPhasingInfoLikeCpp::default()
            },
        );
        assert!(smooth_phasing.is_being_replaced_for_seer_like_cpp(seer));
        assert!(smooth_phasing.info_for_seer_like_cpp(other_seer).is_none());

        smooth_phasing.disable_replacement_for_seer_like_cpp(seer);
        assert!(!smooth_phasing.is_being_replaced_for_seer_like_cpp(seer));

        smooth_phasing.clear_viewer_dependent_info_like_cpp(seer);
        assert!(smooth_phasing.info_for_seer_like_cpp(seer).is_none());
    }

    #[test]
    fn relocate_normalizes_orientation_like_cpp_position() {
        let mut object = WorldObject::new(false, TypeId::GameObject, TypeMask::GAME_OBJECT);

        object.world_relocate(571, Position::new(1.0, 2.0, 3.0, -1.0));

        assert_eq!(object.map_id(), 571);
        assert!((object.position().orientation - (TAU - 1.0)).abs() < 0.0001);
        assert_eq!(object.object().map_id(), Some(571));
    }

    #[test]
    fn set_map_and_reset_map_follow_cpp_binding_rules() {
        let mut object = WorldObject::new(false, TypeId::Corpse, TypeMask::CORPSE);

        assert_eq!(object.set_map(1, 10), Ok(()));
        assert_eq!(object.map_id(), 1);
        assert_eq!(object.instance_id(), 10);
        assert!(object.has_current_map());
        assert_eq!(object.set_map(1, 10), Ok(()));
        assert!(matches!(
            object.set_map(2, 10),
            Err(MapBindingError::AlreadyBound { .. })
        ));

        object.object_mut().add_to_world();
        assert_eq!(object.reset_map(), Err(MapBindingError::ObjectInWorld));
        object.object_mut().remove_from_world();

        assert_eq!(object.reset_map(), Ok(()));
        assert!(!object.has_current_map());
        assert_eq!(object.map_id(), 1);
        assert_eq!(object.instance_id(), 10);
    }

    #[test]
    fn distance_helpers_subtract_combat_reach_and_clamp_zero() {
        let mut a = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut b = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        a.relocate(Position::xyz(0.0, 0.0, 0.0));
        b.relocate(Position::xyz(3.0, 4.0, 12.0));
        a.set_combat_reach(1.5);
        b.set_combat_reach(2.0);

        assert!((a.exact_distance(&b) - 13.0).abs() < 0.001);
        assert!((a.distance(&b) - 9.5).abs() < 0.001);
        assert!((a.distance_2d(&b) - 1.5).abs() < 0.001);
        assert!((a.distance_z(&b) - 8.5).abs() < 0.001);

        b.relocate(Position::xyz(1.0, 0.0, 0.0));
        assert_eq!(a.distance(&b), 0.0);
    }

    #[test]
    fn within_dist_uses_cpp_strict_less_than_and_radius_options() {
        let mut a = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut b = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        a.relocate(Position::xyz(0.0, 0.0, 0.0));
        b.relocate(Position::xyz(3.0, 4.0, 0.0));

        assert!(!a.is_within_dist(&b, 5.0, true, false, false));
        assert!(a.is_within_dist(&b, 5.01, true, false, false));

        a.set_combat_reach(1.0);
        b.set_combat_reach(1.0);
        assert!(a.is_within_dist(&b, 3.1, true, true, true));
        assert!(!a.is_within_dist(&b, 3.1, true, false, false));
    }

    #[test]
    fn within_dist_in_map_requires_world_map_and_phase() {
        let mut a = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut b = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        a.set_map(530, 1).unwrap();
        b.set_map(530, 1).unwrap();
        a.object_mut().add_to_world();
        b.object_mut().add_to_world();
        a.relocate(Position::xyz(0.0, 0.0, 0.0));
        b.relocate(Position::xyz(1.0, 0.0, 0.0));

        a.phase_shift_mut().insert(10);
        b.phase_shift_mut().insert(20);
        assert!(!a.is_within_dist_in_map(&b, 2.0, true));

        b.phase_shift_mut().insert(10);
        assert!(a.is_within_dist_in_map(&b, 2.0, true));
    }

    #[test]
    fn phase_shift_visible_map_ids_reference_count_like_cpp() {
        let mut phase_shift = PhaseShift::default();

        assert!(phase_shift.add_visible_map_id_like_cpp(609, 1));
        assert!(!phase_shift.add_visible_map_id_like_cpp(609, 1));
        assert!(phase_shift.has_visible_map_id_like_cpp(609));
        assert_eq!(phase_shift.visible_map_id_count_like_cpp(), 1);
        assert_eq!(
            phase_shift
                .visible_map_id_ref_like_cpp(609)
                .map(VisibleMapIdRef::references),
            Some(2)
        );
        assert_eq!(
            phase_shift.visible_map_ids_like_cpp().collect::<Vec<_>>(),
            vec![609]
        );

        assert!(!phase_shift.remove_visible_map_id_like_cpp(609));
        assert_eq!(
            phase_shift
                .visible_map_id_ref_like_cpp(609)
                .map(VisibleMapIdRef::references),
            Some(1)
        );
        assert!(phase_shift.remove_visible_map_id_like_cpp(609));
        assert!(!phase_shift.has_visible_map_id_like_cpp(609));
        assert!(!phase_shift.remove_visible_map_id_like_cpp(609));
    }

    #[test]
    fn phase_shift_can_remove_all_phase_and_visible_map_references_like_cpp() {
        let mut phase_shift = PhaseShift::default();
        phase_shift.add_phase_like_cpp(20, PhaseFlags::PERSONAL, 2);
        phase_shift.add_visible_map_id_like_cpp(609, 2);

        let removed_phase = phase_shift
            .remove_phase_all_references_like_cpp(20)
            .expect("phase should be removed");
        assert_eq!(removed_phase.references(), 2);
        assert!(!phase_shift.has_phase_like_cpp(20));
        assert!(!phase_shift.has_personal_phase_like_cpp());

        let removed_visible_map = phase_shift
            .remove_visible_map_id_all_references_like_cpp(609)
            .expect("visible map should be removed");
        assert_eq!(removed_visible_map.references(), 2);
        assert!(!phase_shift.has_visible_map_id_like_cpp(609));
    }

    #[test]
    fn phase_shift_clear_removes_visible_map_ids_like_cpp() {
        let mut phase_shift = PhaseShift::from_phases([10]);
        phase_shift.add_visible_map_id_like_cpp(609, 1);
        phase_shift.add_ui_map_phase_id_like_cpp(42, 1);

        phase_shift.clear();

        assert!(!phase_shift.has_visible_map_id_like_cpp(609));
        assert!(phase_shift.visible_map_ids_like_cpp().next().is_none());
        assert!(!phase_shift.has_ui_map_phase_id_like_cpp(42));
        assert!(phase_shift.ui_map_phase_ids_like_cpp().next().is_none());
        assert!(phase_shift.can_see(&PhaseShift::default()));
        assert!(!phase_shift.can_see(&PhaseShift::from_phases([20])));
    }

    #[test]
    fn phase_shift_ui_map_phase_ids_reference_count_like_cpp() {
        let mut phase_shift = PhaseShift::default();

        assert!(phase_shift.add_ui_map_phase_id_like_cpp(42, 1));
        assert!(!phase_shift.add_ui_map_phase_id_like_cpp(42, 1));
        assert!(phase_shift.has_ui_map_phase_id_like_cpp(42));
        assert_eq!(
            phase_shift
                .ui_map_phase_id_ref_like_cpp(42)
                .map(UiMapPhaseIdRef::references),
            Some(2)
        );
        assert_eq!(
            phase_shift.ui_map_phase_ids_like_cpp().collect::<Vec<_>>(),
            vec![42]
        );

        assert!(!phase_shift.remove_ui_map_phase_id_like_cpp(42));
        assert_eq!(
            phase_shift
                .ui_map_phase_id_ref_like_cpp(42)
                .map(UiMapPhaseIdRef::references),
            Some(1)
        );
        assert!(phase_shift.remove_ui_map_phase_id_like_cpp(42));
        assert!(!phase_shift.has_ui_map_phase_id_like_cpp(42));
        assert!(!phase_shift.remove_ui_map_phase_id_like_cpp(42));
    }

    #[test]
    fn angle_helpers_match_cpp_position_relative_angle_semantics() {
        let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        object.relocate(Position::new(0.0, 0.0, 0.0, PI / 2.0));

        assert!(
            (object.absolute_angle_to_position(Position::xyz(1.0, 0.0, 0.0)) - 0.0).abs() < 0.0001
        );
        assert!((object.to_absolute_angle(PI / 2.0) - PI).abs() < 0.0001);
        assert!((object.to_relative_angle(0.0) - (TAU - PI / 2.0)).abs() < 0.0001);
        assert!(
            (object.relative_angle_to_position(Position::xyz(0.0, 1.0, 0.0)) - 0.0).abs() < 0.0001
        );
    }

    #[test]
    fn arc_front_back_and_line_helpers_match_cpp_boundaries() {
        let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut front = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut side = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut back = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        source.relocate(Position::new(0.0, 0.0, 0.0, 0.0));
        front.relocate(Position::xyz(1.0, 0.0, 0.0));
        side.relocate(Position::xyz(0.0, 1.0, 0.0));
        back.relocate(Position::xyz(-1.0, 0.0, 0.0));

        assert!(source.has_in_arc(PI, &source, 2.0));
        assert!(source.is_in_front(&front, PI));
        assert!(source.is_in_front(&side, PI));
        assert!(!source.is_in_front(&back, PI));
        assert!(source.is_in_back(&back, PI));
        assert!(!source.is_in_back(&front, PI));

        front.relocate(Position::xyz(10.0, 1.0, 0.0));
        assert!(source.has_in_line(&front, 2.0));
        front.relocate(Position::xyz(10.0, 3.0, 0.0));
        assert!(!source.has_in_line(&front, 2.0));
    }

    #[test]
    fn box_and_double_vertical_cylinder_match_cpp_geometry() {
        let mut object = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let center = Position::new(0.0, 0.0, 5.0, PI / 2.0);

        object.relocate(Position::xyz(0.5, 1.5, 5.5));
        assert!(object.is_within_box(center, 2.0, 1.0, 1.0));

        object.relocate(Position::xyz(1.5, 1.5, 5.5));
        assert!(!object.is_within_box(center, 2.0, 1.0, 1.0));

        object.relocate(Position::xyz(3.0, 4.0, 8.0));
        assert!(!object.is_within_double_vertical_cylinder(center, 5.0, 3.0));
        object.relocate(Position::xyz(3.0, 3.9, 8.0));
        assert!(object.is_within_double_vertical_cylinder(center, 5.0, 3.0));
    }

    #[test]
    fn world_object_visibility_range_uses_override_far_visible_then_map_range() {
        let environment = TestEnvironment {
            visibility_range: 123.0,
            ..TestEnvironment::default()
        };
        let mut object = WorldObject::new(false, TypeId::GameObject, TypeMask::GAME_OBJECT);
        assert_eq!(object.get_visibility_range(&environment), 123.0);

        object.set_far_visible(true);
        assert_eq!(
            object.get_visibility_range(&environment),
            MAX_VISIBILITY_DISTANCE
        );

        let environment = TestEnvironment {
            visibility_override: Some(222.0),
            ..environment
        };
        assert_eq!(object.get_visibility_range(&environment), 222.0);

        let mut player = WorldObject::new(false, TypeId::Player, TypeMask::PLAYER | TypeMask::UNIT);
        player.set_far_visible(true);
        assert_eq!(player.get_visibility_range(&environment), 123.0);
    }

    #[test]
    fn world_object_sight_range_matches_representable_cpp_cases() {
        let environment = TestEnvironment {
            visibility_range: 140.0,
            creature_sight_distance: Some(80.0),
            ..TestEnvironment::default()
        };
        let player = WorldObject::new(false, TypeId::Player, TypeMask::PLAYER | TypeMask::UNIT);
        let creature = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let unit = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut dyn_object =
            WorldObject::new(false, TypeId::DynamicObject, TypeMask::DYNAMIC_OBJECT);
        dyn_object.set_active(true);

        assert_eq!(player.get_sight_range(None, &environment), 140.0);
        assert_eq!(creature.get_sight_range(None, &environment), 80.0);
        assert_eq!(
            unit.get_sight_range(None, &TestEnvironment::default()),
            SIGHT_RANGE_UNIT
        );
        assert_eq!(dyn_object.get_sight_range(None, &environment), 140.0);

        let mut target = WorldObject::new(false, TypeId::GameObject, TypeMask::GAME_OBJECT);
        target.set_far_visible(true);
        assert_eq!(
            player.get_sight_range(Some(&target), &environment),
            MAX_VISIBILITY_DISTANCE
        );

        let cinematic = TestEnvironment {
            visibility_range: 140.0,
            cinematic: true,
            ..TestEnvironment::default()
        };
        assert_eq!(
            player.get_sight_range(None, &cinematic),
            DEFAULT_VISIBILITY_INSTANCE
        );
    }

    #[test]
    fn world_object_los_prefilters_map_not_phase_and_uses_partial_raw_endpoint_bridge() {
        let environment = TestEnvironment {
            los: false,
            ..TestEnvironment::default()
        };
        let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut target = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);

        assert!(source.is_within_los(
            Position::xyz(1.0, 0.0, 0.0),
            &environment,
            LineOfSightOptions::default()
        ));
        assert_eq!(environment.los_calls.get(), 0);

        source.set_map(571, 1).unwrap();
        target.set_map(571, 2).unwrap();
        source.object_mut().add_to_world();
        target.object_mut().add_to_world();
        assert!(!source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));
        assert_eq!(environment.los_calls.get(), 0);

        target.object_mut().remove_from_world();
        target.reset_map().unwrap();
        target.set_map(571, 1).unwrap();
        target.object_mut().add_to_world();
        source.phase_shift_mut().insert(10);
        target.phase_shift_mut().insert(20);
        assert!(!source.in_same_phase(&target));
        assert!(!source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));
        assert_eq!(environment.los_calls.get(), 1);

        let environment = TestEnvironment {
            los: true,
            ..environment
        };
        assert!(source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));
        assert_eq!(environment.los_calls.get(), 2);
    }

    #[test]
    fn base_world_object_default_los_endpoints_remain_raw_without_adjustments() {
        let environment = TestEnvironment::default();
        let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        prepare_in_world(&mut source, Position::new(1.0, 2.0, 3.0, 0.25));

        assert_eq!(source.collision_height_like_cpp(), 0.0);
        assert_eq!(source.combat_reach(), 0.0);
        assert!(source.is_within_los(
            Position::new(4.0, 6.0, 8.0, 1.0),
            &environment,
            LineOfSightOptions::default()
        ));

        let query = last_los(&environment);
        assert!(!query.has_target);
        assert_position_close(query.from.position, Position::new(1.0, 2.0, 3.0, 0.0));
        assert_position_close(query.to.position, Position::new(4.0, 6.0, 8.0, 1.0));
        assert!(!query.from.collision_height_adjusted);
        assert!(!query.from.hit_sphere_adjusted);
        assert!(!query.to.collision_height_adjusted);
        assert!(!query.to.hit_sphere_adjusted);
    }

    #[test]
    fn point_los_non_player_uses_collision_height_and_hit_sphere_like_cpp() {
        let environment = TestEnvironment::default();
        let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        prepare_in_world(&mut source, Position::xyz(0.0, 0.0, 0.0));
        source.set_collision_height_like_cpp(2.0);
        source.set_combat_reach(1.0);

        assert!(source.is_within_los(
            Position::xyz(4.0, 0.0, 0.0),
            &environment,
            LineOfSightOptions::default()
        ));

        let query = last_los(&environment);
        assert_position_close(query.to.position, Position::xyz(4.0, 0.0, 2.0));
        assert!(query.to.collision_height_adjusted);
        assert!(!query.to.hit_sphere_adjusted);
        assert_position_close(query.from.position, Position::xyz(1.0, 0.0, 2.0));
        assert!(query.from.collision_height_adjusted);
        assert!(query.from.hit_sphere_adjusted);
    }

    #[test]
    fn los_in_map_non_player_endpoints_use_opposite_collision_height_destinations() {
        let environment = TestEnvironment::default();
        let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut target = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        prepare_in_world(&mut source, Position::xyz(0.0, 0.0, 0.0));
        prepare_in_world(&mut target, Position::xyz(10.0, 0.0, 0.0));
        source.set_collision_height_like_cpp(2.0);
        source.set_combat_reach(1.0);
        target.set_collision_height_like_cpp(4.0);
        target.set_combat_reach(2.0);

        assert!(source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));

        let query = last_los(&environment);
        assert!(query.has_target);
        assert_position_close(
            query.from.position,
            Position::xyz(0.9805807, 0.0, 2.1961162),
        );
        assert!(query.from.collision_height_adjusted);
        assert!(query.from.hit_sphere_adjusted);
        assert_position_close(
            query.to.position,
            Position::new(8.038839, 0.0, 3.6077678, PI),
        );
        assert!(query.to.collision_height_adjusted);
        assert!(query.to.hit_sphere_adjusted);
    }

    #[test]
    fn los_in_map_target_player_uses_source_collision_height_cpp_quirk() {
        let environment = TestEnvironment::default();
        let mut source = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        let mut target = WorldObject::new(false, TypeId::Player, TypeMask::PLAYER | TypeMask::UNIT);
        prepare_in_world(&mut source, Position::xyz(0.0, 0.0, 0.0));
        prepare_in_world(&mut target, Position::xyz(5.0, 0.0, 10.0));
        source.set_collision_height_like_cpp(2.0);
        source.set_combat_reach(0.0);
        target.set_collision_height_like_cpp(7.0);

        assert!(source.is_within_los_in_map(&target, &environment, LineOfSightOptions::default()));

        let query = last_los(&environment);
        assert_position_close(query.to.position, Position::xyz(5.0, 0.0, 12.0));
        assert!(query.to.collision_height_adjusted);
        assert!(!query.to.hit_sphere_adjusted);
        assert_position_close(query.from.position, Position::xyz(0.0, 0.0, 2.0));
        assert!(query.from.collision_height_adjusted);
        assert!(!query.from.hit_sphere_adjusted);
    }

    #[test]
    fn player_source_los_endpoint_uses_position_plus_collision_height_not_hit_sphere() {
        let environment = TestEnvironment::default();
        let mut source = WorldObject::new(false, TypeId::Player, TypeMask::PLAYER | TypeMask::UNIT);
        prepare_in_world(&mut source, Position::xyz(1.0, 2.0, 3.0));
        source.set_collision_height_like_cpp(2.5);
        source.set_combat_reach(3.0);

        assert!(source.is_within_los(
            Position::xyz(11.0, 2.0, 3.0),
            &environment,
            LineOfSightOptions::default()
        ));

        let query = last_los(&environment);
        assert_position_close(query.from.position, Position::xyz(1.0, 2.0, 5.5));
        assert!(query.from.collision_height_adjusted);
        assert!(!query.from.hit_sphere_adjusted);
        assert_position_close(query.to.position, Position::xyz(11.0, 2.0, 5.5));
        assert!(query.to.collision_height_adjusted);
        assert!(!query.to.hit_sphere_adjusted);
    }

    #[test]
    fn world_object_height_floor_and_ground_update_use_map_bridge() {
        let environment = TestEnvironment {
            height: 20.0,
            floor: 30.0,
            ..TestEnvironment::default()
        };
        let mut unit = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);
        unit.set_map(571, 1).unwrap();
        unit.object_mut().add_to_world();
        unit.relocate(Position::xyz(1.0, 2.0, 10.0));

        assert_eq!(
            unit.get_map_height(
                &environment,
                1.0,
                2.0,
                10.0,
                WorldObjectHeightQuery::default()
            ),
            20.0
        );
        assert_eq!(
            unit.update_ground_position_z(&environment, 1.0, 2.0, 10.0, 1.25),
            21.25
        );

        unit.set_static_floor_z(25.0);
        assert_eq!(unit.get_floor_z(&environment), 30.0);

        let environment = TestEnvironment {
            height: INVALID_HEIGHT,
            floor: INVALID_HEIGHT,
            ..environment
        };
        assert_eq!(
            unit.update_ground_position_z(&environment, 1.0, 2.0, 10.0, 1.25),
            10.0
        );
        assert_eq!(unit.get_floor_z(&environment), 25.0);
    }

    #[test]
    fn world_object_transport_relocation_roundtrips_offset_and_global_position() {
        let transport = Position::new(100.0, 200.0, 10.0, PI / 2.0);
        let offset = Position::new(4.0, -2.0, 3.0, PI / 4.0);
        let mut passenger = WorldObject::new(false, TypeId::Unit, TypeMask::UNIT);

        let global = passenger.relocate_on_transport(transport, offset);
        assert_eq!(passenger.position(), global);

        let roundtrip = passenger.transport_offset_from_position(transport);
        assert!((roundtrip.x - offset.x).abs() < 0.0001);
        assert!((roundtrip.y - offset.y).abs() < 0.0001);
        assert!((roundtrip.z - offset.z).abs() < 0.0001);
        assert!((roundtrip.orientation - offset.orientation).abs() < 0.0001);
    }

    #[test]
    fn phase_shift_phase_refs_update_flags_like_cpp() {
        let mut phase_shift = PhaseShift::default();

        assert!(
            phase_shift
                .flags_like_cpp()
                .contains(PhaseShiftFlags::UNPHASED)
        );
        assert!(phase_shift.add_phase_like_cpp(10, PhaseFlags::NONE, 1));
        assert!(phase_shift.has_phase_like_cpp(10));
        assert!(
            !phase_shift
                .flags_like_cpp()
                .contains(PhaseShiftFlags::UNPHASED)
        );
        assert_eq!(
            phase_shift.phase_ref_like_cpp(10).map(PhaseRef::references),
            Some(1)
        );

        assert!(!phase_shift.add_phase_like_cpp(10, PhaseFlags::NONE, 1));
        assert_eq!(
            phase_shift.phase_ref_like_cpp(10).map(PhaseRef::references),
            Some(2)
        );
        assert!(!phase_shift.remove_phase_like_cpp(10));
        assert!(phase_shift.remove_phase_like_cpp(10));
        assert!(
            phase_shift
                .flags_like_cpp()
                .contains(PhaseShiftFlags::UNPHASED)
        );
    }

    #[test]
    fn phase_shift_can_see_honors_always_visible_like_cpp() {
        let viewer = PhaseShift::from_phases([10]);
        let mut target = PhaseShift::from_phases([20]);

        assert!(!viewer.can_see(&target));
        target.set_always_visible_like_cpp(true);
        assert!(viewer.can_see(&target));
    }

    #[test]
    fn phase_shift_can_see_requires_matching_personal_guid_like_cpp() {
        let personal_owner = ObjectGuid::create_player(1, 42);
        let other_owner = ObjectGuid::create_player(1, 43);
        let mut viewer = PhaseShift::default();
        let mut target = PhaseShift::default();

        viewer.add_phase_like_cpp(10, PhaseFlags::PERSONAL, 1);
        target.add_phase_like_cpp(10, PhaseFlags::PERSONAL, 1);
        viewer.set_personal_guid_like_cpp(personal_owner);
        target.set_personal_guid_like_cpp(other_owner);
        assert!(viewer.has_personal_phase_like_cpp());
        assert!(!viewer.can_see(&target));

        target.set_personal_guid_like_cpp(personal_owner);
        assert!(viewer.can_see(&target));
    }

    #[test]
    fn phase_shift_can_see_honors_inverse_like_cpp() {
        let normal = PhaseShift::from_phases([10]);
        let mut inverse_same = PhaseShift::from_phases([10]);
        inverse_same.set_inversed_like_cpp(true);
        assert!(!normal.can_see(&inverse_same));

        let mut inverse_other = PhaseShift::from_phases([20]);
        inverse_other.set_inversed_like_cpp(true);
        assert!(normal.can_see(&inverse_other));

        let mut unphased = PhaseShift::default();
        let mut inverse_unphased = PhaseShift::default();
        inverse_unphased.set_inversed_like_cpp(true);
        assert!(!unphased.can_see(&inverse_unphased));

        unphased.set_always_visible_like_cpp(true);
        assert!(unphased.can_see(&inverse_unphased));
    }
}
