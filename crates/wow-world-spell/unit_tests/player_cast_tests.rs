use super::*;
use std::cell::Cell;
use wow_entities::SpellCastMetadata;

struct Fixture {
    spell: SpellInfo,
    remaining: (u32, u32),
    known: bool,
    power: bool,
    admits_install: bool,
    visual_available: bool,
    native_gcd: (u32, bool),
    pending: Option<PendingSpellCastRequestLikeCpp>,
    active: Option<SpellCastState>,
    allocated: Cell<u32>,
    events: Vec<&'static str>,
    failures: Vec<i32>,
    spell_catalog_failure: Option<&'static str>,
    normal_cost_outcome: Result<bool, &'static str>,
    precondition_outcome: Result<bool, &'static str>,
    caster_death_outcome: Result<bool, &'static str>,
    client_cooldown: Option<bool>,
    server_normal_cooldown: Option<bool>,
    native_gcd_available: bool,
    caster_death_checks: Cell<u32>,
    client_cooldown_checks: Cell<u32>,
    server_cooldown_checks: Cell<u32>,
    native_gcd_checks: Cell<u32>,
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
            native_gcd: (133, false),
            pending: None,
            active: None,
            allocated: Cell::new(0),
            events: Vec::new(),
            failures: Vec::new(),
            spell_catalog_failure: None,
            normal_cost_outcome: Ok(true),
            precondition_outcome: Ok(true),
            caster_death_outcome: Ok(true),
            client_cooldown: Some(false),
            server_normal_cooldown: Some(false),
            native_gcd_available: true,
            caster_death_checks: Cell::new(0),
            client_cooldown_checks: Cell::new(0),
            server_cooldown_checks: Cell::new(0),
            native_gcd_checks: Cell::new(0),
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
    fn spell_for_difficulty(&self, id: i32, requested_difficulty: u8) -> Option<(SpellInfo, u8)> {
        (id == 133).then(|| (self.spell.clone(), requested_difficulty))
    }
    fn spell_for_difficulty_outcome(
        &self,
        id: i32,
        requested_difficulty: u8,
    ) -> Result<Option<(SpellInfo, u8)>, &'static str> {
        if let Some(reason) = self.spell_catalog_failure {
            return Err(reason);
        }
        Ok(self.spell_for_difficulty(id, requested_difficulty))
    }
    fn server_normal_busy(&mut self) -> Option<bool> {
        self.events.push("busy");
        Some(
            self.active
                .as_ref()
                .is_some_and(|active| active.cast_time_ms != 0),
        )
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
    fn check_caster_death_outcome(
        &mut self,
        _: &SpellInfo,
        _: ObjectGuid,
        _: &SpellCastVisualLikeCpp,
        _: &mut wow_entities::SpellCastMetadata,
    ) -> Result<bool, &'static str> {
        self.events.push("caster-death");
        self.caster_death_checks
            .set(self.caster_death_checks.get() + 1);
        let outcome = self.caster_death_outcome;
        if outcome == Ok(false) {
            self.events.push("failure");
            self.failures.push(SpellCastResult::CasterDead as i32);
        }
        outcome
    }
    fn on_cooldown(&self, _: &SpellInfo) -> Option<bool> {
        self.client_cooldown_checks
            .set(self.client_cooldown_checks.get() + 1);
        self.client_cooldown
    }
    fn server_normal_on_cooldown(
        &self,
        _: &SpellInfo,
        _: u8,
        _: wow_entities::SpellCastMetadata,
    ) -> Option<bool> {
        self.server_cooldown_checks
            .set(self.server_cooldown_checks.get() + 1);
        self.server_normal_cooldown
    }
    fn server_normal_global_cooldown_state(&self, _: &SpellInfo, _: u8) -> Option<(u32, bool)> {
        self.native_gcd_checks.set(self.native_gcd_checks.get() + 1);
        self.native_gcd_available.then_some(self.native_gcd)
    }
    fn server_normal_cooldown_at_cast_start(
        &mut self,
        _: &SpellInfo,
        _: u8,
        _: wow_entities::SpellCastMetadata,
    ) -> Option<()> {
        self.events.push("cast-start-cooldown");
        Some(())
    }
    fn check_power(&mut self, _: &SpellInfo, _: ObjectGuid, _: &SpellCastVisualLikeCpp) -> bool {
        self.events.push("power");
        self.power
    }
    fn check_server_normal_costs(
        &mut self,
        _: &SpellInfo,
        _: ObjectGuid,
        _: &SpellCastVisualLikeCpp,
        _: wow_entities::SpellCastMetadata,
    ) -> bool {
        self.events.push("normal-costs");
        true
    }
    fn check_server_normal_costs_outcome(
        &mut self,
        _: &SpellInfo,
        _: ObjectGuid,
        _: &SpellCastVisualLikeCpp,
        _: wow_entities::SpellCastMetadata,
    ) -> Result<bool, &'static str> {
        self.events.push("normal-costs");
        self.normal_cost_outcome
    }
    fn server_normal_effects_available(
        &mut self,
        spell: &SpellInfo,
        target_guid: ObjectGuid,
        _metadata: wow_entities::SpellCastMetadata,
    ) -> Result<(), &'static str> {
        self.events.push("effects-available");
        crate::execution_session::validate_spell_effect_routes_like_cpp(
            spell,
            spell.effect_type,
            ObjectGuid::EMPTY,
            target_guid,
            false,
        )
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
    fn check_preconditions_outcome(
        &mut self,
        _: &SpellInfo,
        _: ObjectGuid,
        _: &SpellCastVisualLikeCpp,
        _: wow_entities::SpellCastMetadata,
    ) -> Result<bool, &'static str> {
        self.precondition_outcome
    }
    fn install(&mut self, cast: SpellCastState) -> bool {
        self.events.push("install");
        if !self.admits_install {
            return false;
        }
        self.active = Some(cast);
        true
    }
    fn install_server_normal(&mut self, cast: SpellCastState) -> bool {
        self.events.push("install-server-normal");
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.cast_time_ms != 0)
            || !self.admits_install
        {
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
fn client_prepare_checks_dead_caster_before_cooldown() {
    let mut runtime = Fixture::new(0);
    runtime.caster_death_outcome = Ok(false);
    runtime.client_cooldown = Some(true);

    assert!(!prepare(&mut runtime, Fixture::request()));
    assert_eq!(runtime.failures, [SpellCastResult::CasterDead as i32]);
    assert_eq!(runtime.client_cooldown_checks.get(), 0);
    assert_eq!(runtime.events, ["prepare", "caster-death", "failure"]);
}

#[test]
fn normal_prepare_dead_caster_precedes_active_spell_cooldown() {
    let mut runtime = Fixture::new(0);
    runtime.caster_death_outcome = Ok(false);
    runtime.server_normal_cooldown = Some(true);

    assert_eq!(
        prepare_server_normal(&mut runtime, 133, 0, ObjectGuid::EMPTY, Default::default()),
        ServerNormalPrepareOutcomeLikeCpp::Rejected
    );
    assert_eq!(runtime.failures, [SpellCastResult::CasterDead as i32]);
    assert_eq!(runtime.native_gcd_checks.get(), 0);
    assert_eq!(runtime.server_cooldown_checks.get(), 0);
    assert_eq!(runtime.events, ["busy", "caster-death", "failure"]);
}

#[test]
fn normal_prepare_busy_caster_check_keeps_spell_in_progress_priority() {
    let mut runtime = Fixture::new(1_000);
    runtime.caster_death_outcome = Ok(false);
    runtime.active = Some(SpellCastState {
        spell_id: 133,
        target_guid: ObjectGuid::EMPTY,
        target_data: Default::default(),
        cast_id: ObjectGuid::EMPTY,
        cast_start_time: std::time::Instant::now(),
        cast_time_ms: 1_000,
        spell_visual: Default::default(),
        metadata: Default::default(),
    });

    assert_eq!(
        prepare_server_normal(&mut runtime, 133, 0, ObjectGuid::EMPTY, Default::default()),
        ServerNormalPrepareOutcomeLikeCpp::Rejected
    );
    assert_eq!(runtime.failures, [SpellCastResult::SpellInProgress as i32]);
    assert_eq!(runtime.caster_death_checks.get(), 0);
    assert_eq!(runtime.events, ["busy", "failure"]);
}

#[test]
fn normal_prepare_dead_caster_precedes_missing_cooldown_authority() {
    let mut runtime = Fixture::new(0);
    runtime.caster_death_outcome = Ok(false);
    runtime.native_gcd_available = false;
    runtime.server_normal_cooldown = None;

    assert_eq!(
        prepare_server_normal(&mut runtime, 133, 0, ObjectGuid::EMPTY, Default::default()),
        ServerNormalPrepareOutcomeLikeCpp::Rejected
    );
    assert_eq!(runtime.failures, [SpellCastResult::CasterDead as i32]);
    assert_eq!(runtime.native_gcd_checks.get(), 0);
    assert_eq!(runtime.server_cooldown_checks.get(), 0);
}

#[test]
fn normal_prepare_checks_native_gcd_category_without_timestamp_fallback() {
    let mut runtime = Fixture::new(0);
    runtime.native_gcd = (133, true);
    assert_eq!(
        prepare_server_normal(&mut runtime, 133, 0, ObjectGuid::EMPTY, Default::default(),),
        ServerNormalPrepareOutcomeLikeCpp::Rejected
    );
    assert_eq!(runtime.failures, [SpellCastResult::NotReady as i32]);
    assert!(!runtime.events.contains(&"start"));
}

#[test]
fn normal_prepare_distinguishes_unavailable_authority_from_cast_rejection() {
    let mut missing_cost_authority = Fixture::new(0);
    missing_cost_authority.normal_cost_outcome = Err("ItemEffect catalog unavailable");
    assert_eq!(
        prepare_server_normal(
            &mut missing_cost_authority,
            133,
            0,
            ObjectGuid::EMPTY,
            Default::default(),
        ),
        ServerNormalPrepareOutcomeLikeCpp::Unavailable("ItemEffect catalog unavailable")
    );

    let mut unaffordable = Fixture::new(0);
    unaffordable.normal_cost_outcome = Ok(false);
    assert_eq!(
        prepare_server_normal(
            &mut unaffordable,
            133,
            0,
            ObjectGuid::EMPTY,
            Default::default(),
        ),
        ServerNormalPrepareOutcomeLikeCpp::Rejected
    );

    let mut missing_precondition_authority = Fixture::new(0);
    missing_precondition_authority.precondition_outcome = Err("Player aura authority unavailable");
    assert_eq!(
        prepare_server_normal(
            &mut missing_precondition_authority,
            133,
            0,
            ObjectGuid::EMPTY,
            Default::default(),
        ),
        ServerNormalPrepareOutcomeLikeCpp::Unavailable("Player aura authority unavailable")
    );
}

#[test]
fn normal_prepare_rejects_instant_and_timed_unimplemented_mutation_before_publication() {
    for cast_time_ms in [0, 1_750] {
        let mut runtime = Fixture::new(cast_time_ms);
        runtime.spell.effect_type = 24;
        runtime.spell.effects = vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: 24,
            ..Default::default()
        }];
        assert_eq!(runtime.spell.cast_time_ms, cast_time_ms);

        assert_eq!(
            prepare_server_normal(&mut runtime, 133, 0, ObjectGuid::EMPTY, Default::default()),
            ServerNormalPrepareOutcomeLikeCpp::Unavailable(
                "normal cast contains an unimplemented spell effect"
            ),
            "the effect remains unavailable for cast time {cast_time_ms}"
        );
        assert_eq!(
            runtime.events,
            [
                "busy",
                "caster-death",
                "normal-costs",
                "power",
                "effects-available"
            ],
            "ordinary admission order for cast time {cast_time_ms}"
        );
        assert!(runtime.active.is_none());
        assert!(!runtime.events.contains(&"cast-start-cooldown"));
        assert!(!runtime.events.contains(&"install-server-normal"));
        assert!(!runtime.events.contains(&"start"));
    }
}

