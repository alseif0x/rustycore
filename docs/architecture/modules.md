# Trusted linked modules

Issue #228 earned the source API; issue #229 makes independent module repositories
compilable into the server through an explicit, reproducible step.

**Bounded source review:** local feature HEAD `7eaf8ddc`, 2026-09-05 (production code
`93e4002a`; no fresh live acceptance in this document review). #228–#231 provide
the source/build tooling, typed configuration and the bounded
`player.login -> SendSystemMessageSelf` contract. They do not yet provide general gameplay
policies, behavior scripting or host-managed persistent module state. The
[approved modularity and ECS plan](modularity-and-ecs-plan.md) defines the next #99
acceptance, implemented by the single #583 macro after #231/#578 and audited by #153
before #133 closes. #578 does not wait for the whole SDK. The approved direction below
is not an inventory of implemented APIs.

## Trust model, stated plainly

These are **in-process trusted modules, not sandboxed plugins**. A module is compiled
into the server binary, runs with its full privileges, and requires a rebuild and
restart to change. There is no isolation boundary, no stable native ABI, and no hot
reload. Install only code you would merge.

What the API does prevent is *accidental* reach: `wow-module-api` is classified
`foundation` with an empty external allowlist, so a module cannot obtain a
`WorldSession`, `Player`, `Map`, database pool or packet writer through a type.

## Layout

```
modules/<checkout>/module.toml     operator-managed, untracked
modules.lock.toml                  generated record of what was composed
crates/world-modules/              generated compositor crate
tools/modules/compose.py           sync | check
tools/modules/fixtures/            copyable example module
```

## `module.toml`

```toml
[module]
id = "example.greeter"          # lowercase [a-z0-9_.], starts with a letter
version = "1.0.0"
display_name = "Example Greeter"
order = 0                       # optional; operator composition order

[build]
package = "example-greeter"     # the Cargo package this checkout provides
crate_path = "."                # relative to this manifest, must stay inside it
registrar = "example_greeter::register"

[compatibility]
source_api = "1"
```

## Composing

```bash
python3 tools/modules/compose.py sync     # regenerate lock + compositor
PROTOC=/home/ubuntu/.local/protoc/bin/protoc cargo build -p world-modules  # composed server
python3 tools/modules/compose.py check    # CI: fail if the tree drifted
```

`sync` is an **explicit operator step**. The build never runs it or the module manager's
Git installation/update workflow; no compositor `build.rs` discovers or rewrites the
source tree implicitly. Ordinary dependency downloads still follow Cargo's configuration.

The compositor invokes registrars in the operator's `order`, then module id
(`tools/modules/compose.py::ordered` / `render_main`). Current login callbacks instead run
in `ModuleId` order, because `ModuleRegistry` stores them in a `BTreeMap`. These are two
distinct deterministic orders, neither inferred from linker inventory. Operator composition
order does not currently set callback order; future modifying hooks need an explicit
composition/conflict contract rather than inheriting this distinction accidentally.

## What the compositor refuses before compiling

- an id, version, package, crate path or registrar that fails its format rule;
- a `crate_path` that escapes the module checkout;
- a duplicate module id or duplicate Cargo package across checkouts;
- a `source_api` this server does not provide.

## `modules.lock.toml`

Records identity, version, package, source path, requested ref, resolved commit,
registrar, source API, enabled order and a content digest for every composed module.
It holds no credentials and no URLs with secrets.

## The zero-module build is unchanged

With nothing under `modules/`, the compositor is a no-op that calls
`world_server::run_with_modules` with an empty registry, and `world-server`'s own
binary still calls `world_server::run`. That entry creates an empty registry; the login adapter
checks `registry.is_empty()` and returns before constructing a snapshot or invoking callbacks.
This is the current zero-module source path, not fresh capture proof for every server action.

## The module manager

`tools/modules/rustycore-module` is the author/operator workflow. Every command is
non-interactive and never prompts, so a shell, a script or an agent drives it the same way.

