import { createContext, useContext, useMemo, useState, type ReactNode } from 'react'

export type WalletState = 'disconnected' | 'wrong-network' | 'ready'
type WalletContextValue = { state: WalletState; setState: (state: WalletState) => void }
const WalletContext = createContext<WalletContextValue | null>(null)

export function WalletProvider({ children, initialState = 'disconnected' }: { children: ReactNode; initialState?: WalletState }) {
  const [state, setState] = useState<WalletState>(initialState)
  const value = useMemo(() => ({ state, setState }), [state])
  return <WalletContext.Provider value={value}>{children}</WalletContext.Provider>
}

export function useWallet() {
  const value = useContext(WalletContext)
  if (!value) throw new Error('useWallet must be used inside WalletProvider')
  return value
}
