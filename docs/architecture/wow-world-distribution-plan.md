# Distribution of wow-world

The 3.4.3 distribution programme closed with [#1263](https://github.com/alseif0x/rustycore/issues/1263). The retired creature runtime and map manager are recorded in #1355 and #1356. [#1342](https://github.com/alseif0x/rustycore/issues/1342) tracks outstanding real-client evidence. For active architecture work see the [refactor plan](refactor-completion-plan.md). The [original phase record](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/architecture/wow-world-distribution-plan.md) preserves the measurements, source anchors, accepted SHAs, holds and decisions for #1241/#1263.

## 3. Rules for a distribution move

- **R1.** Move the implementation and remove its original definition in the same commit. Check the source reduction against growth at the destination.
- **R2.** Preserve behavior, names, canonical paths and C++ anchors. Schedule a behavioral change as a separate delivery.
- **R3.** Each move must compile for `wow-world --all-targets` and the moved crate and pass the moved domain's tests.
- **R4.** Use compiler-guided codemods for repeated mechanical edits.
- **R5.** Report production lines in `wow-world` and the number of `impl WorldSession` methods. Preserve handler signatures, registration, packet order and bytes, phase admission, persistence and recovery, locking, and runtime ownership.

## 4. Compatibility with modules

- **M1.** Domain crates may depend on `wow-module-api` and `wow-script`. Keep those base crates independent of domains. Keep `WorldSession` and private domain state outside the public API.
- **M2.** Module registration and hook handles belong to `SessionCore`. Domain contexts expose the capabilities their logic needs. Preserve hook call position, order and context.
- **M3.** Each domain owns its existing hook call sites. New hooks belong to #583.
- **M4.** Per-crate registration uses inversion of control. `world-server` and `world-modules` are the composition points. F3/F4 moves must also pass the module and hook tests and the `compose.py check` fixture.

## 10. Acceptance for follow-ups that inherit the distribution contract

1. Record the committed SHA, comparison base, checkout, configurations and targets. Use one validation executor, one Cargo job by default and a separate target directory per active worktree.
2. Run the branch's local final with `--architecture --timings --logs`. Verify the manifest with `--require-profile final` and check its recorded head and base. The [validation guide](../operations/validation-v2.md) defines when evidence can be reused.
3. Credit commands already executed by that run. Cover any remaining integration and doc-test targets and the required all-target configurations for `wow-world` (including `test-fixtures`), affected domain crates, `world-server` and `world-modules`. Compare compiler warnings on equivalent configurations.
4. Preserve the full affected library suites, `cargo test -p world-server --lib`, release handler-contract tests, module hook tests and the compositor fixture. Compare test names across source and destination crates, features, ignored cases and configuration. Review every intentional identity or assertion change.
5. Verify the net move, visibility widening and production consumers. For changes to persistence, run the required inventory and snapshot checks. Add production, capture, runtime and physical-budget evidence where the changed operation requires it.
6. Measure the whole acceptance campaign, including any additional checks. Report cold bootstrap, repair, exhaustive audit and live QA separately with their real durations. The ordinary warm target is 600 seconds.

[Original acceptance and phase-specific details](https://github.com/alseif0x/rustycore/blob/5fe18d391a4adb43e72a764f9235d855761fdf24/docs/architecture/wow-world-distribution-plan.md#L8256-L8301)
