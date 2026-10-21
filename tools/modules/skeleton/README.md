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
- `data/sql/{characters,world}/` — where module migrations go once declared in
  `data/sql/manifest.toml` (none by default).

## SQL

A module that needs tables declares its migrations in `data/sql/manifest.toml`
(`docs/architecture/modules.md`, "Module SQL"). `compose.py` records each file's
checksum in `modules.lock.toml`; only an operator's explicit
`rustycore-db module apply <id>` (#1444 H6b) runs it. Tables carry the module's prefix
(`mod_<id with . as _>_`), only the `characters` and `world` databases are open
to modules, and a recorded file never changes. Do not run module SQL by hand or
from a startup hook.

## Trust

This is in-process code compiled into the server binary with full privileges.
It requires a rebuild and restart to change. There is no isolation boundary,
no stable ABI and no hot reload.

`sync` rewrites composition/lock artifacts and `build` invokes Cargo; neither deploys
or restarts the running server. Review module code and build dependencies before compiling.
Installation is not permission to run SQL or delete a checkout containing local work.
Future shared hooks/state/lifecycle and the operator-optional Wasmtime/Core Wasm adapter
belong to #583; consult `docs/architecture/modularity-and-ecs-plan.md` for that approved scope.
