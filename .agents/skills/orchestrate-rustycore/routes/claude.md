# Route: Claude Code / `ocx claude`

Applies only to a Claude Code session (including `ocx claude`). It is the current default
since 2026-10-10. Nothing from the other route files applies or is added here.

- **Parent: Claude Opus 5.5** (`claude-opus-5-5`), effort `low` from
  `.claude/settings.json`; raise it with `/effort medium` only for a hard architecture,
  lock-order or concurrency decision, then return to `low`. The parent owns
  architecture, decomposition, contracts, coordination, review, Git, integration and
  final acceptance. It does not implement: every code change goes to the worker.
- **Worker: `rustycore-worker`** (`.claude/agents/rustycore-worker.md`, Opus inherited
  from the parent). It is the only worker in this route.
- **No other-provider fallback.** If the worker cannot run, state that once and keep the
  parent's safe work moving (contracts, consumers, review); do not substitute another
  model or provider.

In Claude Code, start at most one worker at a time. Reasoning effort is a real runtime
setting: do not claim a model or effort the session configuration does not confirm.
Record the model that actually ran.
