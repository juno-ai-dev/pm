import { render } from '@testing-library/react'
import axe from 'axe-core'
import { describe, expect, it } from 'vitest'
import { App } from '../App'

async function expectAccessible(path: string, wallet: 'disconnected' | 'wrong-network' | 'ready' = 'disconnected') {
  const { container } = render(<App initialPath={path} initialWalletState={wallet} />)
  const results = await axe.run(container, { rules: { 'color-contrast': { enabled: false } } })
  expect(results.violations, results.violations.map((item) => `${item.id}: ${item.help}`).join('\n')).toEqual([])
}

describe('accessibility smoke', () => {
  it('has no automated violations in the market shell', async () => expectAccessible('/markets'))
  it('has no automated violations in empty state', async () => expectAccessible('/markets?state=empty'))
  it('has no automated violations in error state', async () => expectAccessible('/markets?state=error'))
  it('has no automated violations in wrong-network state', async () => expectAccessible('/portfolio', 'wrong-network'))
})
