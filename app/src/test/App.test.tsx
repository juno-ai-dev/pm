import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { App } from '../App'

function renderAt(path = '/markets', wallet: 'disconnected' | 'wrong-network' | 'ready' = 'disconnected') {
  return render(<App initialPath={path} initialWalletState={wallet} />)
}

describe('maintained app shell', () => {
  it('identifies the demo and uni-7 network and exposes primary navigation', () => {
    renderAt()
    expect(screen.getByLabelText('Demo environment')).toHaveTextContent('uni-7')
    const nav = screen.getByRole('navigation', { name: 'Primary navigation' })
    expect(nav).toHaveTextContent('Markets')
    expect(nav).toHaveTextContent('Create')
    expect(nav).toHaveTextContent('Portfolio')
    expect(screen.getByText(/no signing · no live addresses/i)).toBeInTheDocument()
  })

  it('navigates with keyboard and retains a visible focus target', async () => {
    const user = userEvent.setup()
    renderAt()
    await user.tab()
    expect(screen.getByText('Skip to content')).toHaveFocus()
    await user.tab()
    expect(screen.getByRole('link', { name: 'Juno Predict home' })).toHaveFocus()
    const create = screen.getByRole('link', { name: 'Create' })
    create.focus()
    expect(create).toHaveFocus()
    await user.keyboard('{Enter}')
    expect(screen.getByRole('heading', { level: 1, name: 'Create market' })).toBeInTheDocument()
  })

  it('renders the address-based fixture route without presenting a live address', () => {
    renderAt('/markets/fixture-market-001')
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Juno governance')
    expect(screen.getByText(/not a chain address/i)).toBeInTheDocument()
    expect(screen.getByRole('heading', { name: 'Wallet disconnected' })).toBeInTheDocument()
  })
})
