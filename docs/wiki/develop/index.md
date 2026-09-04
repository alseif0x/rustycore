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
5. Run the repository validation required for the PR author before merge.
6. Open a pull request into `3.4.3` and describe the remaining boundary honestly.

Useful project references:

- [Current migration state](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/STATE.md)
- [Port plan](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/PORT_PLAN.md)
- [Ownership and boundaries](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/ownership-and-boundaries.md)
- [Local-first development](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/operations/local-first-development.md)

Do not mark represented code as live-runtime complete without exercising the actual runtime.
