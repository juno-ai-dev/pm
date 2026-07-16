# Juno PM

A prediction market on Juno Network.

## Contract workspace

The maintained Rust workspace is rooted at `contracts/Cargo.toml`. It contains
the existing `cw-reality` oracle, shared `pm-types`, and state-transition-free
package boundaries for the future `binary-market` and `market-factory`
contracts. The `depricated_ui/` tree is unsupported historical material, and
live-network scripts are quarantined under `scripts/unsafe/`.

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
