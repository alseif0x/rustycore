# Client

RustyCore targets the WoW Wrath of the Lich King Classic `3.4.3.54261` protocol. The
[maintained bot wrapper](https://github.com/alseif0x/rustycore/blob/3.4.3/tools/wow-test-bot/run_rustycore_login_smoke.sh)
defaults to build `54261` unless overridden. Older `51943` smoke records are historical
evidence, not a claim about the current client path. A default build number is not itself
acceptance evidence: record the actual client build, scenario and server commit for each run.

The project contains prior smoke coverage for Battle.net authentication, realm selection,
character enumeration, and initial world entry. Full gameplay parity is still under active
migration, so a successful login must not be interpreted as complete server support.

RustyCore does not distribute the game client or proprietary game data. Use a legally
obtained compatible client and extracted data appropriate for your environment.

See [Client setup](./setup) for the connection endpoints and current testing boundary.
