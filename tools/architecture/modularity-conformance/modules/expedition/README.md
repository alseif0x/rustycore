# Independent expedition module

Authored after freeze `5aeadb7a4a889bdfc879f9c69898c85325ea555479204ec303b9a8880fbc9424`
(two-module source HEAD `118171c1`). This is a custom stampbook, not a gameplay
parity claim, production SDK, reward receipt or database migration.

The crate depends only on `conformance-contract`. Module ID **73**, ABI/schema **1**,
order **30**, capabilities **QUERY | CONTRIBUTION**. Native and Rust Wasm execute this
same Rust rule; `c-guests/expedition.c` independently implements its specified bytes
and transitions in freestanding C through the frozen ABI. No canonical state is held
in guest globals. The frozen bindings' diagnostic markers are not module state.

## State and rules

The host owns a non-Clone typed state (or its canonical bytes): reset count `u32`,
lifetime accepted-checkpoint count `u64`, and a variable-length sorted unique list
of up to eight checkpoint IDs in `1..=31`. Encoding is exactly:

```text
offset 0: magic 0x45
offset 1: encoding version 1
offset 2..6: resets, little-endian u32
offset 6..14: accepted_total, little-endian u64 (>= current list length)
offset 14: list length, u8
offset 15..: strictly increasing checkpoint IDs, no trailing bytes
```

Default state is **15 bytes**, maximum **23**. Malformed order, duplicates, unknown
version/magic, invalid IDs/count/history and extra/truncated bytes are rejected,
not normalized. C supplies its own metadata because the frozen `RC_METADATA`
macro's fixed initial-length convention does not describe this codec.

- `STAMP = CUSTOM + 64` (1088): active residence required. A new valid checkpoint
  increments lifetime count, inserts in sorted order and contributes five units per
  current checkpoint. Duplicate input succeeds without a new write or action.
- `COUNT = CUSTOM + 65` (1089): returns the current count, including while detached.
- Detach keeps every state byte and suspends this module's contribution; attach
  restores it from the retained list. Neither transition resets identity/history.
- Reset clears the current list and contribution, increments reset count, and retains
  lifetime count. Removal clears only this module's contribution; host retirement
  then removes its state according to the experimental contract.
- Other events, including the encounter's callbacks and XP policy, return zero
  without changing state. Errors/overflow do not silently wrap or allocate unboundedly.

A state write followed by an action is **not atomic**. If the root budget expires
after accepting a stamp, that write remains and the contribution may lag; duplicate
input must not count it again. A later detach/attach reconciles the derived effect.
Tests expose this boundary; no rollback, restart durability or exactly-once reward
claim is inferred. Snapshot/replay is the lab's same-incarnation in-memory CAS only.

`driver/tests/expedition.rs` contains the four required real-composition integration
tests. Each runs the full lifecycle plus rejection cases, verifies that ID 73 actually
executes with the requested producer, and compares complete root returns, ordered
traces (including nested callback returns), revisions, bytes and observables against
an independently constructed native composition. Executor labels and pointer values
are diagnostic provenance, not semantic equality fields.
