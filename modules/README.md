# Operator module checkouts

Place trusted Rust module repositories here, one directory per module, each with a
`module.toml` at its root. Then run:

```bash
python3 tools/modules/compose.py sync
cargo build -p world-modules
```

Nothing in this directory is tracked by git except this README and `.gitignore`: the
checkouts belong to the operator, and `modules.lock.toml` records exactly what was
composed.

**These are in-process trusted modules, not sandboxed plugins.** They are compiled into
the server binary, run with its full privileges, and require a rebuild and restart to
change. There is no isolation boundary, no stable ABI and no hot reload.

This describes the delivered native source/build workflow, not the planned Wasm executor.
See [the current module guide](../docs/architecture/modules.md) for operator commands and
[the modularity plan](../docs/architecture/modularity-and-ecs-plan.md) for #583's pending
native/Wasm stateful contracts. Apply the shared
[module design guidelines](../docs/architecture/module-design-guidelines.md) to module code/tests.
