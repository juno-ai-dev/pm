# Maintained prediction-market app

`app/` is the maintained, fixture-backed frontend shell. It is independent of
historical `depricated_ui/` and never calls a wallet extension, signs a message,
or treats fixture references as live chain addresses.

## Run locally

```sh
cd app
npm ci
npm run dev
```

Vite serves the responsive Markets, Create, Portfolio, and
`/markets/:address` routes. The committed fixtures use explicit non-address
references. Use the shell controls to preview disconnected, wrong-network, and
`uni-7` fixture-wallet states. The Markets preview links expose ready, loading,
empty, and error states.

## Validate

```sh
npm ci
npm run lint
npm run typecheck
npm test
npm run test:a11y
npm run build
```

The Juno logo assets and centralized tokens are derived from
`juno-ai-dev/juno-design-system`; its MIT notice is retained in
`DESIGN_SYSTEM_LICENSE`. Montserrat and Space Mono are self-hosted by the build
through Fontsource packages.
