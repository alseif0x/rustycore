# Client

RustyCore targets the WoW Wrath of the Lich King Classic `3.4.3.54261` protocol. The
currently exercised login path uses game build `51943`.

The project contains prior smoke coverage for Battle.net authentication, realm selection,
character enumeration, and initial world entry. Full gameplay parity is still under active
migration, so a successful login must not be interpreted as complete server support.

RustyCore does not distribute the game client or proprietary game data. Use a legally
obtained compatible client and extracted data appropriate for your environment.

See [Client setup](./setup) for the connection endpoints and current testing boundary.
