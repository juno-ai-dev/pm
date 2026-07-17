use binary_market::{
    msg::{
        ConfigResponse as ChildConfig, IdentityResponse, InstantiateMsg as ChildInstantiateMsg,
        LifecycleStatus, QueryMsg as ChildQueryMsg, QuestionResponse, StateResponse,
    },
    question,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Coin, Deps, DepsMut, Env, Event, MessageInfo, Order, Reply,
    ReplyOn, Response, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw_reality::{msg::QueryMsg as OracleQueryMsg, state::Config as OracleConfig};
use cw_storage_plus::Bound;
use cw_utils::parse_reply_instantiate_data;
use pm_types::{ProtocolVersion, TierId, UJUNO_DENOM};

use crate::{
    error::ContractError,
    msg::{
        ConfigResponse, CreateMarketMsg, ExecuteMsg, InstantiateMsg, ListMarketsResponse,
        MarketRecord, MarketResponse, NextNonceResponse, QueryMsg,
    },
    state::{Config, PendingCreation, CONFIG, MARKETS, NEXT_NONCE, PENDING},
};

const CONTRACT_NAME: &str = "crates.io:market-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CREATE_REPLY_ID: u64 = 1;
const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 100;
pub const V1_VERDICT_AUTHORITY: &str =
    "juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    if !info.funds.is_empty() {
        return Err(ContractError::InvalidFunds);
    }
    validate_tier(&msg)?;
    let oracle = deps.api.addr_validate(&msg.oracle)?;
    let verdict_authority = deps.api.addr_validate(&msg.verdict_authority)?;
    let oracle_info = deps.querier.query_wasm_contract_info(&oracle)?;
    if oracle_info.code_id != msg.oracle_code_id || oracle_info.admin.is_some() {
        return Err(ContractError::InvalidConfig(
            "oracle must have the pinned code id and no chain admin".into(),
        ));
    }
    let code_info = deps.querier.query_wasm_code_info(msg.oracle_code_id)?;
    if code_info.checksum != msg.oracle_checksum {
        return Err(ContractError::InvalidConfig(
            "oracle checksum mismatch".into(),
        ));
    }
    let oracle_config: OracleConfig = deps
        .querier
        .query_wasm_smart(&oracle, &OracleQueryMsg::Config {})?;
    if oracle_config.admin.is_some()
        || oracle_config.min_initial_bond_floor != msg.oracle_min_initial_bond_floor
        || oracle_config.min_answer_timeout_secs != msg.oracle_min_answer_timeout_secs
    {
        return Err(ContractError::InvalidConfig(
            "oracle smart config does not match the immutable profile".into(),
        ));
    }
    let market_code = deps.querier.query_wasm_code_info(msg.market_code_id)?;
    if market_code.checksum != msg.market_checksum {
        return Err(ContractError::InvalidConfig(
            "market checksum mismatch".into(),
        ));
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(
        deps.storage,
        &Config {
            protocol_version: msg.protocol_version,
            market_code_id: msg.market_code_id,
            market_checksum: msg.market_checksum,
            tier_id: msg.tier_id,
            tier: msg.tier,
            oracle,
            oracle_code_id: msg.oracle_code_id,
            oracle_checksum: msg.oracle_checksum,
            verdict_authority,
            collateral_denom: msg.collateral_denom,
            oracle_min_initial_bond_floor: msg.oracle_min_initial_bond_floor,
            oracle_min_answer_timeout_secs: msg.oracle_min_answer_timeout_secs,
        },
    )?;
    NEXT_NONCE.save(deps.storage, &0)?;
    Ok(Response::new())
}

