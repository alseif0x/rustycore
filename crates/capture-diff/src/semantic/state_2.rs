//! Semantic capture diff state definitions, part 2 of 5.
//!
//! Separated from the semantic.rs root under #658. Behaviour is preserved.

use super::*;

impl SemanticBodyDiff {
    /// True only when both bodies decoded successfully and all stable fields
    /// are equal. Decode failures are never accepted as a clean comparison.
    #[must_use]
    pub fn is_identical(&self) -> bool {
        self.cpp.decode_error.is_none()
            && self.rust.decode_error.is_none()
            && self.cpp.log_xp_gain == self.rust.log_xp_gain
            && self.cpp.loot_removed == self.rust.loot_removed
            && self.cpp.buy_succeeded == self.rust.buy_succeeded
            && self.cpp.send_known_spells == self.rust.send_known_spells
            && self.cpp.spell_go == self.rust.spell_go
            && self.cpp.spell_start == self.rust.spell_start
            && self.cpp.update_object_inv_slots == self.rust.update_object_inv_slots
            && self.cpp.monster_move == self.rust.monster_move
            && self.cpp.raw_body_sha256 == self.rust.raw_body_sha256
    }

    /// Concise field-level explanation for terminal reports.
    #[must_use]
    pub fn mismatch_summary(&self) -> String {
        if let Some(error) = &self.cpp.decode_error {
            return format!("C++ decode error: {error}");
        }
        if let Some(error) = &self.rust.decode_error {
            return format!("Rust decode error: {error}");
        }

        if let (Some(cpp), Some(rust)) = (self.cpp.log_xp_gain, self.rust.log_xp_gain) {
            return mismatch_log_xp_gain(cpp, rust);
        }

        if let (Some(cpp), Some(rust)) = (self.cpp.loot_removed, self.rust.loot_removed) {
            return mismatch_loot_removed(cpp, rust, &self.cpp, &self.rust);
        }

        if let (Some(cpp), Some(rust)) = (self.cpp.buy_succeeded, self.rust.buy_succeeded) {
            return mismatch_buy_succeeded(cpp, rust, &self.cpp, &self.rust);
        }

        if let (Some(cpp), Some(rust)) = (
            self.cpp.send_known_spells.as_ref(),
            self.rust.send_known_spells.as_ref(),
        ) {
            return mismatch_send_known_spells(cpp, rust);
        }

        if let (Some(cpp), Some(rust)) = (self.cpp.spell_go.as_ref(), self.rust.spell_go.as_ref()) {
            return mismatch_spell_go(cpp, rust, &self.cpp, &self.rust);
        }

        if let (Some(cpp), Some(rust)) = (
            self.cpp.spell_start.as_ref(),
            self.rust.spell_start.as_ref(),
        ) {
            if cpp.cast_time != rust.cast_time {
                return "mismatched field(s): cast_time".to_string();
            }
            return mismatch_spell_go(&cpp.cast, &rust.cast, &self.cpp, &self.rust);
        }

        if let (Some(cpp), Some(rust)) = (
            self.cpp.update_object_inv_slots.as_ref(),
            self.rust.update_object_inv_slots.as_ref(),
        ) {
            return mismatch_update_object_inv_slots(cpp, rust);
        }

        if let (Some(cpp), Some(rust)) = (
            self.cpp.monster_move.as_ref(),
            self.rust.monster_move.as_ref(),
        ) {
            return mismatch_monster_move(cpp, rust, &self.cpp, &self.rust);
        }

        "semantic body shape differs or is missing".to_string()
    }
}

