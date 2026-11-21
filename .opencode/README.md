# OpenCode in RustyCore

Start `opencode` from the checkout. [opencode.json](../opencode.json) declares the project agents, models and permissions. The [OpenCode route](../.agents/skills/orchestrate-rustycore/routes/go-mix.md) describes how implementation and review are assigned. Existing sessions retain their selected agent and model.

Run `.opencode/model-route status` to inspect the configured provider. Use `.opencode/model-route go` for the Go route. An operator can explicitly select `.opencode/model-route api` to charge the direct API account. Restart a standalone instance or reconnect to the shared service as directed by the command. Provider credentials belong in the operator's local configuration.

The [environment plugin](plugins/project-environment.js) adds the Cargo binary directory to the shell path and respects explicit environment settings. By default it uses this checkout's `target`, one Cargo job, and per-user temporary and Python cache directories. For protobuf it looks for the version in [.protoc-version](../.protoc-version) under the local work directory, then for the operator's installed compiler. It sets optional reference and OpenSSL paths only when the corresponding files exist.