fn validate_tier(msg: &InstantiateMsg) -> Result<(), ContractError> {
    let t = &msg.tier;
    if msg.protocol_version != ProtocolVersion::V1
        || msg.collateral_denom != UJUNO_DENOM
        || msg.verdict_authority != V1_VERDICT_AUTHORITY
        || msg.market_code_id == 0
        || msg.oracle_code_id == 0
        || msg.oracle_checksum.is_empty()
        || msg.market_checksum.is_empty()
        || msg.tier_id != TierId(1)
        || t.min_initial_liquidity != Uint128::new(100_000_000)
        || t.max_initial_liquidity != Uint128::new(200_000_000)
        || t.collateral_cap != Uint128::new(200_000_000)
        || t.min_oracle_bounty != Uint128::new(1_000_000)
        || t.max_oracle_bounty != Uint128::new(1_000_000)
        || t.oracle_initial_bond != Uint128::new(10_000_000)
        || t.answer_timeout_secs != question::ANSWER_TIMEOUT_SECS
        || t.arbitration_timeout_secs != question::ARBITRATION_TIMEOUT_SECS
        || t.fee_bps != 200
        || t.min_trade != Uint128::new(10_000)
        || t.max_trade_bps != 2_500
        || t.max_position_per_side != Uint128::new(20_000_000)
        || t.challenge_bond != Uint128::new(10_000_000)
        || msg.oracle_min_initial_bond_floor != Uint128::new(10_000_000)
        || msg.oracle_min_answer_timeout_secs != question::ANSWER_TIMEOUT_SECS
    {
        return Err(ContractError::InvalidConfig(
            "tier does not exactly match the accepted v1 canary profile".into(),
        ));
    }
    Ok(())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateMarket(request) => create_market(deps, env, info, request),
    }
}

fn exact_funds(funds: &[Coin], denom: &str, amount: Uint128) -> bool {
    funds.len() == 1 && funds[0].denom == denom && funds[0].amount == amount
}

