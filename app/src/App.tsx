import { createContext, useContext, useEffect, useRef, useState, type MouseEvent, type ReactNode } from 'react'
import { Button, StatePanel } from './components/ui'
import { markets } from './fixtures/markets'
import { useWallet, WalletProvider } from './wallet'

const navigation = [
  { to: '/markets', label: 'Markets' },
  { to: '/create', label: 'Create' },
  { to: '/portfolio', label: 'Portfolio' },
]

type RouteContextValue = { url: string; navigate: (to: string) => void }
const RouteContext = createContext<RouteContextValue | null>(null)

function useRoute() {
  const route = useContext(RouteContext)
  if (!route) throw new Error('useRoute must be used inside the app router')
  return route
}

function Link({ to, children, className, label, current = false }: { to: string; children: ReactNode; className?: string; label?: string; current?: boolean }) {
  const { navigate } = useRoute()
  const follow = (event: MouseEvent<HTMLAnchorElement>) => {
    if (!event.defaultPrevented && event.button === 0 && !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey) {
      event.preventDefault()
      navigate(to)
    }
  }
  return <a href={to} className={className} aria-label={label} aria-current={current ? 'page' : undefined} onClick={follow}>{children}</a>
}

function WalletControl() {
  const { state, setState } = useWallet()
  if (state === 'wrong-network') {
    return (
      <div className="wallet-control wallet-control--wrong" role="status">
        <span>Wrong network</span>
        <Button onClick={() => setState('ready')}>Switch to uni-7</Button>
      </div>
    )
  }
  if (state === 'ready') {
    return (
      <div className="wallet-control" role="status">
        <span className="status-dot" aria-hidden="true" />
        <span>Fixture wallet · uni-7</span>
        <Button variant="ghost" onClick={() => setState('disconnected')}>Disconnect</Button>
      </div>
    )
  }
  return (
    <div className="wallet-control" role="status">
      <span>Wallet disconnected</span>
      <Button onClick={() => setState('ready')}>Use fixture wallet</Button>
      <Button variant="ghost" onClick={() => setState('wrong-network')}>Preview wrong network</Button>
    </div>
  )
}

function Shell() {
  const { url } = useRoute()
  const path = url.split('?')[0]
  const previousPath = useRef(path)
  useEffect(() => {
    if (previousPath.current !== path) {
      document.getElementById('main-content')?.focus()
      previousPath.current = path
    }
  }, [path])
  let route: ReactNode
  if (path === '/markets') route = <Markets />
  else if (path.startsWith('/markets/')) route = <MarketDetail address={decodeURIComponent(path.slice('/markets/'.length))} />
  else if (path === '/create') route = <Create />
  else if (path === '/portfolio') route = <Portfolio />
  else route = <NotFound />
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">Skip to content</a>
      <header className="site-header">
        <Link className="brand" to="/markets" label="Juno Predict home">
          <img src="/assets/juno-logo-salmon.svg" alt="" width="30" height="30" />
          <img src="/assets/juno-wordmark-light.svg" alt="Juno" width="100" height="30" />
          <span>Predict</span>
        </Link>
        <div className="environment" aria-label="Demo environment">
          <span>Demo environment</span><strong>uni-7</strong>
        </div>
        <WalletControl />
      </header>
      <nav className="primary-nav" aria-label="Primary navigation">
        {navigation.map((item) => {
          const current = path === item.to || (item.to === '/markets' && path.startsWith('/markets/'))
          return <Link key={item.to} to={item.to} current={current} className={current ? 'active' : undefined}>{item.label}</Link>
        })}
      </nav>
      <main id="main-content" tabIndex={-1}>{route}</main>
      <footer>
        <span>Fixture-backed interface · no signing · no live addresses</span>
        <span>Juno testnet · uni-7</span>
      </footer>
    </div>
  )
}

function PageHeader({ eyebrow, title, copy }: { eyebrow: string; title: string; copy: string }) {
  return <header className="page-header"><p className="eyebrow">{eyebrow}</p><h1>{title}</h1><p>{copy}</p></header>
}

function Markets() {
  const { url } = useRoute()
  const params = new URLSearchParams(url.split('?')[1] ?? '')
  const state = params.get('state')
  return (
    <div className="page">
      <PageHeader eyebrow="Collective signal · uni-7" title="Markets" copy="Binary demo markets. Fixture thin-pool quotes only." />
      <aside className="state-preview" aria-label="Fixture state previews">
        <span>Preview</span>
        <Link to="/markets">Ready</Link>
        <Link to="/markets?state=loading">Loading</Link>
        <Link to="/markets?state=empty">Empty</Link>
        <Link to="/markets?state=error">Error</Link>
      </aside>
      {state === 'loading' ? <StatePanel kind="loading" title="Reading market fixtures"><p>Holding the frame while local demo data resolves.</p></StatePanel>
        : state === 'error' ? <StatePanel kind="error" title="Market fixtures unavailable" action={<Link className="button button--secondary" to="/markets">Retry fixture</Link>}><p>The local fixture reader returned an error. No network request was made.</p></StatePanel>
        : state === 'empty' ? <StatePanel kind="empty" title="No markets in this fixture"><p>Create flow is available for review; publishing is disabled.</p></StatePanel>
        : <section className="market-grid" aria-label="Demo markets">{markets.map((market) => <article className="market-card" key={market.reference}>
          <div className="market-card__meta"><span>{market.category}</span><span>{market.closes}</span></div>
          <h2><Link to={`/markets/${market.reference}`}>{market.question}</Link></h2>
          <div className="probabilities" aria-label={`Thin-pool quote: Yes ${market.yes} percent, No ${market.no} percent`}>
            <div><span>Yes quote</span><strong>{market.yes}%</strong></div><div><span>No quote</span><strong>{market.no}%</strong></div>
          </div>
          <p className="liquidity">Liquidity · {market.liquidity}</p>
        </article>)}</section>}
    </div>
  )
}

