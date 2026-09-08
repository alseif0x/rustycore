//! Normal Player request lifecycle. Transport and effect execution are adapters;
//! pending/active state remains canonical Player/Unit state.
//!
//! Classic Player::RequestSpellCast / ExecutePendingSpellCastRequest and
//! Spell::prepare. Intentional admission/identity repairs belong to #589.

use wow_constants::SpellCastResult;
use wow_core::ObjectGuid;
use wow_data::SpellInfo;
use wow_entities::{PendingSpellCastRequestLikeCpp, SpellCastState, SpellCastVisualLikeCpp};

#[cfg(test)]
mod tests;

pub(crate) trait Runtime {
    fn spell(&self, id: i32) -> Option<SpellInfo>;
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
    fn check_power(
        &mut self,
        spell: &SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
    ) -> bool;
    fn check_preconditions(
        &mut self,
        spell: &SpellInfo,
        cast: ObjectGuid,
        visual: &SpellCastVisualLikeCpp,
        metadata: wow_entities::SpellCastMetadata,
    ) -> bool;
    fn install(&mut self, cast: SpellCastState) -> bool;
    fn start(&mut self, cast: &SpellCastState, spell: &SpellInfo);
}

/// Admission has one bounded queue window. Keep the original request spell ID:
/// override/known-spell resolution occurs again when that request can prepare.
pub(crate) fn request(runtime: &mut impl Runtime, request: PendingSpellCastRequestLikeCpp) -> bool {
    let Some(spell) = runtime.spell(request.spell_id) else {
        return false;
    };
    let Some((gcd, active)) = runtime.remaining(&spell) else {
        return false;
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
pub(crate) fn prepare(runtime: &mut impl Runtime, request: PendingSpellCastRequestLikeCpp) -> bool {
    let Some(original) = runtime.spell(request.spell_id) else {
        return false;
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
    let Some(visual) = runtime.visual(&spell) else {
        return false;
    };
    let Some((server_id, revision)) = runtime.allocate(spell.spell_id) else {
        return false;
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
        None => return false,
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
    let cast = SpellCastState {
        spell_id: spell.spell_id,
        target_guid: request.target_guid,
        target_data: request.target_data,
        cast_id: server_id,
        cast_start_time: std::time::Instant::now(),
        cast_time_ms: spell.cast_time_ms,
        spell_visual: visual,
        metadata,
    };
    if !runtime.install(cast.clone()) {
        return false;
    }
    runtime.start(&cast, &spell);
    cast.cast_time_ms == 0
}
