# Routes: OpenCode Go and DSH Web `go-mix`

Two related but different runtimes. Each applies only in its own session; neither reads
`.claude/`. Claude Code uses only [claude.md](claude.md).

## OpenCode Go

Source: `forever` b8330b1b5 (2026-10-17), adapted to 3.4.3. Defaults: `opencode.json`,
`.opencode/` (see `.opencode/README.md`). OpenCode injects `AGENTS.md` itself.

- `orchestrator` (primary, DeepSeek V4.1 Flash via OpenCode Go) coordinates, integrates,
  owns Git and final-validation scheduling; small inseparable edits only.
- `scout` (DeepSeek V4.1 Flash), `explore` (DeepSeek V4.1 Flash `#high`) and `general`
  (DeepSeek V4 Pro `#high`) locate, design and review; read-only.
- `builder` (GPT-6 Luna `#max`, OpenAI/Codex account) implements.
- The reviewer stays on a model family different from the builder. **Never Sol.**
- At most one child at a time. Step limits: scout 40, general 60, explore 80, builder 200.
- Quota fallback: no automatic provider fallback. When the Go quota is exhausted,
  `.opencode/model-route api` moves the DeepSeek agents to the direct DeepSeek API (paid
  balance) and `.opencode/model-route go` returns them; Luna is not changed.
- Hard-deny `policies` (force push, `reset --hard`, `clean -f`, stash drop/clear, `.env`
  and OpenCode API-key files) stay.

## DSH Web go-mix

Applies only when the session is DSH Web with the `go-mix` preset (installed in the
user's DSH profile, `~/.dsh/.agent-presets/go-mix`), and only if the user chose it. DSH
takes its models from that preset.

**Concurrency on this route:** at most one worker by default; parallel only on explicit
per-task user authorization, with disjoint write scopes. This replaces the WIP's
campaign-specific parallel and deferred-check authorizations, which are historical.

Where the verbatim text says `.claude/settings.json` or `.claude/agents/rustycore-worker.md`
select Sol/Luna for `ocx claude`, that is historical: Claude Code uses only
[claude.md](claude.md). The native Codex paragraph is superseded by [codex.md](codex.md).

The text below is preserved verbatim from `dsh-orquestacion-wip` (2026-09-28..10-06).

### Models (WIP, verbatim)


The user-selected **DSH Web Go Mix** (`go-mix`) uses OpenCode Go
**DeepSeek V4.1 Flash** (`opencode-go/deepseek-v4.1-flash`) at `high` effort as the
coordinator. Its installed request-recovery plugin switches the same root session
to **MiniMax M3** (`opencode-go/minimax-m3`) at `high` on a classified quota,
rate-limit, server, timeout or transport failure. It retries only the failed model
step: previously completed tools and worker changes are retained. It does not
change the global default on failure or switch back automatically. If MiniMax
also fails on a classified quota or availability error, the user-authorized last
resort is **DeepSeek API** (`deepseek-official/deepseek-v4-flash`) at `high`. This
root-coordinator fallback consumes the user's paid API balance; it is not part
of the Go subscription. No further provider fallback is configured after it.
Authentication, permissions, invalid
requests and context overflow are not fallback triggers. A user model selection
takes precedence. Both routes consume the same Go subscription, so a plan-wide
quota or outage may affect both.

This coordinator owns architecture, decomposition, contracts, task assignment,
review, Git, integration and acceptance scheduling; it does not implement.
Assign implementation through DSH `subagent` using `provider: openai-codex`,
`model: gpt-6-luna`, `reasoning_effort: max`. Give one bounded worker the settled
contract and owned paths. Use the implementation fallback below only when needed.
Use Go DeepSeek for bounded read-only exploration instead of consuming Codex for
routine searches. Sol `high` is a specific consultation option, not a mandatory
second coordinator or reviewer on every unit. New DSH sessions pick up the preset;
existing sessions retain their mounted generation.

The native entry points are separate:

The native Codex parent uses **GPT-6.1 Sol** (`gpt-6.1-sol`) at `high` effort. It owns architecture,
decomposition, contracts, coordination, review, Git, integration and final acceptance;
it does not implement. The implementing worker is **GPT-6 Luna** at max reasoning effort,
spawned natively with `model: gpt-6-luna` and `reasoning_effort: max`.
The local, ignored `.codex/config.toml` sets the parent model/effort and the
`[agents]` default subagent model/effort for this checkout.

For Claude-compatible sessions, use `ocx claude`. `.claude/settings.json` selects the
parent alias `ocx-claude-native--gpt-6.1-sol` at high effort, and
`.claude/agents/rustycore-worker.md` selects `ocx-claude-native--gpt-6-luna` at max
effort. These are defaults: start a fresh session after changing them and inspect the
effective session and successful worker spawn before reporting the model or effort that
actually ran. Never infer runtime use from settings alone.

The user-authorized implementation fallback is **Claude Opus 5.5**
(`anthropic/claude-opus-5-5`) at `low` effort when Luna cannot run because of exhausted
quota or provider/model unavailability. The parent owns this handoff: confirm that
the Luna worker has stopped before starting one Opus worker, inspect any partial
diff, and pass a bounded continuation with the same ownership and acceptance
contract. Do not restart the complete task or let both workers write the same unit.
For an ambiguous failure after dispatch, establish whether the worker is still
running before reassigning it. Report the actual route, effort and fallback reason.
Code/test failures, permissions, missing credentials and unresolved contracts are
not reasons to change providers. Apart from the Go Mix coordinator recovery above,
no other automatic provider fallback or paid API fallback is authorized by this
rule. Use the DSH model-selectable worker for an
explicit Opus effort; native bridges do not expose per-call model/effort selection.

An optional second opinion uses **Claude Opus 5.5** at `medium` effort for a concrete
uncertainty. It is separate from the implementing fallback and is not required for
every unit.


### Running the worker (WIP, verbatim; concurrency as scoped above)


Start one Luna worker by default. When the user authorizes parallel implementation,
use the available native slots for independent, disjoint write scopes; the current
refactor campaign has that authorization. No worker spawns children. Keep doing
useful parent work (settling consumers, preparing the next unit's contract, reviewing
the previous diff) without duplicating worker edits. Worker concurrency does not
authorize concurrent validation or shared target directories.


### Acceptance (WIP, verbatim; deferred-check instruction is historical)


Where authorized, implementation checks stay crate-scoped and sequential (`cargo
check -p`, focused `cargo test -p` with `CARGO_BUILD_JOBS=1`). An explicit whole-sweep
instruction defers these worker checks too: write tests and preserve coherent local
work, then execute the single completed-delivery acceptance campaign. The current
refactor checkpoint records that instruction; do not restart feedback checks at a
worker or helper boundary. Completed-delivery
acceptance follows AGENTS.md: the parent plans it once and may assign the worker as the
exclusive validation executor for the agreed non-live sequence; that assignment does not
include another QA campaign or autonomous repairs. The parent interprets findings and
assigns corrections. Authorized live DB/runtime QA stays with the parent. Reuse valid
evidence for unchanged inputs.
