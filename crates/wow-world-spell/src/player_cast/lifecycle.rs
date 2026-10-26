//! Normal Player request lifecycle. Transport and effect execution are adapters;
//! pending/active state remains canonical Player/Unit state.
//!
//! Classic Player::RequestSpellCast / ExecutePendingSpellCastRequest and
//! Spell::prepare. Intentional admission/identity repairs belong to #589.

use wow_constants::SpellCastResult;
use wow_core::ObjectGuid;
use wow_data::SpellInfo;
use wow_entities::{PendingSpellCastRequestLikeCpp, SpellCastState, SpellCastVisualLikeCpp};

pub trait Runtime {
    fn spell(&self, id: i32) -> Option<SpellInfo>;
    /// Resolve C++ `GetSpellInfo(id, mapDifficulty)` and return its actual key.
    fn spell_for_difficulty(&self, id: i32, difficulty_id: u8) -> Option<(SpellInfo, u8)>;
    /// `true` for a still-installed non-instant generic cast. Source
    /// `IsNonMeleeSpellCast(..., skipInstant=true)` permits instant nesting.
    fn server_normal_busy(&mut self) -> Option<bool>;
    fn known(&self, id: i32) -> bool;
    fn resolve_override(&self, original: &SpellInfo) -> SpellInfo;
    fn passive(&self, spell: i32) -> bool;
    fn remaining(&self, spell: &SpellInfo) -> Option<(u32, u32)>;
    fn replace_pending(&mut self, request: PendingSpellCastRequestLikeCpp);
    fn failure(&mut self, id: ObjectGuid, spell: i32, visual: SpellCastVisualLikeCpp, reason: i32);
    fn allocate(&self, spell: i32) -> Option<(ObjectGuid, Option<u64>)>;
    fn visual(&self, spell: &SpellInfo) -> Option<SpellCastVisualLikeCpp>;
    fn prepare_mapping(&mut self, client: ObjectGuid, server: ObjectGuid);
    fn disabled(&self, spell: i32) -> bool;
    fn on_cooldown(&self, spell: &SpellInfo) -> Option<bool>;
    /// Source `SpellHistory::IsReady` over the selected native spell key,
    /// history ledger, cooldown category, charges, aura modifiers and item.
    fn server_normal_on_cooldown(
        &self,
        spell: &SpellInfo,
        actual_difficulty_id: u8,
        metadata: wow_entities::SpellCastMetadata,
    ) -> Option<bool>;
    /// Native SpellHistory GCD membership for the selected recovery category.
    fn server_normal_global_cooldown_state(
        &self,
        spell: &SpellInfo,
        actual_difficulty_id: u8,
    ) -> Option<(u32, bool)>;
    /// C++ Attr12 calls SendSpellCooldown during prepare, before START and
    /// before the direct-cast branch. This operation also validates the
    /// required cooldown authorities for the later pre-GO path.
    fn server_normal_cooldown_at_cast_start(
        &mut self,
        spell: &SpellInfo,
        actual_difficulty_id: u8,
        metadata: wow_entities::SpellCastMetadata,
    ) -> Option<()>;
    fn check_power(
        &mut self,
        spell: &SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
    ) -> bool;
    fn check_server_normal_costs(
        &mut self,
        spell: &SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: wow_entities::SpellCastMetadata,
    ) -> bool;
    fn check_preconditions(
        &mut self,
        spell: &SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: wow_entities::SpellCastMetadata,
    ) -> bool;
    fn install(&mut self, cast: SpellCastState) -> bool;
    /// Admission for a direct native `CastSpell` preparation. The production
    /// adapter performs the busy check and install under the same map owner.
    fn install_server_normal(&mut self, cast: SpellCastState) -> bool;
    fn start(&mut self, cast: &SpellCastState, spell: &SpellInfo);
    /// C++ `Unit::ModCastingSpeed` product for this caster
    /// (`Unit::ApplyCastTimePercentMod`). `1.0` when the caster has none.
    fn cast_time_multiplier(&self) -> f32 {
        1.0
    }
}

/// C++ `Player::CancelPendingCastRequest` always publishes a CastFailed:
/// "We have to inform the client that the cast has been canceled. Otherwise
/// the cast button will remain highlightened." Every abandoned request in this
/// module therefore reports, even when the cause is a lost canonical input.
fn cancel_like_cpp(
    runtime: &mut impl Runtime,
    cast_id: ObjectGuid,
    spell_id: i32,
    visual: SpellCastVisualLikeCpp,
) -> bool {
    runtime.failure(
        cast_id,
        spell_id,
        visual,
        SpellCastResult::DontReport as i32,
    );
    false
}