fn create_market(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    request: CreateMarketMsg,
) -> Result<Response, ContractError> {
    if PENDING.may_load(deps.storage)?.is_some() {
        return Err(ContractError::CreationPending);
    }
    let config = CONFIG.load(deps.storage)?;
    if request.initial_liquidity < config.tier.min_initial_liquidity
        || request.initial_liquidity > config.tier.max_initial_liquidity
        || request.initial_liquidity > config.tier.collateral_cap
        || request.initial_liquidity.u128() % 2 != 0
        || request.oracle_bounty != config.tier.min_oracle_bounty
    {
        return Err(ContractError::InvalidConfig(
            "principal or bounty is outside the immutable tier".into(),
        ));
    }
    let required = request
        .initial_liquidity
        .checked_add(request.oracle_bounty)?;
    if !exact_funds(&info.funds, &config.collateral_denom, required) {
        return Err(ContractError::InvalidFunds);
    }
    question::canonical_question(
        &request.question,
        &env.contract.address,
        &config.oracle,
        &config.verdict_authority,
        request.close_ts,
        request.opening_ts,
        config.tier.oracle_initial_bond,
        env.block.time.seconds(),
    )
    .map_err(|err| ContractError::InvalidConfig(err.to_string()))?;

    let nonce = NEXT_NONCE.load(deps.storage)?;
    let next_nonce = nonce
        .checked_add(1)
        .ok_or_else(|| ContractError::InvalidConfig("factory nonce exhausted".into()))?;
    NEXT_NONCE.save(deps.storage, &next_nonce)?;
    PENDING.save(
        deps.storage,
        &PendingCreation {
            creator: info.sender.clone(),
            nonce,
            request: request.clone(),
        },
    )?;
    let child = ChildInstantiateMsg {
        factory: env.contract.address.to_string(),
        creator: info.sender.to_string(),
        oracle: config.oracle.to_string(),
        verdict_authority: config.verdict_authority.to_string(),
        tier: config.tier_id.clone(),
        question: request.question,
        nonce,
        close_ts: request.close_ts,
        opening_ts: request.opening_ts,
        initial_liquidity: request.initial_liquidity,
        oracle_bounty: request.oracle_bounty,
        oracle_initial_bond: config.tier.oracle_initial_bond,
        answer_timeout_secs: config.tier.answer_timeout_secs,
        arbitration_timeout_secs: config.tier.arbitration_timeout_secs,
        fee_bps: config.tier.fee_bps,
        min_trade: config.tier.min_trade,
        max_trade_bps: config.tier.max_trade_bps,
        max_position_per_side: config.tier.max_position_per_side,
        collateral_cap: config.tier.collateral_cap,
        challenge_bond: config.tier.challenge_bond,
    };
    Ok(Response::new().add_submessage(SubMsg {
        id: CREATE_REPLY_ID,
        msg: WasmMsg::Instantiate {
            admin: None,
            code_id: config.market_code_id,
            msg: to_json_binary(&child)?,
            funds: info.funds,
            label: format!("juno-pm-v1-{nonce}"),
        }
        .into(),
        gas_limit: None,
        reply_on: ReplyOn::Success,
    }))
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    if reply.id != CREATE_REPLY_ID {
        return Err(ContractError::UnknownReplyId(reply.id));
    }
    let pending = PENDING.load(deps.storage)?;
    reply
        .result
        .clone()
        .into_result()
        .map_err(ContractError::InstantiateFailed)?;
    let parsed = parse_reply_instantiate_data(reply)
        .map_err(|err| ContractError::ChildVerification(err.to_string()))?;
    let market = deps.api.addr_validate(&parsed.contract_address)?;
    let config = CONFIG.load(deps.storage)?;
    let info = deps.querier.query_wasm_contract_info(&market)?;
    if info.code_id != config.market_code_id || info.admin.is_some() {
        return Err(ContractError::ChildVerification(
            "wrong child code id or child has an admin".into(),
        ));
    }
    let child_code = deps.querier.query_wasm_code_info(info.code_id)?;
    if child_code.checksum != config.market_checksum {
        return Err(ContractError::ChildVerification(
            "wrong child checksum".into(),
        ));
    }
    let identity: IdentityResponse = deps
        .querier
        .query_wasm_smart(&market, &ChildQueryMsg::Identity {})?;
    let state: StateResponse = deps
        .querier
        .query_wasm_smart(&market, &ChildQueryMsg::State {})?;
    let child: ChildConfig = deps
        .querier
        .query_wasm_smart(&market, &ChildQueryMsg::Config {})?;
    let question: QuestionResponse = deps
        .querier
        .query_wasm_smart(&market, &ChildQueryMsg::Question {})?;
    if identity.protocol_version != config.protocol_version
        || identity.factory != env.contract.address
        || identity.market != market
        || identity.nonce != pending.nonce
        || identity.question_id.is_none()
        || !state.activated
        || state.status == LifecycleStatus::Initializing
        || child.protocol_version != config.protocol_version
        || child.factory != env.contract.address
        || child.creator != pending.creator
        || child.initial_lp != pending.creator
        || child.oracle != config.oracle
        || child.verdict_authority != config.verdict_authority
        || child.tier != config.tier_id
        || child.collateral_denom != config.collateral_denom
        || child.close_ts != pending.request.close_ts
        || child.opening_ts != pending.request.opening_ts
        || child.initial_liquidity != pending.request.initial_liquidity
        || child.oracle_bounty != pending.request.oracle_bounty
        || child.oracle_initial_bond != config.tier.oracle_initial_bond
        || child.answer_timeout_secs != config.tier.answer_timeout_secs
        || child.arbitration_timeout_secs != config.tier.arbitration_timeout_secs
        || child.fee_bps != config.tier.fee_bps
        || child.min_trade != config.tier.min_trade
        || child.max_trade_bps != config.tier.max_trade_bps
        || child.max_position_per_side != config.tier.max_position_per_side
        || child.collateral_cap != config.tier.collateral_cap
        || child.challenge_bond != config.tier.challenge_bond
        || question.question_id != identity.question_id
        || question.oracle != config.oracle
        || question.close_ts != pending.request.close_ts
        || question.opening_ts != pending.request.opening_ts
        || question.nonce != pending.nonce
    {
        return Err(ContractError::ChildVerification(
            "activated child identity/config does not match pending creation".into(),
        ));
    }
    let question_id = identity.question_id.ok_or_else(|| {
        ContractError::ChildVerification("activated child has no question id".into())
    })?;
    let record = MarketRecord {
        nonce: pending.nonce,
        market: market.to_string(),
        creator: pending.creator.to_string(),
        tier_id: config.tier_id,
        question_id,
        question_hash: question.hash,
        close_ts: pending.request.close_ts,
        opening_ts: pending.request.opening_ts,
        initial_liquidity: pending.request.initial_liquidity,
        oracle_bounty: pending.request.oracle_bounty,
        created_height: env.block.height,
        created_time: env.block.time.seconds(),
    };
    MARKETS.save(deps.storage, pending.nonce, &record)?;
    PENDING.remove(deps.storage);
    let common = |event: Event| {
        event
            .add_attribute("protocol_version", "1")
            .add_attribute("factory", env.contract.address.to_string())
            .add_attribute("market", market.to_string())
            .add_attribute("height", env.block.height.to_string())
            .add_attribute("block_time", env.block.time.seconds().to_string())
    };
    Ok(Response::new()
        .add_event(
            common(Event::new("juno_pm_v1"))
                .add_attribute("action", "market_created")
                .add_attribute("creator", record.creator.clone())
                .add_attribute("nonce", record.nonce.to_string())
                .add_attribute("initial_principal", record.initial_liquidity.to_string())
                .add_attribute("oracle_bounty", record.oracle_bounty.to_string()),
        )
        .add_event(
            common(Event::new("juno_pm_v1"))
                .add_attribute("action", "market_activated")
                .add_attribute("creator", record.creator)
                .add_attribute("lp", pending.creator)
                .add_attribute("question_id", record.question_id.to_base64())
                .add_attribute("question_hash", record.question_hash.to_base64())
                .add_attribute("close_ts", record.close_ts.to_string())
                .add_attribute("opening_ts", record.opening_ts.to_string()),
        ))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<cosmwasm_std::Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&config_response(CONFIG.load(deps.storage)?)),
        QueryMsg::Market { nonce } => to_json_binary(&MarketResponse {
            market: MARKETS.load(deps.storage, nonce)?,
        }),
        QueryMsg::NextNonce {} => to_json_binary(&NextNonceResponse {
            next_nonce: NEXT_NONCE.load(deps.storage)?,
        }),
        QueryMsg::ListMarkets {
            start_after_nonce,
            limit,
        } => {
            let requested = limit.unwrap_or(DEFAULT_LIMIT);
            if requested == 0 {
                return Err(cosmwasm_std::StdError::generic_err(
                    "limit must be greater than zero",
                ));
            }
            let limit = requested.min(MAX_LIMIT) as usize;
            let mut markets = MARKETS
                .range(
                    deps.storage,
                    start_after_nonce.map(Bound::exclusive),
                    None,
                    Order::Ascending,
                )
                .take(limit + 1)
                .map(|item| item.map(|(_, market)| market))
                .collect::<StdResult<Vec<_>>>()?;
            let has_more = markets.len() > limit;
            markets.truncate(limit);
            let next_start_after_nonce =
                has_more.then(|| markets.last().expect("nonzero limit").nonce);
            to_json_binary(&ListMarketsResponse {
                markets,
                next_start_after_nonce,
            })
        }
    }
}

fn config_response(config: Config) -> ConfigResponse {
    ConfigResponse {
        protocol_version: config.protocol_version,
        market_code_id: config.market_code_id,
        market_checksum: config.market_checksum,
        tier_id: config.tier_id,
        tier: config.tier,
        oracle: config.oracle.to_string(),
        oracle_code_id: config.oracle_code_id,
        oracle_checksum: config.oracle_checksum,
        verdict_authority: config.verdict_authority.to_string(),
        collateral_denom: config.collateral_denom,
        oracle_min_initial_bond_floor: config.oracle_min_initial_bond_floor,
        oracle_min_answer_timeout_secs: config.oracle_min_answer_timeout_secs,
    }
}
