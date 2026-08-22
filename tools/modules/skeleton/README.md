# __MODULE_DISPLAY__

A RustyCore trusted linked module.

```bash
python3 tools/modules/rustycore-module install --path modules/__MODULE_DIR__
python3 tools/modules/rustycore-module build
```

## Layout

- `src/lib.rs` — the module and its `register` entry point.
- `tests/hook.rs` — a focused hook test.
- `conf/__MODULE_ID__.toml.example` — example configuration (see #231).
- `data/sql/{auth,characters,world,hotfixes}/` — SQL you apply yourself; the
  manager never runs SQL for you.

## Trust

This is in-process code compiled into the server binary with full privileges.
It requires a rebuild and restart to change. There is no isolation boundary,
no stable ABI and no hot reload.
