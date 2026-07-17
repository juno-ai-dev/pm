use binary_market::{
    msg::{
        ConfigResponse as ChildConfig, IdentityResponse, InstantiateMsg as ChildInstantiateMsg,
        LifecycleStatus, QueryMsg as ChildQueryMsg, QuestionResponse, StateResponse,
    },
    question,
};
use cosmwasm_std::{
    entry_point, to_json_binary, Coin, Deps, DepsMut, Env, Event, MessageInfo, Order, Reply,
    ReplyOn, Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw_reality::{msg::QueryMsg as OracleQueryMsg, state::Config as OracleConfig};
use cw_storage_plus::Bound;
use cw_utils::parse_reply_instantiate_data;
use pm_types::{ProtocolVersion, UJUNO_DENOM};

use crate::{
    error::ContractError,
    msg::{
        ConfigResponse, CreateMarketMsg, ExecuteMsg, InstantiateMsg, ListMarketsResponse,
        MarketRecord, MarketResponse, QueryMsg,
    },
    state::{Config, PendingCreation, CONFIG, MARKETS, NEXT_ID, PENDING},
};

const CONTRACT_NAME: &str = "crates.io:market-factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CREATE_REPLY_ID: u64 = 1;
const DEFAULT_LIMIT: u32 = 30;
const MAX_LIMIT: u32 = 100;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
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
    // Ensure the child code id exists before accepting immutable configuration.
    deps.querier.query_wasm_code_info(msg.market_code_id)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(
        deps.storage,
        &Config {
            protocol_version: msg.protocol_version,
            market_code_id: msg.market_code_id,
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
    NEXT_ID.save(deps.storage, &1)?;
    Ok(Response::new().add_event(
        Event::new("juno_pm_factory_v1")
            .add_attribute("action", "factory_instantiated")
            .add_attribute("protocol_version", "1")
            .add_attribute("market_code_id", msg.market_code_id.to_string()),
    ))
}

fn validate_tier(msg: &InstantiateMsg) -> Result<(), ContractError> {
    let t = &msg.tier;
    if msg.protocol_version != ProtocolVersion::V1
        || msg.collateral_denom != UJUNO_DENOM
        || msg.market_code_id == 0
        || msg.oracle_code_id == 0
        || msg.oracle_checksum.is_empty()
        || t.min_initial_liquidity.is_zero()
        || t.min_initial_liquidity > t.max_initial_liquidity
        || t.max_initial_liquidity > t.collateral_cap
        || t.min_oracle_bounty < cosmwasm_std::Uint128::new(question::MIN_ORACLE_BOUNTY)
        || t.min_oracle_bounty > t.max_oracle_bounty
        || t.oracle_initial_bond < cosmwasm_std::Uint128::new(question::MIN_ORACLE_INITIAL_BOND)
        || t.answer_timeout_secs != question::ANSWER_TIMEOUT_SECS
        || t.arbitration_timeout_secs != question::ARBITRATION_TIMEOUT_SECS
        || t.fee_bps > 10_000
        || t.min_trade.is_zero()
        || t.max_trade_bps == 0
        || t.max_trade_bps > 2_500
        || t.challenge_bond.is_zero()
        || msg.oracle_min_initial_bond_floor > t.oracle_initial_bond
        || msg.oracle_min_answer_timeout_secs > t.answer_timeout_secs
    {
        return Err(ContractError::InvalidConfig(
            "tier violates accepted v1 bounds".into(),
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

fn exact_funds(funds: &[Coin], denom: &str, amount: cosmwasm_std::Uint128) -> bool {
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
        || request.oracle_bounty < config.tier.min_oracle_bounty
        || request.oracle_bounty > config.tier.max_oracle_bounty
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
    // Validate all typed metadata and timestamp bounds before instantiation. The
    // child reconstructs and validates again using its own final address.
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
    PENDING.save(
        deps.storage,
        &PendingCreation {
            creator: info.sender.clone(),
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
        nonce: request.nonce,
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
        collateral_cap: config.tier.collateral_cap,
        challenge_bond: config.tier.challenge_bond,
    };
    Ok(Response::new()
        .add_submessage(SubMsg {
            id: CREATE_REPLY_ID,
            msg: WasmMsg::Instantiate {
                admin: None,
                code_id: config.market_code_id,
                msg: to_json_binary(&child)?,
                funds: info.funds,
                label: format!("juno-pm-v1-{}", request.nonce),
            }
            .into(),
            gas_limit: None,
            reply_on: ReplyOn::Success,
        })
        .add_event(
            Event::new("juno_pm_factory_v1")
                .add_attribute("action", "market_creation_started")
                .add_attribute("creator", info.sender)
                .add_attribute("nonce", request.nonce.to_string()),
        ))
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

    // Reply data identifies only a candidate. Independently query code/admin and
    // every activation identity needed by the registry.
    let info = deps.querier.query_wasm_contract_info(&market)?;
    if info.code_id != config.market_code_id || info.admin.is_some() {
        return Err(ContractError::ChildVerification(
            "wrong child code id or child has an admin".into(),
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
        || child.collateral_cap != config.tier.collateral_cap
        || child.challenge_bond != config.tier.challenge_bond
        || question.question_id != identity.question_id
        || question.oracle != config.oracle
        || question.close_ts != pending.request.close_ts
        || question.opening_ts != pending.request.opening_ts
        || question.nonce != pending.request.nonce
    {
        return Err(ContractError::ChildVerification(
            "activated child identity/config does not match pending creation".into(),
        ));
    }
    let id = NEXT_ID.load(deps.storage)?;
    NEXT_ID.save(
        deps.storage,
        &id.checked_add(1)
            .ok_or_else(|| ContractError::InvalidConfig("registry id exhausted".into()))?,
    )?;
    let record = MarketRecord {
        id,
        market: market.to_string(),
        creator: pending.creator.to_string(),
        tier_id: config.tier_id,
        question_id: identity.question_id,
        question_hash: question.hash,
        close_ts: pending.request.close_ts,
        opening_ts: pending.request.opening_ts,
        initial_liquidity: pending.request.initial_liquidity,
        oracle_bounty: pending.request.oracle_bounty,
        created_height: env.block.height,
        created_time: env.block.time.seconds(),
    };
    MARKETS.save(deps.storage, id, &record)?;
    PENDING.remove(deps.storage);
    Ok(Response::new().add_event(
        Event::new("juno_pm_factory_v1")
            .add_attribute("action", "market_activated")
            .add_attribute("market_id", id.to_string())
            .add_attribute("market", market)
            .add_attribute("creator", record.creator),
    ))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<cosmwasm_std::Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&config_response(CONFIG.load(deps.storage)?)),
        QueryMsg::Market { id } => to_json_binary(&MarketResponse {
            market: MARKETS.load(deps.storage, id)?,
        }),
        QueryMsg::ListMarkets { start_after, limit } => {
            let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
            let markets = MARKETS
                .range(
                    deps.storage,
                    start_after.map(Bound::exclusive),
                    None,
                    Order::Ascending,
                )
                .take(limit)
                .map(|item| item.map(|(_, market)| market))
                .collect::<StdResult<Vec<_>>>()?;
            to_json_binary(&ListMarketsResponse { markets })
        }
    }
}

fn config_response(config: Config) -> ConfigResponse {
    ConfigResponse {
        protocol_version: config.protocol_version,
        market_code_id: config.market_code_id,
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
