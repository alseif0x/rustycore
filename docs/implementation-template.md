# Implementation evidence template

Use this optional outline in the owning issue or checkpoint. Record the branch, tested commit, date, operation and supported client build. State separately what has been implemented, connected to production and verified against the reference. [Operating guide](../AGENTS.md) · [Delivery plan](migration/PORT_PLAN.md)

## Reference behavior

Identify the reference revision, C++ paths and functions. Describe admission, state ownership, persistence and the observable order of effects. For packets, include direction, opcode, connection and layout. Identify the capture and client build when the reference leaves a question open.

## Rust implementation

Name the Rust entry point, its callers, state owner and affected consumers. Explain any deliberate departure from the reference and where it was approved. Identify dependencies that limit the operation.

## Verification and remaining work

Record the exact commands, results, tested configuration and host where relevant. Identify which production, client, capture or database scenarios were exercised. Link the remaining gaps to the issue that owns them.
