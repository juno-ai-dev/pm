export type Market = {
  reference: string
  question: string
  category: string
  closes: string
  yes: number
  no: number
  liquidity: string
}

// Deliberately local fixture references: none are chain addresses or live data.
export const markets: Market[] = [
  {
    reference: 'fixture-market-001',
    question: 'Will Juno governance proposal 400 pass?',
    category: 'Governance',
    closes: 'Demo epoch 18',
    yes: 64,
    no: 36,
    liquidity: '12,500 demo JUNOX',
  },
  {
    reference: 'fixture-market-002',
    question: 'Will Juno ship its next major network upgrade before the demo checkpoint?',
    category: 'Network',
    closes: 'Demo epoch 22',
    yes: 41,
    no: 59,
    liquidity: '8,200 demo JUNOX',
  },
]
