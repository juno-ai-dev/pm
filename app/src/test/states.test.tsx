import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { App } from '../App'

const renderAt = (path: string, wallet: 'disconnected' | 'wrong-network' | 'ready' = 'disconnected') => render(<App initialPath={path} initialWalletState={wallet} />)

describe('explicit product states', () => {
  it.each([
    ['/markets?state=loading', 'Reading market fixtures'],
    ['/markets?state=empty', 'No markets in this fixture'],
    ['/markets?state=error', 'Market fixtures unavailable'],
  ])('renders %s', (path, heading) => {
    renderAt(path)
    expect(screen.getByRole('heading', { name: heading })).toBeInTheDocument()
  })

  it('resolves the wrong-network fixture state to uni-7', async () => {
    const user = userEvent.setup()
    renderAt('/portfolio', 'wrong-network')
    expect(screen.getAllByRole('heading', { name: 'Wrong network' })).not.toHaveLength(0)
    await user.click(screen.getByRole('button', { name: 'Switch to uni-7' }))
    expect(screen.getByRole('heading', { name: 'No fixture positions' })).toBeInTheDocument()
    expect(screen.getByText('Fixture wallet · uni-7')).toBeInTheDocument()
  })

  it('keeps create publishing disabled', () => {
    renderAt('/create', 'ready')
    expect(screen.getByRole('button', { name: /Create disabled/i })).toBeDisabled()
    expect(screen.getByText(/No transaction will be constructed or signed/i)).toBeInTheDocument()
  })
})
