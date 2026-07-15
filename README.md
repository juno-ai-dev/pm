# Juno PM

A prediction market on Juno Network.

## Repository baseline

The only maintained executable package is `contracts/cw-reality`. The
`depricated_ui/` tree is unsupported historical material, and live-network
scripts are quarantined under `scripts/unsafe/`.

Run the core local gate from the repository root:

```sh
make validate
```

The command uses the pinned Rust toolchain and lockfile and checks formatting,
strict clippy, all contract tests, generated-schema drift, and UI policy. See
`CONTRIBUTING.md`, `SECURITY.md`, and `LICENSES.md` before contributing. No
repository-wide license has yet been selected.

GitHub CI additionally performs the Wasm sanity build, negative schema fixture,
link, shell, secret, and dependency/license scans. Those checks use pinned
tools and are authoritative when their external scanners are unavailable
locally.

Historical research notes may preserve absolute paths as source citations;
they are not active setup instructions.
