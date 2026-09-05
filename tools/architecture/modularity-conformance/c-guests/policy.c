/* Equivalent custom policy to modules/policy. This is not GiveXP parity.
 * C++ Player.cpp:2189-2226 anchors placement of a policy hook before award.
 */
#include "contract.h"

RC_METADATA(2, 1, RC_CAP_QUERY | RC_CAP_CONTRIBUTION, 12, 20)

typedef struct {
    u64 calls;
    u32 percent;
} PolicyState;

RC_EXPORT(initial_state_byte) i32 initial_state_byte(u32 index) {
    if (index >= 12)
        return (i32)RC_INVALID;
    return index == 8 ? 100 : 0;
}

static i64 decode_state(PolicyState *state, const u8 *bytes) {
    state->calls = load_u64(bytes);
    state->percent = load_u32(bytes + 8);
    return state->percent <= 1000 ? 0 : RC_INVALID;
}

RC_EXPORT(validate_state) i32 validate_state(u32 length) {
    if (length > 12)
        return (i32)RC_LIMIT;
    if (length != 12)
        return (i32)RC_INVALID;
    u8 bytes[12];
    RC_TRY(validation_record(bytes, length));
    PolicyState state;
    return (i32)decode_state(&state, bytes);
}

static i64 read_state(PolicyState *state, u64 *revision) {
    u8 bytes[12];
    RC_TRY(read_record(bytes, 12, revision));
    return decode_state(state, bytes);
}

static i64 write_state(const PolicyState *state, u64 revision) {
    u8 bytes[12];
    store_u64(bytes, state->calls);
    store_u32(bytes + 8, state->percent);
    return write_record(bytes, 12, revision);
}

RC_EXPORT(invoke) i64 invoke(u32 event, i64 argument) {
    PolicyState state;
    u64 revision;
    switch (event) {
    case RC_POLICY: {
        if (argument < 0 || argument > 1000000)
            return RC_INVALID;
        RC_TRY(read_state(&state, &revision));
        /* Bounds above make the signed product <= 10^9, without overflow. */
        i64 amount = argument * (i64)state.percent / 100;
        if (state.calls == RC_U64_MAX)
            return RC_OVERFLOW;
        ++state.calls;
        RC_TRY(write_state(&state, revision));
        RC_TRY(rc_action(RC_ACTION_CONTRIBUTION, (i64)state.percent));
        return amount;
    }
    case RC_CALLBACK:
        RC_TRY(rc_query(RC_QUERY_SHIELD));
        RC_TRY(read_state(&state, &revision));
        return state.calls <= RC_I64_MAX ? (i64)state.calls : RC_OVERFLOW;
    case RC_RESET:
    case RC_REMOVING:
        RC_TRY(read_state(&state, &revision));
        state.calls = 0;
        state.percent = 100;
        RC_TRY(write_state(&state, revision));
        RC_TRY(rc_action(RC_ACTION_CONTRIBUTION, 0));
        return 0;
    default:
        return 0;
    }
}
