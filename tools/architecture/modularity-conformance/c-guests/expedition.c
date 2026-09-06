/* Independent C producer for the custom expedition contract documented in
 * modules/expedition/README.md. No canonical guest globals, libc or WASI.
 */
#include "contract.h"

#define EXP_HEADER 15u
#define EXP_MAX_CHECKPOINTS 8u
#define EXP_LIMIT 23u
#define EXP_STAMP 1088u
#define EXP_COUNT 1089u
#define EXP_QUERY_RESIDENCE 4u
#define EXP_NOT_ACTIVE (-3LL)

/* Default length differs from the maximum: do not use fixed-size RC_METADATA. */
RC_EXPORT(abi_version) u32 abi_version(void) { return 1; }
RC_EXPORT(module_id) u64 module_id(void) { return 73; }
RC_EXPORT(state_schema) u32 state_schema(void) { return 1; }
RC_EXPORT(capabilities) u64 capabilities(void) {
    return RC_CAP_QUERY | RC_CAP_CONTRIBUTION;
}
RC_EXPORT(state_limit) u32 state_limit(void) { return EXP_LIMIT; }
RC_EXPORT(module_order) i32 module_order(void) { return 30; }
RC_EXPORT(initial_state_len) i32 initial_state_len(void) { return EXP_HEADER; }

RC_EXPORT(initial_state_byte) i32 initial_state_byte(u32 index) {
    if (index >= EXP_HEADER)
        return (i32)RC_INVALID;
    return index == 0 ? 0x45 : (index == 1 ? 1 : 0);
}

typedef struct {
    u32 resets;
    u64 accepted_total;
    u32 count;
    u8 checkpoints[EXP_MAX_CHECKPOINTS];
} ExpeditionState;

static i64 decode_state(ExpeditionState *state, const u8 *bytes, u32 length) {
    if (length > EXP_LIMIT)
        return RC_LIMIT;
    if (length < EXP_HEADER || bytes[0] != 0x45 || bytes[1] != 1)
        return RC_INVALID;
    u32 count = bytes[14];
    if (count > EXP_MAX_CHECKPOINTS || length != EXP_HEADER + count)
        return RC_INVALID;
    state->resets = load_u32(bytes + 2);
    state->accepted_total = load_u64(bytes + 6);
    state->count = count;
    if (state->accepted_total < count)
        return RC_INVALID;
    for (u32 index = 0; index < count; ++index) {
        u8 checkpoint = bytes[EXP_HEADER + index];
        if (checkpoint == 0 || checkpoint > 31 ||
            (index != 0 && state->checkpoints[index - 1] >= checkpoint))
            return RC_INVALID;
        state->checkpoints[index] = checkpoint;
    }
    return 0;
}

RC_EXPORT(validate_state) i32 validate_state(u32 length) {
    if (length > EXP_LIMIT)
        return (i32)RC_LIMIT;
    u8 bytes[EXP_LIMIT];
    RC_TRY(validation_record(bytes, length));
    ExpeditionState state;
    /* The decoder admits exactly one encoding for each state: no sorting,
     * padding, ignored bytes, or implicit reserved fields. */
    return (i32)decode_state(&state, bytes, length);
}

static i64 read_state(ExpeditionState *state, u64 *revision) {
    /* Frozen read_record is for fixed-size records, unlike this variable codec. */
    (void)read_record;
    u8 bytes[EXP_LIMIT];
    u8 encoded_revision[8];
    i32 length = rc_read((u32)bytes, EXP_LIMIT, (u32)encoded_revision);
    RC_TRY(length);
    RC_TRY(decode_state(state, bytes, (u32)length));
    *revision = load_u64(encoded_revision);
    return 0;
}

static i64 write_state(const ExpeditionState *state, u64 revision) {
    u8 bytes[EXP_LIMIT];
    bytes[0] = 0x45;
    bytes[1] = 1;
    store_u32(bytes + 2, state->resets);
    store_u64(bytes + 6, state->accepted_total);
    bytes[14] = (u8)state->count;
    for (u32 index = 0; index < state->count; ++index)
        bytes[EXP_HEADER + index] = state->checkpoints[index];
    return write_record(bytes, EXP_HEADER + state->count, revision);
}

RC_EXPORT(invoke) i64 invoke(u32 event, i64 argument) {
    ExpeditionState state;
    u64 revision;
    switch (event) {
    case EXP_STAMP: {
        if (argument < 1 || argument > 31)
            return RC_INVALID;
        i64 residence = rc_query(EXP_QUERY_RESIDENCE);
        RC_TRY(residence);
        if (residence == 0)
            return EXP_NOT_ACTIVE;
        RC_TRY(read_state(&state, &revision));
        u32 position = 0;
        while (position < state.count && state.checkpoints[position] < argument)
            ++position;
        if (position < state.count && state.checkpoints[position] == argument)
            return state.count;
        if (state.count == EXP_MAX_CHECKPOINTS)
            return RC_LIMIT;
        if (state.accepted_total == RC_U64_MAX)
            return RC_OVERFLOW;
        for (u32 index = state.count; index > position; --index)
            state.checkpoints[index] = state.checkpoints[index - 1];
        state.checkpoints[position] = (u8)argument;
        ++state.count;
        ++state.accepted_total;
        RC_TRY(write_state(&state, revision));
        RC_TRY(rc_action(RC_ACTION_CONTRIBUTION, (i64)state.count * 5));
        return state.count;
    }
    case EXP_COUNT:
        RC_TRY(read_state(&state, &revision));
        return state.count;
    case RC_ATTACHED:
    case RC_DETACHED: {
        i64 residence = rc_query(EXP_QUERY_RESIDENCE);
        RC_TRY(residence);
        i64 expected = 0;
        if (event == RC_ATTACHED) {
            if (argument < 0 || argument > 255)
                return RC_INVALID;
            expected = argument + 1;
        }
        if (residence != expected)
            return RC_INVALID;
        RC_TRY(read_state(&state, &revision));
        if (state.count != 0) {
            RC_TRY(rc_action(RC_ACTION_CONTRIBUTION,
                             residence == 0 ? 0 : (i64)state.count * 5));
        }
        return state.count;
    }
    case RC_RESET:
        RC_TRY(read_state(&state, &revision));
        if (state.resets == 0xffffffffu)
            return RC_OVERFLOW;
        ++state.resets;
        state.count = 0;
        RC_TRY(write_state(&state, revision));
        RC_TRY(rc_action(RC_ACTION_CONTRIBUTION, 0));
        return 0;
    case RC_REMOVING:
        RC_TRY(rc_action(RC_ACTION_CONTRIBUTION, 0));
        return 0;
    default:
        return 0;
    }
}