function MarketDetail({ address }: { address: string }) {
  const market = markets.find((item) => item.reference === address)
  const { state } = useWallet()
  if (!market) return <div className="page"><PageHeader eyebrow="Address route" title="Market not found" copy="The supplied reference is not present in local fixtures." /><StatePanel kind="error" title="Unknown fixture reference"><p>No live address lookup was attempted.</p></StatePanel></div>
  return <div className="page detail"><PageHeader eyebrow={`Fixture reference · ${market.reference}`} title={market.question} copy="Read-only market detail. This identifier is not a chain address." />
    <section className="technical-card"><h2>Market signal</h2><dl><div><dt>Network</dt><dd>uni-7</dd></div><div><dt>Close</dt><dd>{market.closes}</dd></div><div><dt>Liquidity</dt><dd>{market.liquidity}</dd></div></dl><p className="eyebrow">Fixture thin-pool quote · not a probability</p><div className="probabilities" aria-label={`Thin-pool quote: Yes ${market.yes} percent, No ${market.no} percent`}><div><span>Yes quote</span><strong>{market.yes}%</strong></div><div><span>No quote</span><strong>{market.no}%</strong></div></div></section>
    {state === 'disconnected' && <StatePanel kind="wallet" title="Wallet disconnected"><p>Use the fixture wallet in the shell to inspect connected states. No extension or signer is called.</p></StatePanel>}
    {state === 'wrong-network' && <StatePanel kind="error" title="Wrong network"><p>This demo is fixed to Juno testnet uni-7.</p></StatePanel>}
    {state === 'ready' && <StatePanel kind="wallet" title="Trading disabled in demo"><p>Wallet state is simulated. Orders and signatures are outside this foundation.</p></StatePanel>}
  </div>
}

function Create() {
  return <div className="page"><PageHeader eyebrow="Draft node · local only" title="Create market" copy="Compose a fixture draft. Publishing and signing are not implemented." />
    <form className="technical-card form" onSubmit={(event) => event.preventDefault()}><label>Question<input name="question" placeholder="Will…?" /></label><label>Resolution source<textarea name="source" rows={4} placeholder="Describe a verifiable source" /></label><div className="form-row"><label>Close epoch<input name="close" inputMode="numeric" placeholder="Demo epoch" /></label><label>Initial liquidity<input name="liquidity" inputMode="decimal" placeholder="Demo JUNO" /></label></div><Button disabled>Create disabled · demo only</Button><p className="form-note">No transaction will be constructed or signed.</p></form>
  </div>
}

function Portfolio() {
  const { state } = useWallet()
  return <div className="page"><PageHeader eyebrow="Position ledger · uni-7" title="Portfolio" copy="Wallet-specific positions appear here in an integrated release." />
    {state === 'disconnected' ? <StatePanel kind="wallet" title="Connect a wallet state"><p>The maintained shell starts disconnected. Use the fixture control above; no wallet extension is opened.</p></StatePanel>
      : state === 'wrong-network' ? <StatePanel kind="error" title="Wrong network"><p>Portfolio data is scoped to uni-7. Switch the simulated network to continue.</p></StatePanel>
      : <StatePanel kind="empty" title="No fixture positions"><p>The fixture wallet has no positions. Nothing live was queried.</p></StatePanel>}
  </div>
}

function NotFound() {
  return <div className="page"><PageHeader eyebrow="Route error" title="Page not found" copy="Return to the fixture market index." /><Link className="button button--primary" to="/markets">Open markets</Link></div>
}

export function App({ initialWalletState = 'disconnected', initialPath }: { initialWalletState?: 'disconnected' | 'wrong-network' | 'ready'; initialPath?: string }) {
  const browserPath = typeof window === 'undefined' ? '/markets' : `${window.location.pathname}${window.location.search}`
  const [url, setUrl] = useState(initialPath ?? (browserPath === '/' ? '/markets' : browserPath))
  useEffect(() => {
    if (initialPath) return
    const onPopState = () => setUrl(`${window.location.pathname}${window.location.search}`)
    window.addEventListener('popstate', onPopState)
    return () => window.removeEventListener('popstate', onPopState)
  }, [initialPath])
  const navigate = (to: string) => {
    setUrl(to)
    if (!initialPath) window.history.pushState({}, '', to)
  }
  return <RouteContext.Provider value={{ url, navigate }}><WalletProvider initialState={initialWalletState}><Shell /></WalletProvider></RouteContext.Provider>
}