#[test]
fn normal_prepare_keeps_a_supported_instant_effect_on_the_canonical_path() {
    let mut runtime = Fixture::new(0);
    runtime.spell.effect_type = wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL;
    runtime.spell.effects = vec![wow_data::SpellEffectInfo {
        effect_index: 0,
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
        ..Default::default()
    }];

    assert_eq!(
        prepare_server_normal(&mut runtime, 133, 0, ObjectGuid::EMPTY, Default::default()),
        ServerNormalPrepareOutcomeLikeCpp::Instant
    );
    assert_eq!(runtime.active.as_ref().unwrap().cast_time_ms, 0);
    assert_eq!(
        runtime.events,
        [
            "busy",
            "caster-death",
            "normal-costs",
            "power",
            "effects-available",
            "cast-start-cooldown",
            "install-server-normal",
            "start",
        ]
    );
}

#[test]
fn normal_prepare_treats_known_missing_spell_as_rejection_and_catalog_loss_as_unavailable() {
    let mut known_missing = Fixture::new(0);
    assert_eq!(
        prepare_server_normal(
            &mut known_missing,
            134,
            0,
            ObjectGuid::EMPTY,
            Default::default(),
        ),
        ServerNormalPrepareOutcomeLikeCpp::Rejected
    );
    assert!(known_missing.failures.is_empty());

    let mut unavailable_catalog = Fixture::new(0);
    unavailable_catalog.spell_catalog_failure = Some("SpellStore unavailable");
    assert_eq!(
        prepare_server_normal(
            &mut unavailable_catalog,
            133,
            0,
            ObjectGuid::EMPTY,
            Default::default(),
        ),
        ServerNormalPrepareOutcomeLikeCpp::Unavailable("SpellStore unavailable")
    );
}

