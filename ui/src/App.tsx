import { useCallback, useState } from 'react'
import { Link, Route, Switch, useRoute } from 'wouter'

import { CW_REALITY_ADDRESS, JUNO_CHAIN_ID } from './chain/juno'
import type { ConnectedWallet } from './chain/keplr'
import { WalletBar } from './components/WalletBar'
import { AskPage } from './pages/Ask'
import { FeedPage } from './pages/Feed'
import { MyActivityPage } from './pages/MyActivity'
import { QuestionDetailPage } from './pages/QuestionDetail'

export default function App() {
  const [wallet, setWallet] = useState<ConnectedWallet | null>(null)
  const onWallet = useCallback((w: ConnectedWallet | null) => setWallet(w), [])

  return (
    <div className="min-h-screen flex flex-col">
      <Header wallet={wallet} onWallet={onWallet} />
      <main className="flex-1 max-w-4xl mx-auto w-full px-6">
        <Switch>
          <Route path="/" component={FeedPage} />
          <Route path="/ask">{(_p) => <AskPage wallet={wallet} />}</Route>
          <Route path="/activity">{(_p) => <MyActivityPage wallet={wallet} />}</Route>
          <Route path="/q/:qid">
            {(params) => (
              <QuestionDetailPage qid={decodeURIComponent(params.qid)} wallet={wallet} />
            )}
          </Route>
          <Route>{() => <p className="my-12 text-muted">Not found.</p>}</Route>
        </Switch>
      </main>
      <Footer />
    </div>
  )
}

function Header({
  wallet,
  onWallet,
}: {
  wallet: ConnectedWallet | null
  onWallet: (w: ConnectedWallet | null) => void
}) {
  return (
    <header className="border-b border-ink/15">
      <div className="max-w-4xl mx-auto w-full px-6 py-5 flex items-center justify-between">
        <div className="flex items-center gap-8">
          <Link href="/">
            <a className="text-xl tracking-tight font-bold">
              juno<span className="text-accent">.reality</span>
            </a>
          </Link>
          <nav className="hidden md:flex gap-6 text-sm">
            <NavLink href="/">Feed</NavLink>
            <NavLink href="/ask">Ask</NavLink>
            <NavLink href="/activity">My activity</NavLink>
          </nav>
        </div>
        <WalletBar onConnect={onWallet} />
      </div>
      {!CW_REALITY_ADDRESS && (
        <div className="bg-accent text-paper text-[11px] uppercase tracking-wider px-6 py-1.5 text-center">
          pre-mainnet · {JUNO_CHAIN_ID} · cw-reality address not yet recorded
        </div>
      )}
      {wallet === null && CW_REALITY_ADDRESS && (
        <div className="bg-ink text-paper text-[11px] uppercase tracking-wider px-6 py-1.5 text-center">
          connect a wallet to ask or answer
        </div>
      )}
    </header>
  )
}

function NavLink({ href, children }: { href: string; children: React.ReactNode }) {
  const [active] = useRoute(href)
  return (
    <Link href={href}>
      <a
        className={`${
          active ? 'text-ink underline underline-offset-4' : 'text-muted hover:text-ink'
        }`}
      >
        {children}
      </a>
    </Link>
  )
}

function Footer() {
  return (
    <footer className="border-t border-ink/15 mt-12">
      <div className="max-w-4xl mx-auto w-full px-6 py-6 text-xs text-muted flex justify-between">
        <span>
          juno.reality — bond-escalating oracle on{' '}
          <span className="mono">{JUNO_CHAIN_ID}</span>
        </span>
        <span className="mono">AGPL-3.0</span>
      </div>
    </footer>
  )
}
