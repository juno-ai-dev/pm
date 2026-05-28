import { useEffect, useState } from 'react'

import { connect, type ConnectedWallet, type WalletKind } from '../chain/keplr'
import { fmtAddress } from '../format'

export function WalletBar({ onConnect }: { onConnect: (w: ConnectedWallet | null) => void }) {
  const [wallet, setWallet] = useState<ConnectedWallet | null>(null)
  const [err, setErr] = useState<string | null>(null)

  useEffect(() => {
    onConnect(wallet)
  }, [wallet, onConnect])

  async function tryConnect(kind: WalletKind) {
    setErr(null)
    try {
      const w = await connect(kind)
      setWallet(w)
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e))
    }
  }

  if (wallet) {
    return (
      <div className="flex items-center gap-3 text-sm">
        <span className="text-muted">{wallet.kind === 'keplr' ? 'Keplr' : 'Leap'}</span>
        <span className="mono">{fmtAddress(wallet.address)}</span>
        <button className="btn-ghost text-xs" onClick={() => setWallet(null)}>
          DISCONNECT
        </button>
      </div>
    )
  }

  return (
    <div className="flex items-center gap-3 text-sm">
      <button className="btn-primary text-xs" onClick={() => tryConnect('keplr')}>
        CONNECT KEPLR
      </button>
      <button className="btn-ghost text-xs" onClick={() => tryConnect('leap')}>
        CONNECT LEAP
      </button>
      {err && <span className="text-accent text-xs mono">{err}</span>}
    </div>
  )
}
