# Help build RustyCore

There is more than one way to contribute. You can write Rust, reproduce a bug,
investigate protocol behavior or make the documentation easier to follow.

## Pick a starting point

| If you enjoy… | Try this |
| --- | --- |
| Writing and explaining | Improve a setup step or clarify a confusing guide |
| Testing and investigation | Reproduce an issue with the server revision and client build recorded |
| Rust development | Coordinate an open issue and implement its complete agreed scope |
| WoW internals | Compare a specific behavior with versioned reference source and captures |

Browse [open issues](https://github.com/alseif0x/rustycore/issues), or ask where you
can help on [Discussions](https://github.com/alseif0x/rustycore/discussions) and
[Discord](https://discord.gg/mH6ACpGPb2). Check the
[port plan](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/PORT_PLAN.md)
before starting substantial work so dependencies and overlapping contributions are clear.

## Your first pull request

1. Follow [server setup](../server/setup) to build the project.
2. Read the [contributor guide](https://github.com/alseif0x/rustycore/blob/3.4.3/CONTRIBUTING.md)
   and coordinate the scope on an issue.
3. Work on a feature branch based on **3.4.3**. Include affected consumers, tests and docs.
4. Validate the complete change and record the results. Website changes also need
   the [VitePress build](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/wiki/README.md).
5. Open a PR into **3.4.3**, explaining the problem, resulting behavior and verification.

Human and AI-assisted changes follow the same review requirements. The maintained
[contributor guide](https://github.com/alseif0x/rustycore/blob/3.4.3/CONTRIBUTING.md)
owns the detailed workflow and validation links.

## Understanding the port

Our goal is full behavior parity with the TrinityCore-derived 3.4.3 server.
For protocol, gameplay, persistence and runtime work, compare the complete operation
with the exact versioned reference functions. Relevant client/server evidence fills
gaps where the source is ambiguous.

Code being present, connected to production and verified against the reference
are distinct milestones. Record what your change actually proves.

- [Current state and evidence](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/migration/STATE.md)
- [Versioned references](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/README.md#versioned-behavioral-references)
- [Shared engineering guide](https://github.com/alseif0x/rustycore/blob/3.4.3/AGENTS.md)
- [Architecture and ownership](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/ownership-and-boundaries.md)
- [Module development](https://github.com/alseif0x/rustycore/blob/3.4.3/docs/architecture/modules.md)

## Reporting problems

Use the [issue forms](https://github.com/alseif0x/rustycore/issues/new/choose).
Include reproduction steps, expected and actual behavior, server commit and client
build. Share only sanitized logs; keep credentials, private configuration and
session data out of reports.