#[test]
fn direct_normal_prepare_skips_active_start_and_gcd_but_still_checks_gcd() {
    let mut runtime = Fixture::new(0);
    let mut metadata = wow_entities::SpellCastMetadata {
        trigger_cast_flags_like_cpp: wow_entities::TRIGGERED_CAST_DIRECTLY_LIKE_CPP,
        ..Default::default()
    };
    assert!(matches!(
        prepare_server_normal(&mut runtime, 133, 0, ObjectGuid::EMPTY, metadata),
        ServerNormalPrepareOutcomeLikeCpp::Direct(_)
    ));
    assert!(runtime.active.is_none());
    assert!(!runtime.events.contains(&"install-server-normal"));
    assert!(!runtime.events.contains(&"start"));
    assert!(runtime.events.contains(&"cast-start-cooldown"));

    runtime.events.clear();
    runtime.failures.clear();
    runtime.native_gcd = (133, true);
    metadata.trigger_cast_flags_like_cpp |=
        wow_entities::TRIGGERED_IGNORE_POWER_AND_REAGENT_COST_LIKE_CPP;
    assert_eq!(
        prepare_server_normal(&mut runtime, 133, 0, ObjectGuid::EMPTY, metadata),
        ServerNormalPrepareOutcomeLikeCpp::Rejected
    );
    assert_eq!(runtime.failures, [SpellCastResult::NotReady as i32]);
    assert!(!runtime.events.contains(&"power"));
}