| Command | Does |
|---|---|
| `new <id>` | scaffold from the official skeleton |
| `install --path P` / `--git URL [--ref R] [--commit C]` | register a checkout |
| `update <checkout> [--ref R] [--commit C]` | fetch a Git checkout; select a revision when requested |
| `remove <id>` | delete exactly one validated checkout |
| `list` | report installed modules |
| `sync` / `check` | regenerate, or verify without writing |
| `build` / `test` | cargo build/test the composed server |
| `doctor` | diagnose the installation |

Add `--json` to any command for machine-readable manager reports; manager errors go to stderr
as JSON carrying the same exit code. `build` and `test` also inherit Cargo's output, so their
entire stdout is not guaranteed to be a single JSON document.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | usage or validation error |
| 2 | requested module not found |
| 3 | source or network error |
| 4 | refused: dirty or conflicting checkout |

### Safety

- Only `install` and `update` perform manager-controlled Git network operations. Cargo
  build/test may download dependencies and execute build scripts from the trusted source.
- Neither ever executes a script from the fetched repository: the manager clones or copies,
  reads `module.toml`, and stops.
- A rejected install leaves nothing behind — the partial checkout is removed.
- `update` refuses a checkout with local modifications rather than discarding them.
- `remove` resolves the id to exactly one validated checkout inside `modules/` and refuses any
  path that escapes it.
- The manager never runs SQL. `data/sql/` is a reserved layout, not an integrated migration
  mechanism or permission to bypass the project's explicit database-migration authority.

The Git v1→v2 workflow still needs product-level acceptance. Inspection found that
`install` calls `_pin`, which writes revision metadata into the checkout's `module.toml`,
while `update` rejects any dirty checkout before fetching. This can make the manager's own
metadata block an update; the risk has **not been reproduced** in this review. Existing
local-path/composition tests do not establish a successful Git upgrade and restart cycle.

### The skeleton

`rustycore-module new` produces a module that compiles and tests as-is: manifest, `src/lib.rs`
with a working `register`, a focused hook test asserting the module greets only on first login,
an example configuration, `data/sql/{auth,characters,world,hotfixes}/` and a README stating the
trust model. Optional directories are documented rather than generated empty.

## Typed configuration

Options are namespaced by validated `ModuleId`, so two modules cannot collide on a key.

```toml
# module.toml — immutable package defaults, shipped with the module
[config]
enabled = true
welcome_text = "Example Greeter is installed."
```

```toml
# conf/modules/example.greeter.toml — operator overrides, outside the module repository
welcome_text = "Welcome to our realm!"
```

The override path is outside the checkout so module source updates do not replace that file.
Keep credentials out of module repositories and embedded options: composition writes typed
configuration into generated source/binaries, which are not secret storage. A separate path
does not itself guarantee that configuration values contain no secrets.

`sync` merges defaults with overrides, validates them, and **embeds the typed result** in the
generated compositor. A module therefore reads its configuration exactly once, at registration:
no callback touches a file, and there is no live reload to race against.

In the current skeleton, `enabled` is interpreted by the module itself: it suppresses the
greeting but does not prevent registration. It is not a host-managed disable/unload contract.
Changing embedded configuration requires sync, rebuild and restart.

### Validation happens before activation

A module takes the options it knows about and calls `finish()`. Anything left over is an
operator typo and fails registration, so a misspelled key is never silently ignored. Wrong types
and values the type system cannot express — a blank string, a negative count — are refused the
same way. A module that refuses its configuration is **not registered**, so an invalid value is
caught at startup rather than at a player's login.

### Digest

Every module's exact configuration has a deterministic digest, recorded in `modules.lock.toml`
and reported by `list` and `doctor`. It is computed identically on both sides — the Rust
`ModuleConfig::digest` and `compose.py` pin the same literals in their tests — so the lock always
describes what the module will actually see. Insertion order does not affect it, and the digest
is fixed at construction so it still describes what the module was *given* after it has read
its options.

