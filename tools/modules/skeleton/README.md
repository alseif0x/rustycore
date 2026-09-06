# __MODULE_DISPLAY__

A RustyCore trusted linked module. This skeleton implements the current bounded
`player.login -> SendSystemMessageSelf` source API, not the planned stateful
gameplay or Wasm SDK.

After `rustycore-module new` creates this directory, run these commands from the
RustyCore repository root. Do not `install --path` the already-created checkout:
`install` rejects an existing destination.

```bash
python3 tools/modules/rustycore-module sync
python3 tools/modules/rustycore-module check
PROTOC=/home/ubuntu/.local/protoc/bin/protoc python3 tools/modules/rustycore-module build
```

## Layout

- `src/lib.rs` — the module and its `register` entry point.
- `tests/hook.rs` — a focused hook test.
- `module.toml` — module metadata and configuration defaults embedded at composition.
- `conf/__MODULE_ID__.toml.example` — example operator configuration; copy reviewed
  overrides to the core's `conf/modules/__MODULE_ID__.toml`, outside this checkout
  (see `docs/architecture/modules.md`).
- `data/sql/{auth,characters,world,hotfixes}/` — reserved SQL artifact locations,
  not registered or automatically applied module migrations.

Module-local SQL discovery and migration lifecycle are not implemented by this
skeleton or manager. Placing a file here neither adds it to the current
`rustycore-db` manifest nor authorizes applying it. Schema changes must use the
reviewed manifest/artifact path through `rustycore-db`, with the applicable
operator approval and recovery requirements; do not bypass that authority with
ad-hoc SQL or a module startup hook. Managed module migration integration and
retained upgrade/removal history remain #583 delivery work.

## Trust

This is in-process code compiled into the server binary with full privileges.
It requires a rebuild and restart to change. There is no isolation boundary,
no stable ABI and no hot reload.

`sync` rewrites composition/lock artifacts and `build` invokes Cargo; neither deploys
or restarts the running server. Review module code and build dependencies before compiling.
Installation is not permission to run SQL or delete a checkout containing local work.
Future shared hooks/state/lifecycle and the operator-optional Wasmtime/Core Wasm adapter
belong to #583; consult `docs/architecture/modularity-and-ecs-plan.md` for that approved scope.
