#ifndef CONFORMANCE_CONTRACT_H
#define CONFORMANCE_CONTRACT_H

/* Private Core Wasm ABI 1; freestanding wasm32, no libc/WASI required.
 * Mirrors contract/src/guest.rs. These modules contain no canonical guest state.
 * Pointers/counts are u32 byte offsets, state/revision records little endian.
 * Host scopes identity, validates/copies ranges, and drops borrows before reentry.
 */
typedef unsigned char u8;
typedef unsigned int u32;
typedef int i32;
typedef unsigned long long u64;
typedef long long i64;
_Static_assert(sizeof(u8) == 1 && sizeof(u32) == 4 && sizeof(u64) == 8,
               "ABI requires wasm32 integer widths");

#define RC_EXPORT(name) __attribute__((export_name(#name)))
#define RC_IMPORT(name) __attribute__((import_module("conformance"), import_name(#name)))
#define RC_INVALID (-1LL)
#define RC_LIMIT (-8LL)
#define RC_MAX_STATE_BYTES 256u
#define RC_OVERFLOW (-12LL)
#define RC_QUERY_SHIELD 1u
#define RC_QUERY_SUMMONS 2u
#define RC_ACTION_SHIELD 1u
#define RC_ACTION_SUMMON 2u
#define RC_ACTION_CONTRIBUTION 3u
#define RC_ACTION_REENTER 4u
#define RC_ACTION_FAIL 5u
#define RC_UPDATE 1u
#define RC_CALLBACK 2u
#define RC_RESET 3u
#define RC_REMOVING 4u
#define RC_ATTACHED 5u
#define RC_DETACHED 6u
#define RC_POLICY 7u
#define RC_CAP_QUERY 1ULL
#define RC_CAP_SHIELD 2ULL
#define RC_CAP_SUMMON 4ULL
#define RC_CAP_CONTRIBUTION 8ULL
#define RC_CAP_REENTRY 16ULL
#define RC_U64_MAX (~0ULL)
#define RC_I64_MAX (RC_U64_MAX >> 1)

/* read: length >=0 / negative Fault, writes revision as 8 LE bytes.
 * write: 0 / negative Fault; query/action: nonnegative value / negative Fault.
 */
RC_IMPORT(read) i32 rc_read(u32 pointer, u32 capacity, u32 revision_pointer);
RC_IMPORT(write) i32 rc_write(u32 pointer, u32 length, u64 revision);
RC_IMPORT(query) i64 rc_query(u32 query);
RC_IMPORT(action) i64 rc_action(u32 action, i64 argument);
/* Only available during pure codec validation; ordinary imports have no authority. */
RC_IMPORT(validation_read) i32 rc_validation_read(u32 index);

static u32 load_u32(const u8 *bytes) {
    u32 value = 0;
    for (u32 index = 0; index < 4; ++index)
        value |= (u32)bytes[index] << (index * 8);
    return value;
}

static u64 load_u64(const u8 *bytes) {
    u64 value = 0;
    for (u32 index = 0; index < 8; ++index)
        value |= (u64)bytes[index] << (index * 8);
    return value;
}

static void store_u32(u8 *bytes, u32 value) {
    for (u32 index = 0; index < 4; ++index)
        bytes[index] = (u8)(value >> (index * 8));
}

static void store_u64(u8 *bytes, u64 value) {
    for (u32 index = 0; index < 8; ++index)
        bytes[index] = (u8)(value >> (index * 8));
}

static i64 read_record(u8 *bytes, u32 length, u64 *revision) {
    u8 encoded_revision[8];
    i32 result = rc_read((u32)bytes, length, (u32)encoded_revision);
    if (result < 0)
        return result;
    if ((u32)result != length)
        return RC_INVALID;
    *revision = load_u64(encoded_revision);
    return 0;
}

static i64 write_record(const u8 *bytes, u32 length, u64 revision) {
    i32 result = rc_write((u32)bytes, length, revision);
    return result <= 0 ? result : RC_INVALID;
}

#define RC_TRY(expression) do { i64 rc_status = (expression); \
    if (rc_status < 0) return rc_status; } while (0)

/* The caller supplies a buffer of length bytes. No canonical guest state. */
static i32 validation_record(u8 *bytes, u32 length) {
    if (length > RC_MAX_STATE_BYTES)
        return (i32)RC_LIMIT;
    for (u32 index = 0; index < length; ++index) {
        i32 value = rc_validation_read(index);
        if (value < 0)
            return value;
        if (value > 255)
            return (i32)RC_INVALID;
        bytes[index] = (u8)value;
    }
    return 0;
}

#define RC_METADATA(id, schema, caps, limit, order) \
    RC_EXPORT(abi_version) u32 abi_version(void) { return 1; } \
    RC_EXPORT(module_id) u64 module_id(void) { return id; } \
    RC_EXPORT(state_schema) u32 state_schema(void) { return schema; } \
    RC_EXPORT(capabilities) u64 capabilities(void) { return caps; } \
    RC_EXPORT(state_limit) u32 state_limit(void) { return limit; } \
    RC_EXPORT(module_order) i32 module_order(void) { return order; } \
    RC_EXPORT(initial_state_len) i32 initial_state_len(void) { return limit; }

/* Diagnostic marker only: not module state, persistence, or recovery authority. */
static volatile u32 diagnostic_stage;

RC_EXPORT(probe_stage) u32 probe_stage(void) {
    return diagnostic_stage;
}

RC_EXPORT(probe_grow) i32 probe_grow(u32 pages) {
    return (i32)__builtin_wasm_memory_grow(0, pages);
}

RC_EXPORT(probe_spin) void probe_spin(void) {
    for (;;) {
        __asm__ volatile("" ::: "memory");
    }
}

RC_EXPORT(probe_burn) u64 probe_burn(u32 iterations) {
    volatile u64 value = 0x9e3779b97f4a7c15ULL;
    for (u32 index = 0; index < iterations; ++index) {
        u64 current = value;
        value = ((current << 7) | (current >> 57)) ^
                ((u64)index * 0xd1342543de82ef95ULL);
    }
    return value;
}

RC_EXPORT(probe_nested) i64 probe_nested(u32 iterations) {
    diagnostic_stage = 1;
    u64 first = probe_burn(iterations);
    diagnostic_stage = 2;
    RC_TRY(rc_action(RC_ACTION_REENTER, 0));
    diagnostic_stage = 3;
    u64 second = probe_burn(iterations);
    diagnostic_stage = 4;
    return (i64)((first ^ second) & RC_I64_MAX);
}

/* Diagnostic rejection exercises the actual guest import and codec. */
RC_EXPORT(probe_write) i64 probe_write(u32 length, u32 index, u32 value) {
    if (length > RC_MAX_STATE_BYTES)
        return RC_LIMIT;
    if (index >= length || value > 255)
        return RC_INVALID;
    u8 bytes[RC_MAX_STATE_BYTES];
    u8 encoded_revision[8];
    i32 current = rc_read((u32)bytes, RC_MAX_STATE_BYTES, (u32)encoded_revision);
    RC_TRY(current);
    if ((u32)current > RC_MAX_STATE_BYTES)
        return RC_LIMIT;
    for (u32 position = (u32)current; position < length; ++position)
        bytes[position] = 0;
    bytes[index] = (u8)value;
    return write_record(bytes, length, load_u64(encoded_revision));
}

#endif
