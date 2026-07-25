# Juno PM

A prediction market on Juno Network.

## Maintained frontend

The fixture-backed Juno app lives in [`app/`](app/README.md). It provides the
responsive Markets, Create, Portfolio, and market-reference routes for the
`uni-7` demo environment. Run its independent validation with `npm ci`,
`npm run lint`, `npm run typecheck`, `npm test`, and `npm run build` from `app/`. The
`depricated_ui/` tree remains unsupported historical material.

## Contract workspace

The maintained Rust workspace is rooted at `contracts/Cargo.toml`. It contains
the existing `cw-reality` oracle, shared `pm-types`, and state-transition-free
package boundaries for the future `binary-market` and `market-factory`
contracts. Live-network scripts are quarantined under `scripts/unsafe/`.

Run the core local gate from the repository root:

```sh
./scripts/validate.sh
```

The command uses the pinned Rust toolchain and lockfile and checks formatting,
strict clippy, all workspace tests, every contract package's Wasm sanity build,
generated-schema drift, and UI policy. See `CONTRIBUTING.md`, `SECURITY.md`, and
`LICENSES.md` before contributing. No repository-wide license has yet been
selected.

GitHub CI additionally performs the Wasm sanity build, negative schema fixture,
link, shell, secret, and dependency/license scans. Those checks use pinned
tools and are authoritative when their external scanners are unavailable
locally.

Historical research notes may preserve absolute paths as source citations;
they are not active setup instructions.