/// Admission has one bounded queue window. Keep the original request spell ID:
/// override/known-spell resolution occurs again when that request can prepare.
pub fn request(runtime: &mut impl Runtime, request: PendingSpellCastRequestLikeCpp) -> bool {
    let Some(spell) = runtime.spell(request.spell_id) else {
        return false;
    };
    let Some((gcd, active)) = runtime.remaining(&spell) else {
        return cancel_like_cpp(
            runtime,
            request.cast_id,
            request.spell_id,
            Default::default(),
        );
    };
    if gcd > 400 || active > 400 {
        runtime.failure(
            request.cast_id,
            request.spell_id,
            Default::default(),
            SpellCastResult::SpellInProgress as i32,
        );
        return false;
    }
    // Replacement also cancels the previous pending request on an immediate
    // path, just as Player::RequestSpellCast replaces before executing.
    runtime.replace_pending(request);
    gcd == 0 && active == 0
}

/// Consume a taken pending request through the same preparation for instant and
/// timed casts. Returns true only when the installed active cast is ready now.
pub fn prepare(runtime: &mut impl Runtime, request: PendingSpellCastRequestLikeCpp) -> bool {
    let Some(original) = runtime.spell(request.spell_id) else {
        return cancel_like_cpp(
            runtime,
            request.cast_id,
            request.spell_id,
            Default::default(),
        );
    };
    if !runtime.known(request.spell_id) {
        runtime.failure(
            request.cast_id,
            request.spell_id,
            Default::default(),
            SpellCastResult::DontReport as i32,
        );
        return false;
    }
    let spell = runtime.resolve_override(&original);
    if runtime.passive(spell.spell_id) {
        runtime.failure(
            request.cast_id,
            request.spell_id,
            Default::default(),
            SpellCastResult::DontReport as i32,
        );
        return false;
    }
    // C++ `GetCastSpellXSpellVisualId` always resolves a visual. `None` here is
    // an unrepresented selection input, so the request is rejected explicitly
    // instead of leaving the client waiting on a cast that never publishes.
    let Some(visual) = runtime.visual(&spell) else {
        return cancel_like_cpp(
            runtime,
            request.cast_id,
            request.spell_id,
            Default::default(),
        );
    };
    let Some((server_id, revision)) = runtime.allocate(spell.spell_id) else {
        return cancel_like_cpp(runtime, request.cast_id, request.spell_id, visual);
    };
    runtime.prepare_mapping(request.cast_id, server_id);
    if runtime.disabled(spell.spell_id) {
        runtime.failure(server_id, spell.spell_id, visual, 128);
        return false;
    }
    match runtime.on_cooldown(&spell) {
        Some(false) => {}
        Some(true) => {
            runtime.failure(
                server_id,
                spell.spell_id,
                visual,
                SpellCastResult::NotReady as i32,
            );
            return false;
        }
        // A lost cooldown owner is still an abandoned cast, and the mapping to
        // this server identity has already been published.
        None => return cancel_like_cpp(runtime, server_id, spell.spell_id, visual),
    }
    if !runtime.check_preconditions(&spell, server_id, &visual, request.metadata)
        || !runtime.check_power(&spell, server_id, &visual)
    {
        return false;
    }
    let mut metadata = request.metadata;
    metadata.client_cast_id = Some(request.cast_id);
    metadata.prepared_residence_revision = revision;
    metadata.original_cast_id = ObjectGuid::EMPTY;
    metadata.from_client = true;
    metadata.client_started_global_cooldown = spell.cooldown_ms != 0;
    // C++ `Spell::prepare` stores `m_casttime = GetSpellCastTime(...)`, which
    // already includes the caster's `ModCastingSpeed`.
    let cast_time_ms = (spell.cast_time_ms as f32 * runtime.cast_time_multiplier()).max(0.0) as u32;
    let cast = SpellCastState {
        spell_id: spell.spell_id,
        target_guid: request.target_guid,
        target_data: request.target_data,
        cast_id: server_id,
        cast_start_time: std::time::Instant::now(),
        cast_time_ms,
        spell_visual: visual,
        metadata,
    };
    if !runtime.install(cast.clone()) {
        return cancel_like_cpp(runtime, server_id, spell.spell_id, cast.spell_visual);
    }
    runtime.start(&cast, &spell);
    cast.cast_time_ms == 0
}

/// Prepare a normal Player `CastSpell` call without client request admission.
/// Returns true only for an instant cast that must execute before the caller
/// returns. Timed casts remain in canonical active state for the cast driver.
#[derive(Debug, Clone, PartialEq)]
pub enum ServerNormalPrepareOutcomeLikeCpp {
    Rejected,
    Started,
    Instant,
    Direct(SpellCastState),
}

