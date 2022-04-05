use std::str::FromStr;

use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Reply, Response, StdError, StdResult, SubMsg, SubMsgExecutionResponse, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use astroport::asset::{Asset, AssetInfo};
use astroport::router::SwapOperation;

use steak::helpers::unwrap_reply;
use steak::zapper::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg};

use crate::helpers::{new_cw20, new_native_from_funds};
use crate::state::State;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State::default();

    state.steak_hub.save(deps.storage, &deps.api.addr_validate(&msg.steak_hub)?)?;
    state.steak_token.save(deps.storage, &deps.api.addr_validate(&msg.steak_token)?)?;
    state.astro_router.save(deps.storage, &deps.api.addr_validate(&msg.astro_router)?)?;

    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => receive(deps, info, cw20_msg),
        ExecuteMsg::Zap {
            minimum_received,
        } => zap(
            deps,
            new_native_from_funds(&info.funds)?,
            info.sender,
            minimum_received,
        ),
    }
}

fn receive(deps: DepsMut, info: MessageInfo, cw20_msg: Cw20ReceiveMsg) -> StdResult<Response> {
    let api = deps.api;
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::Zap {
            minimum_received,
        } => zap(
            deps,
            new_cw20(info.sender, cw20_msg.amount),
            api.addr_validate(&cw20_msg.sender)?,
            minimum_received,
        ),
    }
}

fn zap(
    deps: DepsMut,
    asset: Asset,
    receiver: Addr,
    minimum_received: Option<Uint128>,
) -> StdResult<Response> {
    let state = State::default();
    let steak_hub = state.steak_hub.load(deps.storage)?;
    let astro_router = state.astro_router.load(deps.storage)?;

    state.receiver.save(deps.storage, &receiver)?;
    state.minimum_recieved.save(deps.storage, &minimum_received.unwrap_or_else(Uint128::zero))?;

    // If the asset is Luna, we skip the swap and jump to bonding directly; otherwise, we swap it
    // into Luna first on Astroport. We assume there is a direct pair consisting of the asset and Luna
    let submsg: SubMsg = match &asset.info {
        AssetInfo::NativeToken {
            denom,
        } => {
            if denom == "uluna" {
                SubMsg::reply_on_success(
                    CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: steak_hub.into(),
                        msg: to_binary(&steak::hub::ExecuteMsg::Bond {
                            receiver: Some(receiver.into()),
                        })?,
                        funds: vec![Coin::new(asset.amount.u128(), "uluna")],
                    }),
                    2,
                )
            } else {
                SubMsg::reply_on_success(
                    CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: astro_router.into(),
                        msg: to_binary(&astroport::router::ExecuteMsg::ExecuteSwapOperations {
                            operations: vec![SwapOperation::AstroSwap {
                                offer_asset_info: asset.info.clone(),
                                ask_asset_info: AssetInfo::NativeToken {
                                    denom: "uluna".to_string(),
                                },
                            }],
                            minimum_receive: None,
                            to: None,
                        })?,
                        funds: vec![Coin::new(asset.amount.u128(), denom)],
                    }),
                    1,
                )
            }
        },
        AssetInfo::Token {
            contract_addr,
        } => SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.into(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: astro_router.into(),
                    amount: asset.amount,
                    msg: to_binary(&astroport::router::Cw20HookMsg::ExecuteSwapOperations {
                        operations: vec![SwapOperation::AstroSwap {
                            offer_asset_info: asset.info,
                            ask_asset_info: AssetInfo::NativeToken {
                                denom: "uluna".to_string(),
                            },
                        }],
                        minimum_receive: None,
                        to: None,
                    })?,
                })?,
                funds: vec![],
            }),
            1,
        ),
    };

    Ok(Response::new()
        .add_submessage(submsg)
        .add_attribute("action", "steakzap/zap"))
}

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        1 => after_swap(deps, unwrap_reply(reply)?),
        2 => after_bond(deps, unwrap_reply(reply)?),
        id => Err(StdError::generic_err(format!("invalid reply id: {}; must be 1-2", id))),
    }
}

fn after_swap(deps: DepsMut, response: SubMsgExecutionResponse) -> StdResult<Response> {
    let event = response
        .events
        .iter()
        .find(|event| event.ty == "from_contract")
        .ok_or_else(|| StdError::generic_err("cannot find `from_contract` event"))?;

    let return_amount_str = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "return_amount")
        .ok_or_else(|| StdError::generic_err("cannot find `return_amount` attribute"))?
        .value;

    // NOTE: Technically we need to subtract tax amount to find the actual received amount, but in
    // reality the tax rate for Luna has always been zero, so we simply assume this is the case.
    let return_amount = Uint128::from_str(return_amount_str)?;

    let state = State::default();
    let steak_hub = state.steak_hub.load(deps.storage)?;
    let receiver = state.receiver.load(deps.storage)?;

    Ok(Response::new()
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: steak_hub.into(),
                msg: to_binary(&steak::hub::ExecuteMsg::Bond {
                    receiver: Some(receiver.into()),
                })?,
                funds: vec![Coin::new(return_amount.u128(), "uluna")],
            }),
            2,
        ))
        .add_attribute("action", "steakzap/after_swap"))
}

fn after_bond(deps: DepsMut, response: SubMsgExecutionResponse) -> StdResult<Response> {
    let event = response
        .events
        .iter()
        .find(|event| event.ty == "steakhub/bonded")
        .ok_or_else(|| StdError::generic_err("cannot find `steakhub/bonded` event"))?;

    let receive_amount_str = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "usteak_minted")
        .ok_or_else(|| StdError::generic_err("cannot find `usteak_minted` attribute"))?
        .value;

    let receive_amount = Uint128::from_str(receive_amount_str)?;

    let state = State::default();
    let minimum_received = state.minimum_recieved.load(deps.storage)?;
    if receive_amount < minimum_received {
        return Err(StdError::generic_err(
            format!("too little received; expecting at least {}, received {}", minimum_received, receive_amount)
        ));
    }

    state.receiver.remove(deps.storage);
    state.minimum_recieved.remove(deps.storage);

    Ok(Response::new().add_attribute("action", "steakhub/after_bond"))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();
    Ok(ConfigResponse {
        steak_hub: state.steak_hub.load(deps.storage)?.into(),
        steak_token: state.steak_token.load(deps.storage)?.into(),
        astro_router: state.astro_router.load(deps.storage)?.into(),
    })
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::new()) // do nothing
}
