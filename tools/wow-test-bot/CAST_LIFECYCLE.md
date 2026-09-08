# Player cast lifecycle QA (#589)

The integrated bot uses the same ordinary login, packet capture, encryption and
two-socket drain as other scenarios. This mode performs no account provisioning,
fixture SQL, service restart or data overlay installation. Login, casting and
normal logout still mutate live server state: use only authorized accounts and
runtime targets.

After building the bot at the candidate source, run against either Classic or
Rust with its existing private configuration and an explicit action plan:

```sh
tools/wow-test-bot/target/debug/wow-test-bot \
  --config /absolute/private/bot-config.json --single TESTBOT1@bot.local \
  --cast-lifecycle-plan /absolute/private/cast-plan.json \
  --report /absolute/private/cast-report.json
```

Use the actual binary location selected by `CARGO_TARGET_DIR`. Do not combine this
mode with `--login-only`, other post-login modes, or `--ensure-test-accounts`.
`WOW_BOT_CAST_LIFECYCLE_PLAN` also selects the mode. Retain the existing capture
configuration and a distinct report/capture destination for each paired run.

`cast-lifecycle-plan.example.json` describes the explicitly synthetic #587
30798→6197 fixture, not stock gameplay and not evidence of a completed #589 run.
It requires the reviewed effective data, known source spell and matching server
visual. Do not run it blindly against an unrelated character or data set.

## Plan and acceptance

Actions are `cast` (spell ID, unique client CastID and optional unit target),
`wait` (milliseconds), and `cancel` (`phase: active` or `pending`, cast ID and spell
ID). Omitted/null target preserves the empty wire target; a unit target is
`{"low": ..., "high": ...}`. Active cancellation sends 0x329F with packed CastID
and spell ID. Pending cancellation sends 0x3182 with an empty body; its IDs in the
plan document the intended pending request.

The ordered `expect` list names `prepare`, `start`, `go`, `cast_failed`,
`spell_failure` or `spell_failed_other`. Start/Go require a spell ID and target
(`"empty"` or `{"unit": {"low": ..., "high": ...}}`). Use client IDs for the
caster: preceding SpellPrepare must establish the server ID. An observer with a
known server ID can use `server_cast_id` explicitly instead. Failure before
preparation can reference the original client request directly.

Start/Go can require flags and `metadata` (visual ID, OriginalCastID, cast time and
the exact `remaining_power` rows). Go also supports hit/miss counts and the
full-combat-log bit; failures support their reason. All named events must arrive
on INSTANCE, as registered by Classic `Opcodes.cpp`; the same packet seen on the
realm socket fails.

Each cast action needs a distinct client CastID: reusing one is rejected at load
because it would make the SpellPrepare mapping ambiguous. Every observed
SpellPrepare/SpellStart/SpellGo/CastFailed/SpellFailure/SpellFailedOther that
resolves — directly or through an observed SpellPrepare mapping — to an identity
named anywhere in `expect` must be consumed by one of those rules. An unconsumed
plan-bound packet fails the run, so a Go after cancellation, a duplicated
Start/Go, a second SpellPrepare for the same client CastID or an unlisted
SpellPrepare cannot be hidden by an otherwise matched failure. List every
plan-bound packet you expect, including SpellPrepare. Ambient cast traffic for
an unrelated identity is still recorded but does not fail acceptance.

Any packet the decoder cannot read exactly — truncated, over-long or with a
structurally wrong optional section — is retained with its `error` and fails the
run; a wrong payload can never be matched. Reports retain ordered decoded events,
hashes, identities, power/target sections and connection, including unmatched
ambient events. A process exit, a decode error or an empty expectation list does
not establish a pass.

Use separate plans for instant/timed casts, the 400 ms queue boundary, replacement,
active/pending cancellation and late failure. Script timings alone do not prove
the exact boundary: retain observed preparation and cast duration, and use the
deterministic application tests for 400/401 ms admission. A late-power scenario
needs an independently prepared, authorized source of the resource change.

## Paired caster/observer correlation

A nearby observer does not receive SpellPrepare, so it never learns the client
CastID. `observe_only` collects facts and can never pass: the loader forbids
actions and expectations in that mode, and the evaluator additionally forces
`passed: false` for any `observe_only` evidence. The bot exits unsuccessfully by
design. Collection is not QA acceptance, and an observed server CastID must never
be relabelled as a client CastID.

Reproducible procedure for paired acceptance, using two pre-existing authorized
accounts and a separate report path per session:

1. **Discovery pair.** Start the observer with
   `{"observe_only": true, "timeout_ms": 10000, "actions": [], "expect": []}`,
   then run the caster with its full plan. Both reports are kept; both runs are
   expected to be unsuccessful for the observer.
2. **Read the mapping from the caster only.** Take `client_cast_id` →
   `server_cast_id` from the caster report's `spell_prepare` event. Do not infer
   it from a failure, from packet order or from the observer capture.
3. **Author the observer acceptance plan.** Write the observer's `expect` rules
   with that explicit `server_cast_id` (no `client_cast_id`), the same spell ID,
   target shape, flags and metadata the caster proved.
4. **Acceptance pair.** Re-run the identical caster plan together with the
   observer acceptance plan. Both reports must show `passed: true`, and the
   observer's `event_sequence` values must be present for every rule.
5. **Compare the pair.** Diff the caster and observer `spell_start`/`spell_go`
   `body_sha256`, identities, targets and connection before claiming scoped live
   acceptance.

The server CastID is per cast attempt: step 3's plan is only valid for the run in
step 4, and step 4 must be re-done whenever the caster plan changes. If step 1
produced no SpellPrepare, if a report is truncated, or if any rule is unmatched,
the pair FAILS — there is no partial credit and no manual annotation of a pass.

## Standalone tests and completion limits

The bot is outside the root workspace and has its own manifest:

```sh
CARGO_BUILD_JOBS=1 cargo test --manifest-path tools/wow-test-bot/Cargo.toml \
  --locked --offline --bin wow-test-bot cast_lifecycle
```

Those tests cover the request builders, the 3.4.3 decoders including every
optional SpellCastData section, plan validation, identity correlation, ordering,
socket selection, the unexpected-event rejection and the run-result gate. They
are deterministic byte-level tests: they do not prove any live server behavior.

The scenario requests normal logout and requires an empty LogoutComplete on
REALM; a logout on the instance socket, one carrying a payload, a missing
confirmation or a transport error fails the run. It does not query saved database
state, prove durability, or establish full spell/effect parity. Pair runtime/data
identities and compare the relevant packets before claiming scoped live
acceptance; #587 save/relogin retains its separate persistence checks.