## Source API compatibility

`module.toml` declares the `source_api` it was written against. Composition refuses anything this
server does not provide, with an actionable message, before a single line is compiled:

```
modules/incompatible_api/module.toml: source_api '2' is not supported;
this server provides ['1']. Update the module or pin an older server.
```

`tools/modules/fixtures/incompatible_api/` is that fixture, and `doctor --json` reports
`supported_source_apis` alongside each module's own.

## Approved next direction — #583 under #99, not implemented yet

The [modularity and ECS plan](modularity-and-ecs-plan.md) keeps external extension contracts
independent of private runtime storage. #99 owns the module product; #578 retains its full
canonical ownership/lifetime/phase acceptance and supplies the boundaries the modules use.
#583 is the single next functional module macro; #153 audits it and #578 before #133 closes.
Use coherent macrodeliverables with internal checkpoints, not a hook-by-hook micro-PR tree.
Neither a pilot module nor a backend choice closes the full #133/#578 contract.

### Useful extension contracts

Earn three kinds of capability through real modules, without prebuilding a universal framework:

- **Pre-decision policies:** typed allow/deny/modify results applied at a named point before
  the canonical transition, with explicit ordering, conflicts and failure behavior.
- **Behavior capabilities:** scoped actions and typed replies, including interactions that
  trigger nested/reentrant callbacks. Define immediate versus deferred execution, lifetime,
  cancellation and ordering per interaction; scripts are not all passive observers, and
  deferring every action can change C++ behavior.
- **Confirmed-result notifications:** immutable observations after the relevant transition
  and persistence boundary, not a veto after the change is committed.

The host validates and applies permitted actions through canonical owners. Modules receive no
Session/Map internals, entity guards, SQL connections, ECS handles or packet writers. Preserve
C++ hook placement and base-server behavior with zero optional modules (required base scripts
remain enabled); optional custom rules have their
own explicit contract and cannot bypass core identity, economy or transaction invariants.

### Selected execution product — native Rust and operator-optional Wasm

This is **planned #583 functionality**, not a new interpretation of the currently implemented
source manager commands above. First-party and custom modules use the same extension contract;
provenance does not select an executor or make native code isolated.

- Native Rust remains the default: independent crates, explicit composition/build/restart.
- Wasmtime/Core Wasm is the selected optional adapter. Deliver versioned ABI/bindings and a
  real Rust and C guest before claiming multilang support; broader language support or WIT is
  not implied. V1 tested Rust -> Core Wasm only, not the second language or production loader.
- Share the semantic hooks, scoped actions/results, error categories and state/lifecycle rules.
  Native representations and Wasm encodings may differ. Native registered types must not require
  host cases per module; Wasm state is bounded/versioned data, not Rust pointers or ECS components
  exported to the guest. Prevent stale outer state writes after synchronous nested callbacks.
- Native and Wasm modules can compose in one host. Select exactly one executor/artifact per ID;
  reject duplicates, incompatible versions/capabilities and exclusive conflicts before dispatch.
- Schema version is distinct from mutation revision: reject stale outer state writes after
  nested callbacks. An executor switch with existing data needs a compatible durable format or
  explicit validated conversion; unsupported switches reject before callbacks with data retained.
- Wasm imports have explicit capabilities, no ambient filesystem/network/DB access, and bounded
  guest memory, nested-invocation fuel, depth, actions, payloads and host work. Fuel does not bound
  blocking host I/O. Test trap/error partial effects and the hook's reject/disable/fallback policy.
- Ship native-only and Wasm-enabled configurations. Artifact replacement is an explicit validated
  lifecycle operation with restart/drain semantics, not automatic hot reload. A new compatible
  Wasm guest can be loaded on restart without rebuilding core.

Optional **to enable** is not optional **to deliver**: this expands #583 and therefore #133/#153
acceptance. The bounded executor/Rust+C proof does not wait for M6; broader ecosystem expansion
still receives a fresh review. Existing native trust warnings remain true.

