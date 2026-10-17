# Route: Codex

Applies only to a native Codex session. Defaults: `.codex/config.toml` and
`.codex/agents/luna-worker.toml`.

- **Parent: GPT-6.1 Sol** (`gpt-6.1-sol`) at `medium` (forever's default; raise only when the effective session confirms it). It owns architecture,
  decomposition, contracts, coordination, review, Git, integration and final acceptance;
  it does not implement routine units.
- **Worker: `luna_worker`**, GPT-6 Luna (`gpt-6-luna`) at `max`, for a bounded
  implementation, focused read-only exploration or the parent's exclusive final sequence.
  Custom TOML roles apply only on clients that load them; otherwise pass the supported
  model/effort and the role instructions explicitly.
- At most one worker at a time (`max_concurrent_threads_per_session = 1`). If Luna is
  unavailable, disclose it once and keep safe work in the parent; no paid-API fallback.
