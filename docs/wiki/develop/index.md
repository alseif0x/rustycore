# Development and contributing

RustyCore is a behavioral port, not a greenfield reinterpretation. Gameplay, protocol,
database, map, and persistence changes must be contrasted with the legacy C++ server under
`/home/server/woltk-trinity-legacy` or with real packet evidence when the C++ source is
ambiguous.

The normal contribution flow is:

1. Start from an issue-linked feature branch based on `3.4.3`.
2. Locate and record the relevant C++ behavior before editing Rust.
3. Implement a bounded, faithful change with positive and negative tests.
4. Run focused crate checks during development.
5. Commit validated work, then run `./tools/validation-v2 final --base origin/3.4.3`
   on that clean HEAD before an authorized push. Run issue-specific acceptance checks too;
   site changes additionally require the local VitePress build described in this site's README.
6. After an authorized push, open or update the pull request into `3.4.3` with its issue
   closing keyword and remaining boundary. External authors also require remote checks and
   review. Publication does not itself authorize merge.

Useful project references:

- [Documentation map](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/README.md)
- [Shared operating guide](https://github.com/alseif0x/rustycore/blob/3.4.3/AGENTS.md)
- [Current migration state](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/STATE.md)
- [Port plan](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/PORT_PLAN.md)
- [Ownership and boundaries](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/ownership-and-boundaries.md)
- [Module design and responsibility separation](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/module-design-guidelines.md)
- [Delivered module tooling](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/modules.md)
- [Approved modularity/ECS plan](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/modularity-and-ecs-plan.md)
- [Local-first development](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/operations/local-first-development.md)

Do not mark represented code as live-runtime complete without exercising the actual runtime.
The delivered module tooling is not the complete planned native/Wasm product; the owning
plan and #578/#583 acceptance distinguish implemented behavior from pending integration.
