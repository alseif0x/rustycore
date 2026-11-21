# Remaining core architecture work

Use [#584](https://github.com/alseif0x/rustycore/issues/584) to select core work and [#49](https://github.com/alseif0x/rustycore/issues/49) for dependencies across the port. Check the [branch checkpoint](../migration/STATE.md) before using evidence from another client version.

| Responsibility | Acceptance and ownership |
| --- | --- |
| Creature runtime | The legacy owner was retired on 3.4.3. #1342 owns the real-client terminal scenarios. #1354 tracks relocation and visibility. Distinguish merged implementation from the live evidence required on each branch. |
| Persistence and publication | Preserve canonical identity, phase order, cancellation, commit/unknown-commit handling and recovery. Use the [ownership policy](ownership-and-boundaries.md) and the [quest reward contract](quest-reward-operation-contract.md) for their affected paths. |
| Remaining core and physical splits | #584 tracks remaining group/raid fields and pet, transport and vehicle lifecycles. Use the [C4 split plan](c4-physical-split-plan.md) to retain owner and size obligations, including those carried forward from retired exceptions. #1441 owns domain test moves. |
| Stateful native/Wasm modules | After required core boundaries are ready, #583 owns the shared hook/host contract, independent stateful modules, durable operation and receipt, author/operator lifecycle, and native, Rust-Wasm, C-Wasm and mixed execution. See [module tooling](modules.md) for the shipped surface. |
| Closure | #153 owns the independent terminal audit of production ownership, exceptions and product acceptance. The gameplay milestones and the full-port parity map retain their own exits. |

For each responsibility, trace callers, consumers and writers before choosing the cut. Preserve exact packet order, lifetime, persistence and access boundaries. Move tests and production integrations with their owner and record the checks and outstanding evidence in the issue. [Module design](module-design-guidelines.md) · [Detailed P0–P6 contracts and dated delivery evidence](https://github.com/alseif0x/rustycore/blob/d287f333d69d643ccfd7f84426e51a925836bded/docs/architecture/refactor-completion-plan.md)
