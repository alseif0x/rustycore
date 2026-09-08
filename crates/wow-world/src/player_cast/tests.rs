use super::*;
use std::cell::Cell;

struct Fixture {
    spell: SpellInfo,
    remaining: (u32, u32),
    known: bool,
    power: bool,
    admits_install: bool,
    visual_available: bool,
    pending: Option<PendingSpellCastRequestLikeCpp>,
    active: Option<SpellCastState>,
    allocated: Cell<u32>,
    events: Vec<&'static str>,
    failures: Vec<i32>,
}

impl Fixture {
    fn new(cast_time_ms: u32) -> Self {
        Self {
            spell: SpellInfo {
                spell_id: 133,
                cast_time_ms,
                cooldown_ms: 1500,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                power_costs: Vec::new(),
                effects: Vec::new(),
            },
            remaining: (0, 0),
            known: true,
            power: true,
            admits_install: true,
            visual_available: true,
            pending: None,
            active: None,
            allocated: Cell::new(0),
            events: Vec::new(),
            failures: Vec::new(),
        }
    }

    fn request() -> PendingSpellCastRequestLikeCpp {
        PendingSpellCastRequestLikeCpp {
            cast_id: ObjectGuid::EMPTY,
            spell_id: 133,
            casting_unit_guid: ObjectGuid::EMPTY,
            target_guid: ObjectGuid::EMPTY,
            target_data: Default::default(),
            spell_visual: SpellCastVisualLikeCpp {
                spell_visual_id: 999,
                script_visual_id: 0,
            },
            metadata: Default::default(),
        }
    }
}

impl Runtime for Fixture {
    fn spell(&self, id: i32) -> Option<SpellInfo> {
        (id == 133).then(|| self.spell.clone())
    }
    fn known(&self, _: i32) -> bool {
        self.known
    }
    fn passive(&self, _: i32) -> bool {
        false
    }
    fn resolve_override(&self, original: &SpellInfo) -> SpellInfo {
        original.clone()
    }
    fn remaining(&self, _: &SpellInfo) -> Option<(u32, u32)> {
        Some(self.remaining)
    }
    fn replace_pending(&mut self, request: PendingSpellCastRequestLikeCpp) {
        if self.pending.replace(request).is_some() {
            self.events.push("cancel-old");
        }
    }
    fn failure(&mut self, _: ObjectGuid, _: i32, _: SpellCastVisualLikeCpp, reason: i32) {
        self.events.push("failure");
        self.failures.push(reason);
    }
    fn allocate(&self, _: i32) -> Option<(ObjectGuid, Option<u64>)> {
        self.allocated.set(self.allocated.get() + 1);
        Some((ObjectGuid::EMPTY, Some(7)))
    }
    fn visual(&self, _: &SpellInfo) -> Option<SpellCastVisualLikeCpp> {
        self.visual_available.then(|| SpellCastVisualLikeCpp {
            spell_visual_id: 123,
            script_visual_id: 0,
        })
    }
    fn prepare_mapping(&mut self, _: ObjectGuid, _: ObjectGuid) {
        self.events.push("prepare");
    }
    fn disabled(&self, _: i32) -> bool {
        false
    }
    fn on_cooldown(&self, _: &SpellInfo) -> Option<bool> {
        Some(false)
    }
    fn check_power(&mut self, _: &SpellInfo, _: ObjectGuid, _: &SpellCastVisualLikeCpp) -> bool {
        self.events.push("power");
        self.power
    }
    fn check_preconditions(
        &mut self,
        _: &SpellInfo,
        _: ObjectGuid,
        _: &SpellCastVisualLikeCpp,
        _: wow_entities::SpellCastMetadata,
    ) -> bool {
        true
    }
    fn install(&mut self, cast: SpellCastState) -> bool {
        self.events.push("install");
        if !self.admits_install {
            return false;
        }
        self.active = Some(cast);
        true
    }
    fn start(&mut self, _: &SpellCastState, _: &SpellInfo) {
        self.events.push("start");
    }
}

