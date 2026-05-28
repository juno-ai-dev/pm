// Thin wrappers around CosmJS for cw-reality.

import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { GasPrice } from '@cosmjs/stargate'
import type { OfflineSigner } from '@cosmjs/proto-signing'

import { JUNO_RPC, JUNO_GAS_PRICE, CW_REALITY_ADDRESS } from '../chain/juno'
import type {
  BalanceResponse,
  ConfigResponse,
  FinalAnswerResponse,
  HistoryEntry,
  QuestionResponse,
  QuestionsListResponse,
  State,
} from './types'

let readClient: CosmWasmClient | null = null

export async function getReadClient(): Promise<CosmWasmClient> {
  if (!readClient) readClient = await CosmWasmClient.connect(JUNO_RPC)
  return readClient
}

function ensureContract(): string {
  if (!CW_REALITY_ADDRESS) {
    throw new Error('cw-reality not yet deployed on juno-1 — see /workspace/memory/onchain-log.md')
  }
  return CW_REALITY_ADDRESS
}

// ---- Queries ----

export async function queryConfig(): Promise<ConfigResponse> {
  const c = await getReadClient()
  return c.queryContractSmart(ensureContract(), { config: {} })
}

export async function queryQuestion(qid: string): Promise<QuestionResponse> {
  const c = await getReadClient()
  return c.queryContractSmart(ensureContract(), { question: { question_id: qid } })
}

export async function queryFinalAnswer(qid: string): Promise<FinalAnswerResponse> {
  const c = await getReadClient()
  return c.queryContractSmart(ensureContract(), { final_answer: { question_id: qid } })
}

export async function queryList(opts: {
  startAfter?: string
  limit?: number
  status?: State
} = {}): Promise<QuestionsListResponse> {
  const c = await getReadClient()
  return c.queryContractSmart(ensureContract(), {
    list: {
      start_after: opts.startAfter,
      limit: opts.limit ?? 30,
      status: opts.status,
    },
  })
}

export async function queryBalance(address: string, denom: string): Promise<BalanceResponse> {
  const c = await getReadClient()
  return c.queryContractSmart(ensureContract(), { balance: { address, denom } })
}

// ---- Signing client ----

export async function getSigningClient(signer: OfflineSigner): Promise<SigningCosmWasmClient> {
  return SigningCosmWasmClient.connectWithSigner(JUNO_RPC, signer, {
    gasPrice: GasPrice.fromString(JUNO_GAS_PRICE),
  })
}

// ---- Executes ----

interface ExecOpts {
  signer: OfflineSigner
  sender: string
  funds?: { denom: string; amount: string }[]
  memo?: string
}

async function exec(opts: ExecOpts, msg: object) {
  const client = await getSigningClient(opts.signer)
  return client.execute(opts.sender, ensureContract(), msg, 'auto', opts.memo, opts.funds)
}

export async function askQuestion(opts: ExecOpts, params: {
  text: string
  answer_type: 'bool' | 'uint' | 'string' | 'bytes'
  bond_denom: string
  initial_bond: string
  answer_timeout_secs: number
  arbitrator: string | null
  arbitration_timeout_secs: number | null
  opening_ts: number | null
  nonce: number
}) {
  return exec(opts, { ask_question: { ...params } })
}

export async function fundBounty(opts: ExecOpts, qid: string) {
  return exec(opts, { fund_bounty: { question_id: qid } })
}

export async function submitAnswer(opts: ExecOpts, params: {
  question_id: string
  answer: string
  current_bond_seen?: string
}) {
  return exec(opts, { submit_answer: { ...params, current_bond_seen: params.current_bond_seen ?? null } })
}

export async function disputeAnswer(opts: ExecOpts, params: {
  question_id: string
  new_answer: string
  current_bond_seen?: string
}) {
  return exec(opts, { dispute_answer: { ...params, current_bond_seen: params.current_bond_seen ?? null } })
}

export async function requestArbitration(opts: ExecOpts, qid: string) {
  return exec(opts, { request_arbitration: { question_id: qid, current_bond_seen: null } })
}

export async function cancelArbitration(opts: ExecOpts, qid: string) {
  return exec(opts, { cancel_arbitration: { question_id: qid } })
}

export async function submitArbitration(opts: ExecOpts, params: {
  question_id: string
  winning_answer: string
  payee: string
}) {
  return exec(opts, { submit_arbitration: { ...params } })
}

export async function claim(opts: ExecOpts, qid: string, entries: HistoryEntry[]) {
  return exec(opts, { claim: { question_id: qid, history_entries: entries } })
}

export async function withdraw(opts: ExecOpts, denom: string) {
  return exec(opts, { withdraw: { denom } })
}