pub(super) fn mismatch_monster_move(
    cpp: &MonsterMoveBody,
    rust: &MonsterMoveBody,
    cpp_side: &SemanticBodySide,
    rust_side: &SemanticBodySide,
) -> String {
    let mut fields = Vec::new();
    if cpp.mover != rust.mover {
        fields.push("mover");
    }
    if cpp.current_position != rust.current_position {
        fields.push("current_position");
    }
    if cpp.destination != rust.destination {
        fields.push("destination");
    }
    if cpp.crz_teleport != rust.crz_teleport {
        fields.push("crz_teleport");
    }
    if cpp.stop_distance_tolerance != rust.stop_distance_tolerance {
        fields.push("stop_distance_tolerance");
    }
    if cpp.flags != rust.flags {
        fields.push("flags");
    }
    if cpp.elapsed != rust.elapsed {
        fields.push("elapsed");
    }
    if cpp.move_time != rust.move_time {
        fields.push("move_time");
    }
    if cpp.fade_object_time != rust.fade_object_time {
        fields.push("fade_object_time");
    }
    if cpp.mode != rust.mode {
        fields.push("mode");
    }
    if cpp.transport != rust.transport {
        fields.push("transport");
    }
    if cpp.vehicle_seat != rust.vehicle_seat {
        fields.push("vehicle_seat");
    }
    if cpp.face != rust.face {
        fields.push("face");
    }
    if cpp.vehicle_exit_voluntary != rust.vehicle_exit_voluntary {
        fields.push("vehicle_exit_voluntary");
    }
    if cpp.interpolate != rust.interpolate {
        fields.push("interpolate");
    }
    if cpp.points != rust.points {
        fields.push("points");
    }
    if cpp.packed_deltas != rust.packed_deltas {
        fields.push("packed_deltas");
    }
    if cpp.spline_filter != rust.spline_filter {
        fields.push("spline_filter");
    }
    if cpp.spell_effect_extra != rust.spell_effect_extra {
        fields.push("spell_effect_extra");
    }
    if cpp.jump_extra != rust.jump_extra {
        fields.push("jump_extra");
    }
    if cpp.anim_tier_transition != rust.anim_tier_transition {
        fields.push("anim_tier_transition");
    }

    if fields.is_empty() {
        if cpp_side.raw_body_sha256 == rust_side.raw_body_sha256 {
            "semantic values are equal".to_string()
        } else {
            "raw body identity differs outside the reviewed issue-#24 fixture shape".to_string()
        }
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

pub(super) fn mismatch_send_known_spells(
    cpp: &SendKnownSpellsBody,
    rust: &SendKnownSpellsBody,
) -> String {
    let mut fields = Vec::new();
    if cpp.initial_login != rust.initial_login {
        fields.push("initial_login");
    }
    if cpp.known_spells != rust.known_spells {
        fields.push("known_spells");
    }
    if cpp.favorite_spells != rust.favorite_spells {
        fields.push("favorite_spells");
    }

    if fields.is_empty() {
        "semantic values are equal".to_string()
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

pub(super) fn mismatch_spell_go(
    cpp: &SpellGoBody,
    rust: &SpellGoBody,
    cpp_side: &SemanticBodySide,
    rust_side: &SemanticBodySide,
) -> String {
    let mut fields = Vec::new();
    if cpp.caster_guid != rust.caster_guid {
        fields.push("caster_guid");
    }
    if cpp.caster_unit != rust.caster_unit {
        fields.push("caster_unit");
    }
    if cpp.cast_id != rust.cast_id {
        fields.push("cast_id.identity");
    }
    if cpp.original_cast_id != rust.original_cast_id {
        fields.push("original_cast_id");
    }
    if cpp.spell_id != rust.spell_id {
        fields.push("spell_id");
    }
    if cpp.spell_visual_id != rust.spell_visual_id {
        fields.push("spell_visual_id");
    }
    if cpp.cast_flags != rust.cast_flags {
        fields.push("cast_flags");
    }
    if cpp.cast_flags_ex != rust.cast_flags_ex {
        fields.push("cast_flags_ex");
    }
    if cpp.missile_travel_time != rust.missile_travel_time
        || cpp.missile_pitch_bits != rust.missile_pitch_bits
    {
        fields.push("missile_trajectory");
    }
    if cpp.dest_loc_spell_cast_index != rust.dest_loc_spell_cast_index {
        fields.push("dest_loc_spell_cast_index");
    }
    if cpp.immunities_school != rust.immunities_school
        || cpp.immunities_value != rust.immunities_value
    {
        fields.push("immunities");
    }
    if cpp.prediction_points != rust.prediction_points
        || cpp.prediction_type != rust.prediction_type
        || cpp.prediction_beacon != rust.prediction_beacon
    {
        fields.push("prediction");
    }
    if cpp.target != rust.target {
        fields.push("target");
    }
    if cpp.hit_targets != rust.hit_targets {
        fields.push("hit_targets");
    }
    if cpp.miss_targets != rust.miss_targets {
        fields.push("miss_targets");
    }
    if cpp.miss_status != rust.miss_status {
        fields.push("miss_status");
    }
    if cpp.remaining_power != rust.remaining_power {
        fields.push("remaining_power");
    }
    if cpp.remaining_runes != rust.remaining_runes {
        fields.push("remaining_runes");
    }
    if cpp.target_points != rust.target_points {
        fields.push("target_points");
    }
    if cpp.ammo_display_id != rust.ammo_display_id {
        fields.push("ammo_display_id");
    }
    if cpp.ammo_inventory_type != rust.ammo_inventory_type {
        fields.push("ammo_inventory_type");
    }
    if fields.is_empty() {
        if cpp_side.raw_body_sha256 == rust_side.raw_body_sha256 {
            "semantic values are equal".to_string()
        } else {
            "raw body identity differs outside the reviewed Creature-cast shape".to_string()
        }
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

pub(super) fn mismatch_log_xp_gain(cpp: LogXpGainBody, rust: LogXpGainBody) -> String {
    let mut fields = Vec::new();
    if cpp.victim.high_type != rust.victim.high_type {
        fields.push("victim.high_type");
    }
    if cpp.victim.realm_id != rust.victim.realm_id {
        fields.push("victim.realm_id");
    }
    if cpp.victim.map_id != rust.victim.map_id {
        fields.push("victim.map_id");
    }
    if cpp.victim.entry != rust.victim.entry {
        fields.push("victim.entry");
    }
    if cpp.victim.subtype != rust.victim.subtype {
        fields.push("victim.subtype");
    }
    if cpp.victim.server_id != rust.victim.server_id {
        fields.push("victim.server_id");
    }
    if cpp.original != rust.original {
        fields.push("original");
    }
    if cpp.reason != rust.reason {
        fields.push("reason");
    }
    if cpp.amount != rust.amount {
        fields.push("amount");
    }
    if cpp.group_bonus_bits != rust.group_bonus_bits {
        fields.push("group_bonus");
    }

    if fields.is_empty() {
        "semantic values are equal".to_string()
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

pub(super) fn mismatch_loot_removed(
    cpp: LootRemovedBody,
    rust: LootRemovedBody,
    cpp_side: &SemanticBodySide,
    rust_side: &SemanticBodySide,
) -> String {
    let mut fields = Vec::new();
    if cpp.owner.high_type != rust.owner.high_type {
        fields.push("owner.high_type");
    }
    if cpp.owner.realm_id != rust.owner.realm_id {
        fields.push("owner.realm_id");
    }
    if cpp.owner.map_id != rust.owner.map_id {
        fields.push("owner.map_id");
    }
    if cpp.owner.entry != rust.owner.entry {
        fields.push("owner.entry");
    }
    if cpp.owner.subtype != rust.owner.subtype {
        fields.push("owner.subtype");
    }
    if cpp.owner.server_id != rust.owner.server_id {
        fields.push("owner.server_id");
    }
    if cpp.loot_obj.low != rust.loot_obj.low {
        fields.push("loot_obj.low");
    }
    if cpp.loot_obj.high != rust.loot_obj.high {
        fields.push("loot_obj.high");
    }
    if cpp.loot_list_id != rust.loot_list_id {
        fields.push("loot_list_id");
    }

    if fields.is_empty() {
        if cpp_side.raw_body_sha256 == rust_side.raw_body_sha256 {
            "semantic values are equal".to_string()
        } else {
            "raw body identity differs outside the normalized Creature-owner shape".to_string()
        }
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

pub(super) fn mismatch_update_object_inv_slots(
    cpp: &UpdateObjectInvSlotsBody,
    rust: &UpdateObjectInvSlotsBody,
) -> String {
    let mut fields = Vec::new();
    if cpp.map_id != rust.map_id {
        fields.push("map_id");
    }
    if cpp.player.low != rust.player.low || cpp.player.high != rust.player.high {
        fields.push("player");
    }
    if cpp.inv_slots.len() == rust.inv_slots.len() {
        for (cpp_slot, rust_slot) in cpp.inv_slots.iter().zip(&rust.inv_slots) {
            if cpp_slot.slot != rust_slot.slot {
                fields.push("inv_slots.slot");
            }
            if cpp_slot.item != rust_slot.item {
                fields.push("inv_slots.item");
            }
        }
    } else {
        fields.push("inv_slots.length");
    }

    fields.sort_unstable();
    fields.dedup();
    if fields.is_empty() {
        "semantic values are equal".to_string()
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

pub(super) fn mismatch_buy_succeeded(
    cpp: BuySucceededBody,
    rust: BuySucceededBody,
    cpp_side: &SemanticBodySide,
    rust_side: &SemanticBodySide,
) -> String {
    let mut fields = Vec::new();
    if cpp.vendor.high_type != rust.vendor.high_type {
        fields.push("vendor.high_type");
    }
    if cpp.vendor.realm_id != rust.vendor.realm_id {
        fields.push("vendor.realm_id");
    }
    if cpp.vendor.map_id != rust.vendor.map_id {
        fields.push("vendor.map_id");
    }
    if cpp.vendor.entry != rust.vendor.entry {
        fields.push("vendor.entry");
    }
    if cpp.vendor.subtype != rust.vendor.subtype {
        fields.push("vendor.subtype");
    }
    if cpp.vendor.server_id != rust.vendor.server_id {
        fields.push("vendor.server_id");
    }
    if cpp.muid != rust.muid {
        fields.push("muid");
    }
    if cpp.new_quantity != rust.new_quantity {
        fields.push("new_quantity");
    }
    if cpp.quantity_bought != rust.quantity_bought {
        fields.push("quantity_bought");
    }

    if fields.is_empty() {
        if cpp_side.raw_body_sha256 == rust_side.raw_body_sha256 {
            "semantic values are equal".to_string()
        } else {
            "raw body identity differs outside the reviewed vendor-success shape".to_string()
        }
    } else {
        format!("mismatched field(s): {}", fields.join(", "))
    }
}

/// Compare packet bodies semantically when a reviewed narrow comparator exists.
///
/// Routing is not normalized here: [`crate::diff::DiffReport`] still compares
/// `connection_id`, so `SMSG_LOG_XP_GAIN` must use the realm socket and
/// `SMSG_LOOT_REMOVED` / the reviewed `SMSG_UPDATE_OBJECT` must use the
/// instance socket like C++.
#[must_use]
pub fn compare_packet_bodies(
    direction: Direction,
    opcode: u16,
    cpp: &[u8],
    rust: &[u8],
) -> Option<SemanticBodyDiff> {
    if direction != Direction::S2C {
        return None;
    }

    match opcode {
        SMSG_LOG_XP_GAIN => compare_log_xp_gain_bodies(cpp, rust),
        SMSG_LOOT_REMOVED => compare_loot_removed_bodies(cpp, rust),
        SMSG_BUY_SUCCEEDED => compare_buy_succeeded_bodies(cpp, rust),
        SMSG_SEND_KNOWN_SPELLS => Some(compare_send_known_spells_bodies(cpp, rust)),
        SMSG_SPELL_GO => Some(compare_spell_go_bodies(cpp, rust)),
        SMSG_SPELL_START => Some(compare_spell_start_bodies(cpp, rust)),
        SMSG_UPDATE_OBJECT => compare_update_object_inv_slots_bodies(cpp, rust),
        SMSG_ON_MONSTER_MOVE => compare_monster_move_bodies(cpp, rust),
        _ => None,
    }
}

pub(super) fn compare_spell_go_bodies(cpp: &[u8], rust: &[u8]) -> SemanticBodyDiff {
    SemanticBodyDiff {
        comparator: "smsg_spell_go_creature_runtime_counters_and_cast_time".to_string(),
        cpp: SemanticBodySide::from_decoded_spell_go(decode_spell_go_body(cpp), cpp),
        rust: SemanticBodySide::from_decoded_spell_go(decode_spell_go_body(rust), rust),
    }
}

pub(super) fn compare_spell_start_bodies(cpp: &[u8], rust: &[u8]) -> SemanticBodyDiff {
    SemanticBodyDiff {
        comparator: "smsg_spell_start_creature_runtime_counters".to_string(),
        cpp: SemanticBodySide::from_decoded_spell_start(decode_spell_start_body(cpp), cpp),
        rust: SemanticBodySide::from_decoded_spell_start(decode_spell_start_body(rust), rust),
    }
}

pub(super) fn compare_monster_move_bodies(cpp: &[u8], rust: &[u8]) -> Option<SemanticBodyDiff> {
    let mut cpp_decoded = decode_monster_move_body(cpp);
    let mut rust_decoded = decode_monster_move_body(rust);
    let cpp_is_legacy_fixture = cpp_decoded
        .as_ref()
        .is_ok_and(|decoded| validate_legacy_cpp_detour_chase_monster_move(decoded).is_ok());
    let cpp_is_repaired_fixture = cpp_decoded
        .as_ref()
        .is_ok_and(|decoded| validate_detour_chase_monster_move(decoded).is_ok());
    let cpp_is_fixture = cpp_is_legacy_fixture || cpp_is_repaired_fixture;
    let rust_is_repaired_fixture = rust_decoded
        .as_ref()
        .is_ok_and(|decoded| validate_detour_chase_monster_move(decoded).is_ok());
    let rust_is_legacy_fixture = rust_decoded
        .as_ref()
        .is_ok_and(|decoded| validate_legacy_cpp_detour_chase_monster_move(decoded).is_ok());
    let rust_is_fixture = rust_is_repaired_fixture || rust_is_legacy_fixture;

    // This comparator is intentionally not a generic movement normalization.
    // If neither side satisfies the complete issue-#24 identity, position,
    // topology, and facing contract, ordinary byte comparison remains
    // authoritative.
    if !cpp_is_fixture && !rust_is_fixture {
        return None;
    }

    if cpp_is_fixture && rust_is_fixture {
        for decoded in [&mut cpp_decoded, &mut rust_decoded]
            .into_iter()
            .filter_map(|decoded| decoded.as_mut().ok())
        {
            decoded.body.current_position = ISSUE_24_CREATURE_START;
            decoded.body.move_time = 0;
            decoded.body.points.clear();
            decoded.body.packed_deltas.clear();
            if let MonsterMoveFaceBody::Target { direction_bits, .. } = &mut decoded.body.face {
                // The direction is derived from the final segment. The reviewed
                // repair may route around the opposite side, but the exact
                // target GUID remains authoritative.
                *direction_bits = 0;
            }
        }
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_on_monster_move_issue_24_process_local_route_fields".to_string(),
        cpp: SemanticBodySide::from_decoded_monster_move(cpp_decoded, cpp, cpp_is_fixture),
        rust: SemanticBodySide::from_decoded_monster_move(rust_decoded, rust, rust_is_fixture),
    })
}

pub(super) fn compare_send_known_spells_bodies(cpp: &[u8], rust: &[u8]) -> SemanticBodyDiff {
    SemanticBodyDiff {
        comparator: "smsg_send_known_spells_unordered_spell_sets".to_string(),
        cpp: SemanticBodySide::from_decoded_send_known_spells(
            decode_send_known_spells_body(cpp),
            cpp,
        ),
        rust: SemanticBodySide::from_decoded_send_known_spells(
            decode_send_known_spells_body(rust),
            rust,
        ),
    }
}

pub(super) fn compare_buy_succeeded_bodies(cpp: &[u8], rust: &[u8]) -> Option<SemanticBodyDiff> {
    let cpp_decoded = decode_buy_succeeded_body_with_counter(cpp);
    let rust_decoded = decode_buy_succeeded_body_with_counter(rust);

    // Normalize only the exact G'eras fixture identity. The bot preflight
    // pins SQL spawn 96654 and rejects an overlapping same-entry spawn before
    // it performs the purchase.
    let cpp_has_reviewed_vendor = cpp_decoded
        .as_ref()
        .is_ok_and(DecodedBuySucceededBody::has_issue_108_vendor_identity);
    let rust_has_reviewed_vendor = rust_decoded
        .as_ref()
        .is_ok_and(DecodedBuySucceededBody::has_issue_108_vendor_identity);
    if cpp_decoded.is_ok()
        && rust_decoded.is_ok()
        && !cpp_has_reviewed_vendor
        && !rust_has_reviewed_vendor
    {
        return None;
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_buy_succeeded_without_vendor_runtime_guid_counter".to_string(),
        cpp: SemanticBodySide::from_decoded_buy_succeeded(cpp_decoded, cpp),
        rust: SemanticBodySide::from_decoded_buy_succeeded(rust_decoded, rust),
    })
}

pub(super) fn compare_log_xp_gain_bodies(cpp: &[u8], rust: &[u8]) -> Option<SemanticBodyDiff> {
    let cpp_decoded = decode_log_xp_gain_body_with_counter(cpp);
    let rust_decoded = decode_log_xp_gain_body_with_counter(rust);

    // Normalize runtime counters only for the reviewed rested-XP shape: kill
    // XP whose victim is a real Creature GUID. For every other valid XP shape
    // (for example NO_KILL with ObjectGuid::Empty), retain raw byte comparison.
    let cpp_is_creature_kill = cpp_decoded
        .as_ref()
        .is_ok_and(DecodedLogXpGainBody::is_creature_kill);
    let rust_is_creature_kill = rust_decoded
        .as_ref()
        .is_ok_and(DecodedLogXpGainBody::is_creature_kill);
    if cpp_decoded.is_ok()
        && rust_decoded.is_ok()
        && !cpp_is_creature_kill
        && !rust_is_creature_kill
    {
        return None;
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_log_xp_gain_without_runtime_guid_counter".to_string(),
        cpp: SemanticBodySide::from_decoded_log_xp_gain(cpp_decoded, cpp),
        rust: SemanticBodySide::from_decoded_log_xp_gain(rust_decoded, rust),
    })
}

pub(super) fn compare_loot_removed_bodies(cpp: &[u8], rust: &[u8]) -> Option<SemanticBodyDiff> {
    let cpp_decoded = decode_loot_removed_body_with_counter(cpp);
    let rust_decoded = decode_loot_removed_body_with_counter(rust);

    // Normalize only the exact Doctor/LootObject/list shape proven by the
    // paired issue-#106 capture. A generic Creature predicate would conflate
    // two same-entry world objects because the omitted counter is their only
    // per-instance wire identity. The bot's fixture preflight additionally
    // proves that this entry has one SQL spawn on map 530.
    let cpp_has_reviewed_owner = cpp_decoded
        .as_ref()
        .is_ok_and(DecodedLootRemovedBody::has_issue_106_owner_identity);
    let rust_has_reviewed_owner = rust_decoded
        .as_ref()
        .is_ok_and(DecodedLootRemovedBody::has_issue_106_owner_identity);
    if cpp_decoded.is_ok()
        && rust_decoded.is_ok()
        && !cpp_has_reviewed_owner
        && !rust_has_reviewed_owner
    {
        return None;
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_loot_removed_without_creature_owner_runtime_guid_counter".to_string(),
        cpp: SemanticBodySide::from_decoded_loot_removed(cpp_decoded, cpp),
        rust: SemanticBodySide::from_decoded_loot_removed(rust_decoded, rust),
    })
}

pub(super) fn compare_update_object_inv_slots_bodies(
    cpp: &[u8],
    rust: &[u8],
) -> Option<SemanticBodyDiff> {
    let cpp_decoded = decode_update_object_inv_slots_candidate(cpp);
    let rust_decoded = decode_update_object_inv_slots_candidate(rust);

    let cpp_candidate = cpp_decoded.candidate();
    let rust_candidate = rust_decoded.candidate();
    match (cpp_candidate, rust_candidate) {
        // This is the only accepted asymmetry: C++ accumulated UnitData parent
        // bit 116 with no child or payload; Rust emitted only the identical
        // ActivePlayer InvSlots delta.
        (Some(cpp), Some(rust))
            if cpp.has_empty_unit_power_parent && !rust.has_empty_unit_power_parent => {}
        // Identical orientation has no normalization to perform. Leave these
        // UpdateObject bodies under ordinary byte comparison.
        (Some(cpp), Some(rust))
            if cpp.has_empty_unit_power_parent == rust.has_empty_unit_power_parent =>
        {
            return None;
        }
        (None, None)
            if matches!(cpp_decoded, UpdateObjectInvSlotsDecode::NotEligible(_))
                && matches!(rust_decoded, UpdateObjectInvSlotsDecode::NotEligible(_)) =>
        {
            return None;
        }
        // A non-candidate packet paired with a malformed candidate-shaped
        // packet is not evidence that this narrowly reviewed asymmetry is in
        // play. Keep the ordinary raw-body diff in either orientation. Two
        // malformed candidate-shaped packets remain fail-closed below.
        (None, None)
            if matches!(cpp_decoded, UpdateObjectInvSlotsDecode::NotEligible(_))
                && matches!(rust_decoded, UpdateObjectInvSlotsDecode::Malformed(_)) =>
        {
            return None;
        }
        (None, None)
            if matches!(cpp_decoded, UpdateObjectInvSlotsDecode::Malformed(_))
                && matches!(rust_decoded, UpdateObjectInvSlotsDecode::NotEligible(_)) =>
        {
            return None;
        }
        _ => {}
    }

    let reverse_orientation = matches!(
        (cpp_candidate, rust_candidate),
        (Some(cpp), Some(rust))
            if !cpp.has_empty_unit_power_parent && rust.has_empty_unit_power_parent
    );
    let mut cpp_side = SemanticBodySide::from_update_object_inv_slots_decode(cpp_decoded, cpp);
    let mut rust_side = SemanticBodySide::from_update_object_inv_slots_decode(rust_decoded, rust);
    if reverse_orientation {
        let error = "empty UnitData Power parent appears only on Rust; normalization is C++-only"
            .to_string();
        cpp_side.decode_error = Some(error.clone());
        cpp_side.raw_body_sha256 = Some(raw_body_sha256(cpp));
        rust_side.decode_error = Some(error);
        rust_side.raw_body_sha256 = Some(raw_body_sha256(rust));
    }

    Some(SemanticBodyDiff {
        comparator: "smsg_update_object_without_cpp_empty_unit_power_parent".to_string(),
        cpp: cpp_side,
        rust: rust_side,
    })
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DecodedLogXpGainBody {
    pub(super) body: LogXpGainBody,
    pub(super) runtime_counter: u64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DecodedLootRemovedBody {
    pub(super) body: LootRemovedBody,
    pub(super) owner_runtime_counter: u64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DecodedBuySucceededBody {
    pub(super) body: BuySucceededBody,
    pub(super) vendor_runtime_counter: u64,
}

#[derive(Debug, Clone)]
pub(super) struct DecodedUpdateObjectInvSlotsBody {
    pub(super) body: UpdateObjectInvSlotsBody,
    pub(super) has_empty_unit_power_parent: bool,
}

#[derive(Debug)]
pub(super) enum UpdateObjectInvSlotsDecode {
    Candidate(DecodedUpdateObjectInvSlotsBody),
    NotEligible(&'static str),
    Malformed(String),
}

impl UpdateObjectInvSlotsDecode {
    pub(super) fn candidate(&self) -> Option<&DecodedUpdateObjectInvSlotsBody> {
        match self {
            Self::Candidate(decoded) => Some(decoded),
            Self::NotEligible(_) | Self::Malformed(_) => None,
        }
    }
}

pub(super) enum UpdateObjectInvSlotsFailure {
    NotEligible(&'static str),
    Malformed(String),
}

impl DecodedLootRemovedBody {
    pub(super) fn has_issue_106_owner_identity(&self) -> bool {
        self.body.owner == ISSUE_106_CREATURE_IDENTITY
    }

    pub(super) fn issue_106_shape_error(&self) -> Option<String> {
        if !self.has_issue_106_owner_identity() {
            return None;
        }
        if self.owner_runtime_counter == 0 {
            return Some(
                "loot-removed issue-#106 Creature owner has a zero runtime GUID counter"
                    .to_string(),
            );
        }

        if let Some(error) = issue_106_loot_object_error(self.body.loot_obj) {
            return Some(error);
        }
        if self.body.loot_list_id != 0 {
            return Some("loot_list_id is not the reviewed deterministic slot 0".to_string());
        }
        None
    }

    pub(super) fn is_issue_106_reviewed_shape(&self) -> bool {
        self.has_issue_106_owner_identity() && self.issue_106_shape_error().is_none()
    }
}

impl DecodedBuySucceededBody {
    pub(super) fn has_issue_108_vendor_identity(&self) -> bool {
        self.body.vendor == ISSUE_108_VENDOR_IDENTITY
    }

    pub(super) fn issue_108_shape_error(&self) -> Option<String> {
        if !self.has_issue_108_vendor_identity() {
            return None;
        }
        if self.vendor_runtime_counter == 0 {
            return Some("issue-#108 vendor has a zero runtime GUID counter".to_string());
        }
        if self.body.muid != ISSUE_108_VENDOR_MUID {
            return Some(format!(
                "issue-#108 vendor MUID is {}, expected {}",
                self.body.muid, ISSUE_108_VENDOR_MUID
            ));
        }
        if self.body.new_quantity != ISSUE_108_VENDOR_NEW_QUANTITY {
            return Some(format!(
                "issue-#108 vendor NewQuantity is {}, expected {}",
                self.body.new_quantity, ISSUE_108_VENDOR_NEW_QUANTITY
            ));
        }
        if self.body.quantity_bought != ISSUE_108_VENDOR_QUANTITY_BOUGHT {
            return Some(format!(
                "issue-#108 vendor QuantityBought is {}, expected {}",
                self.body.quantity_bought, ISSUE_108_VENDOR_QUANTITY_BOUGHT
            ));
        }
        None
    }

    pub(super) fn is_issue_108_reviewed_shape(&self) -> bool {
        self.has_issue_108_vendor_identity() && self.issue_108_shape_error().is_none()
    }
}

pub(super) fn issue_106_loot_object_error(loot_obj: ExactObjectGuid) -> Option<String> {
    let stable = stable_object_guid(loot_obj.low, loot_obj.high);
    if stable.high_type != HIGH_GUID_LOOT_OBJECT
        || stable.realm_id != ISSUE_106_CREATURE_IDENTITY.realm_id
        || stable.map_id != ISSUE_106_CREATURE_IDENTITY.map_id
        || stable.entry != 0
        || stable.subtype != 0
        || stable.server_id != 0
    {
        return Some("loot_obj.high is not the reviewed map-530 LootObject identity".to_string());
    }
    if loot_obj.low & OBJECT_GUID_COUNTER_MASK == 0 {
        return Some("loot_obj.low has a zero runtime GUID counter".to_string());
    }
    None
}

impl DecodedLogXpGainBody {
    pub(super) fn is_creature_kill(&self) -> bool {
        self.body.reason == XP_GAIN_REASON_KILL && self.body.victim.high_type == HIGH_GUID_CREATURE
    }
}

pub(super) fn decode_update_object_inv_slots_candidate(body: &[u8]) -> UpdateObjectInvSlotsDecode {
    match decode_update_object_inv_slots_candidate_inner(body) {
        Ok(decoded) => UpdateObjectInvSlotsDecode::Candidate(decoded),
        Err(UpdateObjectInvSlotsFailure::NotEligible(reason)) => {
            UpdateObjectInvSlotsDecode::NotEligible(reason)
        }
        Err(UpdateObjectInvSlotsFailure::Malformed(error)) => {
            UpdateObjectInvSlotsDecode::Malformed(error)
        }
    }
}