#[test]
fn instant_and_timed_requests_share_mapping_validation_install_and_start() {
    for time in [0, 1500] {
        let mut runtime = Fixture::new(time);
        assert_eq!(prepare(&mut runtime, Fixture::request()), time == 0);
        assert_eq!(
            runtime.events,
            ["prepare", "power", "install", "start"]
        );
        let cast = runtime.active.unwrap();
        assert_eq!(cast.cast_time_ms, time);
        assert_eq!(cast.spell_visual.spell_visual_id, 123);
        assert_eq!(cast.metadata.prepared_residence_revision, Some(7));
        assert_eq!(cast.metadata.original_cast_id, ObjectGuid::EMPTY);
        assert_eq!(cast.metadata.client_cast_id, Some(ObjectGuid::EMPTY));
    }
}

#[test]
fn queue_boundary_is_inclusive_without_allocating_a_server_identity() {
    for remaining in [(400, 0), (0, 400), (400, 400)] {
        let mut runtime = Fixture::new(1500);
        runtime.remaining = remaining;
        assert!(!request(&mut runtime, Fixture::request()));
        assert!(runtime.pending.is_some());
        assert_eq!(runtime.allocated.get(), 0);
    }
    for remaining in [(401, 0), (0, 401)] {
        let mut runtime = Fixture::new(1500);
        runtime.remaining = remaining;
        assert!(!request(&mut runtime, Fixture::request()));
        assert!(runtime.pending.is_none());
        assert_eq!(runtime.failures, [SpellCastResult::SpellInProgress as i32]);
    }
}

#[test]
fn queued_request_revalidates_knowledge_before_mapping_or_installing() {
    let mut runtime = Fixture::new(1500);
    runtime.remaining = (300, 0);
    request(&mut runtime, Fixture::request());
    runtime.known = false;
    let request = runtime.pending.take().unwrap();
    assert!(!prepare(&mut runtime, request));
    assert_eq!(runtime.allocated.get(), 0);
    assert_eq!(runtime.events, ["failure"]);
    assert_eq!(runtime.failures, [SpellCastResult::DontReport as i32]);
    assert!(runtime.active.is_none());
}

#[test]
fn failed_power_retains_mapping_but_cannot_start_or_install() {
    let mut runtime = Fixture::new(0);
    runtime.power = false;
    assert!(!prepare(&mut runtime, Fixture::request()));
    assert_eq!(runtime.events, ["prepare", "power"]);
    assert_eq!(runtime.allocated.get(), 1);
    assert!(runtime.active.is_none());
}

#[test]
fn rejected_residence_install_cannot_publish_start() {
    let mut runtime = Fixture::new(0);
    runtime.admits_install = false;
    assert!(!prepare(&mut runtime, Fixture::request()));
    assert_eq!(runtime.events, ["prepare", "power", "install", "failure"]);
    assert!(runtime.active.is_none());
}

#[test]
fn immediate_request_also_retires_an_older_pending_request() {
    let mut runtime = Fixture::new(0);
    runtime.pending = Some(Fixture::request());
    assert!(request(&mut runtime, Fixture::request()));
    assert_eq!(runtime.events, ["cancel-old"]);
}

#[test]
fn unresolvable_visual_reports_a_cancellation_instead_of_dropping_the_request() {
    let mut runtime = Fixture::new(1500);
    runtime.visual_available = false;
    assert!(!prepare(&mut runtime, Fixture::request()));
    // C++ `GetCastSpellXSpellVisualId` always resolves; an unrepresented
    // selection must not leave the client waiting on a silent request.
    assert_eq!(runtime.events, ["failure"]);
    assert_eq!(runtime.failures, [SpellCastResult::DontReport as i32]);
    assert_eq!(runtime.allocated.get(), 0);
}
