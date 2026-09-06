/* Equivalent to modules/encounter, not a complete C++ boss port.
 * Order anchors: boss_anomalus.cpp:154-168, TemporarySummon.cpp:249-264.
 */
#include "contract.h"

RC_METADATA(1, 1, RC_CAP_QUERY | RC_CAP_SHIELD | RC_CAP_SUMMON | RC_CAP_REENTRY, 12, 10)

typedef struct {
    u32 phase;
    u64 callbacks;
} EncounterState;

RC_EXPORT(initial_state_byte) i32 initial_state_byte(u32 index) {
    return index < 12 ? 0 : (i32)RC_INVALID;
}

static i64 decode_state(EncounterState *state, const u8 *bytes) {
    state->phase = load_u32(bytes);
    state->callbacks = load_u64(bytes + 4);
    return state->phase <= 1 ? 0 : RC_INVALID;
}

RC_EXPORT(validate_state) i32 validate_state(u32 length) {
    if (length > 12)
        return (i32)RC_LIMIT;
    if (length != 12)
        return (i32)RC_INVALID;
    u8 bytes[12];
    RC_TRY(validation_record(bytes, length));
    EncounterState state;
    return (i32)decode_state(&state, bytes);
}

static i64 read_state(EncounterState *state, u64 *revision) {
    u8 bytes[12];
    RC_TRY(read_record(bytes, 12, revision));
    return decode_state(state, bytes);
}

static i64 write_state(const EncounterState *state, u64 revision) {
    u8 bytes[12];
    store_u32(bytes, state->phase);
    store_u64(bytes + 4, state->callbacks);
    return write_record(bytes, 12, revision);
}

RC_EXPORT(invoke) i64 invoke(u32 event, i64 argument) {
    EncounterState state;
    u64 revision;
    switch (event) {
    case RC_UPDATE: {
        if (argument < 0 || argument > 1)
            return RC_INVALID;
        RC_TRY(read_state(&state, &revision));
        if (state.phase == 0) {
            state.phase = 1;
            RC_TRY(write_state(&state, revision));
            RC_TRY(rc_action(RC_ACTION_SHIELD, 1));
            /* Zero is nullable failure; phase/shield remain published. */
            RC_TRY(rc_action(RC_ACTION_SUMMON, argument));
        }
        i64 summons = rc_query(RC_QUERY_SUMMONS);
        RC_TRY(summons);
        RC_TRY(read_state(&state, &revision));
        return state.phase == 1 ? summons : RC_INVALID;
    }
    case RC_CALLBACK:
        if (argument < 0)
            return RC_INVALID;
        RC_TRY(read_state(&state, &revision));
        RC_TRY(rc_query(RC_QUERY_SHIELD));
        if (state.callbacks == RC_U64_MAX)
            return RC_OVERFLOW;
        ++state.callbacks;
        RC_TRY(write_state(&state, revision));
        if (argument > 0)
            RC_TRY(rc_action(RC_ACTION_REENTER, argument - 1));
        RC_TRY(read_state(&state, &revision));
        return state.callbacks <= RC_I64_MAX ? (i64)state.callbacks : RC_OVERFLOW;
    case RC_RESET:
    case RC_REMOVING:
        RC_TRY(read_state(&state, &revision));
        state.phase = 0;
        state.callbacks = 0;
        RC_TRY(write_state(&state, revision));
        RC_TRY(rc_action(RC_ACTION_SHIELD, 0));
        return 0;
    case RC_ATTACHED:
    case RC_DETACHED:
        return 0;
    case 1024: /* Deliberately stale outer snapshot after nested callback. */
        RC_TRY(read_state(&state, &revision));
        RC_TRY(rc_action(RC_ACTION_SUMMON, 1));
        state.phase = 1;
        RC_TRY(write_state(&state, revision));
        return 0;
    case 1025: /* Failure retains previously applied shield. */
        RC_TRY(rc_action(RC_ACTION_SHIELD, 1));
        return rc_action(RC_ACTION_FAIL, 0);
    default:
        return 0;
    }
}
