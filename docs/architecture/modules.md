# Trusted linked modules

Issue #228 earned the source API; issue #229 makes independent module repositories
compilable into the server through an explicit, reproducible step.

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
cargo build -p world-modules              # build the server with modules
python3 tools/modules/compose.py check    # CI: fail if the tree drifted
```

`sync` is an **explicit operator step**. The build never runs it: cargo never fetches
from the network, and no `build.rs` discovers or rewrites the source tree implicitly.

Composition order is the operator's `order`, then module id — never registration order
and never linker inventory, so the same installed set always produces the same sequence.

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
binary still calls `world_server::run`. A server without modules never consults a
registry and its capture and state behaviour are untouched.

## The module manager

`tools/modules/rustycore-module` is the author/operator workflow. Every command is
non-interactive and never prompts, so a shell, a script or an agent drives it the same way.

| Command | Does |
|---|---|
| `new <id>` | scaffold from the official skeleton |
| `install --path P` / `--git URL [--ref R] [--commit C]` | register a checkout |
| `update <id>` | refresh a Git checkout to its pinned or requested ref |
| `remove <id>` | delete exactly one validated checkout |
| `list` | report installed modules |
| `sync` / `check` | regenerate, or verify without writing |
| `build` / `test` | cargo build/test the composed server |
| `doctor` | diagnose the installation |

Add `--json` to any command for machine output. **Stdout carries JSON and nothing else**, so an
agent can parse it without stripping prose; errors go to stderr as JSON carrying the same exit
code.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | usage or validation error |
| 2 | requested module not found |
| 3 | source or network error |
| 4 | refused: dirty or conflicting checkout |

### Safety

- Only `install` and `update` touch the network.
- Neither ever executes a script from the fetched repository: the manager clones or copies,
  reads `module.toml`, and stops.
- A rejected install leaves nothing behind — the partial checkout is removed.
- `update` refuses a checkout with local modifications rather than discarding them.
- `remove` resolves the id to exactly one validated checkout inside `modules/` and refuses any
  path that escapes it.
- The manager never runs SQL. `data/sql/` is yours to apply.

### The skeleton

`rustycore-module new` produces a module that compiles and tests as-is: manifest, `src/lib.rs`
with a working `register`, a focused hook test asserting the module greets only on first login,
an example configuration, `data/sql/{auth,characters,world,hotfixes}/` and a README stating the
trust model. Optional directories are documented rather than generated empty.

## Out of scope here

Remote repository management, typed configuration, SQL, Wasm and live reload —
#230 and #231.