`hecs` is selected privately for composable entity state; cohesive domain aggregates remain.
Neither execution adapter depends publicly on hecs. The finite third-module conformance proof in
the [plan §5](modularity-and-ecs-plan.md#5-ecs-decision-now-selective-private-hecs-cohesive-aggregates-retained)
runs before production storage migration, without waiting for all of #583. It has not run yet.

### State, persistence and removal

Module-owned state needs a host-managed namespace, declared scope (for example character,
map instance or realm), version and lifetime. Define transfer/detach, unload, restart and
retirement behavior for each scope; persist only declared durable state. This is extension
state, not a second Player, money balance or inventory. Concrete storage APIs/formats must be
earned by the use case, not frozen here as a generic all-database repository.

A reward and the module's durable receipt/progress must have a coherent, retry-safe
commit/recovery contract. Independently saving each side is not sufficient. Confirmed-result
publication must not duplicate a reward after a lost response or restart. Preserve the same
logical reward/receipt identity across retries, new runtime incarnations and schema/config
upgrades; a data version must not accidentally become a new reward entitlement.

Extend the existing `rustycore-db` authority for approved module migrations: review/dry-run,
immutable checksums, component history and explicit recovery. Current runtime validation uses
the embedded core manifest and rejects applied history absent from that manifest
(`wow-database/src/migration.rs::bundled_manifest` / `build_report`). Module integration must
retain verifiable migration history and required artifacts independently of the active module
checkout; existing `Archived` entries are a useful primitive, not that completed integration.
**Disable is not purge:** deactivate callbacks while retaining data/history; deleting module
code must not implicitly delete data. Purging requires a separate explicit operation.
MariaDB DDL may already be committed after failure: do not promise automatic rollback or a
safe binary downgrade without a proven compatible schema or explicit recovery procedure.

### Product acceptance

An independent repository must exercise useful customization through the public contract,
with no core patch or forbidden internal dependency. For a stateful module, acceptance includes
Git v1 installation, configuration, real runtime use, restart/state recovery, an approved v1→v2
migration, disable/restart with retained state, and reactivation or code removal with an honest
data-retention contract. Exercise failed upgrades, incompatible versions, interrupted operations,
duplicate requests and reward/receipt recovery where applicable. Preserve the last usable
installation until the replacement's required checks pass; distinguish source composition,
build, schema compatibility and runtime activation instead of treating `install` as deployment.

Also exercise the same supported behavior as native Rust, Rust Wasm and C Wasm, plus mixed
executor composition, zero optional modules, incompatible ABI/capabilities, duplicate execution
rejection, resource exhaustion and preserved partial effects. The real install/update/disable
workflow must cover the Wasm artifact as well as the native source module. Do not present lab
snapshot/replay as MariaDB durability or an imported guest as completed encounter integration.

These are implementation acceptance requirements, not results of this documentation update.
Use focused tests during internal cuts and the relevant production/live evidence at acceptance;
existing migration, deployment, push and merge approvals remain unchanged.

Follow the [shared reanalysis cadence](modularity-and-ecs-plan.md#reanalysis-checkpoints--evidence-before-replication):
#578 first proves conformance and a real C1/C2 vertical before replication, then completes its
C4 balance. #583 tests its first independent production module before extending the API and
finishes the durable/operator contract before #153 audits both macros. These are internal
evidence checkpoints, not another issue or routine approval for each hook.

## Remaining boundaries

Remote checkout management and typed configuration are already implemented by #230/#231.
The broader policy/behavior/state contracts and integrated migration/disable lifecycle above
remain #583 work under #99, now including the operator-optional Wasm adapter and Rust/C proof.
No production sandbox, native ABI stability or hot reload exists. The selected executor/backend
direction is not a claim of installation or acceptance; current source manager commands and the
bounded login hook remain the implemented product at the reviewed code above.