#[test]
fn instant_and_timed_requests_share_mapping_validation_install_and_start() {
    for time in [0, 1500] {
        let mut runtime = Fixture::new(time);
        assert_eq!(prepare(&mut runtime, Fixture::request()), time == 0);
        assert_eq!(
            runtime.events,
            ["prepare", "caster-death", "power", "install", "start"]
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
    assert_eq!(runtime.events, ["prepare", "caster-death", "power"]);
    assert_eq!(runtime.allocated.get(), 1);
    assert!(runtime.active.is_none());
}

#[test]
fn rejected_residence_install_cannot_publish_start() {
    let mut runtime = Fixture::new(0);
    runtime.admits_install = false;
    assert!(!prepare(&mut runtime, Fixture::request()));
    assert_eq!(
        runtime.events,
        ["prepare", "caster-death", "power", "install", "failure"]
    );
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

#[test]
fn server_normal_spell_go_keeps_empty_original_cast_id() {
    let cast_id = ObjectGuid::new(6, 0xCA57);
    assert_eq!(
        crate::execution_session::spell_go_original_cast_id_like_cpp(
            SpellCastMetadata {
                server_normal_cast_like_cpp: true,
                ..Default::default()
            },
            cast_id,
            false,
        ),
        ObjectGuid::EMPTY,
        "Source normal casts leave OriginalCastID at its explicit default"
    );

    let original = ObjectGuid::new(6, 0xA11);
    assert_eq!(
        crate::execution_session::spell_go_original_cast_id_like_cpp(
            SpellCastMetadata {
                server_normal_cast_like_cpp: true,
                original_cast_id: original,
                ..Default::default()
            },
            cast_id,
            false,
        ),
        original,
        "an explicitly supplied OriginalCastID remains intact"
    );
    assert_eq!(
        crate::execution_session::spell_go_original_cast_id_like_cpp(
            SpellCastMetadata::default(),
            cast_id,
            false,
        ),
        cast_id,
        "existing non-normal behavior retains its fallback"
    );
}