pub fn prepare_server_normal(
    runtime: &mut impl Runtime,
    spell_id: i32,
    requested_difficulty_id: u8,
    target_guid: ObjectGuid,
    mut metadata: wow_entities::SpellCastMetadata,
) -> ServerNormalPrepareOutcomeLikeCpp {
    let Some((spell, actual_difficulty_id)) =
        runtime.spell_for_difficulty(spell_id, requested_difficulty_id)
    else {
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    };
    let Some(visual) = runtime.visual(&spell) else {
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    };
    let Some((cast_id, revision)) = runtime.allocate(spell.spell_id) else {
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    };
    if runtime.disabled(spell.spell_id) {
        runtime.failure(
            cast_id,
            spell.spell_id,
            visual,
            SpellCastResult::SpellUnavailable as i32,
        );
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    }
    metadata.from_client = false;
    metadata.client_cast_id = None;
    metadata.prepared_residence_revision = revision;
    metadata.actual_spell_difficulty_id_like_cpp = Some(actual_difficulty_id);
    metadata.server_normal_cast_like_cpp = true;
    match runtime.server_normal_busy() {
        Some(false) => {}
        Some(true) => {
            runtime.failure(
                cast_id,
                spell.spell_id,
                visual,
                SpellCastResult::SpellInProgress as i32,
            );
            return ServerNormalPrepareOutcomeLikeCpp::Rejected;
        }
        None => return ServerNormalPrepareOutcomeLikeCpp::Rejected,
    }
    let ignore_gcd =
        metadata.trigger_cast_flags_like_cpp & wow_entities::TRIGGERED_IGNORE_GCD_LIKE_CPP != 0;
    let Some((gcd_category_id, gcd_active)) = (if ignore_gcd {
        Some((0, false))
    } else {
        runtime.server_normal_global_cooldown_state(&spell, actual_difficulty_id)
    }) else {
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    };
    if gcd_active {
        runtime.failure(
            cast_id,
            spell.spell_id,
            visual,
            SpellCastResult::NotReady as i32,
        );
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    }
    if metadata.trigger_cast_flags_like_cpp
        & wow_entities::TRIGGERED_IGNORE_SPELL_AND_CATEGORY_CD_LIKE_CPP
        == 0
        && runtime.server_normal_on_cooldown(&spell, actual_difficulty_id, metadata) != Some(false)
    {
        runtime.failure(
            cast_id,
            spell.spell_id,
            visual,
            SpellCastResult::NotReady as i32,
        );
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    }
    let direct =
        metadata.trigger_cast_flags_like_cpp & wow_entities::TRIGGERED_CAST_DIRECTLY_LIKE_CPP != 0;
    metadata.server_normal_started_global_cooldown =
        spell.cooldown_ms != 0 && gcd_category_id != 0 && !ignore_gcd && !direct;
    metadata.client_started_global_cooldown = false;
    if !runtime.check_preconditions(&spell, cast_id, &visual, metadata)
        || !runtime.check_server_normal_costs(&spell, cast_id, &visual, metadata)
        || (metadata.cast_item_guid.is_none()
            && metadata.trigger_cast_flags_like_cpp
                & wow_entities::TRIGGERED_IGNORE_POWER_AND_REAGENT_COST_LIKE_CPP
                == 0
            && !runtime.check_power(&spell, cast_id, &visual))
    {
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    }

    let cast_time_ms = (spell.cast_time_ms as f32 * runtime.cast_time_multiplier()).max(0.0) as u32;
    let cast = SpellCastState {
        spell_id: spell.spell_id,
        target_guid,
        target_data: Default::default(),
        cast_id,
        cast_start_time: std::time::Instant::now(),
        cast_time_ms,
        spell_visual: visual,
        metadata,
    };
    if runtime
        .server_normal_cooldown_at_cast_start(&spell, actual_difficulty_id, metadata)
        .is_none()
    {
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    }
    if direct {
        return ServerNormalPrepareOutcomeLikeCpp::Direct(cast);
    }
    if !runtime.install_server_normal(cast.clone()) {
        return ServerNormalPrepareOutcomeLikeCpp::Rejected;
    }
    runtime.start(&cast, &spell);
    if cast.cast_time_ms == 0 {
        ServerNormalPrepareOutcomeLikeCpp::Instant
    } else {
        ServerNormalPrepareOutcomeLikeCpp::Started
    }
}

#[cfg(test)]
#[path = "../../unit_tests/player_cast_tests.rs"]
mod tests;
