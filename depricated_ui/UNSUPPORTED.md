# Unsupported historical UI

This directory is retained only as historical reference. It is misspelled for
compatibility with existing history and is **not maintained, supported,
deployed, or part of validation**. It must not be presented as a production UI.

No package lifecycle scripts run in repository CI. If an investigator chooses
to inspect it locally, use the frozen lockfile and noninteractive pnpm defaults.
The workspace allowlist permits install scripts only for `esbuild` and
`protobufjs`; adding another build-script dependency requires review.

A maintained UI must be introduced in a separate reviewed change under a real
path, with tests and ownership, before UI build CI is enabled.
